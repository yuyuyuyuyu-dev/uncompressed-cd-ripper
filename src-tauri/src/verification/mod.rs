use std::collections::BTreeMap;

use cdtoc::Toc;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ripping::{Hardware, TableOfContents};

mod accuraterip;

pub use accuraterip::AccurateRip;

// What a track came to, worked out both of the ways AccurateRip has counted a
// track over the years. An entry in the database does not say which of the two
// it is, so both are worked out here and either one matching is a match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Checksums {
    pub v1: u32,
    pub v2: u32,
}

// What the database had to say about one track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase", tag = "outcome")]
pub enum Verdict {
    // Other people's drives read this track and came to the same samples. How
    // many is the whole point: one is a coincidence to be wary of, and fifty
    // is fifty machines that cannot all have gone wrong the same way.
    Matched { others: u8 },
    // The database holds this track and nobody who sent it in came to these
    // samples. Either this rip is wrong, or the disc is a pressing nobody has
    // sent in before.
    Different,
    // Nobody has ever sent this disc in, so there is nothing to compare with.
    // Not a failure, and not a verdict either.
    Unknown,
}

// A moment of sound on both channels, which is what a checksum counts and what
// a drive's read offset is measured in.
const CHANNELS: usize = 2;

// The very start of the first track and the very end of the last are left out
// of a checksum, because that is where drives disagree about where the disc
// begins and ends. Five sectors' worth at each end, one frame fewer at the
// start: AccurateRip keeps the frames whose multiplier reaches five sectors,
// and a multiplier runs one ahead of the frame it belongs to.
const SKIPPED_AT_THE_START: usize = 5 * 588 - 1;
const SKIPPED_AT_THE_END: usize = 5 * 588;

// Where the track sits among the audio tracks, which is all a checksum has to
// know about the rest of the disc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub first: bool,
    pub last: bool,
}

// Both numbers in one pass, because the samples are tens of millions long and
// the two counts differ only in what they do with the same multiplication.
pub fn checksums(samples: &[i32], position: Position) -> Checksums {
    let frames = samples.len() / CHANNELS;
    let from = if position.first {
        SKIPPED_AT_THE_START
    } else {
        0
    };
    let to = if position.last {
        frames.saturating_sub(SKIPPED_AT_THE_END)
    } else {
        frames
    };

    let mut v1: u32 = 0;
    let mut v2: u32 = 0;

    for index in from..to {
        let frame = frame(samples, index);
        // Counted from one, and counted from the start of the track rather
        // than from the first frame that is looked at: a frame left out at the
        // start still moved the multiplier on.
        let multiplier = index as u32 + 1;

        // Both of these run past what they are held in and are meant to. A
        // checksum is a fingerprint, not a sum of anything.
        v1 = v1.wrapping_add(frame.wrapping_mul(multiplier));

        let whole = u64::from(frame) * u64::from(multiplier);
        v2 = v2
            .wrapping_add(whole as u32)
            .wrapping_add((whole >> u32::BITS) as u32);
    }

    Checksums { v1, v2 }
}

// One frame as AccurateRip counts it: the two channels of one moment side by
// side in a single number, the left one in the low half, which is the order
// they sit in on the disc.
fn frame(samples: &[i32], index: usize) -> u32 {
    let left = samples[index * CHANNELS] as u16;
    let right = samples[index * CHANNELS + 1] as u16;

    u32::from(left) | (u32::from(right) << u16::BITS)
}

// The one part of asking AccurateRip that a test cannot have, and therefore as
// little as it can be: fetch what is at an address, hand back the bytes.
pub trait VerificationApi {
    // Nothing where AccurateRip has never heard of what was asked for, which
    // is an answer rather than a failure: most discs have never been sent in.
    fn get(&self, url: &str) -> Result<Option<Vec<u8>>, String>;
}

// Holds the whole disc's checksums against the ones other people arrived at.
// The tracks are in the order they play, which is the order the answer holds
// them in.
pub fn verify(
    toc: &TableOfContents,
    ours: &[Checksums],
    api: &impl VerificationApi,
) -> Result<Vec<Verdict>, String> {
    let disc = disc_id(toc)?;

    let Some(answer) = api.get(&disc.checksum_url())? else {
        return Ok(vec![Verdict::Unknown; ours.len()]);
    };

    // A disc pressed twice sits in the answer twice, once per pressing, and
    // this counts a track's readings across all of them: what is being asked
    // is whether anybody anywhere arrived at these samples.
    let theirs = disc
        .parse_checksums(&answer)
        .map_err(|error| format!("what AccurateRip sent back could not be read: {error}"))?;

    Ok(ours
        .iter()
        .enumerate()
        .map(|(index, ours)| verdict(ours, theirs.get(index)))
        .collect())
}

fn verdict(ours: &Checksums, theirs: Option<&BTreeMap<u32, u8>>) -> Verdict {
    let Some(theirs) = theirs.filter(|theirs| !theirs.is_empty()) else {
        return Verdict::Unknown;
    };

    // The newer count first, because a rip that matches both is far likelier
    // to be a disc whose two entries agree than a coincidence, and the newer
    // count is the one the database is filling up with now.
    match theirs.get(&ours.v2).or_else(|| theirs.get(&ours.v1)) {
        Some(others) => Verdict::Matched { others: *others },
        None => Verdict::Different,
    }
}

fn disc_id(toc: &TableOfContents) -> Result<cdtoc::AccurateRip, String> {
    Toc::from_parts(toc.audio.clone(), toc.data, toc.leadout)
        .map(|toc| toc.accuraterip_id())
        .map_err(|error| format!("the disc's table of contents makes no sense: {error}"))
}

// How far along a drive is reading when it says it is at the start of a track,
// counted in frames, and nothing where AccurateRip has never been told about
// this drive.
//
// Every checksum in the database was worked out from samples with the drive's
// own head start taken off. A rip that leaves it on is shifted against all of
// them and matches nothing, however faultlessly it was read.
pub fn read_offset(hardware: &Hardware, api: &impl VerificationApi) -> Result<Option<i32>, String> {
    let Some(list) = api.get(cdtoc::AccurateRip::DRIVE_OFFSET_URL)? else {
        return Err("AccurateRip no longer keeps a list of drive read offsets".to_owned());
    };

    let offsets = cdtoc::AccurateRip::parse_drive_offsets(&list).map_err(|error| {
        format!("AccurateRip's list of drive read offsets could not be read: {error}")
    })?;

    let offset = offsets
        .into_iter()
        .find(|((vendor, model), _)| same(vendor, &hardware.vendor) && same(model, &hardware.model))
        .map(|(_, offset)| i32::from(offset));

    Ok(offset)
}

// Two names stand for the same drive when they read the same once the spacing
// and the case are set aside. The list was built from what other people's
// drives reported, and a drive answers out of fixed-width fields padded with
// however many spaces are left over.
fn same(listed: &str, reported: &str) -> bool {
    let mut listed = listed.split_whitespace();
    let mut reported = reported.split_whitespace();

    loop {
        match (listed.next(), reported.next()) {
            (None, None) => return true,
            (Some(listed), Some(reported)) if listed.eq_ignore_ascii_case(reported) => {}
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests;
