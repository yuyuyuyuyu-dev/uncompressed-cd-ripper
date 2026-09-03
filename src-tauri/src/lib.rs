pub mod artwork;
mod error_report;
pub mod logging;
mod metadata;
pub mod ripping;
pub mod verification;
pub mod watching;

use std::path::Path;
use std::thread;

use tauri_specta::{collect_commands, collect_events, Builder, Event};

#[tauri::command]
#[specta::specta]
fn drives() -> Vec<String> {
    ripping::drives()
}

#[tauri::command]
#[specta::specta]
fn tracks(drive: String) -> Result<Vec<ripping::Track>, String> {
    Ok(ripping::tracks(
        &ripping::Drive::open(&drive)?,
        &logging::Logger,
    ))
}

#[tauri::command]
#[specta::specta]
fn already_there(destination: String, tracks: Vec<ripping::TrackFile>) -> Vec<String> {
    ripping::already_there(Path::new(&destination), &tracks, &logging::Logger)
}

#[tauri::command(async)]
#[specta::specta]
fn look_up_disc(drive: String) -> Result<Vec<metadata::Album>, String> {
    let toc = ripping::table_of_contents(&ripping::Drive::open(&drive)?)?;

    metadata::look_up(&toc, &metadata::MusicBrainz, &logging::Logger)
}

#[tauri::command(async)]
#[specta::specta]
fn look_up_artwork(release: String) -> Result<Option<artwork::Artwork>, String> {
    artwork::look_up(&release, &artwork::Ureq, &logging::Logger)
}

#[tauri::command]
#[specta::specta]
fn read_artwork(path: String) -> Result<artwork::Artwork, String> {
    artwork::chosen(Path::new(&path))
}

#[tauri::command]
#[specta::specta]
fn drive_name(drive: String) -> Result<String, String> {
    Ok(ripping::Drive::open(&drive)?.hardware()?.to_string())
}

#[tauri::command(async)]
#[specta::specta]
fn read_offset(drive: String) -> Result<Option<i32>, String> {
    let drive = ripping::Drive::open(&drive)?;

    verification::read_offset(
        &drive.hardware()?,
        &verification::AccurateRip,
        &logging::Logger,
    )
}

#[tauri::command(async)]
#[specta::specta]
fn check_rip(
    drive: String,
    checksums: Vec<verification::Checksums>,
) -> Result<Vec<verification::Verdict>, String> {
    let toc = ripping::table_of_contents(&ripping::Drive::open(&drive)?)?;

    verification::verify(
        &toc,
        &checksums,
        &verification::AccurateRip,
        &logging::Logger,
    )
}

#[tauri::command(async)]
#[specta::specta]
fn rip_track(
    drive: String,
    track: u8,
    destination: String,
    tags: Option<ripping::TrackTags>,
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
        &logging::Logger,
        |so_far| {
            let _ = progress.send(so_far);
        },
    )
}

#[tauri::command]
#[specta::specta]
fn environment() -> error_report::Environment {
    error_report::Environment::current()
}

#[tauri::command]
#[specta::specta]
fn log_error(error: String) {
    logging::failed(&error, &logging::Logger);
}

#[tauri::command]
#[specta::specta]
fn breadcrumbs() -> Vec<logging::Breadcrumb> {
    logging::breadcrumbs()
}

#[tauri::command]
#[specta::specta]
fn send_error_report(report: error_report::ErrorReport) -> Result<(), String> {
    error_report::send(&report, &error_report::Sentry::configured()?)
}

pub fn builder() -> Builder<tauri::Wry> {
    Builder::new()
        .constant("AGREEMENTS_REQUIRED", ripping::AGREEMENTS_REQUIRED)
        .constant("READS_ALLOWED", ripping::READS_ALLOWED)
        .events(collect_events![watching::DrivesChanged])
        .commands(collect_commands![
            environment,
            send_error_report,
            log_error,
            breadcrumbs,
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
    let builder = builder();

    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            let window = app.handle().clone();

            thread::spawn(move || {
                if let Err(failure) =
                    watching::watch(&watching::Drives, ripping::drives, |drives| {
                        let _ = watching::DrivesChanged(drives).emit(&window);
                    })
                {
                    logging::failed(&failure, &logging::Logger);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
