use uncompressed_cd_ripper_lib::ripping;

fn main() {
    let Some(drive) = ripping::drives().first().cloned() else {
        eprintln!("no drive with an audio CD in it");
        std::process::exit(1);
    };

    println!("holding {drive}");

    if let Err(failure) = ripping::eject_disc(&drive) {
        eprintln!("{failure}");
        std::process::exit(1);
    }

    println!("ejected {drive}");
}
