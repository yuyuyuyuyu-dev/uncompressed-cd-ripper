use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
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

// Where every track on the disc begins. This is the disc's fingerprint: an
// identifier worked out from it is what a database is asked about, and no two
// pressings that differ share one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableOfContents {
    pub audio: Vec<u32>,
    pub data: Option<u32>,
    pub leadout: u32,
}

// What a track will be filed as. The title is what the disc was looked up as,
// and nothing where it was not looked up or the answer had no title for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrackFile {
    pub number: u8,
    pub title: Option<String>,
}

// What a player shows for a track: the part of the metadata that ends up in
// the file rather than staying on the screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrackTags {
    pub album: String,
    pub artist: String,
    pub title: String,
}

pub fn drives() -> Vec<String> {
    drive::holding_an_audio_disc()
}

pub fn tracks(device: &str) -> Result<Vec<Track>, String> {
    Ok(drive::Drive::open(device)?.audio_tracks())
}

pub fn table_of_contents(device: &str) -> Result<TableOfContents, String> {
    drive::Drive::open(device)?.table_of_contents()
}

// What a name cannot hold. The list is Windows', which is the widest of the
// three: a name that is allowed there is allowed on the two this runs on now.
const FORBIDDEN: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

// A name is capped at 255 bytes on the filesystems this writes to, and a title
// written in three-byte characters reaches that sooner than its length looks.
const ROOM_FOR_A_TITLE: usize = 255 - "00_".len() - ".flac".len();

fn usable(title: &str) -> String {
    // One character for one rather than dropped, so that a title made of
    // nothing else still leaves a name, and so that "AC/DC" does not come out
    // as one word.
    let mut name: String = title
        .chars()
        .map(|character| {
            if FORBIDDEN.contains(&character) || character.is_control() {
                '_'
            } else {
                character
            }
        })
        .collect();

    while name.len() > ROOM_FOR_A_TITLE {
        name.pop();
    }

    // Windows drops a trailing dot or space without saying so, which would
    // leave the file under a name nothing here goes looking for.
    name.trim().trim_end_matches(['.', ' ']).to_owned()
}

// A leading zero so that a listing sorts the way the disc plays. Nothing has
// to be done about the device names Windows keeps for itself, since every one
// of these begins with a digit and none of those do.
pub fn file_name(number: u8, title: Option<&str>) -> String {
    match title.map(usable).filter(|title| !title.is_empty()) {
        Some(title) => format!("{number:02}_{title}.flac"),
        None => format!("{number:02}.flac"),
    }
}

pub fn already_there(destination: &Path, tracks: &[TrackFile]) -> Vec<String> {
    tracks
        .iter()
        .map(|track| file_name(track.number, track.title.as_deref()))
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
    tags: Option<&TrackTags>,
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

    store(&samples, number, destination, tags)
}

fn store(
    samples: &[i32],
    number: u8,
    destination: &Path,
    tags: Option<&TrackTags>,
) -> Result<PathBuf, String> {
    let file = destination.join(file_name(number, tags.map(|tags| tags.title.as_str())));

    flac::write_uncompressed(samples, &file, number, tags)?;

    Ok(file)
}
