use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::artwork::Artwork;
use crate::logging::{self, Happening, Log};
use crate::verification::{self, Checksums, Position};

mod drive;
mod flac;
#[cfg(not(windows))]
mod paranoia;
mod secure;
#[cfg(windows)]
mod win32;

pub use drive::{Drive, Hardware, ReportedTrack};
pub use flac::Flac;
pub use secure::{AGREEMENTS_REQUIRED, READS_ALLOWED};

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

// What a track will be filed as. The title is whatever the track is named as
// on screen, and nothing where it is unnamed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrackFile {
    pub number: u8,
    pub title: Option<String>,
}

// What a player shows for a track: the part of the metadata that ends up in
// the file rather than staying on the screen. Each field goes missing on its
// own, because a disc named by hand can be left half-named.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrackTags {
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub artist: Option<String>,
    pub title: Option<String>,
    // The album artwork, which every track on the disc carries a copy
    // of: a file has to stand on its own once it is in a library.
    pub artwork: Option<Artwork>,
}

pub fn drives() -> Vec<String> {
    drive::holding_an_audio_disc()
}

// The one part of reading a disc that a test cannot have, and therefore as
// little as it can be: say what is on the disc, and hand over the samples in
// every sector of a track, in the order they sit on it.
pub trait Disc {
    fn reported_tracks(&self) -> Result<Vec<ReportedTrack>, String>;

    // The offset is the drive's own, in frames, and moves what is read along
    // the disc by that much. Nothing here decides what it should be.
    fn read_track<R: FnMut(&[i16])>(
        &self,
        number: u8,
        offset: i32,
        receive: R,
    ) -> Result<(), String>;
}

// A disc is addressed from two seconds before its first track, which is where
// the lead-in ends. libcdio counts from the first track instead, so every
// offset an identifier is worked out from sits this much further along than
// the sector libcdio names.
const LEAD_IN: u32 = 150;

pub fn tracks(disc: &impl Disc, log: &impl Log) -> Vec<Track> {
    // A drive that will not say what is on the disc lists nothing rather than
    // failing, as it always has.
    let tracks = match disc.reported_tracks() {
        Ok(reported) => listing(&reported),
        Err(_) => Vec::new(),
    };

    logging::record(
        Happening::AudioTracksListed {
            tracks: tracks.len() as u8,
        },
        log,
    );

    tracks
}

// A track that cannot be placed is left out rather than offered as something
// that cannot be read.
fn listing(reported: &[drive::ReportedTrack]) -> Vec<Track> {
    reported
        .iter()
        .filter(|track| track.audio && track.first >= 0 && track.last >= track.first)
        .map(|track| Track {
            number: track.number,
            sectors: track.last.abs_diff(track.first) + 1,
        })
        .collect()
}

pub fn table_of_contents(disc: &impl Disc) -> Result<TableOfContents, String> {
    assembled(&disc.reported_tracks()?)
}

// Unlike a listing, a track that cannot be placed is refused rather than left
// out: an identifier stands for the whole disc, and one worked out from part
// of it names a different disc.
fn assembled(reported: &[drive::ReportedTrack]) -> Result<TableOfContents, String> {
    let mut toc = TableOfContents {
        audio: Vec::new(),
        data: None,
        leadout: 0,
    };

    for track in reported {
        if track.first < 0 || track.last < track.first {
            return Err(format!(
                "the drive will not say where track {} is",
                track.number
            ));
        }

        let start = track.first.unsigned_abs() + LEAD_IN;

        if track.audio {
            toc.audio.push(start);
        } else {
            toc.data = Some(start);
        }

        toc.leadout = toc.leadout.max(track.last.unsigned_abs() + 1 + LEAD_IN);
    }

    Ok(toc)
}

// What a name cannot hold. The list is Windows', which is the widest of the
// three: a name that is allowed there is allowed on the two this runs on now.
const FORBIDDEN: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

// A name is capped at 255 bytes on the filesystems this writes to, and a title
// written in three-byte characters reaches that sooner than its length looks.
const ROOM_FOR_A_TITLE: usize = 255 - "00 - ".len() - ".flac".len();

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

// A leading zero so that a listing sorts the way the disc plays. The separator
// is neither the underscore, which is what a character a name cannot hold
// turns into, nor the dot, which ends the name: as either one, it would read
// as part of the title. Nothing has to be done about the device names Windows
// keeps for itself, since every one of these begins with a digit and none of
// those do.
pub fn file_name(number: u8, title: Option<&str>) -> String {
    match title.map(usable).filter(|title| !title.is_empty()) {
        Some(title) => format!("{number:02} - {title}.flac"),
        None => format!("{number:02}.flac"),
    }
}

pub fn already_there(destination: &Path, tracks: &[TrackFile], log: &impl Log) -> Vec<String> {
    logging::record(
        Happening::FolderCheckedForOverwrites {
            tracks: tracks.len() as u8,
        },
        log,
    );

    tracks
        .iter()
        .map(|track| file_name(track.number, track.title.as_deref()))
        .filter(|name| destination.join(name).exists())
        .collect()
}

// How far along a track a read has got. Which read it is comes with it,
// because a bar that started over says nothing about why on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrackProgress {
    pub read: u8,
    pub sectors: u32,
    pub matched: u8,
}

// What a track is written as. Behind it is the file and every player that
// will ever open it, which is why what it does is stated by a job holding its
// output against tools from the other side of the format.
pub trait Encoder {
    fn write(
        &self,
        samples: &[i32],
        destination: &Path,
        number: u8,
        tags: Option<&TrackTags>,
    ) -> Result<(), String>;
}

// What a finished track leaves behind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Ripped {
    pub file: String,
    // Worked out here rather than read back off the file, because this is what
    // came off the disc and a file is what an encoder made of it. Asking
    // AccurateRip about them waits until the whole disc is read.
    pub checksums: Checksums,
}

// The disc, which track, where it is filed, how it is named, how far along
// the disc to read, what writes it and where a line about it goes: seven
// things the caller settles, and a struct to hold them would only move the
// list somewhere else.
#[allow(clippy::too_many_arguments)]
pub fn rip(
    disc: &impl Disc,
    number: u8,
    destination: &Path,
    tags: Option<&TrackTags>,
    offset: i32,
    encoder: &impl Encoder,
    log: &impl Log,
    progress: impl FnMut(TrackProgress),
) -> Result<Ripped, String> {
    // Asked of the listing first, so that a number the disc answers to with
    // data, or with nothing at all, is refused rather than read as audio.
    let audio = listing(&disc.reported_tracks()?);

    let Some(position) = position(&audio, number) else {
        return Err(format!("the disc has no audio track {number}"));
    };

    logging::record(Happening::TrackRipStarted { track: number }, log);

    // The encoder is handed a finished run of samples, so the whole track is
    // held first.
    let samples = secure::samples(disc, number, offset, log, progress)?;
    let checksums = verification::checksums(&samples, position);
    let file = store(&samples, number, destination, tags, encoder)?;

    logging::record(Happening::TrackFileWritten { track: number }, log);

    Ok(Ripped {
        file: file.to_string_lossy().into_owned(),
        checksums,
    })
}

// Which of the audio tracks this is, counting only the audio ones: a disc that
// carries data as well still has a first and a last track of music, and those
// are the two a checksum leaves the edges off.
fn position(audio: &[Track], number: u8) -> Option<Position> {
    let at = audio.iter().position(|track| track.number == number)?;

    Some(Position {
        first: at == 0,
        last: at + 1 == audio.len(),
    })
}

fn store(
    samples: &[i32],
    number: u8,
    destination: &Path,
    tags: Option<&TrackTags>,
    encoder: &impl Encoder,
) -> Result<PathBuf, String> {
    let file = destination.join(file_name(
        number,
        tags.and_then(|tags| tags.title.as_deref()),
    ));

    encoder.write(samples, &file, number, tags)?;

    Ok(file)
}

#[cfg(test)]
mod tests;
