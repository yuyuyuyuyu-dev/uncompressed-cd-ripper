use std::collections::BTreeMap;

use cdtoc::Toc;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::logging::{self, Happening, Log};
use crate::ripping::{Hardware, TableOfContents};

mod accuraterip;

pub use accuraterip::AccurateRip;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Checksums {
    pub v1: u32,
    pub v2: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase", tag = "outcome")]
pub enum Verdict {
    Matched { others: u8 },
    Different,
    Unknown,
}

const CHANNELS: usize = 2;

const SKIPPED_AT_THE_START: usize = 5 * 588 - 1;
const SKIPPED_AT_THE_END: usize = 5 * 588;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub first: bool,
    pub last: bool,
}

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
        let multiplier = index as u32 + 1;

        v1 = v1.wrapping_add(frame.wrapping_mul(multiplier));

        let whole = u64::from(frame) * u64::from(multiplier);
        v2 = v2
            .wrapping_add(whole as u32)
            .wrapping_add((whole >> u32::BITS) as u32);
    }

    Checksums { v1, v2 }
}

fn frame(samples: &[i32], index: usize) -> u32 {
    let left = samples[index * CHANNELS] as u16;
    let right = samples[index * CHANNELS + 1] as u16;

    u32::from(left) | (u32::from(right) << u16::BITS)
}

pub trait VerificationApi {
    fn get(&self, url: &str) -> Result<Option<Vec<u8>>, String>;
}

pub fn verify(
    toc: &TableOfContents,
    ours: &[Checksums],
    api: &impl VerificationApi,
    log: &impl Log,
) -> Result<Vec<Verdict>, String> {
    let disc = disc_id(toc)?;

    let Some(answer) = api.get(&disc.checksum_url())? else {
        return Ok(vec![Verdict::Unknown; ours.len()]);
    };

    let theirs = disc
        .parse_checksums(&answer)
        .map_err(|error| format!("what AccurateRip sent back could not be read: {error}"))?;

    let verdicts: Vec<Verdict> = ours
        .iter()
        .enumerate()
        .map(|(index, ours)| verdict(ours, theirs.get(index)))
        .collect();

    logging::record(
        Happening::RipCheckedAgainstAccurateRip {
            tracks: verdicts.len() as u32,
        },
        log,
    );

    Ok(verdicts)
}

fn verdict(ours: &Checksums, theirs: Option<&BTreeMap<u32, u8>>) -> Verdict {
    let Some(theirs) = theirs.filter(|theirs| !theirs.is_empty()) else {
        return Verdict::Unknown;
    };

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

pub fn read_offset(
    hardware: &Hardware,
    api: &impl VerificationApi,
    log: &impl Log,
) -> Result<Option<i32>, String> {
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

    logging::record(
        Happening::ReadOffsetLookedUp {
            found: offset.is_some(),
        },
        log,
    );

    Ok(offset)
}

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
