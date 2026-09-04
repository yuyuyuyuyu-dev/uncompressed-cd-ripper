use sentry_types::{Dsn, ParseDsnError};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::logging::Breadcrumb;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub struct ErrorReport {
    pub event_id: String,
    pub timestamp: String,
    pub platform: String,
    pub release: String,
    pub exception: Exceptions,
    pub breadcrumbs: Breadcrumbs,
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
pub struct Breadcrumbs {
    pub values: Vec<Breadcrumb>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub struct Tags {
    pub architecture: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub struct Extra {
    pub stacktrace: String,
    pub component_stack: String,
    pub comment: String,
}

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

#[cfg(not(debug_assertions))]
const _: () = {
    if option_env!("SENTRY_DSN").is_none() {
        panic!("a release build needs SENTRY_DSN in its environment");
    }
};

pub struct Sentry {
    dsn: Dsn,
}

impl Sentry {
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
        let mut response = ureq::post(self.dsn.store_api_url().as_str())
            .header("X-Sentry-Auth", self.dsn.to_auth(None).to_string())
            .header("Content-Type", "application/json")
            .config()
            .http_status_as_error(false)
            .build()
            .send(body)
            .map_err(|error| error.to_string())?;

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status();
        let explanation = response.body_mut().read_to_string().unwrap_or_default();

        Err(if explanation.is_empty() {
            status.to_string()
        } else {
            format!("{status}: {explanation}")
        })
    }
}

#[cfg(test)]
mod tests;
