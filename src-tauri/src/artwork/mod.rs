use std::ops::RangeInclusive;

use base64::prelude::{Engine as _, BASE64_STANDARD};
use serde::{Deserialize, Serialize};
use specta::Type;

mod dimensions;
mod http;

pub use dimensions::measured;
pub use http::Ureq;

// The front of the sleeve, on its way to the screen and then into the files.
// Base64 rather than the bytes themselves: what carries this to the window is
// JSON, where a byte is written as a number and a number costs several
// characters, and a scan is millions of bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Cover {
    // What the image is, in the words a browser and a FLAC file both take.
    pub media_type: String,
    pub data: String,
}

impl Cover {
    pub fn image(&self) -> Result<Vec<u8>, String> {
        BASE64_STANDARD
            .decode(&self.data)
            .map_err(|error| format!("the cover art could not be read: {error}"))
    }
}

// What came back from a request, cut down to what the archive is read through.
// Nothing here is interpreted: which status means what, and what the header
// says the picture is, are settled below.
pub struct Answer {
    pub status: u16,
    // The header as it stands, whatever else it carries after the media type.
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}

pub enum Failed {
    // The body ran past what the caller said it would read. Its own case
    // rather than a message, so that what to tell the user is settled in one
    // place with everything else that is.
    TooLong,
    // The network, the name, the certificate: everything that never got as far
    // as an answer.
    Reason(String),
}

// The one part of fetching a sleeve that a test cannot have, and therefore as
// little as it can be: make the request, hand back what arrived.
pub trait Http {
    fn get(&self, url: &str, within: u64) -> Result<Answer, Failed>;
}

// A FLAC metadata block states how long it is in twenty-four bits, and a
// picture goes into one whole. A scan past this would be written with its
// length wrapped round and the file would not open, so it is turned down
// where it arrives instead. The margin is for the fields that describe the
// picture beside it.
pub const ROOM_FOR_A_COVER: u64 = (1 << 24) - 1024;

// The archive keeps the sleeve under the release rather than under the disc,
// which is why this is asked by the identifier a lookup answered with rather
// than by the fingerprint the lookup was asked with. It keeps the picture
// itself elsewhere again and answers with where, which the request follows.
fn front_of(release: &str) -> String {
    format!("https://coverartarchive.org/release/{release}/front")
}

const NOT_FOUND: u16 = 404;
const SUCCEEDED: RangeInclusive<u16> = 200..=299;

pub fn look_up(release: &str, http: &impl Http) -> Result<Option<Cover>, String> {
    let answer = http
        .get(&front_of(release), ROOM_FOR_A_COVER)
        .map_err(|failed| match failed {
            // Neither the archive's doing nor the network's: the scan is
            // simply larger than a FLAC file can carry.
            Failed::TooLong => "the cover art is too large to write into a file".to_owned(),
            Failed::Reason(reason) => format!("the cover art could not be fetched: {reason}"),
        })?;

    // Nothing where no front cover has been added for this release, which is
    // an answer rather than a failure: plenty of releases have none.
    if answer.status == NOT_FOUND {
        return Ok(None);
    }

    if !SUCCEEDED.contains(&answer.status) {
        return Err(format!(
            "the cover art could not be fetched: {}",
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
        .ok_or("what came back for the cover art does not say what it is")?;

    // The archive keeps other things under a release, a booklet scanned to a
    // PDF among them, and none of those is something a player can show beside
    // a track.
    if !media_type.starts_with("image/") {
        return Err(format!(
            "what came back for the cover art is {media_type}, which is not an image"
        ));
    }

    Ok(Some(Cover {
        media_type: media_type.to_owned(),
        data: BASE64_STANDARD.encode(&answer.body),
    }))
}

