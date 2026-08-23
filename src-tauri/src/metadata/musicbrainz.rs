use super::MetadataApi;

pub struct MusicBrainz;

// MusicBrainz asks every program using it to say what it is, and turns away
// the ones that will not. It also asks for no more than one request a second,
// which a person putting discs in a drive one at a time cannot reach.
const AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " ( https://github.com/yuyuyuyuyu-dev/uncompressed-cd-ripper )"
);

const NOT_FOUND: u16 = 404;

impl MetadataApi for MusicBrainz {
    fn get(&self, disc_id: &str) -> Result<Option<String>, String> {
        // The artists and the recordings have to be asked for by name, or the
        // answer carries a release with no one credited and no track titles.
        // Stubs are turned down: a stub is what somebody typed in without a
        // release behind it, and it comes back in a different shape.
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
