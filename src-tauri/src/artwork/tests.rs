use std::cell::RefCell;
use std::env::temp_dir;
use std::fs;

use super::*;

// A picture rather than something standing in for one: one red pixel, which is
// about the smallest a PNG comes.
const SLEEVE: [u8; 69] = [
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0xC9, 0xFE, 0x92, 0xEF, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

// The same picture written the way it crosses to the window.
const WRITTEN: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4z8AAAAMBAQDJ/pLvAAAAAElFTkSuQmCC";

const OK: u16 = 200;

// The archive is across the internet, so the address it was reached at is
// kept: a picture is only the right one for having been asked for under the
// release the disc turned out to be.
#[derive(Default)]
struct FakeArchive {
    asked: RefCell<String>,
}

impl Http for FakeArchive {
    fn get(&self, url: &str, _within: u64) -> Result<Answer, Failed> {
        self.asked.replace(url.to_owned());

        Ok(Answer {
            status: OK,
            content_type: Some("image/png".to_owned()),
            body: SLEEVE.to_vec(),
        })
    }
}

#[test]
fn should_fetch_the_album_artwork_from_the_internet() {
    // Arrange
    let archive = FakeArchive::default();

    // Act
    let cover =
        look_up("d3dc4be9-9749-4959-99e5-133d0cb467fe", &archive).expect("the fake answers");

    // Assert
    assert_eq!(
        archive.asked.into_inner(),
        "https://coverartarchive.org/release/d3dc4be9-9749-4959-99e5-133d0cb467fe/front"
    );
    assert_eq!(
        cover,
        Some(Cover {
            media_type: "image/png".to_owned(),
            data: WRITTEN.to_owned(),
        })
    );
}

#[test]
fn should_let_the_album_artwork_be_chosen_from_this_computer() {
    // Arrange
    let path = temp_dir().join("chosen-album-artwork.png");
    fs::write(&path, SLEEVE).expect("somewhere on this computer to put a picture");

    // Act
    let cover = chosen(&path).expect("a picture that is there");

    // Assert
    assert_eq!(
        cover,
        Cover {
            media_type: "image/png".to_owned(),
            data: WRITTEN.to_owned(),
        }
    );
}
