use std::path::Path;

use super::*;
use crate::ripping::{self, Disc, Encoder, ReportedTrack, TrackTags};

// The track the disc below holds. Numbered so that no other case in this crate
// reads it: the trail is one list for the whole process, and cases run
// alongside each other.
const TRACK: u8 = 9;

// One sector, as short as a sector can be here. Two samples rather than a real
// sector's worth because nothing listens to this one, and the same samples
// every read so that the reads settle.
const SECTOR: [i16; 2] = [7, 7];

// A disc holding a single audio track. The one part of a rip a test cannot
// have is the drive.
struct FakeDisc;

impl Disc for FakeDisc {
    fn reported_tracks(&self) -> Result<Vec<ReportedTrack>, String> {
        Ok(vec![ReportedTrack {
            number: TRACK,
            audio: true,
            first: 0,
            last: 0,
        }])
    }

    fn read_track<R: FnMut(&[i16])>(
        &self,
        _number: u8,
        _offset: i32,
        mut receive: R,
    ) -> Result<(), String> {
        receive(&SECTOR);

        Ok(())
    }
}

// An encoder that writes nowhere, because what a file came to is not what this
// case is about.
struct FakeEncoder;

impl Encoder for FakeEncoder {
    fn write(
        &self,
        _samples: &[i32],
        _destination: &Path,
        _number: u8,
        _tags: Option<&TrackTags>,
    ) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn should_record_what_the_window_did_and_what_the_backend_did_in_one_trail() {
    // Arrange
    // A rip of one track, which the disc hands over the same way every time.
    // Three reads have to agree before the samples are believed, so the two
    // reads after the first are the backend's own doing rather than anything
    // asked for from outside.
    let expected = [
        ("window", Happening::FolderChosen),
        ("ripping", Happening::TrackStarted { track: TRACK }),
        (
            "ripping",
            Happening::TrackReadAgain {
                track: TRACK,
                read: 2,
            },
        ),
        (
            "ripping",
            Happening::TrackReadAgain {
                track: TRACK,
                read: 3,
            },
        ),
        ("ripping", Happening::TrackWritten { track: TRACK }),
    ]
    .map(|(category, happening)| (category.to_owned(), happening.to_string()));

    // Act
    // Through the command, which is the whole of the window's way in.
    crate::record(Happening::FolderChosen);
    ripping::rip(
        &FakeDisc,
        TRACK,
        Path::new("wherever"),
        None,
        0,
        &FakeEncoder,
        |_| {},
    )
    .expect("the fake disc answers");

    // Assert
    // Cut down to this case's own entries, because the cases running beside it
    // record into the same trail. What is being stated is that the window's
    // entry and the backend's are in one list, in the order they happened: an
    // account kept on either side alone, or stamped by two clocks, would not
    // come back in this order.
    let ours: Vec<(String, String)> = trail()
        .into_iter()
        .map(|crumb| (crumb.category, crumb.message))
        .filter(|entry| expected.contains(entry))
        .collect();

    assert_eq!(ours, expected);
}
