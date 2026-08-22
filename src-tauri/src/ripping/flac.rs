use std::path::Path;

use flacenc::bitsink::MemSink;
use flacenc::component::BitRepr;
use flacenc::config;
use flacenc::error::Verify;
use flacenc::source::MemSource;

const SAMPLE_RATE: usize = 44_100;
const CHANNELS: usize = 2;
const BITS_PER_SAMPLE: usize = 16;

pub fn write_uncompressed(samples: &[i32], destination: &Path) -> Result<(), String> {
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

    let mut encoded = MemSink::<u8>::new();
    stream
        .write(&mut encoded)
        .map_err(|error| format!("the encoded track could not be laid out: {error}"))?;

    std::fs::write(destination, encoded.as_slice())
        .map_err(|error| format!("{} could not be written: {error}", destination.display()))
}
