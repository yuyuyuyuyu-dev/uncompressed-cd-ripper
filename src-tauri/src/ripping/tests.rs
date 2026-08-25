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
    // Two readings of the same track, one coming back three times and the
    // other twice. The three are neither the first to arrive nor in a row, so
    // taking whichever came first, or counting a run, would take the other
    // one, and so would settling for two.
    let agreed = reading(1000);
    let odd = reading(-1000);
    let disc = FakeDisc::holding(vec![
        odd.clone(),
        agreed.clone(),
        odd,
        agreed.clone(),
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
    // Ten readings, no two of them the same, which is a disc that cannot be
    // read rather than one that reads slowly. Ten because the specification
    // says ten: reading the number off the app would let the number change
    // and this go on passing.
    let disc = FakeDisc::holding((0..10).map(reading).collect());

    // Act
    let samples = secure::samples(&disc, 1, |_| {});

    // Assert
    assert!(
        samples.is_err(),
        "the track was handed over although no three reads of it agreed"
    );
    assert_eq!(disc.reads.get(), 10);
}
