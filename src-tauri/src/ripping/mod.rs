use std::path::{Path, PathBuf};

use serde::Serialize;
use specta::Type;

mod drive;
mod flac;

/// One audio track as the disc itself describes it.
///
/// A CD carries no titles of its own, so a track is a number and a length and
/// nothing more until something is looked up about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub number: u8,
    /// Sectors rather than seconds: it is what the disc is addressed in, and
    /// 75 of them make a second.
    pub sectors: u32,
}

/// The devices that currently hold an audio CD.
pub fn drives() -> Vec<String> {
    drive::holding_an_audio_disc()
}

/// The audio tracks on the disc in the given drive.
pub fn tracks(device: &str) -> Result<Vec<Track>, String> {
    Ok(drive::Drive::open(device)?.audio_tracks())
}

/// What a track is called once it is on disk.
///
/// A leading zero so that a listing sorts the way the disc plays.
pub fn file_name(number: u8) -> String {
    format!("{number:02}.flac")
}

/// Which of the given tracks already have a file waiting for them.
pub fn already_there(destination: &Path, tracks: &[u8]) -> Vec<String> {
    tracks
        .iter()
        .map(|number| file_name(*number))
        .filter(|name| destination.join(name).exists())
        .collect()
}

// Progress is reported once a second of audio rather than once a sector,
// because a sector is a seventy-fifth of a second and nothing on screen moves
// that finely. Seventy-five times fewer messages cross to the other side for a
// bar that looks exactly the same.
const SECTORS_BETWEEN_REPORTS: u32 = 75;

/// Reads one track off the disc and leaves it in `destination` as FLAC.
///
/// `progress` is told how many of the track's sectors have been read so far,
/// which is the only part of this that takes long enough to watch.
pub fn rip(
    device: &str,
    number: u8,
    destination: &Path,
    mut progress: impl FnMut(u32),
) -> Result<PathBuf, String> {
    let drive = drive::Drive::open(device)?;

    let sectors = drive
        .audio_tracks()
        .into_iter()
        .find(|track| track.number == number)
        .ok_or_else(|| format!("the disc has no audio track {number}"))?
        .sectors;

    // The whole track is held before any of it is encoded, because the encoder
    // is handed a finished run of samples. A CD track is a few tens of
    // megabytes, and asking for the room up front keeps the read from stopping
    // to grow the buffer.
    let mut samples = Vec::with_capacity(sectors as usize * drive::SAMPLES_PER_SECTOR);
    let mut read = 0;

    drive.read_track(number, |sector| {
        samples.extend(sector.iter().copied().map(i32::from));
        read += 1;

        if read % SECTORS_BETWEEN_REPORTS == 0 {
            progress(read);
        }
    })?;

    // The last stretch is shorter than the gap between reports, so without
    // this the bar would stop just short of the end of every track.
    progress(read);

    let file = destination.join(file_name(number));
    flac::write_uncompressed(&samples, &file)?;

    Ok(file)
}
