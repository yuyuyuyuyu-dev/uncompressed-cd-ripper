use std::path::Path;
use std::process::ExitCode;

use uncompressed_cd_ripper_lib::artwork;
use uncompressed_cd_ripper_lib::logging;
use uncompressed_cd_ripper_lib::ripping::{self, TrackTags};

const USAGE: &str = "usage: rip --disc <device or disc image> -o <folder> \
                     [--read-offset <frames>] [--album-name <name>] \
                     [--album-artist-name <name>] [--album-artwork <image>] \
                     [--track-title <title>]...";

#[derive(Default)]
struct Given {
    disc: Option<String>,
    destination: Option<String>,
    offset: i32,
    album: Option<String>,
    album_artist: Option<String>,
    artwork: Option<String>,
    titles: Vec<String>,
}

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

    let artwork = given
        .artwork
        .as_ref()
        .map(|path| artwork::chosen(Path::new(path)))
        .transpose()?;

    let named = given.album.is_some() || given.album_artist.is_some() || artwork.is_some();

    let disc = ripping::Drive::open(disc)?;

    for (index, track) in ripping::tracks(&disc, &logging::Logger)
        .into_iter()
        .enumerate()
    {
        let title = given.titles.get(index).cloned();

        let tags = (named || title.is_some()).then(|| TrackTags {
            album: given.album.clone(),
            album_artist: given.album_artist.clone(),
            artist: given.album_artist.clone(),
            title,
            artwork: artwork.clone(),
        });

        let ripped = ripping::rip(
            &disc,
            track.number,
            destination,
            tags.as_ref(),
            given.offset,
            &ripping::Flac,
            &logging::Logger,
            |_| {},
        )?;

        println!(
            "{} {:08x} {:08x}",
            ripped.file, ripped.checksums.v1, ripped.checksums.v2
        );
    }

    Ok(())
}
