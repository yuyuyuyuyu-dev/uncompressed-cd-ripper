use std::collections::VecDeque;
use std::fmt::{self, Display, Formatter};
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

// How much of the trail is kept. An error report carries whatever is in it, so
// this is what holds a report to a size somebody can read: a disc that fights
// back leaves dozens of entries per track, and the ones nearest the failure
// are the ones worth having.
const KEPT: usize = 100;

// Everything the app can record, and the whole of it. What a person chose is
// not in here: no variant carries a folder, a disc title or a file name, so a
// trail cannot come to hold one, and an error report built from a trail cannot
// either.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "happening", rename_all = "camelCase")]
pub enum Happening {
    DriveChosen,
    FolderChosen,
    RipRequested { tracks: u8 },
    DiscRead { tracks: u8 },
    LookedUp { service: Service, found: u32 },
    TrackStarted { track: u8 },
    TrackReadAgain { track: u8, read: u8 },
    TrackWritten { track: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Service {
    MusicBrainz,
    CoverArtArchive,
    AccurateRip,
}

impl Display for Service {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MusicBrainz => write!(out, "MusicBrainz"),
            Self::CoverArtArchive => write!(out, "the Cover Art Archive"),
            Self::AccurateRip => write!(out, "AccurateRip"),
        }
    }
}

impl Display for Happening {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DriveChosen => write!(out, "a drive was chosen"),
            Self::FolderChosen => write!(out, "a folder to rip into was chosen"),
            Self::RipRequested { tracks } => write!(out, "a rip of {tracks} tracks was asked for"),
            Self::DiscRead { tracks } => write!(out, "the disc holds {tracks} audio tracks"),
            Self::LookedUp { service, found } => {
                write!(out, "{service} was asked and answered with {found}")
            }
            Self::TrackStarted { track } => write!(out, "track {track} was started"),
            Self::TrackReadAgain { track, read } => {
                write!(out, "track {track} is being read again, read {read}")
            }
            Self::TrackWritten { track } => write!(out, "track {track} was written"),
        }
    }
}

impl Happening {
    // What the entry is filed under: the log file writes it beside the line,
    // and Sentry offers it as a way through a long trail.
    fn category(&self) -> &'static str {
        match self {
            Self::DriveChosen | Self::FolderChosen | Self::RipRequested { .. } => "window",
            Self::DiscRead { .. }
            | Self::TrackStarted { .. }
            | Self::TrackReadAgain { .. }
            | Self::TrackWritten { .. } => "ripping",
            Self::LookedUp { .. } => "lookup",
        }
    }
}

// One entry of the trail, in the shape Sentry keeps a breadcrumb in. The
// window puts these into an error report as they are handed over, so what it
// shows before sending is what was recorded here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub struct Breadcrumb {
    pub timestamp: String,
    pub category: String,
    pub message: String,
}

static TRAIL: LazyLock<Mutex<VecDeque<Breadcrumb>>> =
    LazyLock::new(|| Mutex::new(VecDeque::with_capacity(KEPT)));

// The one way into the trail, for the window and for this side alike. The
// window reaches it through a command, so both accounts are stamped by the
// same clock and land in the order they happened rather than in the order two
// clocks disagreed about.
pub fn record(happening: Happening) {
    log::info!(target: happening.category(), "{happening}");

    let crumb = Breadcrumb {
        timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        category: happening.category().to_owned(),
        message: happening.to_string(),
    };
    let mut trail = kept();

    if trail.len() == KEPT {
        trail.pop_front();
    }

    trail.push_back(crumb);
}

pub fn trail() -> Vec<Breadcrumb> {
    kept().iter().cloned().collect()
}

// A thread that panicked while holding the trail poisons it. That trail is
// still the account of the run that panicked, and it is what the report about
// that panic is owed, so it is taken as it stands.
fn kept() -> MutexGuard<'static, VecDeque<Breadcrumb>> {
    TRAIL.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
