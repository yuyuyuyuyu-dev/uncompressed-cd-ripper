use std::fmt;

use super::Disc;

#[cfg(not(windows))]
pub use super::paranoia::{holding_an_audio_disc, Drive};
#[cfg(windows)]
pub use super::win32::{holding_an_audio_disc, Drive};

pub(super) const BYTES_PER_SECTOR: usize = 2352;

pub const SAMPLES_PER_SECTOR: usize = BYTES_PER_SECTOR / size_of::<i16>();

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

// Everything the drive will say about a track, and the last of it that needs a
// drive. What a listing or a table of contents is made of it is arithmetic,
// and arithmetic is kept where a test can reach it without a disc.
//
// The sectors are the ones a drive counts, which begin at the first track
// rather than at the lead-in before it.
pub struct ReportedTrack {
    pub number: u8,
    pub audio: bool,
    pub first: i32,
    pub last: i32,
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
pub(super) fn named(field: &str) -> String {
    field.trim().to_owned()
}

impl Disc for Drive {
    fn reported_tracks(&self) -> Result<Vec<ReportedTrack>, String> {
        self.tracks()
    }

    fn read_track<R: FnMut(&[i16])>(
        &self,
        number: u8,
        offset: i32,
        mut receive: R,
    ) -> Result<(), String> {
        let tracks = self.tracks()?;
        let track = tracks
            .iter()
            .find(|track| track.number == number)
            .filter(|track| track.first >= 0 && track.last >= track.first)
            .ok_or_else(|| format!("the drive will not say where track {number} is"))?;

        // Where the track's first frame really sits. A drive with a read
        // offset hands over what is a little further along than what was
        // asked for, so what the track begins with is found a little further
        // along than the disc says.
        let begins_at = track.first as i64 * FRAMES_PER_SECTOR as i64 + i64::from(offset);
        let mut sector = begins_at.div_euclid(FRAMES_PER_SECTOR as i64) as i32;
        // How much of that first sector belongs to whatever comes before the
        // track. A read offset is hardly ever a whole number of sectors.
        let mut before = begins_at.rem_euclid(FRAMES_PER_SECTOR as i64) as usize * CHANNELS;

        let sectors = track.last.abs_diff(track.first) as usize + 1;
        // The audio on the disc, from wherever it starts to wherever it ends,
        // which a read offset can reach past at either end. Not the track's
        // own sectors: reading into the track next door is how a shifted read
        // gets the frames it is short of.
        let recorded = self.recorded()?;

        let mut reading = self.reading()?;
        // What has been read and not yet handed on, which is under two sectors
        // because a sector is handed on as soon as one is there.
        let mut held: Vec<i16> = Vec::with_capacity(2 * SAMPLES_PER_SECTOR);
        let mut handed = 0;

        while handed < sectors {
            if recorded.contains(&sector) {
                held.extend_from_slice(reading.read(sector)?);
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
