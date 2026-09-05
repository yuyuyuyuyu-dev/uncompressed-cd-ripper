use std::process::Command;

const DISKUTIL: &str = "/usr/sbin/diskutil";

pub fn eject_disc(device: &str) -> Result<(), String> {
    let stuck = |reason: &str| format!("{device} would not eject its disc: {reason}");

    let outcome = Command::new(DISKUTIL)
        .arg("eject")
        .arg(bsd_name(device))
        .output()
        .map_err(|_| stuck("diskutil could not be run"))?;

    if outcome.status.success() {
        return Ok(());
    }

    Err(stuck(&said(&outcome.stderr, &outcome.stdout)))
}

fn bsd_name(device: &str) -> &str {
    let name = match device.rfind('/') {
        Some(slash) => &device[slash + 1..],
        None => device,
    };

    name.strip_prefix('r').unwrap_or(name)
}

fn said(complaint: &[u8], answer: &[u8]) -> String {
    [complaint, answer]
        .iter()
        .map(|said| String::from_utf8_lossy(said).trim().to_owned())
        .find(|said| !said.is_empty())
        .unwrap_or_else(|| "diskutil said nothing".to_owned())
}
