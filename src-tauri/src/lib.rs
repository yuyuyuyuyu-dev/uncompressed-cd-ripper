mod error_report;

use tauri_specta::{collect_commands, Builder};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
#[specta::specta]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// Taking the whole report as an argument is what stops anything being added
// to it here: there is no field to add without changing the type the frontend
// was generated from.
#[tauri::command]
#[specta::specta]
fn send_error_report(report: error_report::ErrorReport) -> Result<(), String> {
    error_report::send(&report, &error_report::Sentry::configured()?)
}

fn builder() -> Builder<tauri::Wry> {
    Builder::new().commands(collect_commands![greet, send_error_report])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(builder().invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    // The documented alternative writes the bindings while the app runs in
    // debug mode, which would leave CI unable to regenerate them without
    // starting a window. Writing them from a test case means `cargo test`
    // regenerates them, and the job asserting the working tree stayed clean
    // is what turns a stale file into a failure.
    #[test]
    fn should_generate_typescript_bindings_matching_the_commands() {
        builder()
            .export(
                specta_typescript::Typescript::default(),
                "../src/bindings.ts",
            )
            .expect("the bindings are written next to the frontend sources");
    }
}
