use std::path::Path;

use flacenc::bitsink::MemSink;
use flacenc::component::{BitRepr, MetadataBlockData};
use flacenc::config;
use flacenc::error::Verify;
use flacenc::source::MemSource;

use super::{Encoder, TrackTags};
use crate::artwork::{self, Artwork};

const SAMPLE_RATE: usize = 44_100;
const CHANNELS: usize = 2;
const BITS_PER_SAMPLE: usize = 16;

const VORBIS_COMMENT: u8 = 4;

const PICTURE: u8 = 6;

const FRONT_COVER: u32 = 3;

const ROOM_IN_A_BLOCK: usize = (1 << 24) - 1;

fn vorbis_comment(tags: &TrackTags, number: u8) -> Vec<u8> {
    let fields: Vec<String> = [
        tags.title.as_ref().map(|title| format!("TITLE={title}")),
        tags.artist
            .as_ref()
            .map(|artist| format!("ARTIST={artist}")),
        tags.album.as_ref().map(|album| format!("ALBUM={album}")),
        tags.album_artist
            .as_ref()
            .map(|artist| format!("ALBUMARTIST={artist}")),
        Some(format!("TRACKNUMBER={number}")),
    ]
    .into_iter()
    .flatten()
    .collect();

    fn counted(text: &str, block: &mut Vec<u8>) {
        block.extend((text.len() as u32).to_le_bytes());
        block.extend(text.as_bytes());
    }

    let mut block = Vec::new();

    counted(
        concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION")),
        &mut block,
    );
    block.extend((fields.len() as u32).to_le_bytes());

    for field in &fields {
        counted(field, &mut block);
    }

    block
}

fn picture(artwork: &Artwork) -> Result<Vec<u8>, String> {
    let image = artwork.image()?;
    let dimensions = artwork::measured(&artwork.media_type, &image);

    fn counted(bytes: &[u8], block: &mut Vec<u8>) {
        block.extend((bytes.len() as u32).to_be_bytes());
        block.extend(bytes);
    }

    let mut block = Vec::new();

    block.extend(FRONT_COVER.to_be_bytes());
    counted(artwork.media_type.as_bytes(), &mut block);
    counted(b"", &mut block);
    block.extend(dimensions.width.to_be_bytes());
    block.extend(dimensions.height.to_be_bytes());
    block.extend(dimensions.depth.to_be_bytes());
    block.extend(0u32.to_be_bytes());
    counted(&image, &mut block);

    if block.len() > ROOM_IN_A_BLOCK {
        return Err("the album artwork is too large to write into the file".to_owned());
    }

    Ok(block)
}

pub struct Flac;

impl Encoder for Flac {
    fn write(
        &self,
        samples: &[i32],
        destination: &Path,
        number: u8,
        tags: Option<&TrackTags>,
    ) -> Result<(), String> {
        write_uncompressed(samples, destination, number, tags)
    }
}

fn write_uncompressed(
    samples: &[i32],
    destination: &Path,
    number: u8,
    tags: Option<&TrackTags>,
) -> Result<(), String> {
    let mut config = config::Encoder::default();

    config.subframe_coding.use_constant = false;
    config.subframe_coding.use_fixed = false;
    config.subframe_coding.use_lpc = false;
    config.stereo_coding.use_leftside = false;
    config.stereo_coding.use_rightside = false;
    config.stereo_coding.use_midside = false;

    let config = config
        .into_verified()
        .map_err(|(_, error)| format!("the encoder was set up wrongly: {error}"))?;

    let source = MemSource::from_samples(samples, CHANNELS, BITS_PER_SAMPLE, SAMPLE_RATE);
    let mut stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|error| format!("the track could not be encoded: {error}"))?;

    stream
        .stream_info_mut()
        .set_block_sizes(config.block_size, config.block_size)
        .map_err(|error| format!("the block size was rejected: {error}"))?;

    if let Some(tags) = tags {
        let block = MetadataBlockData::new_unknown(VORBIS_COMMENT, &vorbis_comment(tags, number))
            .map_err(|error| format!("the tags were rejected: {error}"))?;

        stream.add_metadata_block(block);
    }

    if let Some(artwork) = tags.and_then(|tags| tags.artwork.as_ref()) {
        let block = MetadataBlockData::new_unknown(PICTURE, &picture(artwork)?)
            .map_err(|error| format!("the album artwork was rejected: {error}"))?;

        stream.add_metadata_block(block);
    }

    let mut encoded = MemSink::<u8>::new();
    stream
        .write(&mut encoded)
        .map_err(|error| format!("the encoded track could not be laid out: {error}"))?;

    std::fs::write(destination, encoded.as_slice())
        .map_err(|error| format!("{} could not be written: {error}", destination.display()))
}
