use std::path::Path;
use std::process::ExitCode;

use uncompressed_cd_ripper_lib::ripping::{self, TrackTags};

// The rip a window would start, reached from a command line so that CI can run
// one. libcdio takes a disc image where it takes a drive, so the argument is a
// device path on a desk and a cue sheet in CI.
//
// The album and the artist stand for what is on the screen when a rip starts,
// and the titles after them go to the tracks in the order the disc plays. An
// argument left empty is a field left blank, and a run with none of them at
// all rips a disc nobody said anything about.
//
// An example rather than a test case, because it asserts nothing. The jobs
// that run it are where the assertions live.
fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let (Some(disc), Some(destination)) = (arguments.next(), arguments.next()) else {
        eprintln!("usage: rip <device or disc image> <folder> [<album> <artist> <title>...]");
        eprintln!("an argument left empty is a field left blank");

        return ExitCode::FAILURE;
    };

    let metadata = arguments.collect::<Vec<_>>();

    if let Err(error) = rip(&disc, Path::new(&destination), &metadata) {
        eprintln!("{error}");

        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

// Nothing to write rather than something empty to write, which is what the
// window makes of a field somebody left blank.
fn given(text: &str) -> Option<String> {
    (!text.is_empty()).then(|| text.to_owned())
}

fn rip(disc: &str, destination: &Path, metadata: &[String]) -> Result<(), String> {
    std::fs::create_dir_all(destination).map_err(|error| error.to_string())?;

    let (album, artist, titles) = match metadata {
        [] => (None, None, [].as_slice()),
        [album, artist, titles @ ..] => (given(album), given(artist), titles),
        _ => return Err("an album needs an artist after it".to_owned()),
    };

    for (index, track) in ripping::tracks(disc)?.into_iter().enumerate() {
        let title = titles.get(index).and_then(|title| given(title));

        // Tags at all only where something was filled in, as the window sends
        // none for a disc nobody named.
        let tags = (album.is_some() || artist.is_some() || title.is_some()).then(|| TrackTags {
            album: album.clone(),
            artist: artist.clone(),
            title,
        });

        // Nothing here is watching the progress a window would draw a bar from.
        let file = ripping::rip(disc, track.number, destination, tags.as_ref(), |_| {})?;

        println!("{}", file.display());
    }

    Ok(())
}
