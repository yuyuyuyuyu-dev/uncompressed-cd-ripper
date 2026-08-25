// Public so that the example beside it can put artwork into a rip, as the
// TypeScript side does.
pub mod artwork;
mod error_report;
mod metadata;
// Public so that the example beside it can reach a rip without a window.
pub mod ripping;

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
    ripping::tracks(&drive)
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
    metadata::look_up(&ripping::table_of_contents(&drive)?, &metadata::MusicBrainz)
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
    progress: tauri::ipc::Channel<ripping::TrackProgress>,
) -> Result<String, String> {
    let file = ripping::rip(
        &drive,
        track,
        Path::new(&destination),
        tags.as_ref(),
        |so_far| {
            // Only fails once the window has gone, which the read does not care about.
            let _ = progress.send(so_far);
        },
    )?;

    Ok(file.to_string_lossy().into_owned())
}

#[tauri::command]
#[specta::specta]
fn environment() -> error_report::Environment {
    error_report::Environment::current()
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
            drives,
            tracks,
            already_there,
            look_up_disc,
            look_up_artwork,
            read_artwork,
            rip_track
        ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(builder().invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
