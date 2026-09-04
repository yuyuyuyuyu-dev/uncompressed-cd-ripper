use std::cell::RefCell;

use super::*;
use crate::ripping::{Disc, ReportedTrack};

const TRACKS: [(i32, i32); 4] = [(0, 11412), (11413, 25023), (25024, 45712), (45713, 55219)];

struct FakeDisc;

impl Disc for FakeDisc {
    fn reported_tracks(&self) -> Result<Vec<ReportedTrack>, String> {
        Ok(TRACKS
            .iter()
            .enumerate()
            .map(|(index, &(first, last))| ReportedTrack {
                number: index as u8 + 1,
                audio: true,
                first,
                last,
            })
            .collect())
    }

    fn read_track<R: FnMut(&[i16])>(
        &self,
        _number: u8,
        _offset: i32,
        _receive: R,
    ) -> Result<(), String> {
        unreachable!("a checksum is handed over rather than read off the disc")
    }
}

const DISC: [u8; 13] = [
    4, 0x9a, 0x18, 0x02, 0x00, 0x33, 0x7f, 0x08, 0x00, 0x04, 0xe0, 0x02, 0x1f,
];

fn block(confidence: u8, checksums: [u32; 4]) -> Vec<u8> {
    let mut block = DISC.to_vec();

    for checksum in checksums {
        block.push(confidence);
        block.extend(checksum.to_le_bytes());
        block.extend(0u32.to_le_bytes());
    }

    block
}

struct FakeAccurateRip {
    answer: Vec<u8>,
    asked: RefCell<Vec<String>>,
}

impl VerificationApi for FakeAccurateRip {
    fn get(&self, url: &str) -> Result<Option<Vec<u8>>, String> {
        self.asked.borrow_mut().push(url.to_owned());

        Ok(Some(self.answer.clone()))
    }
}

#[test]
fn should_fetch_the_accuraterip_confidence_for_each_ripped_track() {
    // Arrange
    let mut answer = block(5, [0xf32b_60a7, 0xa993_ca05, 0x4f15_db9a, 0x830a_143a]);
    answer.extend(block(
        3,
        [0x91d1_959a, 0xb8ef_6fc4, 0xe973_cff3, 0x8b6b_1645],
    ));

    let accuraterip = FakeAccurateRip {
        answer,
        asked: RefCell::default(),
    };

    let ours = vec![
        Checksums {
            v1: 0,
            v2: 0x91d1_959a,
        },
        Checksums {
            v1: 0xa993_ca05,
            v2: 0,
        },
        Checksums { v1: 0, v2: 0 },
        Checksums {
            v1: 0,
            v2: 0x8b6b_1645,
        },
    ];

    // Act
    let toc = crate::ripping::table_of_contents(&FakeDisc).expect("the fake disc answers");
    let verdicts =
        verify(&toc, &ours, &accuraterip, &crate::logging::Logger).expect("AccurateRip answered");

    // Assert
    assert_eq!(
        accuraterip.asked.into_inner(),
        vec![
            "https://www.accuraterip.com/accuraterip/a/9/8/\
             dBAR-004-0002189a-00087f33-1f02e004.bin"
        ]
    );
    assert_eq!(
        verdicts,
        vec![
            Verdict::Matched { others: 3 },
            Verdict::Matched { others: 5 },
            Verdict::Different,
            Verdict::Matched { others: 3 },
        ]
    );
}
