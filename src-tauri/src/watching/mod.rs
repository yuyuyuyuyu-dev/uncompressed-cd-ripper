use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

#[cfg(windows)]
mod device_change;
#[cfg(target_os = "macos")]
mod disk_arbitration;
#[cfg(target_os = "linux")]
mod udev;

#[cfg(windows)]
pub use device_change::Drives;
#[cfg(target_os = "macos")]
pub use disk_arbitration::Drives;
#[cfg(target_os = "linux")]
pub use udev::Drives;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Event)]
pub struct DrivesChanged(pub Vec<String>);

pub trait Media {
    fn watch(&self, react: impl FnMut()) -> Result<(), String>;
}

pub fn watch(
    media: &impl Media,
    listed: impl Fn() -> Vec<String>,
    mut announce: impl FnMut(Vec<String>),
) -> Result<(), String> {
    let mut holding = listed();

    media.watch(|| {
        let found = listed();

        if found != holding {
            holding.clone_from(&found);
            announce(found);
        }
    })
}
