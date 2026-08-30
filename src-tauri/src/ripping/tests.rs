use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};

use super::*;

// A track three sectors long, which is a twenty-fifth of a second. A real one
// is minutes; this is the least that still has the reading arrive in pieces,
// as it does off a disc.
const SECTORS: usize = 3;

// One whole reading of that track, every sample the same so that one reading
// can be told from another at a glance.
fn reading(sample: i16) -> Vec<i16> {
    vec![sample; SECTORS * drive::SAMPLES_PER_SECTOR]
}

fn written(reading: &[i16]) -> Vec<i32> {
    reading.iter().copied().map(i32::from).collect()
}

// Somewhere to rip to, emptied first so that whatever is in it afterwards is
// this run's doing. Under the build directory, because a case writing anywhere
// else would be caught by the job watching for files outside the working one.
fn destination(case: &str) -> PathBuf {
    let destination = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/ripped-by-the-tests")
        .join(case);

    let _ = fs::remove_dir_all(&destination);
    fs::create_dir_all(&destination).expect("the build directory can be written to");

    destination
}

// A disc carrying one audio track, which hands back the readings the test laid
// out, one for each time it is read, and counts the times it is asked. Asked
// rather than read, so that a read past the end of the readings is still
// counted and the count still says how many times the track was gone back to.
// Nothing here decides anything: which readings there are, and what they ought
// to come to, is the test's.
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

// An encoder that keeps the samples rather than writing them. What FLAC makes
// of samples is stated by the job that rips a disc and holds the file against
// tools from the other side of the format; what is being stated here is which
// samples were believed, and reading those back out of a written file would
// only say that two encodings match, which is the same thing only for an
// encoder that is faithful.
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
    // Ten readings of the same track. The one that is believed comes back on
    // the second, the sixth and the tenth, so its third agreement lands on the
    // last read a track is allowed: a read that stopped counting part way
    // through would never reach it. Three other readings come back twice
    // each, one of them getting there first, so settling for two would settle
    // for the wrong one. Nothing comes back twice in a row, so counting a run
    // would find nothing at all.
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
    // Nothing of this reaches the filesystem, so the folder is only something
    // for a name to be built against.
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
    // Ten readings, one of which comes back twice and none three times. The
    // pair is what makes this more than a disc of ten strangers: a track
    // handed over on the strength of whichever reading showed up most, once
    // the reads ran out, would be handed over here. Ten because the
    // specification says ten: reading the number off the app would let the
    // number change and this go on passing.
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
    // The real encoder, because nothing here would pass that should not: a
    // rip that failed writes no file, and an empty folder says so plainly.
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
