use std::collections::HashMap;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::Serialize;

const HOME_DIRECTORY_PLACEHOLDER: &str = "<home>";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    pub app_version: String,
    pub operating_system: String,
    pub home_directory: String,
}

impl Environment {
    pub fn current() -> Self {
        Self {
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            operating_system: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
            home_directory: std::env::home_dir()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorReport {
    pub id: u64,
    pub occurred_at: String,
    pub app_version: String,
    pub operating_system: String,
    pub message: String,
    pub stack: Option<String>,
}

impl ErrorReport {
    pub fn new(
        id: u64,
        message: &str,
        stack: Option<&str>,
        occurred_at: DateTime<Utc>,
        environment: &Environment,
    ) -> Self {
        let scrub = |text: &str| scrub_home_directory(text, &environment.home_directory);

        Self {
            id,
            occurred_at: occurred_at.to_rfc3339(),
            app_version: environment.app_version.clone(),
            operating_system: environment.operating_system.clone(),
            message: scrub(message),
            stack: stack.map(scrub),
        }
    }
}

// Paths carry the account name on every desktop platform, so anything holding
// one leaves with the name in it unless it goes through here first.
fn scrub_home_directory(text: &str, home_directory: &str) -> String {
    // Replacing an empty pattern would wedge the placeholder between every
    // character of the text.
    if home_directory.is_empty() {
        return text.to_owned();
    }

    text.replace(home_directory, HOME_DIRECTORY_PLACEHOLDER)
}

#[derive(Debug, Default)]
pub struct PendingReports {
    inner: Mutex<Pending>,
}

#[derive(Debug, Default)]
struct Pending {
    next_id: u64,
    reports: HashMap<u64, ErrorReport>,
}

impl PendingReports {
    // The report is handed back so that whatever shows it to the user and
    // whatever eventually sends it are looking at the same value.
    pub fn add(
        &self,
        message: &str,
        stack: Option<&str>,
        occurred_at: DateTime<Utc>,
        environment: &Environment,
    ) -> ErrorReport {
        let mut pending = self
            .inner
            .lock()
            .expect("the pending reports lock is poisoned");

        pending.next_id += 1;
        let report = ErrorReport::new(pending.next_id, message, stack, occurred_at, environment);
        pending.reports.insert(report.id, report.clone());

        report
    }

    pub fn take(&self, id: u64) -> Option<ErrorReport> {
        let mut pending = self
            .inner
            .lock()
            .expect("the pending reports lock is poisoned");

        pending.reports.remove(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment() -> Environment {
        Environment {
            app_version: "0.1.0".to_owned(),
            operating_system: "macos aarch64".to_owned(),
            home_directory: "/Users/someone".to_owned(),
        }
    }

    fn occurred_at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-08-12T09:00:00Z")
            .expect("the fixture timestamp is valid")
            .with_timezone(&Utc)
    }

    #[test]
    fn should_carry_the_message_it_was_built_from() {
        // Act
        let report = ErrorReport::new(
            1,
            "the drive stopped responding",
            None,
            occurred_at(),
            &environment(),
        );

        // Assert
        assert_eq!(report.message, "the drive stopped responding");
    }

    #[test]
    fn should_carry_the_application_version() {
        // Act
        let report = ErrorReport::new(1, "any message", None, occurred_at(), &environment());

        // Assert
        assert_eq!(report.app_version, "0.1.0");
    }

    #[test]
    fn should_carry_the_operating_system() {
        // Act
        let report = ErrorReport::new(1, "any message", None, occurred_at(), &environment());

        // Assert
        assert_eq!(report.operating_system, "macos aarch64");
    }

    #[test]
    fn should_record_when_the_error_occurred() {
        // Act
        let report = ErrorReport::new(1, "any message", None, occurred_at(), &environment());

        // Assert
        assert_eq!(report.occurred_at, "2026-08-12T09:00:00+00:00");
    }

    #[test]
    fn should_replace_the_home_directory_in_the_message() {
        // Act
        let report = ErrorReport::new(
            1,
            "cannot write /Users/someone/Music/track.flac",
            None,
            occurred_at(),
            &environment(),
        );

        // Assert
        assert_eq!(report.message, "cannot write <home>/Music/track.flac");
    }

    #[test]
    fn should_replace_the_home_directory_in_the_stack() {
        // Act
        let report = ErrorReport::new(
            1,
            "any message",
            Some("at /Users/someone/app/src/rip.rs:12"),
            occurred_at(),
            &environment(),
        );

        // Assert
        assert_eq!(report.stack, Some("at <home>/app/src/rip.rs:12".to_owned()));
    }

    #[test]
    fn should_leave_text_holding_no_home_directory_unchanged() {
        // Act
        let report = ErrorReport::new(
            1,
            "no such disc in /Volumes/Audio",
            None,
            occurred_at(),
            &environment(),
        );

        // Assert
        assert_eq!(report.message, "no such disc in /Volumes/Audio");
    }

    #[test]
    fn should_leave_text_unchanged_when_the_home_directory_is_unknown() {
        // Arrange
        let environment = Environment {
            home_directory: String::new(),
            ..environment()
        };

        // Act
        let report = ErrorReport::new(1, "a message", None, occurred_at(), &environment);

        // Assert
        assert_eq!(report.message, "a message");
    }

    #[test]
    fn should_give_each_pending_report_a_distinct_id() {
        // Arrange
        let pending = PendingReports::default();

        // Act
        let first = pending.add("first", None, occurred_at(), &environment());
        let second = pending.add("second", None, occurred_at(), &environment());

        // Assert
        assert_ne!(first.id, second.id);
    }

    #[test]
    fn should_hand_back_the_pending_report_that_was_added() {
        // Arrange
        let pending = PendingReports::default();
        let added = pending.add("a message", None, occurred_at(), &environment());

        // Act
        let taken = pending.take(added.id);

        // Assert
        assert_eq!(taken, Some(added));
    }

    #[test]
    fn should_hand_back_a_pending_report_only_once() {
        // Arrange
        let pending = PendingReports::default();
        let added = pending.add("a message", None, occurred_at(), &environment());
        pending.take(added.id);

        // Act
        let taken_again = pending.take(added.id);

        // Assert
        assert_eq!(taken_again, None);
    }

    #[test]
    fn should_hand_back_nothing_for_an_unknown_report() {
        // Arrange
        let pending = PendingReports::default();

        // Act
        let taken = pending.take(404);

        // Assert
        assert_eq!(taken, None);
    }
}
