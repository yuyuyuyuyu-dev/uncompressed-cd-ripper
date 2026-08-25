use super::drive::{Drive, SAMPLES_PER_SECTOR};
use super::TrackProgress;

// A sector is a seventy-fifth of a second and no bar moves that finely, so
// seventy-five times fewer messages cross for a bar that looks the same.
const SECTORS_BETWEEN_REPORTS: usize = 75;

// How many reads of a sector have to come back with the same samples before
// those samples are the ones written. No number is standard: EAC reads twice
// and re-reads only where the two disagree, cdparanoia checks overlapping
// reads against each other inside a single pass, and AccurateRip compares a
// finished rip against other people's rather than repeating its own.
pub const AGREEMENTS_REQUIRED: u8 = 3;

// How many times the whole track is read before a sector that never agreed
// with itself is called unreadable.
pub const READS_ALLOWED: u8 = 10;

// One reading of one sector, and how many times it has come back.
struct Candidate {
    samples: Box<[i16]>,
    agreements: u8,
}

// Every distinct reading of one sector so far. Nearly always one long, because
// a drive that reads a sector correctly reads it correctly again.
#[derive(Default)]
struct Votes(Vec<Candidate>);

impl Votes {
    fn count(&mut self, samples: &[i16]) {
        match self
            .0
            .iter_mut()
            .find(|candidate| candidate.samples.as_ref() == samples)
        {
            Some(candidate) => candidate.agreements += 1,
            None => self.0.push(Candidate {
                samples: samples.into(),
                agreements: 1,
            }),
        }
    }

    fn agreed(&self) -> Option<&[i16]> {
        self.0
            .iter()
            .find(|candidate| candidate.agreements >= AGREEMENTS_REQUIRED)
            .map(|candidate| candidate.samples.as_ref())
    }
}

// Reads the track again and again until every sector has come back the same
// way often enough to be believed.
//
// The counting is per sector rather than per track. A disc that misreads one
// sector at a time, a different one on each read, would never hand back two
// identical tracks to compare, while every sector on it still settles.
pub fn samples(
    drive: &Drive,
    number: u8,
    mut progress: impl FnMut(TrackProgress),
) -> Result<Vec<i32>, String> {
    let mut sectors: Vec<Votes> = Vec::new();

    for read in 1..=READS_ALLOWED {
        let mut so_far = 0;

        drive.read_track(number, |samples| {
            // The first read is what says how long the track is. Every read
            // after it covers the same sectors, so it lands on the same votes.
            if so_far == sectors.len() {
                sectors.push(Votes::default());
            }

            sectors[so_far].count(samples);
            so_far += 1;

            if so_far.is_multiple_of(SECTORS_BETWEEN_REPORTS) {
                progress(TrackProgress {
                    read,
                    sectors: so_far as u32,
                });
            }
        })?;

        // The last stretch is shorter than the gap between reports, so without
        // this the bar stops short of the end of every read.
        progress(TrackProgress {
            read,
            sectors: so_far as u32,
        });

        if sectors.iter().all(|votes| votes.agreed().is_some()) {
            break;
        }
    }

    settled(&sectors, number)
}

fn settled(sectors: &[Votes], number: u8) -> Result<Vec<i32>, String> {
    let mut samples = Vec::with_capacity(sectors.len() * SAMPLES_PER_SECTOR);
    let mut unreadable = 0;

    for votes in sectors {
        match votes.agreed() {
            Some(agreed) => samples.extend(agreed.iter().copied().map(i32::from)),
            None => unreadable += 1,
        }
    }

    // Nothing is written rather than something silently patched up, which is
    // the whole of what reading a track several times is for.
    if unreadable > 0 {
        return Err(format!(
            "{unreadable} of the {} sectors in track {number} never came back the \
             same way {AGREEMENTS_REQUIRED} times, over {READS_ALLOWED} reads of the track",
            sectors.len()
        ));
    }

    Ok(samples)
}
