use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::os::raw::c_int;
use std::ptr;

use libcdio_sys::{
    cdio_cddap_close, cdio_cddap_identify, cdio_cddap_open, cdio_cddap_track_audiop,
    cdio_cddap_track_firstsector, cdio_cddap_track_lastsector, cdio_cddap_tracks,
    cdio_free_device_list, cdio_fs_t_CDIO_FS_AUDIO, cdio_get_devices_with_cap, cdio_paranoia_free,
    cdio_paranoia_init, cdio_paranoia_modeset, cdio_paranoia_read, cdio_paranoia_seek,
    cdio_track_enums_CDIO_INVALID_TRACK, cdrom_drive_t, cdrom_paranoia_t,
    paranoia_cdda_enums_t_CDDA_MESSAGE_FORGETIT, paranoia_mode_t_PARANOIA_MODE_FULL,
    CDIO_CD_FRAMESIZE_RAW,
};

use super::Track;

// A CD sector carries this many bytes of audio, which as 16-bit samples across
// both channels is what one paranoia read hands back.
pub const SAMPLES_PER_SECTOR: usize = CDIO_CD_FRAMESIZE_RAW as usize / size_of::<i16>();

/// The devices that currently hold an audio CD.
pub fn holding_an_audio_disc() -> Vec<String> {
    let mut devices = Vec::new();

    // SAFETY: a null search list asks libcdio to scan for itself, and the
    // result is a null-terminated array of C strings owned by libcdio until it
    // is handed back below.
    unsafe {
        let list = cdio_get_devices_with_cap(
            ptr::null_mut(),
            cdio_fs_t_CDIO_FS_AUDIO as c_int,
            // Every capability asked for has to hold rather than any of them.
            // Only one is asked for, so this only settles how an empty drive
            // is treated: it is not a match.
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

/// An opened drive with an audio CD in it.
pub struct Drive {
    handle: *mut cdrom_drive_t,
}

impl Drive {
    pub fn open(device: &str) -> Result<Self, String> {
        let device = CString::new(device)
            .map_err(|_| "a device path cannot contain a zero byte".to_owned())?;

        // SAFETY: the path stays alive for the call, and libcdio is told to
        // discard its messages rather than to fill in a buffer, so the third
        // argument is unused.
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

        let drive = Self { handle };

        // SAFETY: the handle came back non-null from identify above, and the
        // wrapper now owns it, so a failure here still gets closed on drop.
        if unsafe { cdio_cddap_open(drive.handle) } != 0 {
            return Err(format!(
                "{} could not be opened for reading",
                device.to_string_lossy()
            ));
        }

        Ok(drive)
    }

    /// The audio tracks on the disc, in the order they sit on it.
    pub fn audio_tracks(&self) -> Vec<Track> {
        // SAFETY: the handle is open for as long as this wrapper lives, which
        // is what every call below needs.
        let count = unsafe { cdio_cddap_tracks(self.handle) };

        // A drive that will not say how many tracks there are answers with the
        // value that stands for no track at all, which is above anything a CD
        // can hold and would otherwise be walked through as if it were a count.
        if u32::from(count) == cdio_track_enums_CDIO_INVALID_TRACK {
            return Vec::new();
        }

        (1..=count)
            .filter(|number| unsafe { cdio_cddap_track_audiop(self.handle, *number) } == 1)
            .filter_map(|number| {
                let first = unsafe { cdio_cddap_track_firstsector(self.handle, number) };
                let last = unsafe { cdio_cddap_track_lastsector(self.handle, number) };

                // A track whose extent the drive will not give up is left out
                // rather than offered as something that cannot be read.
                (first >= 0 && last >= first).then(|| Track {
                    number,
                    sectors: last.abs_diff(first) + 1,
                })
            })
            .collect()
    }

    /// Reads one track, handing each sector's samples over as they arrive.
    pub fn read_track(&self, number: u8, mut receive: impl FnMut(&[i16])) -> Result<(), String> {
        // SAFETY: as above, the handle is open for the life of the wrapper.
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
        // SAFETY: the handle is the one identify returned, closed exactly once
        // because nothing else can reach it.
        unsafe { cdio_cddap_close(self.handle) };
    }
}

/// The reader that re-reads and overlaps until the samples agree.
///
/// It reads through the drive it was built from, so it borrows it: the drive
/// cannot be closed while a reader is still working on it.
struct Paranoia<'drive> {
    handle: *mut cdrom_paranoia_t,
    drive: PhantomData<&'drive Drive>,
}

impl<'drive> Paranoia<'drive> {
    fn init(drive: &'drive Drive) -> Result<Self, String> {
        // SAFETY: the borrow above is what keeps the drive open for as long as
        // this reader can be used.
        let handle = unsafe { cdio_paranoia_init(drive.handle) };

        if handle.is_null() {
            return Err("the drive could not be set up for careful reading".to_owned());
        }

        // Everything the library can do, including never giving up on a sector
        // it cannot agree with itself about. A read that would have been
        // silently filled in is the one outcome this app exists to avoid.
        unsafe { cdio_paranoia_modeset(handle, paranoia_mode_t_PARANOIA_MODE_FULL as c_int) };

        Ok(Self {
            handle,
            drive: PhantomData,
        })
    }

    fn seek(&mut self, sector: i32) -> Result<(), String> {
        // SAFETY: the handle came back non-null from init.
        if unsafe { cdio_paranoia_seek(self.handle, sector, libc::SEEK_SET) } < 0 {
            return Err(format!("the drive could not be moved to sector {sector}"));
        }

        Ok(())
    }

    /// The samples of the sector the reader is on, which it then leaves behind.
    ///
    /// The buffer belongs to the reader and the next read overwrites it, which
    /// is why the returned slice borrows for only as long as the reader is
    /// left alone.
    fn read(&mut self, sector: i32) -> Result<&[i16], String> {
        // SAFETY: no callback is offered, so the library reports progress to
        // nobody and the read is an ordinary blocking call.
        let samples = unsafe { cdio_paranoia_read(self.handle, None) };

        if samples.is_null() {
            return Err(format!("sector {sector} could not be read"));
        }

        // SAFETY: a successful read fills a whole sector's worth of samples in
        // a buffer the reader owns, and the borrow above keeps the reader
        // still for as long as the slice is held.
        Ok(unsafe { std::slice::from_raw_parts(samples, SAMPLES_PER_SECTOR) })
    }
}

impl Drop for Paranoia<'_> {
    fn drop(&mut self) {
        // SAFETY: the handle is the one init returned, freed exactly once, and
        // before the drive it was built from is closed.
        unsafe { cdio_paranoia_free(self.handle) };
    }
}
