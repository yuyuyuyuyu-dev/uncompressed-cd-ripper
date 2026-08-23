use std::path::Path;

use flacenc::bitsink::MemSink;
use flacenc::component::{BitRepr, MetadataBlockData};
use flacenc::config;
use flacenc::error::Verify;
use flacenc::source::MemSource;

use super::TrackTags;
use crate::artwork::{self, Cover};

const SAMPLE_RATE: usize = 44_100;
const CHANNELS: usize = 2;
const BITS_PER_SAMPLE: usize = 16;

// The number FLAC gives the block a player reads an album and a title out of.
const VORBIS_COMMENT: u8 = 4;

// The number it gives the block a player takes a picture out of.
const PICTURE: u8 = 6;

// Which of the pictures a release can carry this one is. Three is the front of
// the sleeve, and it is the one a player shows beside the track.
const FRONT_COVER: u32 = 3;

// A block states how long it is in twenty-four bits, and the picture goes into
// one whole. Past this the length would be written wrapped round and the file
// would not open, so it is refused instead.
const ROOM_IN_A_BLOCK: usize = (1 << 24) - 1;

// What is written into the block, and the order a player is used to seeing.
// The names are the ones the Vorbis comment specification settled on, which is
// what makes the tags show up rather than sit there unread.
//
// A field nobody filled in is left out rather than written empty, because a
// player that finds an empty title shows a blank line where one that finds no
// title falls back to the name of the file.
fn vorbis_comment(tags: &TrackTags, number: u8) -> Vec<u8> {
    let fields: Vec<String> = [
        tags.title.as_ref().map(|title| format!("TITLE={title}")),
        tags.artist
            .as_ref()
            .map(|artist| format!("ARTIST={artist}")),
        tags.album.as_ref().map(|album| format!("ALBUM={album}")),
        // One word, which is what Picard writes and what most of what reads a
        // library goes looking for. The spelling with a space also exists, and
        // writing both would leave a tag editor showing the album's artist
        // twice.
        tags.album_artist
            .as_ref()
            .map(|artist| format!("ALBUMARTIST={artist}")),
        Some(format!("TRACKNUMBER={number}")),
    ]
    .into_iter()
    .flatten()
    .collect();

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

// Unlike the Vorbis comment, every length here is written big-endian, which is
// the way round the rest of the file is.
fn picture(cover: &Cover) -> Result<Vec<u8>, String> {
    let image = cover.image()?;
    let dimensions = artwork::measured(&cover.media_type, &image);

    fn counted(bytes: &[u8], block: &mut Vec<u8>) {
        block.extend((bytes.len() as u32).to_be_bytes());
        block.extend(bytes);
    }

    let mut block = Vec::new();

    block.extend(FRONT_COVER.to_be_bytes());
    counted(cover.media_type.as_bytes(), &mut block);
    // A description is what tells one picture from another where a file
    // carries several, and this one carries the sleeve alone.
    counted(b"", &mut block);
    block.extend(dimensions.width.to_be_bytes());
    block.extend(dimensions.height.to_be_bytes());
    block.extend(dimensions.depth.to_be_bytes());
    // How large a palette the image was drawn from. Nothing that is measured
    // here is drawn from one.
    block.extend(0u32.to_be_bytes());
    counted(&image, &mut block);

    if block.len() > ROOM_IN_A_BLOCK {
        return Err("the cover art is too large to write into the file".to_owned());
    }

    Ok(block)
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

    if let Some(cover) = tags.and_then(|tags| tags.cover.as_ref()) {
        let block = MetadataBlockData::new_unknown(PICTURE, &picture(cover)?)
            .map_err(|error| format!("the cover art was rejected: {error}"))?;

        stream.add_metadata_block(block);
    }

    let mut encoded = MemSink::<u8>::new();
    stream
        .write(&mut encoded)
        .map_err(|error| format!("the encoded track could not be laid out: {error}"))?;

    std::fs::write(destination, encoded.as_slice())
        .map_err(|error| format!("{} could not be written: {error}", destination.display()))
}
