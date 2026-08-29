use std::collections::VecDeque;
use std::fmt::{self, Display, Formatter};
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

// How many breadcrumbs are kept. An error report carries whatever is here, so
// this is what holds a report to a size somebody can read: a disc that fights
// back leaves dozens per track, and the ones nearest the failure are the ones
// worth having.
const KEPT: usize = 100;

// Everything the app can record, and the whole of it. Each of these is one
// thing that happens, named after what happens rather than after what the code
// was doing at the time, and each is recorded where it happens.
//
// What a person chose is not in here: no variant carries a folder, a disc
// title or a file name, only numbers and whether something was found. A
// breadcrumb cannot come to hold one, and an error report built out of
// breadcrumbs cannot either.
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

// One line each, saying what the name says and carrying the numbers with it.
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
    // What the breadcrumb is filed under: the log writes it beside the line,
    // and Sentry offers it as a way through a long list of them.
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

// The one part of this a test cannot have: where a line goes once it has left
// the app. Written as little as it can be, so that what is behind it is free
// to be a file, a console, or nothing at all.
pub trait Log {
    fn write(&self, category: &str, message: &str);

    // Something that failed. Its own way in, because a log marks these and
    // because what a failure says is whatever was thrown, which is not
    // something a breadcrumb is allowed to carry.
    fn write_failure(&self, category: &str, message: &str);
}

// What the app logs through. The line is handed to the log crate, which hands
// it to whichever logger was installed at startup: in the app that is the
// plugin that writes the file, and in a test or an example it is nobody.
pub struct Plugin;

impl Log for Plugin {
    fn write(&self, category: &str, message: &str) {
        log::info!(target: category, "{message}");
    }

    fn write_failure(&self, category: &str, message: &str) {
        log::error!(target: category, "{message}");
    }
}

// One breadcrumb, in the shape Sentry keeps one in. The window puts these into
// an error report as they are handed over, so what it shows before sending is
// what was recorded here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub struct Breadcrumb {
    pub timestamp: String,
    pub category: String,
    pub message: String,
}

static BREADCRUMBS: LazyLock<Mutex<VecDeque<Breadcrumb>>> =
    LazyLock::new(|| Mutex::new(VecDeque::with_capacity(KEPT)));

// The one way in. What is written to the log and what an error report carries
// are the same words: two accounts of one run would be worse than one.
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

// What the window caught, which reaches the log and stops there. The report
// about it carries the whole of what was thrown already, and breadcrumbs are
// the part that leaves the machine, so what was thrown is not made into one.
pub fn failed(message: &str, log: &impl Log) {
    log.write_failure("window", message);
}

pub fn breadcrumbs() -> Vec<Breadcrumb> {
    kept().iter().cloned().collect()
}

#[cfg(test)]
mod tests;

// A thread that panicked while holding these poisons them. They are still the
// account of the run that panicked, and they are what the report about that
// panic is owed, so they are taken as they stand.
fn kept() -> MutexGuard<'static, VecDeque<Breadcrumb>> {
    BREADCRUMBS.lock().unwrap_or_else(PoisonError::into_inner)
}
