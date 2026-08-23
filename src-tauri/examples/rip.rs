use std::path::Path;
use std::process::ExitCode;

use uncompressed_cd_ripper_lib::ripping::{self, TrackTags};

// The rip a window would start, reached from a command line so that CI can run
// one. libcdio takes a disc image where it takes a drive, so the argument is a
// device path on a desk and a cue sheet in CI.
//
// The album and the artist stand for what a lookup answered, and the titles
// after them go to the tracks in the order the disc plays. The one artist is
// credited with the album and with every track on it, which is a disc by one
// artist throughout. A run with none of them rips a disc nobody looked up.
//
// An example rather than a test case, because it asserts nothing. The jobs
// that run it are where the assertions live.
fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let (Some(disc), Some(destination)) = (arguments.next(), arguments.next()) else {
        eprintln!("usage: rip <device or disc image> <folder> [<album> <artist> <title>...]");

        return ExitCode::FAILURE;
    };

    let metadata = arguments.collect::<Vec<_>>();

    if let Err(error) = rip(&disc, Path::new(&destination), &metadata) {
        eprintln!("{error}");

        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn rip(disc: &str, destination: &Path, metadata: &[String]) -> Result<(), String> {
    std::fs::create_dir_all(destination).map_err(|error| error.to_string())?;

    let (album, artist, titles) = match metadata {
        [] => (None, None, [].as_slice()),
        [album, artist, titles @ ..] => (Some(album), Some(artist), titles),
        _ => return Err("an album needs an artist after it".to_owned()),
    };

    for (index, track) in ripping::tracks(disc)?.into_iter().enumerate() {
        let tags = titles.get(index).map(|title| TrackTags {
            album: album.cloned(),
            album_artist: artist.cloned(),
            artist: artist.cloned(),
            title: Some(title.clone()),
        });

        // Nothing here is watching the progress a window would draw a bar from.
        let file = ripping::rip(disc, track.number, destination, tags.as_ref(), |_| {})?;

        println!("{}", file.display());
    }

    Ok(())
}
