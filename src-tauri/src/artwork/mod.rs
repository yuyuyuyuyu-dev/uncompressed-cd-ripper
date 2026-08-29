use std::fs::File;
use std::io::Read;
use std::ops::RangeInclusive;
use std::path::Path;

use base64::prelude::{Engine as _, BASE64_STANDARD};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::logging::{self, Happening, Service};

mod dimensions;
mod http;

pub(crate) use dimensions::measured;
pub(crate) use http::Ureq;

// The album artwork, on its way to the screen and then into the files.
// Base64 rather than the bytes themselves: what carries this to the TypeScript
// side is JSON, where a byte is written as a number and a number costs several
// characters, and a scan is millions of bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Artwork {
    // What the image is, in the words a browser and a FLAC file both take.
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

// What came back from a request, cut down to what the archive is read through.
// Nothing here is interpreted: which status means what, and what the header
// says the picture is, are settled below.
pub(crate) struct Answer {
    pub(crate) status: u16,
    // The header as it stands, whatever else it carries after the media type.
    pub(crate) content_type: Option<String>,
    pub(crate) body: Vec<u8>,
}

pub(crate) enum Failed {
    // The body ran past what the caller said it would read. Its own case
    // rather than a message, so that what to tell the user is settled in one
    // place with everything else that is.
    TooLong,
    // The network, the name, the certificate: everything that never got as far
    // as an answer.
    Reason(String),
}

// The one part of fetching artwork that a test cannot have, and therefore as
// little as it can be: make the request, hand back what arrived.
pub(crate) trait Http {
    fn get(&self, url: &str, within: u64) -> Result<Answer, Failed>;
}

// A FLAC metadata block states how long it is in twenty-four bits, and a
// picture goes into one whole. A scan past this would be written with its
// length wrapped round and the file would not open, so it is turned down
// where it arrives instead. The margin is for the fields that describe the
// picture beside it.
pub(crate) const ROOM_FOR_ARTWORK: u64 = (1 << 24) - 1024;

// The archive keeps the artwork under the release rather than under the disc,
// which is why this is asked by the identifier a lookup answered with rather
// than by the fingerprint the lookup was asked with. It keeps the picture
// itself elsewhere again and answers with where, which the request follows.
fn front_of(release: &str) -> String {
    format!("https://coverartarchive.org/release/{release}/front")
}

const NOT_FOUND: u16 = 404;
const SUCCEEDED: RangeInclusive<u16> = 200..=299;

pub(crate) fn look_up(release: &str, http: &impl Http) -> Result<Option<Artwork>, String> {
    let answer = http
        .get(&front_of(release), ROOM_FOR_ARTWORK)
        .map_err(|failed| match failed {
            // Neither the archive's doing nor the network's: the scan is
            // simply larger than a FLAC file can carry.
            Failed::TooLong => "the album artwork is too large to write into a file".to_owned(),
            Failed::Reason(reason) => format!("the album artwork could not be fetched: {reason}"),
        })?;

    // Nothing where no front cover has been added for this release, which is
    // an answer rather than a failure: plenty of releases have none.
    if answer.status == NOT_FOUND {
        logging::record(Happening::LookedUp {
            service: Service::CoverArtArchive,
            found: 0,
        });

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
        // A media type can be followed by a semicolon and more about it, none
        // of which belongs in the file.
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .ok_or("what came back for the album artwork does not say what it is")?;

    // The archive keeps other things under a release, a booklet scanned to a
    // PDF among them, and none of those is something a player can show beside
    // a track.
    if !media_type.starts_with("image/") {
        return Err(format!(
            "what came back for the album artwork is {media_type}, which is not an image"
        ));
    }

    logging::record(Happening::LookedUp {
        service: Service::CoverArtArchive,
        found: 1,
    });

    Ok(Some(Artwork {
        media_type: media_type.to_owned(),
        data: BASE64_STANDARD.encode(&answer.body),
    }))
}

const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
const JPEG: [u8; 3] = [0xFF, 0xD8, 0xFF];

// Read from the image itself rather than from the name of the file: an
// extension is whatever somebody typed, and a picture block that says the wrong
// thing is artwork no player draws. The two recognised here are the two a block
// is measured for, and the two the archive serves.
fn kind(image: &[u8]) -> Option<&'static str> {
    if image.starts_with(&PNG) {
        return Some("image/png");
    }

    if image.starts_with(&JPEG) {
        return Some("image/jpeg");
    }

    None
}

// Artwork off this computer rather than off the archive, for a disc no database
// has artwork for, or one whose artwork is wrong.
pub fn chosen(path: &Path) -> Result<Artwork, String> {
    let unreadable =
        |error: std::io::Error| format!("the album artwork could not be read: {error}");

    let file = File::open(path).map_err(unreadable)?;
    let mut image = Vec::new();

    // Capped where a fetched one is, and for the same reason. One byte past
    // what fits, so that a file which is exactly too large is still seen to be.
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
