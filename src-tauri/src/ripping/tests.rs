use super::*;

fn vorbis_comment(file: &[u8]) -> Vec<String> {
    let mut at = "fLaC".len();

    loop {
        let (last, kind) = (file[at] & 0x80 != 0, file[at] & 0x7f);
        let length = u32::from_be_bytes([0, file[at + 1], file[at + 2], file[at + 3]]) as usize;

        at += 4;

        if kind == 4 {
            return fields(&file[at..at + length]);
        }

        assert!(!last, "the file carries no tags at all");

        at += length;
    }
}

fn fields(block: &[u8]) -> Vec<String> {
    fn counted(block: &[u8], at: &mut usize) -> String {
        let length = u32::from_le_bytes(block[*at..*at + 4].try_into().unwrap()) as usize;

        *at += 4;

        let text = String::from_utf8(block[*at..*at + length].to_vec()).unwrap();

        *at += length;

        text
    }

    let mut at = 0;
    let _vendor = counted(block, &mut at);
    let count = u32::from_le_bytes(block[at..at + 4].try_into().unwrap()) as usize;

    at += 4;

    (0..count).map(|_| counted(block, &mut at)).collect()
}

#[test]
fn should_write_the_fetched_metadata_into_the_file_a_track_is_ripped_to() {
    // Arrange
    let destination = tempfile::tempdir().expect("a folder to write into");
    let tags = TrackTags {
        album: "Sea Change".to_owned(),
        artist: "Marina Blue".to_owned(),
        title: "Harbour Lights".to_owned(),
    };

    // Act
    let file = store(&[0; 8192], 3, destination.path(), Some(&tags)).expect("the track is written");

    // Assert
    assert_eq!(
        vorbis_comment(&std::fs::read(file).expect("the file that was just written")),
        [
            "TITLE=Harbour Lights",
            "ARTIST=Marina Blue",
            "ALBUM=Sea Change",
            "TRACKNUMBER=3",
        ]
    );
}
