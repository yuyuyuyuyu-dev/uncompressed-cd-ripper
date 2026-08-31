use uncompressed_cd_ripper_lib::{ripping, watching};

fn main() {
    println!("watching");

    if let Err(failure) = watching::watch(&watching::Drives, ripping::drives, |drives| {
        if drives.is_empty() {
            println!("holding none");
        } else {
            println!("holding {}", drives.join(" "));
        }
    }) {
        eprintln!("{failure}");
        std::process::exit(1);
    }
}
