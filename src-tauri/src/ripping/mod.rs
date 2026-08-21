use std::path::{Path, PathBuf};

use serde::Serialize;
use specta::Type;

mod drive;
mod flac;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub number: u8,
    // Sectors rather than seconds, which is what the disc is addressed in.
    pub sectors: u32,
}

pub fn drives() -> Vec<String> {
    drive::holding_an_audio_disc()
}

pub fn tracks(device: &str) -> Result<Vec<Track>, String> {
    Ok(drive::Drive::open(device)?.audio_tracks())
}

// A leading zero so that a listing sorts the way the disc plays.
pub fn file_name(number: u8) -> String {
    format!("{number:02}.flac")
}

pub fn already_there(destination: &Path, tracks: &[u8]) -> Vec<String> {
    tracks
        .iter()
        .map(|number| file_name(*number))
        .filter(|name| destination.join(name).exists())
        .collect()
}

// A sector is a seventy-fifth of a second and no bar moves that finely, so
// seventy-five times fewer messages cross for a bar that looks the same.
const SECTORS_BETWEEN_REPORTS: u32 = 75;

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

    // The encoder is handed a finished run of samples, so the whole track is
    // held first. Asking for the room up front keeps the read from stopping to
    // grow the buffer.
    let mut samples = Vec::with_capacity(sectors as usize * drive::SAMPLES_PER_SECTOR);
    let mut read = 0;

    drive.read_track(number, |sector| {
        samples.extend(sector.iter().copied().map(i32::from));
        read += 1;

        if read % SECTORS_BETWEEN_REPORTS == 0 {
            progress(read);
        }
    })?;

    // The last stretch is shorter than the gap between reports, so without this
    // the bar stops short of the end of every track.
    progress(read);

    let file = destination.join(file_name(number));
    flac::write_uncompressed(&samples, &file)?;

    Ok(file)
}
