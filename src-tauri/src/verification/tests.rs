use std::cell::RefCell;

use super::*;

// A real disc, and the one cdtoc's own documentation works through: four
// tracks, and AccurateRip has an answer for it.
fn toc() -> TableOfContents {
    TableOfContents {
        audio: vec![150, 11563, 25174, 45863],
        data: None,
        leadout: 55370,
    }
}

// Where AccurateRip keeps that disc's answer. Written out rather than worked
// out from the table of contents above, so that a change to how the identifier
// is put together fails here rather than quietly asking about another disc.
const ANSWER_FOR_THE_DISC: &str = "https://www.accuraterip.com/accuraterip/a/9/8/\
                                   dBAR-004-0002189a-00087f33-1f02e004.bin";

// The thirteen bytes every block of an answer begins with: how many audio
// tracks the disc has, and then the three numbers AccurateRip knows it by,
// each of them little endian.
const DISC: [u8; 13] = [
    4, 0x9a, 0x18, 0x02, 0x00, 0x33, 0x7f, 0x08, 0x00, 0x04, 0xe0, 0x02, 0x1f,
];

// One block of an answer, which is one set of submissions that agreed with
// each other. Nine bytes a track: how many submissions came out this way, the
// checksum they came out with, and the checksum AccurateRip finds pressing
// offsets with, which nothing here reads.
fn block(confidence: u8, checksums: [u32; 4]) -> Vec<u8> {
    let mut block = DISC.to_vec();

    for checksum in checksums {
        block.push(confidence);
        block.extend(checksum.to_le_bytes());
        block.extend(0u32.to_le_bytes());
    }

    block
}

// The one part of asking AccurateRip that a test cannot have, and therefore as
// little as it can be. It writes down every address it is asked for, so that
// which disc was asked about is stated alongside what came back.
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
    // Two blocks, as a disc that has been pressed more than once comes back:
    // five submissions arrived at one set of checksums and three at another.
    let mut answer = block(5, [0xf32b_60a7, 0xa993_ca05, 0x4f15_db9a, 0x830a_143a]);
    answer.extend(block(
        3,
        [0x91d1_959a, 0xb8ef_6fc4, 0xe973_cff3, 0x8b6b_1645],
    ));

    let accuraterip = FakeAccurateRip {
        answer,
        asked: RefCell::default(),
    };

    // One track out of each block, one out of neither, and one out of the
    // second again, so that a confidence landing on the wrong track shows up.
    // The second track is only in the block of five, and only under the older
    // of the two ways a track is counted: a rip that asked about the newer way
    // alone would come back with nothing for it.
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
    let verdicts = verify(&toc(), &ours, &accuraterip).expect("AccurateRip answered");

    // Assert
    assert_eq!(accuraterip.asked.into_inner(), vec![ANSWER_FOR_THE_DISC]);
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
