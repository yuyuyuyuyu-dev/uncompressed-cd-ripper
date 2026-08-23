use super::*;

// The table of contents MusicBrainz works through on its own page about how a
// disc is identified, and the identifier it arrives at. Taking their example
// rather than inventing one means the answer this is held against is theirs.
const DISC_ID: &str = "49HHV7Eb8UKF3aQiNmu1GR8vKTY-";

fn toc() -> TableOfContents {
    TableOfContents {
        audio: vec![150, 15363, 32314, 46592, 63414, 80489],
        data: None,
        leadout: 95462,
    }
}

struct FakeApi;

impl MetadataApi for FakeApi {
    fn get(&self, disc_id: &str) -> Result<Option<String>, String> {
        if disc_id != DISC_ID {
            return Err(format!("the fake holds nothing about {disc_id}"));
        }

        Ok(Some(ANSWER.to_owned()))
    }
}

fn titles() -> Vec<TitledTrack> {
    [
        "Harbour Lights",
        "Low Tide",
        "Saltwater",
        "The Long Jetty",
        "Nightfishing",
        "Coming About",
    ]
    .into_iter()
    .enumerate()
    .map(|(index, title)| TitledTrack {
        number: index as u8 + 1,
        title: title.to_owned(),
    })
    .collect()
}

#[test]
fn should_look_up_the_metadata_for_a_disc() {
    // Arrange
    let toc = toc();

    // Act
    let albums = look_up(&toc, &FakeApi).expect("the fake answers");

    // Assert
    assert_eq!(
        albums,
        [
            Album {
                id: "8f468b26-4d5f-4c2d-9e5d-3f1c2b7a9e01".to_owned(),
                title: "Sea Change".to_owned(),
                artist: "Marina Blue".to_owned(),
                released: Some("1998-03-02".to_owned()),
                country: Some("GB".to_owned()),
                tracks: titles(),
            },
            Album {
                id: "1c9d7e52-8b3a-4f6e-9d02-5a7b1c3e8f90".to_owned(),
                title: "Sea Change (Japanese Edition)".to_owned(),
                artist: "Marina Blue & The Tide".to_owned(),
                released: Some("1998-04-22".to_owned()),
                country: Some("JP".to_owned()),
                tracks: titles(),
            },
        ]
    );
}

const ANSWER: &str = r#"{
  "id": "49HHV7Eb8UKF3aQiNmu1GR8vKTY-",
  "releases": [
    {
      "id": "8f468b26-4d5f-4c2d-9e5d-3f1c2b7a9e01",
      "title": "Sea Change",
      "status": "Official",
      "date": "1998-03-02",
      "country": "GB",
      "artist-credit": [
        {
          "name": "Marina Blue",
          "joinphrase": "",
          "artist": { "id": "d1a2c3b4-1111-2222-3333-444455556666", "name": "Marina Blue" }
        }
      ],
      "media": [
        {
          "position": 1,
          "format": "CD",
          "track-count": 6,
          "discs": [{ "id": "49HHV7Eb8UKF3aQiNmu1GR8vKTY-", "sectors": 95462 }],
          "tracks": [
            { "position": 1, "number": "1", "title": "Harbour Lights", "length": 205000 },
            { "position": 2, "number": "2", "title": "Low Tide", "length": 226000 },
            { "position": 3, "number": "3", "title": "Saltwater", "length": 190000 },
            { "position": 4, "number": "4", "title": "The Long Jetty", "length": 224000 },
            { "position": 5, "number": "5", "title": "Nightfishing", "length": 227000 },
            { "position": 6, "number": "6", "title": "Coming About", "length": 199000 }
          ]
        }
      ]
    },
    {
      "id": "1c9d7e52-8b3a-4f6e-9d02-5a7b1c3e8f90",
      "title": "Sea Change (Japanese Edition)",
      "status": "Official",
      "date": "1998-04-22",
      "country": "JP",
      "artist-credit": [
        {
          "name": "Marina Blue",
          "joinphrase": " & ",
          "artist": { "id": "d1a2c3b4-1111-2222-3333-444455556666", "name": "Marina Blue" }
        },
        {
          "name": "The Tide",
          "joinphrase": "",
          "artist": { "id": "aabbccdd-7777-8888-9999-000011112222", "name": "The Tide" }
        }
      ],
      "media": [
        {
          "position": 1,
          "format": "CD",
          "track-count": 2,
          "discs": [{ "id": "TqvKjMu7dMu6UBOMkAYfNGXnRvA-", "sectors": 41230 }],
          "tracks": [
            { "position": 1, "number": "1", "title": "Harbour Lights (Demo)", "length": 201000 },
            { "position": 2, "number": "2", "title": "Saltwater (Demo)", "length": 188000 }
          ]
        },
        {
          "position": 2,
          "format": "CD",
          "track-count": 6,
          "discs": [{ "id": "49HHV7Eb8UKF3aQiNmu1GR8vKTY-", "sectors": 95462 }],
          "tracks": [
            { "position": 1, "number": "1", "title": "Harbour Lights", "length": 205000 },
            { "position": 2, "number": "2", "title": "Low Tide", "length": 226000 },
            { "position": 3, "number": "3", "title": "Saltwater", "length": 190000 },
            { "position": 4, "number": "4", "title": "The Long Jetty", "length": 224000 },
            { "position": 5, "number": "5", "title": "Nightfishing", "length": 227000 },
            { "position": 6, "number": "6", "title": "Coming About", "length": 199000 }
          ]
        }
      ]
    }
  ]
}"#;
