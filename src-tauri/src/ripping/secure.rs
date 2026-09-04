use super::drive::SAMPLES_PER_SECTOR;
use super::{Disc, TrackProgress};
use crate::logging::{self, Happening, Log};

const SECTORS_BETWEEN_REPORTS: usize = 75;

pub const AGREEMENTS_REQUIRED: u8 = 3;

pub const READS_ALLOWED: u8 = 10;

struct Candidate {
    samples: Box<[i16]>,
    agreements: u8,
}

#[derive(Default)]
struct Votes(Vec<Candidate>);

impl Votes {
    fn count(&mut self, samples: &[i16]) {
        if self.agreed().is_some() {
            return;
        }

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

    fn matches(&self) -> u8 {
        self.0
            .iter()
            .map(|candidate| candidate.agreements)
            .max()
            .unwrap_or(0)
    }

    fn agreed(&self) -> Option<&[i16]> {
        self.0
            .iter()
            .find(|candidate| candidate.agreements >= AGREEMENTS_REQUIRED)
            .map(|candidate| candidate.samples.as_ref())
    }
}

pub fn samples(
    disc: &impl Disc,
    number: u8,
    offset: i32,
    log: &impl Log,
    mut progress: impl FnMut(TrackProgress),
) -> Result<Vec<i32>, String> {
    let mut sectors: Vec<Votes> = Vec::new();
    let mut matched = 0;

    for read in 1..=READS_ALLOWED {
        if read > 1 {
            logging::record(
                Happening::TrackReadAgain {
                    track: number,
                    read,
                },
                log,
            );
        }

        let mut so_far = 0;

        disc.read_track(number, offset, |samples| {
            if so_far == sectors.len() {
                sectors.push(Votes::default());
            }

            sectors[so_far].count(samples);
            so_far += 1;

            if so_far.is_multiple_of(SECTORS_BETWEEN_REPORTS) {
                progress(TrackProgress {
                    read,
                    sectors: so_far as u32,
                    matched,
                });
            }
        })?;

        progress(TrackProgress {
            read,
            sectors: so_far as u32,
            matched,
        });

        matched = sectors.iter().map(Votes::matches).min().unwrap_or(0);

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

    if unreadable > 0 {
        return Err(format!(
            "{unreadable} of the {} sectors in track {number} never came back the \
             same way {AGREEMENTS_REQUIRED} times, over {READS_ALLOWED} reads of the track",
            sectors.len()
        ));
    }

    Ok(samples)
}
