use std::fs::File;
use std::io::Read;
use std::ops::RangeInclusive;
use std::path::Path;

use base64::prelude::{Engine as _, BASE64_STANDARD};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::logging::{self, Happening, Log};

mod dimensions;
mod http;

pub(crate) use dimensions::measured;
pub(crate) use http::Ureq;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Artwork {
    pub(crate) media_type: String,
    pub(crate) data: String,
}

impl Artwork {
    pub(crate) fn image(&self) -> Result<Vec<u8>, String> {
        BASE64_STANDARD
            .decode(&self.data)
            .map_err(|error| format!("the album artwork could not be read: {error}"))
    }
}

pub(crate) struct Answer {
    pub(crate) status: u16,
    pub(crate) content_type: Option<String>,
    pub(crate) body: Vec<u8>,
}

pub(crate) enum Failed {
    TooLong,
    Reason(String),
}

pub(crate) trait Http {
    fn get(&self, url: &str, within: u64) -> Result<Answer, Failed>;
}

pub(crate) const ROOM_FOR_ARTWORK: u64 = (1 << 24) - 1024;

fn front_of(release: &str) -> String {
    format!("https://coverartarchive.org/release/{release}/front")
}

const NOT_FOUND: u16 = 404;
const SUCCEEDED: RangeInclusive<u16> = 200..=299;

pub(crate) fn look_up(
    release: &str,
    http: &impl Http,
    log: &impl Log,
) -> Result<Option<Artwork>, String> {
    let answer = http
        .get(&front_of(release), ROOM_FOR_ARTWORK)
        .map_err(|failed| match failed {
            Failed::TooLong => "the album artwork is too large to write into a file".to_owned(),
            Failed::Reason(reason) => format!("the album artwork could not be fetched: {reason}"),
        })?;

    if answer.status == NOT_FOUND {
        logging::record(Happening::ArtworkLookedUp { found: false }, log);

        return Ok(None);
    }

    if !SUCCEEDED.contains(&answer.status) {
        return Err(format!(
            "the album artwork could not be fetched: {}",
            answer.status
        ));
    }

    let media_type = answer
        .content_type
        .as_deref()
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .ok_or("what came back for the album artwork does not say what it is")?;

    if !media_type.starts_with("image/") {
        return Err(format!(
            "what came back for the album artwork is {media_type}, which is not an image"
        ));
    }

    logging::record(Happening::ArtworkLookedUp { found: true }, log);

    Ok(Some(Artwork {
        media_type: media_type.to_owned(),
        data: BASE64_STANDARD.encode(&answer.body),
    }))
}

const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
const JPEG: [u8; 3] = [0xFF, 0xD8, 0xFF];

fn kind(image: &[u8]) -> Option<&'static str> {
    if image.starts_with(&PNG) {
        return Some("image/png");
    }

    if image.starts_with(&JPEG) {
        return Some("image/jpeg");
    }

    None
}

pub fn chosen(path: &Path) -> Result<Artwork, String> {
    let unreadable =
        |error: std::io::Error| format!("the album artwork could not be read: {error}");

    let file = File::open(path).map_err(unreadable)?;
    let mut image = Vec::new();

    file.take(ROOM_FOR_ARTWORK + 1)
        .read_to_end(&mut image)
        .map_err(unreadable)?;

    if image.len() as u64 > ROOM_FOR_ARTWORK {
        return Err("the album artwork is too large to write into a file".to_owned());
    }

    let media_type =
        kind(&image).ok_or("what was chosen for the album artwork is neither a PNG nor a JPEG")?;

    Ok(Artwork {
        media_type: media_type.to_owned(),
        data: BASE64_STANDARD.encode(&image),
    })
}

#[cfg(test)]
mod tests;
