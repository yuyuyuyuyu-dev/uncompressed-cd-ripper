use super::VerificationApi;

pub struct AccurateRip;

const AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " ( https://github.com/yuyuyuyuyu-dev/uncompressed-cd-ripper )"
);

const NOT_FOUND: u16 = 404;

const ROOM_FOR_AN_ANSWER: u64 = 8 * 1024 * 1024;

impl VerificationApi for AccurateRip {
    fn get(&self, url: &str) -> Result<Option<Vec<u8>>, String> {
        let mut response = ureq::get(url)
            .header("User-Agent", AGENT)
            .config()
            .http_status_as_error(false)
            .build()
            .call()
            .map_err(|error| error.to_string())?;

        if response.status() == NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            return Err(format!(
                "AccurateRip could not be asked: {}",
                response.status()
            ));
        }

        response
            .body_mut()
            .with_config()
            .limit(ROOM_FOR_AN_ANSWER)
            .read_to_vec()
            .map(Some)
            .map_err(|error| format!("what AccurateRip sent back could not be read: {error}"))
    }
}
