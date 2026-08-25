use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::os::raw::c_int;
use std::ptr;

use super::Disc;
use libcdio_sys::{
    cdio_cddap_close, cdio_cddap_identify, cdio_cddap_open, cdio_cddap_track_audiop,
    cdio_cddap_track_firstsector, cdio_cddap_track_lastsector, cdio_cddap_tracks,
    cdio_free_device_list, cdio_fs_t_CDIO_FS_AUDIO, cdio_get_devices_with_cap, cdio_paranoia_free,
    cdio_paranoia_init, cdio_paranoia_modeset, cdio_paranoia_read, cdio_paranoia_seek,
    cdio_track_enums_CDIO_INVALID_TRACK, cdrom_drive_t, cdrom_paranoia_t,
    paranoia_cdda_enums_t_CDDA_MESSAGE_FORGETIT, paranoia_mode_t_PARANOIA_MODE_FULL,
    CDIO_CD_FRAMESIZE_RAW,
};

pub const SAMPLES_PER_SECTOR: usize = CDIO_CD_FRAMESIZE_RAW as usize / size_of::<i16>();

// Everything libcdio will say about a track, and the last of it that needs a
// drive. What a listing or a table of contents is made of it is arithmetic,
// and arithmetic is kept where a test can reach it without a disc.
//
// The sectors are the ones libcdio counts, which begin at the first track
// rather than at the lead-in before it.
pub struct ReportedTrack {
    pub number: u8,
    pub audio: bool,
    pub first: i32,
    pub last: i32,
}

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
}

impl Disc for Drive {
    fn reported_tracks(&self) -> Result<Vec<ReportedTrack>, String> {
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

    fn read_track<R: FnMut(&[i16])>(&self, number: u8, mut receive: R) -> Result<(), String> {
        let first = unsafe { cdio_cddap_track_firstsector(self.handle, number) };
        let last = unsafe { cdio_cddap_track_lastsector(self.handle, number) };

        if first < 0 || last < first {
            return Err(format!("the drive will not say where track {number} is"));
        }

        let mut paranoia = Paranoia::init(self)?;
        paranoia.seek(first)?;

        for sector in first..=last {
            receive(paranoia.read(sector)?);
        }

        Ok(())
    }
}

impl Drop for Drive {
    fn drop(&mut self) {
        unsafe { cdio_cddap_close(self.handle) };
    }
}

struct Paranoia<'drive> {
    handle: *mut cdrom_paranoia_t,
    // Borrowed so that the drive cannot be closed while this still reads it.
    drive: PhantomData<&'drive Drive>,
}

impl<'drive> Paranoia<'drive> {
    fn init(drive: &'drive Drive) -> Result<Self, String> {
        let handle = unsafe { cdio_paranoia_init(drive.handle) };

        if handle.is_null() {
            return Err("the drive could not be set up for careful reading".to_owned());
        }

        // Everything the library can do, including never giving up on a sector
        // it cannot agree with itself about. Being quietly handed a filled-in
        // read is the outcome this app exists to avoid.
        unsafe { cdio_paranoia_modeset(handle, paranoia_mode_t_PARANOIA_MODE_FULL as c_int) };

        Ok(Self {
            handle,
            drive: PhantomData,
        })
    }

    fn seek(&mut self, sector: i32) -> Result<(), String> {
        if unsafe { cdio_paranoia_seek(self.handle, sector, libc::SEEK_SET) } < 0 {
            return Err(format!("the drive could not be moved to sector {sector}"));
        }

        Ok(())
    }

    // Takes &mut self because the next read overwrites the buffer returned here.
    fn read(&mut self, sector: i32) -> Result<&[i16], String> {
        let samples = unsafe { cdio_paranoia_read(self.handle, None) };

        if samples.is_null() {
            return Err(format!("sector {sector} could not be read"));
        }

        // SAFETY: a successful read fills a whole sector in a buffer the reader
        // owns, and the borrow above holds it still while the slice lives.
        Ok(unsafe { std::slice::from_raw_parts(samples, SAMPLES_PER_SECTOR) })
    }
}

impl Drop for Paranoia<'_> {
    fn drop(&mut self) {
        unsafe { cdio_paranoia_free(self.handle) };
    }
}
