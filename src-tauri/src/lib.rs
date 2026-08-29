// Public so that the example beside it can put artwork into a rip, as the
// TypeScript side does.
pub mod artwork;
mod error_report;
mod logging;
mod metadata;
// Public so that the example beside it can reach a rip without a window.
pub mod ripping;
// Public so that the example beside it can read what a track came to, as the
// TypeScript side does.
pub mod verification;

use std::path::Path;

use tauri_specta::{collect_commands, Builder};

#[tauri::command]
#[specta::specta]
fn drives() -> Vec<String> {
    ripping::drives()
}

#[tauri::command]
#[specta::specta]
fn tracks(drive: String) -> Result<Vec<ripping::Track>, String> {
    Ok(ripping::tracks(&ripping::Drive::open(&drive)?))
}

#[tauri::command]
#[specta::specta]
fn already_there(destination: String, tracks: Vec<ripping::TrackFile>) -> Vec<String> {
    ripping::already_there(Path::new(&destination), &tracks)
}

// Reaching a server takes long enough to hold the window still, so this is
// handed to a worker thread as well.
#[tauri::command(async)]
#[specta::specta]
fn look_up_disc(drive: String) -> Result<Vec<metadata::Album>, String> {
    let toc = ripping::table_of_contents(&ripping::Drive::open(&drive)?)?;

    metadata::look_up(&toc, &metadata::MusicBrainz)
}

// The album artwork comes from another server, and waiting on it holds the window
// still just as the lookup does.
#[tauri::command(async)]
#[specta::specta]
fn look_up_artwork(release: String) -> Result<Option<artwork::Artwork>, String> {
    artwork::look_up(&release, &artwork::Ureq)
}

#[tauri::command]
#[specta::specta]
fn read_artwork(path: String) -> Result<artwork::Artwork, String> {
    artwork::chosen(Path::new(&path))
}

// What the drive calls itself, which is what a read offset is kept under: a
// device path is whatever the operating system handed out this time, and the
// offset belongs to the drive.
#[tauri::command]
#[specta::specta]
fn drive_name(drive: String) -> Result<String, String> {
    Ok(ripping::Drive::open(&drive)?.hardware()?.to_string())
}

// AccurateRip's list of drive read offsets is a third of a megabyte, so this
// waits on the network as the lookups do.
#[tauri::command(async)]
#[specta::specta]
fn read_offset(drive: String) -> Result<Option<i32>, String> {
    let drive = ripping::Drive::open(&drive)?;

    verification::read_offset(&drive.hardware()?, &verification::AccurateRip)
}

// Asked once the whole disc is read rather than track by track, because one
// answer covers the disc and asking per track would fetch it again each time.
#[tauri::command(async)]
#[specta::specta]
fn check_rip(
    drive: String,
    checksums: Vec<verification::Checksums>,
) -> Result<Vec<verification::Verdict>, String> {
    let toc = ripping::table_of_contents(&ripping::Drive::open(&drive)?)?;

    verification::verify(&toc, &checksums, &verification::AccurateRip)
}

// Reading a track blocks for minutes, so it is handed to a worker thread.
#[tauri::command(async)]
#[specta::specta]
fn rip_track(
    drive: String,
    track: u8,
    destination: String,
    // Nothing where the disc was never named, by a lookup or by hand. The file
    // is then written as it always was.
    tags: Option<ripping::TrackTags>,
    // The drive's own read offset, which the window looked up before it began.
    // Zero where AccurateRip has never been told about this drive, which reads
    // the track exactly as the drive hands it over.
    offset: i32,
    progress: tauri::ipc::Channel<ripping::TrackProgress>,
) -> Result<ripping::Ripped, String> {
    ripping::rip(
        &ripping::Drive::open(&drive)?,
        track,
        Path::new(&destination),
        tags.as_ref(),
        offset,
        &ripping::Flac,
        |so_far| {
            // Only fails once the window has gone, which the read does not care about.
            let _ = progress.send(so_far);
        },
    )
}

#[tauri::command]
#[specta::specta]
fn environment() -> error_report::Environment {
    error_report::Environment::current()
}

// Asked for the moment an error is caught rather than when the report is
// built, so that a report says what the app was doing when it failed and not
// what it did while the notification sat on screen.
#[tauri::command]
#[specta::specta]
fn trail() -> Vec<logging::Breadcrumb> {
    logging::trail()
}

// Taking the whole report as an argument is what stops anything being added
// to it here: there is no field to add without changing the type the frontend
// was generated from.
#[tauri::command]
#[specta::specta]
fn send_error_report(report: error_report::ErrorReport) -> Result<(), String> {
    error_report::send(&report, &error_report::Sentry::configured()?)
}

pub fn builder() -> Builder<tauri::Wry> {
    Builder::new()
        // Said on this side alone, so that what the window tells the user
        // about a read and what the read does cannot drift apart.
        .constant("AGREEMENTS_REQUIRED", ripping::AGREEMENTS_REQUIRED)
        .constant("READS_ALLOWED", ripping::READS_ALLOWED)
        .commands(collect_commands![
            environment,
            send_error_report,
            trail,
            drives,
            tracks,
            already_there,
            look_up_disc,
            look_up_artwork,
            read_artwork,
            drive_name,
            read_offset,
            rip_track,
            check_rip
        ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Left at its defaults, which write to the log directory each of the
        // three platforms keeps one at and hold the file to a size by rotating
        // it. Where that directory is on each platform is exactly the part
        // that would be got wrong by hand.
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .invoke_handler(builder().invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
