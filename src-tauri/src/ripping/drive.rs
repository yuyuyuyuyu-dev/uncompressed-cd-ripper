use std::ffi::{CStr, CString};
use std::fmt;
use std::marker::PhantomData;
use std::os::raw::c_int;
use std::ptr;

use super::Disc;
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

pub const SAMPLES_PER_SECTOR: usize = CDIO_CD_FRAMESIZE_RAW as usize / size_of::<i16>();

// A frame is one moment of sound, a sample of it on each of the two channels.
// Sectors are what a disc is addressed in; frames are what a drive's read
// offset is measured in, and what a checksum counts.
const CHANNELS: usize = 2;
const FRAMES_PER_SECTOR: usize = SAMPLES_PER_SECTOR / CHANNELS;

// What stands in where there is no audio on the disc to read. Only ever the
// frames past either edge of it, which a drive with a read offset reaches for
// and nothing else does. AccurateRip's own list holds no offset further out
// than the five sectors at each edge that a checksum leaves out, so what
// stands in here reaches no checksum.
const NOTHING_RECORDED: [i16; SAMPLES_PER_SECTOR] = [0; SAMPLES_PER_SECTOR];

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
                vendor: named(CStr::from_ptr(reported.psz_vendor.as_ptr())),
                model: named(CStr::from_ptr(reported.psz_model.as_ptr())),
            })
        }
    }
}

// The two fields a drive answers with, which is all AccurateRip's list of read
// offsets is keyed by. What this app keeps a read offset under as well: the
// offset belongs to the drive rather than to whichever device path the
// operating system handed out this time.
pub struct Hardware {
    pub vendor: String,
    pub model: String,
}

// One line for a person to read and for a setting to be filed under. A drive
// that names no maker leaves no gap at the front.
impl fmt::Display for Hardware {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.vendor.is_empty() {
            write!(out, "{} ", self.vendor)?;
        }

        write!(out, "{}", self.model)
    }
}

// A field comes back padded out to a fixed width with spaces, which are the
// drive's rather than part of the name.
fn named(field: &CStr) -> String {
    field.to_string_lossy().trim().to_owned()
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

    fn read_track<R: FnMut(&[i16])>(
        &self,
        number: u8,
        offset: i32,
        mut receive: R,
    ) -> Result<(), String> {
        let first = unsafe { cdio_cddap_track_firstsector(self.handle, number) };
        let last = unsafe { cdio_cddap_track_lastsector(self.handle, number) };

        if first < 0 || last < first {
            return Err(format!("the drive will not say where track {number} is"));
        }

        // Where the track's first frame really sits. A drive with a read
        // offset hands over what is a little further along than what was
        // asked for, so what the track begins with is found a little further
        // along than the disc says.
        let begins_at = first as i64 * FRAMES_PER_SECTOR as i64 + i64::from(offset);
        let mut sector = begins_at.div_euclid(FRAMES_PER_SECTOR as i64) as i32;
        // How much of that first sector belongs to whatever comes before the
        // track. A read offset is hardly ever a whole number of sectors.
        let mut before = begins_at.rem_euclid(FRAMES_PER_SECTOR as i64) as usize * CHANNELS;

        let sectors = last.abs_diff(first) as usize + 1;
        // The audio on the disc, from wherever it starts to wherever it ends,
        // which a read offset can reach past at either end. Not the track's
        // own sectors: reading into the track next door is how a shifted read
        // gets the frames it is short of.
        let recorded = unsafe {
            cdio_cddap_disc_firstsector(self.handle)..=cdio_cddap_disc_lastsector(self.handle)
        };

        let mut paranoia = Paranoia::init(self)?;
        // Where the reader stands, so that a seek happens where the reading
        // jumps rather than before every sector.
        let mut standing = None;
        // What has been read and not yet handed on, which is under two sectors
        // because a sector is handed on as soon as one is there.
        let mut held: Vec<i16> = Vec::with_capacity(2 * SAMPLES_PER_SECTOR);
        let mut handed = 0;

        while handed < sectors {
            if recorded.contains(&sector) {
                if standing != Some(sector) {
                    paranoia.seek(sector)?;
                }

                held.extend_from_slice(paranoia.read(sector)?);
                standing = Some(sector + 1);
            } else {
                held.extend_from_slice(&NOTHING_RECORDED);
            }

            sector += 1;

            let dropped = before.min(held.len());
            held.drain(..dropped);
            before -= dropped;

            // A sector at a time, aligned to the track rather than to the
            // disc, so that what arrives here is what the track holds however
            // far the reading was shifted.
            while handed < sectors && held.len() >= SAMPLES_PER_SECTOR {
                receive(&held[..SAMPLES_PER_SECTOR]);
                held.drain(..SAMPLES_PER_SECTOR);
                handed += 1;
            }
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
