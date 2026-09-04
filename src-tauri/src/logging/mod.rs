use std::collections::VecDeque;
use std::fmt::{self, Display, Formatter};
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

const KEPT: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Happening {
    AudioTracksListed { tracks: u8 },
    FolderCheckedForOverwrites { tracks: u8 },
    DiscLookedUp { releases: u32 },
    ArtworkLookedUp { found: bool },
    ReadOffsetLookedUp { found: bool },
    TrackRipStarted { track: u8 },
    TrackReadAgain { track: u8, read: u8 },
    TrackFileWritten { track: u8 },
    RipCheckedAgainstAccurateRip { tracks: u32 },
}

impl Display for Happening {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::AudioTracksListed { tracks } => {
                write!(out, "the disc's audio tracks were listed: {tracks}")
            }
            Self::FolderCheckedForOverwrites { tracks } => write!(
                out,
                "the folder was checked for files a rip of {tracks} tracks would replace"
            ),
            Self::DiscLookedUp { releases } => {
                write!(out, "the disc was looked up: {releases} releases came back")
            }
            Self::ArtworkLookedUp { found } => write!(
                out,
                "the album artwork was looked up: {}",
                if *found { "found" } else { "none" }
            ),
            Self::ReadOffsetLookedUp { found } => write!(
                out,
                "the drive's read offset was looked up: {}",
                if *found {
                    "found"
                } else {
                    "the drive is not listed"
                }
            ),
            Self::TrackRipStarted { track } => write!(out, "the rip of track {track} started"),
            Self::TrackReadAgain { track, read } => {
                write!(out, "track {track} was read again: read {read}")
            }
            Self::TrackFileWritten { track } => {
                write!(out, "the file for track {track} was written")
            }
            Self::RipCheckedAgainstAccurateRip { tracks } => write!(
                out,
                "the rip was checked against AccurateRip: {tracks} tracks"
            ),
        }
    }
}

impl Happening {
    fn category(&self) -> &'static str {
        match self {
            Self::AudioTracksListed { .. }
            | Self::FolderCheckedForOverwrites { .. }
            | Self::TrackRipStarted { .. }
            | Self::TrackReadAgain { .. }
            | Self::TrackFileWritten { .. } => "ripping",
            Self::DiscLookedUp { .. }
            | Self::ArtworkLookedUp { .. }
            | Self::ReadOffsetLookedUp { .. }
            | Self::RipCheckedAgainstAccurateRip { .. } => "lookup",
        }
    }
}

pub trait Log {
    fn write(&self, category: &str, message: &str);

    fn write_failure(&self, category: &str, message: &str);
}

pub struct Logger;

impl Log for Logger {
    fn write(&self, category: &str, message: &str) {
        log::info!(target: category, "{message}");
    }

    fn write_failure(&self, category: &str, message: &str) {
        log::error!(target: category, "{message}");
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub struct Breadcrumb {
    pub timestamp: String,
    pub category: String,
    pub message: String,
}

static BREADCRUMBS: LazyLock<Mutex<VecDeque<Breadcrumb>>> =
    LazyLock::new(|| Mutex::new(VecDeque::with_capacity(KEPT)));

pub fn record(happening: Happening, log: &impl Log) {
    let category = happening.category();
    let message = happening.to_string();

    log.write(category, &message);

    let mut breadcrumbs = kept();

    if breadcrumbs.len() == KEPT {
        breadcrumbs.pop_front();
    }

    breadcrumbs.push_back(Breadcrumb {
        timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        category: category.to_owned(),
        message,
    });
}

pub fn failed(message: &str, log: &impl Log) {
    log.write_failure("window", message);
}

pub fn breadcrumbs() -> Vec<Breadcrumb> {
    kept().iter().cloned().collect()
}

#[cfg(test)]
mod tests;

fn kept() -> MutexGuard<'static, VecDeque<Breadcrumb>> {
    BREADCRUMBS.lock().unwrap_or_else(PoisonError::into_inner)
}
