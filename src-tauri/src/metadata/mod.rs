use cdtoc::Toc;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::logging::{self, Happening, Log};
use crate::ripping::TableOfContents;

mod musicbrainz;

pub use musicbrainz::MusicBrainz;

// One answer about the disc. A disc can match several: the same recording is
// pressed again for another country or another year, and the tracks are in the
// same places on all of them, so nothing about the disc itself tells them
// apart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    // What the service calls this answer. Two pressings of the same record can
    // agree on every other field, so this is the only thing that reliably tells
    // one from another.
    pub id: String,
    pub title: String,
    pub artist: String,
    // Both are missing often enough that the screen has to cope, but between
    // them they are what makes one pressing recognisable next to another.
    pub released: Option<String>,
    pub country: Option<String>,
    pub tracks: Vec<TitledTrack>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TitledTrack {
    pub number: u8,
    pub title: String,
    // Its own, because a track on a compilation is credited to somebody the
    // rest of the disc is not.
    pub artist: String,
}

pub trait MetadataApi {
    // Nothing where the service has heard of no such disc, which is an answer
    // rather than a failure: plenty of discs have never been added.
    fn get(&self, disc_id: &str) -> Result<Option<String>, String>;
}

pub fn look_up(
    toc: &TableOfContents,
    api: &impl MetadataApi,
    log: &impl Log,
) -> Result<Vec<Album>, String> {
    let disc_id = disc_id(toc)?;
    let albums = match api.get(&disc_id)? {
        Some(answer) => albums(&answer, &disc_id)?,
        None => Vec::new(),
    };

    logging::record(
        Happening::DiscLookedUp {
            releases: albums.len() as u32,
        },
        log,
    );

    Ok(albums)
}

fn disc_id(toc: &TableOfContents) -> Result<String, String> {
    Toc::from_parts(toc.audio.clone(), toc.data, toc.leadout)
        .map(|toc| toc.musicbrainz_id().to_string())
        .map_err(|error| format!("the disc's table of contents makes no sense: {error}"))
}

// Cut down to what is shown. Unknown fields are allowed through rather than
// refused, the opposite of the error report: this shape is not ours, and the
// service adds to it whenever it likes.
#[derive(Deserialize)]
struct Answer {
    #[serde(default)]
    releases: Vec<Release>,
}

#[derive(Deserialize)]
struct Release {
    id: String,
    title: String,
    date: Option<String>,
    country: Option<String>,
    #[serde(rename = "artist-credit", default)]
    artists: Vec<Credit>,
    #[serde(default)]
    media: Vec<Medium>,
}

// An album or a track can be credited to several artists at once, each
// carrying the words that join it to the next, so that "and" or a comma reads
// the way the album has it.
#[derive(Deserialize)]
struct Credit {
    name: String,
    #[serde(default)]
    joinphrase: String,
}

fn credited(artists: &[Credit]) -> String {
    artists
        .iter()
        .map(|credit| format!("{}{}", credit.name, credit.joinphrase))
        .collect::<String>()
        .trim_end()
        .to_owned()
}

#[derive(Deserialize)]
struct Medium {
    #[serde(default)]
    discs: Vec<Disc>,
    #[serde(default)]
    tracks: Vec<MediumTrack>,
}

#[derive(Deserialize)]
struct Disc {
    id: String,
}

#[derive(Deserialize)]
struct MediumTrack {
    // Where the track sits on this disc. The field called `number` is a label
    // rather than a count, and on a record it reads "A1".
    position: u8,
    title: String,
    #[serde(rename = "artist-credit", default)]
    artists: Vec<Credit>,
}

fn albums(answer: &str, disc_id: &str) -> Result<Vec<Album>, String> {
    let answer: Answer = serde_json::from_str(answer)
        .map_err(|error| format!("the answer could not be read: {error}"))?;

    Ok(answer
        .releases
        .into_iter()
        .map(|release| release.album(disc_id))
        .collect())
}

impl Release {
    fn album(self, disc_id: &str) -> Album {
        let Self {
            id,
            title,
            date,
            country,
            artists,
            mut media,
        } = self;

        // A release can be a box of several discs, and only one of them is the
        // one in the drive.
        let held = media
            .iter()
            .position(|medium| medium.discs.iter().any(|disc| disc.id == disc_id));

        let tracks = match held {
            Some(held) => media.swap_remove(held).tracks,
            None => Vec::new(),
        };

        Album {
            id,
            title,
            released: date,
            country,
            artist: credited(&artists),
            tracks: tracks
                .into_iter()
                .map(|track| TitledTrack {
                    number: track.position,
                    title: track.title,
                    artist: credited(&track.artists),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests;
