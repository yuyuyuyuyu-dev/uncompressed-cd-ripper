mod error_report;

use chrono::Utc;
use tauri::State;

use error_report::{Environment, ErrorReport, PendingReports};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// The whole report goes back to the caller so that the screen asking for
// consent can show exactly what pressing send would hand over.
#[tauri::command]
fn report_error(
    message: &str,
    stack: Option<&str>,
    pending: State<'_, PendingReports>,
) -> ErrorReport {
    pending.add(message, stack, Utc::now(), &Environment::current())
}

#[tauri::command]
fn submit_error_report(id: u64, pending: State<'_, PendingReports>) {
    // Clearing the report is the whole of submitting so far, because nothing
    // carries it anywhere yet. The transport arrives with the Sentry wiring.
    pending.take(id);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(PendingReports::default())
        .invoke_handler(tauri::generate_handler![
            greet,
            report_error,
            submit_error_report
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
