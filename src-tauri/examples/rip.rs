use std::path::Path;
use std::process::ExitCode;

use uncompressed_cd_ripper_lib::ripping;

// The rip a window would start, reached from a command line so that CI can run
// one. libcdio takes a disc image where it takes a drive, so the argument is a
// device path on a desk and a cue sheet in CI.
//
// An example rather than a test case, because it asserts nothing. The jobs
// that run it are where the assertions live.
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
        // Nothing here is watching the progress a window would draw a bar from.
        let file = ripping::rip(disc, track.number, destination, |_| {})?;

        println!("{}", file.display());
    }

    Ok(())
}
