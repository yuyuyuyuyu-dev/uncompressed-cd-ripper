use std::cell::RefCell;
use std::path::Path;

use super::*;
use crate::ripping::{self, Disc, Encoder, ReportedTrack, TrackTags};

const TRACK: u8 = 1;

// One sector, as short as a sector can be here. Two samples rather than a real
// sector's worth because nothing listens to this one, and the same samples
// every read so that the reads settle.
const SECTOR: [i16; 2] = [7, 7];

// A disc holding a single audio track. The drive is the one part of a rip a
// case cannot have.
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

// A log that keeps what it was handed rather than writing it anywhere. Where a
// line goes once it has left the app is the other part a case cannot have.
#[derive(Default)]
struct FakeLog {
    written: RefCell<Vec<(String, String)>>,
    failures: RefCell<Vec<(String, String)>>,
}

impl Log for FakeLog {
    fn write(&self, category: &str, message: &str) {
        self.written
            .borrow_mut()
            .push((category.to_owned(), message.to_owned()));
    }

    fn write_failure(&self, category: &str, message: &str) {
        self.failures
            .borrow_mut()
            .push((category.to_owned(), message.to_owned()));
    }
}

#[test]
fn should_record_a_log_of_what_happened() {
    // Arrange
    // A disc that hands back the same samples every time, so the three
    // agreements a rip waits for land on the third read. The two reads after
    // the first are the disc failing to agree with itself, which is the whole
    // reason any of this is written down: nothing else in the app says a word
    // about them.
    let log = FakeLog::default();

    // Act
    ripping::rip(
        &FakeDisc,
        TRACK,
        Path::new("wherever"),
        None,
        0,
        &FakeEncoder,
        &log,
        |_| {},
    )
    .expect("the fake disc answers");
    // What the window caught, which reaches this side through the command that
    // hands it to exactly this call.
    failed("BackendError: the drive stopped responding", &log);

    // Assert
    // Every line, in order, in the words it is written in: a rip that recorded
    // one thing more, one thing fewer, or the same things in another order
    // fails here.
    assert_eq!(
        log.written.into_inner(),
        [
            ("ripping", "the rip of track 1 started"),
            ("ripping", "track 1 was read again: read 2"),
            ("ripping", "track 1 was read again: read 3"),
            ("ripping", "the file for track 1 was written"),
        ]
        .map(|(category, message)| (category.to_owned(), message.to_owned()))
    );
    // A failure is called out rather than filed beside the rest, and it says
    // what was thrown.
    assert_eq!(
        log.failures.into_inner(),
        [(
            "window".to_owned(),
            "BackendError: the drive stopped responding".to_owned()
        )]
    );
}
