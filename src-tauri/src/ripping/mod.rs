use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::artwork::Artwork;
use crate::logging::{self, Happening, Log};
use crate::verification::{self, Checksums, Position};

#[cfg(target_os = "macos")]
mod diskutil;
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
    pub sectors: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableOfContents {
    pub audio: Vec<u32>,
    pub data: Option<u32>,
    pub leadout: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrackFile {
    pub number: u8,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrackTags {
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub artist: Option<String>,
    pub title: Option<String>,
    pub artwork: Option<Artwork>,
}

pub fn drives() -> Vec<String> {
    drive::holding_an_audio_disc()
}

pub fn eject_disc(drive: &str) -> Result<(), String> {
    drive::eject_disc(drive)
}

pub trait Disc {
    fn reported_tracks(&self) -> Result<Vec<ReportedTrack>, String>;

    fn read_track<R: FnMut(&[i16])>(
        &self,
        number: u8,
        offset: i32,
        receive: R,
    ) -> Result<(), String>;
}

const LEAD_IN: u32 = 150;

pub fn tracks(disc: &impl Disc, log: &impl Log) -> Vec<Track> {
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

const FORBIDDEN: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

const ROOM_FOR_A_TITLE: usize = 255 - "00 - ".len() - ".flac".len();

fn usable(title: &str) -> String {
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

    name.trim().trim_end_matches(['.', ' ']).to_owned()
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrackProgress {
    pub read: u8,
    pub sectors: u32,
    pub matched: u8,
}

pub trait Encoder {
    fn write(
        &self,
        samples: &[i32],
        destination: &Path,
        number: u8,
        tags: Option<&TrackTags>,
    ) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Ripped {
    pub file: String,
    pub checksums: Checksums,
}

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
    let audio = listing(&disc.reported_tracks()?);

    let Some(position) = position(&audio, number) else {
        return Err(format!("the disc has no audio track {number}"));
    };

    logging::record(Happening::TrackRipStarted { track: number }, log);

    let samples = secure::samples(disc, number, offset, log, progress)?;
    let checksums = verification::checksums(&samples, position);
    let file = store(&samples, number, destination, tags, encoder)?;

    logging::record(Happening::TrackFileWritten { track: number }, log);

    Ok(Ripped {
        file: file.to_string_lossy().into_owned(),
        checksums,
    })
}

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
