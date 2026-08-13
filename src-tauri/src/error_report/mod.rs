use sentry_types::{Dsn, ParseDsnError};
use serde::{Deserialize, Serialize};
use specta::Type;

// The shape below is Sentry's event payload rather than a shape of our own.
// The frontend assembles it, shows it to the user and hands it over, and the
// only reason it is declared here is that Tauri Specta derives the frontend's
// type from it: one declaration, so the two cannot drift apart.
//
// Every field is spelled out and unknown ones are refused, which is what makes
// "the report holds nothing the user did not see" something the compiler
// enforces rather than something a test case hopes for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub struct ErrorReport {
    pub event_id: String,
    pub timestamp: String,
    pub platform: String,
    pub release: String,
    pub exception: Exceptions,
    pub contexts: Contexts,
    pub tags: Tags,
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub struct Exceptions {
    pub values: Vec<ThrownError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub struct ThrownError {
    #[serde(rename = "type")]
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub struct Contexts {
    pub os: OperatingSystem,
    pub device: Device,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub struct OperatingSystem {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub struct Device {
    pub arch: String,
}

// Sentry indexes tags and offers them as filters, which contexts and extra do
// not get. The operating system needs no entry here: Sentry builds an `os` tag
// out of the context by itself, and repeating it only widens the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub struct Tags {
    pub architecture: String,
}

// Sentry's stacktrace field takes parsed frames rather than the string a
// JavaScript error carries, so the raw text goes here instead. Frames can
// replace it later without the rest of the payload moving.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub struct Extra {
    pub stacktrace: String,
    pub comment: String,
}

// What the frontend cannot find out for itself. A WebView reports the OS
// through its user agent, which on macOS has been frozen at 10_15_7 for years
// and calls Apple Silicon machines Intel, so the answers have to come from
// this side. Answering a question is not the same as adding to the report:
// the frontend decides what of this ends up in one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub release: String,
    pub os_name: String,
    pub os_version: String,
    pub architecture: String,
}

impl Environment {
    pub fn current() -> Self {
        let os = os_info::get();

        Self {
            release: format!("{}@{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
            os_name: os.os_type().to_string(),
            os_version: os.version().to_string(),
            architecture: std::env::consts::ARCH.to_owned(),
        }
    }
}

pub trait ReportApi {
    fn post(&self, body: &str) -> Result<(), String>;
}

pub fn send(report: &ErrorReport, api: &impl ReportApi) -> Result<(), String> {
    let body = serde_json::to_string(report).map_err(|error| error.to_string())?;

    api.post(&body)
}

pub struct Sentry {
    dsn: Dsn,
}

impl Sentry {
    // The DSN arrives at compile time from the release workflow, so a build
    // nobody published, a fork's included, carries no destination and cannot
    // reach the project.
    pub fn configured() -> Result<Self, String> {
        let dsn = option_env!("SENTRY_DSN")
            .ok_or_else(|| "this build carries no error reporting destination".to_owned())?;

        Ok(Self {
            dsn: dsn
                .parse()
                .map_err(|error: ParseDsnError| error.to_string())?,
        })
    }
}

impl ReportApi for Sentry {
    fn post(&self, body: &str) -> Result<(), String> {
        // The store endpoint takes the event as the whole request body, which
        // is what lets the bytes the user agreed to travel without a wrapper
        // built around them here.
        ureq::post(self.dsn.store_api_url().as_str())
            .header("X-Sentry-Auth", self.dsn.to_auth(None).to_string())
            .header("Content-Type", "application/json")
            .send(body)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct FakeApi {
        posted: RefCell<Vec<String>>,
    }

    impl ReportApi for FakeApi {
        fn post(&self, body: &str) -> Result<(), String> {
            self.posted.borrow_mut().push(body.to_owned());

            Ok(())
        }
    }

    fn report() -> ErrorReport {
        ErrorReport {
            event_id: "fc6d8c0c43fc4630ad850ee518f1b9d0".to_owned(),
            timestamp: "2026-08-12T09:00:00Z".to_owned(),
            platform: "javascript".to_owned(),
            release: "uncompressed-cd-ripper@0.1.0".to_owned(),
            exception: Exceptions {
                values: vec![ThrownError {
                    kind: "TypeError".to_owned(),
                    value: "cannot read properties of undefined".to_owned(),
                }],
            },
            contexts: Contexts {
                os: OperatingSystem {
                    name: "macOS".to_owned(),
                    version: "26.6.1".to_owned(),
                },
                device: Device {
                    arch: "aarch64".to_owned(),
                },
            },
            tags: Tags {
                architecture: "aarch64".to_owned(),
            },
            extra: Extra {
                stacktrace: "at rip (index.js:1:1)".to_owned(),
                comment: "it stopped on the third track".to_owned(),
            },
        }
    }

    #[test]
    fn should_hand_the_error_report_it_was_given_to_the_api_unchanged() {
        // Arrange
        let api = FakeApi::default();
        let report = report();

        // Act
        send(&report, &api).expect("the fake accepts everything");

        // Assert
        let expected = serde_json::to_string(&report).expect("the report serialises");
        assert_eq!(api.posted.borrow().as_slice(), [expected]);
    }
}
