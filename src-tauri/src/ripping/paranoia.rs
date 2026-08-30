use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::ops::RangeInclusive;
use std::os::raw::c_int;
use std::ptr;

use libcdio_sys::{
    cdio_cddap_close, cdio_cddap_disc_firstsector, cdio_cddap_disc_lastsector, cdio_cddap_identify,
    cdio_cddap_open, cdio_cddap_track_audiop, cdio_cddap_track_firstsector,
    cdio_cddap_track_lastsector, cdio_cddap_tracks, cdio_free_device_list, cdio_fs_t_CDIO_FS_AUDIO,
    cdio_get_devices_with_cap, cdio_get_hwinfo, cdio_hwinfo_t, cdio_paranoia_free,
    cdio_paranoia_init, cdio_paranoia_modeset, cdio_paranoia_read, cdio_paranoia_seek,
    cdio_track_enums_CDIO_INVALID_TRACK, cdrom_drive_t, cdrom_paranoia_t,
    paranoia_cdda_enums_t_CDDA_MESSAGE_FORGETIT, paranoia_mode_t_PARANOIA_MODE_FULL,
    CDIO_CD_FRAMESIZE_RAW,
};

use super::drive::{named, Hardware, ReportedTrack, SAMPLES_PER_SECTOR};

const _: () = assert!(SAMPLES_PER_SECTOR == CDIO_CD_FRAMESIZE_RAW as usize / size_of::<i16>());

pub fn holding_an_audio_disc() -> Vec<String> {
    let mut devices = Vec::new();

    // SAFETY: a null search list asks libcdio to scan for itself, and the list
    // it returns is owned by libcdio until it is handed back below.
    unsafe {
        let list = cdio_get_devices_with_cap(
            ptr::null_mut(),
            cdio_fs_t_CDIO_FS_AUDIO as c_int,
            // All of the capabilities asked for rather than any, so an empty
            // drive is not a match.
            false,
        );

        if list.is_null() {
            return devices;
        }

        let mut entry = list;
        while !(*entry).is_null() {
            devices.push(CStr::from_ptr(*entry).to_string_lossy().into_owned());
            entry = entry.add(1);
        }

        cdio_free_device_list(list);
    }

    devices
}

pub struct Drive {
    handle: *mut cdrom_drive_t,
}

impl Drive {
    pub fn open(device: &str) -> Result<Self, String> {
        let device = CString::new(device)
            .map_err(|_| "a device path cannot contain a zero byte".to_owned())?;

        // SAFETY: libcdio is told to discard its messages, so the buffer the
        // third argument would name is never written to.
        let handle = unsafe {
            cdio_cddap_identify(
                device.as_ptr(),
                paranoia_cdda_enums_t_CDDA_MESSAGE_FORGETIT as c_int,
                ptr::null_mut(),
            )
        };

        if handle.is_null() {
            return Err(format!(
                "{} is not a drive with an audio CD in it",
                device.to_string_lossy()
            ));
        }

        // Wrapped before the call that can fail, so a failure still closes it.
        let drive = Self { handle };

        if unsafe { cdio_cddap_open(drive.handle) } != 0 {
            return Err(format!(
                "{} could not be opened for reading",
                device.to_string_lossy()
            ));
        }

        Ok(drive)
    }

    // What the drive says it is. AccurateRip keeps a read offset for every
    // drive it has been told about, under the maker and the model exactly as
    // the drive answers with them.
    pub fn hardware(&self) -> Result<Hardware, String> {
        let mut reported = cdio_hwinfo_t {
            psz_vendor: [0; 9],
            psz_model: [0; 17],
            psz_revision: [0; 5],
        };

        // SAFETY: the handle was opened above and carries the device this asks
        // about, the struct is filled in place, and libcdio closes each field
        // it fills with a zero byte inside the array it was given.
        unsafe {
            if !cdio_get_hwinfo((*self.handle).p_cdio, &mut reported) {
                return Err("the drive will not say what it is".to_owned());
            }

            Ok(Hardware {
                vendor: named(&CStr::from_ptr(reported.psz_vendor.as_ptr()).to_string_lossy()),
                model: named(&CStr::from_ptr(reported.psz_model.as_ptr()).to_string_lossy()),
            })
        }
    }

    pub(super) fn tracks(&self) -> Result<Vec<ReportedTrack>, String> {
        let count = unsafe { cdio_cddap_tracks(self.handle) };

        // A drive that will not say how many tracks there are answers with the
        // value standing for no track, which counting would walk straight into.
        if u32::from(count) == cdio_track_enums_CDIO_INVALID_TRACK {
            return Err("the drive will not say what is on the disc".to_owned());
        }

        Ok((1..=count)
            .map(|number| ReportedTrack {
                number,
                audio: unsafe { cdio_cddap_track_audiop(self.handle, number) } == 1,
                first: unsafe { cdio_cddap_track_firstsector(self.handle, number) },
                last: unsafe { cdio_cddap_track_lastsector(self.handle, number) },
            })
            .collect())
    }

    pub(super) fn recorded(&self) -> Result<RangeInclusive<i32>, String> {
        Ok(unsafe {
            cdio_cddap_disc_firstsector(self.handle)..=cdio_cddap_disc_lastsector(self.handle)
        })
    }

    pub(super) fn reading(&self) -> Result<Reading<'_>, String> {
        let handle = unsafe { cdio_paranoia_init(self.handle) };

        if handle.is_null() {
            return Err("the drive could not be set up for careful reading".to_owned());
        }

        // Everything the library can do, including never giving up on a sector
        // it cannot agree with itself about. Being quietly handed a filled-in
        // read is the outcome this app exists to avoid.
        unsafe { cdio_paranoia_modeset(handle, paranoia_mode_t_PARANOIA_MODE_FULL as c_int) };

        Ok(Reading {
            handle,
            standing: None,
            drive: PhantomData,
        })
    }
}

impl Drop for Drive {
    fn drop(&mut self) {
        unsafe { cdio_cddap_close(self.handle) };
    }
}

pub(super) struct Reading<'drive> {
    handle: *mut cdrom_paranoia_t,
    // Where the reader stands, so that a seek happens where the reading jumps
    // rather than before every sector.
    standing: Option<i32>,
    // Borrowed so that the drive cannot be closed while this still reads it.
    drive: PhantomData<&'drive Drive>,
}

impl Reading<'_> {
    // Takes &mut self because the next read overwrites the buffer returned here.
    pub(super) fn read(&mut self, sector: i32) -> Result<&[i16], String> {
        if self.standing != Some(sector)
            && unsafe { cdio_paranoia_seek(self.handle, sector, libc::SEEK_SET) } < 0
        {
            return Err(format!("the drive could not be moved to sector {sector}"));
        }

        let samples = unsafe { cdio_paranoia_read(self.handle, None) };

        if samples.is_null() {
            return Err(format!("sector {sector} could not be read"));
        }

        self.standing = Some(sector + 1);

        // SAFETY: a successful read fills a whole sector in a buffer the reader
        // owns, and the borrow above holds it still while the slice lives.
        Ok(unsafe { std::slice::from_raw_parts(samples, SAMPLES_PER_SECTOR) })
    }
}

impl Drop for Reading<'_> {
    fn drop(&mut self) {
        unsafe { cdio_paranoia_free(self.handle) };
    }
}
