use std::cell::RefCell;
use std::path::Path;

use super::*;
use crate::ripping::{self, Disc, Encoder, ReportedTrack, TrackTags};

const TRACK: u8 = 1;

const SECTOR: [i16; 2] = [7, 7];

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
    failed("BackendError: the drive stopped responding", &log);

    // Assert
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
    assert_eq!(
        log.failures.into_inner(),
        [(
            "window".to_owned(),
            "BackendError: the drive stopped responding".to_owned()
        )]
    );
}
