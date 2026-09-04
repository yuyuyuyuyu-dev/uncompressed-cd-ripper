use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};

use super::*;

const SECTORS: usize = 3;

fn reading(sample: i16) -> Vec<i16> {
    vec![sample; SECTORS * drive::SAMPLES_PER_SECTOR]
}

fn written(reading: &[i16]) -> Vec<i32> {
    reading.iter().copied().map(i32::from).collect()
}

fn destination(case: &str) -> PathBuf {
    let destination = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/ripped-by-the-tests")
        .join(case);

    let _ = fs::remove_dir_all(&destination);
    fs::create_dir_all(&destination).expect("the build directory can be written to");

    destination
}

struct FakeDisc {
    readings: Vec<Vec<i16>>,
    reads: Cell<usize>,
}

impl FakeDisc {
    fn holding(readings: Vec<Vec<i16>>) -> Self {
        Self {
            readings,
            reads: Cell::new(0),
        }
    }
}

impl Disc for FakeDisc {
    fn reported_tracks(&self) -> Result<Vec<ReportedTrack>, String> {
        Ok(vec![ReportedTrack {
            number: 1,
            audio: true,
            first: 0,
            last: SECTORS as i32 - 1,
        }])
    }

    fn read_track<R: FnMut(&[i16])>(
        &self,
        _number: u8,
        _offset: i32,
        mut receive: R,
    ) -> Result<(), String> {
        let read = self.reads.get();
        self.reads.set(read + 1);

        let reading = self
            .readings
            .get(read)
            .ok_or_else(|| format!("the fake disc has only {} readings", self.readings.len()))?;

        for sector in reading.chunks(drive::SAMPLES_PER_SECTOR) {
            receive(sector);
        }

        Ok(())
    }
}

#[derive(Default)]
struct FakeEncoder {
    written: RefCell<Vec<i32>>,
}

impl Encoder for FakeEncoder {
    fn write(
        &self,
        samples: &[i32],
        _destination: &Path,
        _number: u8,
        _tags: Option<&TrackTags>,
    ) -> Result<(), String> {
        self.written.replace(samples.to_vec());

        Ok(())
    }
}

#[test]
fn should_write_the_samples_that_three_reads_of_the_disc_agreed_on() {
    // Arrange
    let agreed = reading(1000);
    let disc = FakeDisc::holding(vec![
        reading(-1),
        agreed.clone(),
        reading(-1),
        reading(-2),
        reading(-2),
        agreed.clone(),
        reading(-3),
        reading(-3),
        reading(-4),
        agreed.clone(),
    ]);
    let encoder = FakeEncoder::default();

    // Act
    rip(
        &disc,
        1,
        Path::new("wherever"),
        None,
        0,
        &encoder,
        &crate::logging::Logger,
        |_| {},
    )
    .expect("the fake disc answers");

    // Assert
    assert_eq!(encoder.written.into_inner(), written(&agreed));
}

#[test]
fn should_fail_when_ten_reads_of_the_disc_never_agree_three_times() {
    // Arrange
    let twice = reading(3);
    let disc = FakeDisc::holding(vec![
        reading(0),
        reading(1),
        twice.clone(),
        reading(2),
        reading(4),
        reading(5),
        reading(6),
        twice,
        reading(7),
        reading(8),
    ]);
    let destination = destination("never agreed");

    // Act
    let file = rip(
        &disc,
        1,
        &destination,
        None,
        0,
        &Flac,
        &crate::logging::Logger,
        |_| {},
    );

    // Assert
    assert!(
        file.is_err(),
        "the track was written although no three reads of it agreed"
    );
    assert_eq!(disc.reads.get(), 10);
    assert_eq!(
        fs::read_dir(&destination)
            .expect("the folder ripped to is there")
            .count(),
        0,
        "the folder should have been left as empty as it was found"
    );
}
