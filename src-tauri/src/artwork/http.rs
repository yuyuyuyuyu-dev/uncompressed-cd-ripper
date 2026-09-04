use super::{Answer, Failed, Http};

pub struct Ureq;

const AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " ( https://github.com/yuyuyuyuyu-dev/uncompressed-cd-ripper )"
);

impl Http for Ureq {
    fn get(&self, url: &str, within: u64) -> Result<Answer, Failed> {
        let mut response = ureq::get(url)
            .header("User-Agent", AGENT)
            .config()
            .http_status_as_error(false)
            .build()
            .call()
            .map_err(|error| Failed::Reason(error.to_string()))?;

        let status = response.status().as_u16();

        let content_type = response
            .headers()
            .get("Content-Type")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);

        let body = response
            .body_mut()
            .with_config()
            .limit(within)
            .read_to_vec()
            .map_err(|error| match error {
                ureq::Error::BodyExceedsLimit(_) => Failed::TooLong,
                error => Failed::Reason(error.to_string()),
            })?;

        Ok(Answer {
            status,
            content_type,
            body,
        })
    }
}
