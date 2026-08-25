use std::cell::Cell;

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

// A disc that hands back the readings the test laid out, one for each time it
// is read, and counts the times it is asked. Asked rather than read, so that a
// read past the end of the readings is still counted and the count still says
// how many times the track was gone back to. Nothing here decides anything:
// which readings there are, and what they ought to come to, is the test's.
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
    fn read_track<R: FnMut(&[i16])>(&self, _number: u8, mut receive: R) -> Result<(), String> {
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

    // Act
    let samples = secure::samples(&disc, 1, |_| {}).expect("the fake disc answers");

    // Assert
    assert_eq!(samples, written(&agreed));
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

    // Act
    let samples = secure::samples(&disc, 1, |_| {});

    // Assert
    assert!(
        samples.is_err(),
        "the track was handed over although no three reads of it agreed"
    );
    assert_eq!(disc.reads.get(), 10);
}
