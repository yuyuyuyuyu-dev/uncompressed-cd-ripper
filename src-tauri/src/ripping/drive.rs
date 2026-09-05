use std::fmt;

use super::Disc;

#[cfg(target_os = "macos")]
pub use super::diskutil::eject_disc;
#[cfg(target_os = "linux")]
pub use super::paranoia::eject_disc;
#[cfg(not(windows))]
pub use super::paranoia::{holding_an_audio_disc, Drive};
#[cfg(windows)]
pub use super::win32::{eject_disc, holding_an_audio_disc, Drive};

pub(super) const BYTES_PER_SECTOR: usize = 2352;

pub const SAMPLES_PER_SECTOR: usize = BYTES_PER_SECTOR / size_of::<i16>();

const CHANNELS: usize = 2;
const FRAMES_PER_SECTOR: usize = SAMPLES_PER_SECTOR / CHANNELS;

const NOTHING_RECORDED: [i16; SAMPLES_PER_SECTOR] = [0; SAMPLES_PER_SECTOR];

pub struct ReportedTrack {
    pub number: u8,
    pub audio: bool,
    pub first: i32,
    pub last: i32,
}

pub struct Hardware {
    pub vendor: String,
    pub model: String,
}

impl fmt::Display for Hardware {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.vendor.is_empty() {
            write!(out, "{} ", self.vendor)?;
        }

        write!(out, "{}", self.model)
    }
}

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

        let begins_at = track.first as i64 * FRAMES_PER_SECTOR as i64 + i64::from(offset);
        let mut sector = begins_at.div_euclid(FRAMES_PER_SECTOR as i64) as i32;
        let mut before = begins_at.rem_euclid(FRAMES_PER_SECTOR as i64) as usize * CHANNELS;

        let sectors = track.last.abs_diff(track.first) as usize + 1;
        let recorded = self.recorded()?;

        let mut reading = self.reading()?;
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

            while handed < sectors && held.len() >= SAMPLES_PER_SECTOR {
                receive(&held[..SAMPLES_PER_SECTOR]);
                held.drain(..SAMPLES_PER_SECTOR);
                handed += 1;
            }
        }

        Ok(())
    }
}
