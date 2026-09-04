use cdtoc::Toc;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::logging::{self, Happening, Log};
use crate::ripping::TableOfContents;

mod musicbrainz;

pub use musicbrainz::MusicBrainz;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub released: Option<String>,
    pub country: Option<String>,
    pub tracks: Vec<TitledTrack>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TitledTrack {
    pub number: u8,
    pub title: String,
    pub artist: String,
}

pub trait MetadataApi {
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
