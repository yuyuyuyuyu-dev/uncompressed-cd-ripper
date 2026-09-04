use super::MetadataApi;

pub struct MusicBrainz;

const AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " ( https://github.com/yuyuyuyuyu-dev/uncompressed-cd-ripper )"
);

const NOT_FOUND: u16 = 404;

impl MetadataApi for MusicBrainz {
    fn get(&self, disc_id: &str) -> Result<Option<String>, String> {
        let url = format!(
            "https://musicbrainz.org/ws/2/discid/{disc_id}\
             ?inc=artist-credits+recordings&cdstubs=no&fmt=json"
        );

        let mut response = ureq::get(&url)
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
                "the disc could not be looked up: {}",
                response.status()
            ));
        }

        response
            .body_mut()
            .read_to_string()
            .map(Some)
            .map_err(|error| format!("the answer could not be read: {error}"))
    }
}
