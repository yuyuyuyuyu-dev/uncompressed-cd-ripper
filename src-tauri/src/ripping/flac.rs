use std::path::Path;

use flacenc::bitsink::MemSink;
use flacenc::component::{BitRepr, MetadataBlockData};
use flacenc::config;
use flacenc::error::Verify;
use flacenc::source::MemSource;

use super::TrackTags;

const SAMPLE_RATE: usize = 44_100;
const CHANNELS: usize = 2;
const BITS_PER_SAMPLE: usize = 16;

// The number FLAC gives the block a player reads an album and a title out of.
const VORBIS_COMMENT: u8 = 4;

// What is written into the block, and the order a player is used to seeing.
// The names are the ones the Vorbis comment specification settled on, which is
// what makes the tags show up rather than sit there unread.
fn vorbis_comment(tags: &TrackTags, number: u8) -> Vec<u8> {
    let fields = [
        format!("TITLE={}", tags.title),
        format!("ARTIST={}", tags.artist),
        format!("ALBUM={}", tags.album),
        format!("TRACKNUMBER={number}"),
    ];

    // Every length in this one block is written the other way round from the
    // rest of the file, which is big-endian throughout. The block is a Vorbis
    // comment, and Vorbis counts the other way.
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

pub fn write_uncompressed(
    samples: &[i32],
    destination: &Path,
    number: u8,
    tags: Option<&TrackTags>,
) -> Result<(), String> {
    let mut config = config::Encoder::default();

    // A verbatim subframe is the one kind that holds samples as they came in,
    // and an encoder reaches for it only when nothing else fits, so
    // uncompressed is arranged by taking everything else away.
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

    // The minimum block size a file states excludes its last block, which is
    // short whenever the audio does not divide evenly. flacenc counts it in
    // anyway, and the file then declares a varying block size while every frame
    // says otherwise. Apple's decoder believes the header and refuses to play.
    stream
        .stream_info_mut()
        .set_block_sizes(config.block_size, config.block_size)
        .map_err(|error| format!("the block size was rejected: {error}"))?;

    if let Some(tags) = tags {
        let block = MetadataBlockData::new_unknown(VORBIS_COMMENT, &vorbis_comment(tags, number))
            .map_err(|error| format!("the tags were rejected: {error}"))?;

        stream.add_metadata_block(block);
    }

    let mut encoded = MemSink::<u8>::new();
    stream
        .write(&mut encoded)
        .map_err(|error| format!("the encoded track could not be laid out: {error}"))?;

    std::fs::write(destination, encoded.as_slice())
        .map_err(|error| format!("{} could not be written: {error}", destination.display()))
}
