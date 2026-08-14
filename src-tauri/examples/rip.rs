use std::path::Path;
use std::process::ExitCode;

use uncompressed_cd_ripper_lib::ripping;

// Rips every audio track of a disc to a folder, which is what the window does
// when its button is pressed, reached from a command line instead.
//
// It exists for the CI job that asserts what comes out. libcdio takes a disc
// image where it takes a drive, and reads it through the same driver
// interface, so a machine with no optical drive at all can still be handed a
// disc: the argument below is a device path on a desk and a path to a cue
// sheet in CI.
//
// An example rather than a test case, because it asserts nothing. It exists to
// produce files, and the job that runs it is where the assertions live.
fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let (Some(disc), Some(destination)) = (arguments.next(), arguments.next()) else {
        eprintln!("usage: rip <device or disc image> <destination folder>");

        return ExitCode::FAILURE;
    };

    if let Err(error) = rip(&disc, Path::new(&destination)) {
        eprintln!("{error}");

        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn rip(disc: &str, destination: &Path) -> Result<(), String> {
    std::fs::create_dir_all(destination).map_err(|error| error.to_string())?;

    for track in ripping::tracks(disc)? {
        // Progress is what a window draws a bar from. Nothing here is watching.
        let file = ripping::rip(disc, track.number, destination, |_| {})?;

        println!("{}", file.display());
    }

    Ok(())
}
