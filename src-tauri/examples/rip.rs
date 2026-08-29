use std::path::Path;
use std::process::ExitCode;

use uncompressed_cd_ripper_lib::artwork;
use uncompressed_cd_ripper_lib::ripping::{self, TrackTags};

const USAGE: &str = "usage: rip --disc <device or disc image> -o <folder> \
                     [--read-offset <frames>] [--album-name <name>] \
                     [--album-artist-name <name>] [--album-artwork <image>] \
                     [--track-title <title>]...";

// What a lookup would have answered, as a command line can say it. Every flag
// but the disc and the folder is optional, and a run with none of them rips a
// disc nobody looked up.
#[derive(Default)]
struct Given {
    disc: Option<String>,
    destination: Option<String>,
    // What a window looks up from AccurateRip before it starts. Zero unless it
    // is given, which reads the disc exactly as it sits.
    offset: i32,
    album: Option<String>,
    album_artist: Option<String>,
    artwork: Option<String>,
    // One per track, in the order the disc plays, because that is how a title
    // is told which track it belongs to.
    titles: Vec<String>,
}

// The rip a window would start, reached from a command line so that CI can run
// one. libcdio takes a disc image where it takes a drive, so the disc is a
// device path on a desk and a cue sheet in CI.
//
// An example rather than a test case, because it asserts nothing. The jobs
// that run it are where the assertions live.
fn main() -> ExitCode {
    if let Err(error) = run() {
        eprintln!("{error}");

        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn run() -> Result<(), String> {
    let given = given()?;

    let (Some(disc), Some(destination)) = (given.disc.as_ref(), given.destination.as_ref()) else {
        return Err(USAGE.to_owned());
    };

    rip(&given, disc, Path::new(destination))
}

fn given() -> Result<Given, String> {
    let mut given = Given::default();
    let mut arguments = std::env::args().skip(1);

    while let Some(flag) = arguments.next() {
        let mut after = || {
            arguments
                .next()
                .ok_or_else(|| format!("{flag} needs something after it"))
        };

        match flag.as_str() {
            "--disc" => given.disc = Some(after()?),
            "-o" => given.destination = Some(after()?),
            "--read-offset" => {
                given.offset = after()?
                    .parse()
                    .map_err(|_| "--read-offset needs a whole number of frames".to_owned())?;
            }
            "--album-name" => given.album = Some(after()?),
            "--album-artist-name" => given.album_artist = Some(after()?),
            "--album-artwork" => given.artwork = Some(after()?),
            "--track-title" => given.titles.push(after()?),
            _ => return Err(format!("there is no {flag}\n{USAGE}")),
        }
    }

    Ok(given)
}

fn rip(given: &Given, disc: &str, destination: &Path) -> Result<(), String> {
    std::fs::create_dir_all(destination).map_err(|error| error.to_string())?;

    // Read once for the whole disc: every track carries a copy of the same
    // artwork, and reading it again per track would say nothing new.
    let artwork = given
        .artwork
        .as_ref()
        .map(|path| artwork::chosen(Path::new(path)))
        .transpose()?;

    // Whether anything was said about the disc at all, which the titles can
    // add to a track at a time.
    let named = given.album.is_some() || given.album_artist.is_some() || artwork.is_some();

    // Opened once for the whole disc, as the window opens it once per track:
    // there is a window between one track and the next and there is none here.
    let disc = ripping::Drive::open(disc)?;

    for (index, track) in ripping::tracks(&disc).into_iter().enumerate() {
        let title = given.titles.get(index).cloned();

        // Nothing where nothing was given, as the TypeScript side leaves a disc nobody
        // named: a file is better untagged than tagged with a row of blanks.
        let tags = (named || title.is_some()).then(|| TrackTags {
            album: given.album.clone(),
            album_artist: given.album_artist.clone(),
            // The one artist is credited with the album and with every track
            // on it, which is a disc by one artist throughout.
            artist: given.album_artist.clone(),
            title,
            artwork: artwork.clone(),
        });

        // Nothing here is watching the progress a window would draw a bar from.
        let ripped = ripping::rip(
            &disc,
            track.number,
            destination,
            tags.as_ref(),
            given.offset,
            &ripping::Flac,
            |_| {},
        )?;

        // The checksums beside the file, because they are worked out from what
        // came off the disc and nothing can read them back off the file.
        println!(
            "{} {:08x} {:08x}",
            ripped.file, ripped.checksums.v1, ripped.checksums.v2
        );
    }

    Ok(())
}
