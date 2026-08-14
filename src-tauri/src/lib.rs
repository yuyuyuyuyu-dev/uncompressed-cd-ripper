mod error_report;
mod ripping;

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
fn already_there(destination: String, tracks: Vec<u8>) -> Vec<String> {
    ripping::already_there(Path::new(&destination), &tracks)
}

// Reading a track takes minutes and blocks the whole time, so it is handed to
// a worker thread rather than run where the window is drawn. The channel is
// how the bar on screen hears about it in the meantime.
#[tauri::command(async)]
#[specta::specta]
fn rip_track(
    drive: String,
    track: u8,
    destination: String,
    progress: tauri::ipc::Channel<u32>,
) -> Result<String, String> {
    let file = ripping::rip(&drive, track, Path::new(&destination), |sectors| {
        // A progress report that cannot be delivered says nothing about the
        // read, which carries on. The window has gone away, and that is the
        // only way this fails.
        let _ = progress.send(sectors);
    })?;

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
    Builder::new().commands(collect_commands![
        environment,
        send_error_report,
        drives,
        tracks,
        already_there,
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
