use std::path::Path;

use flacenc::bitsink::MemSink;
use flacenc::component::BitRepr;
use flacenc::config;
use flacenc::error::Verify;
use flacenc::source::MemSource;

// What a CD holds, and therefore what goes into the file unchanged.
const SAMPLE_RATE: usize = 44_100;
const CHANNELS: usize = 2;
const BITS_PER_SAMPLE: usize = 16;

/// Writes the samples as a FLAC file that carries them uncompressed.
///
/// FLAC stores each channel of each block as a subframe, and a verbatim
/// subframe is the one kind that holds the samples as they came in. The
/// encoder reaches for it only when nothing else fits, so "uncompressed" is
/// arranged by taking everything else away: no constant, fixed or LPC
/// subframes, and no coding of one channel against the other, which would
/// store a difference rather than the channel itself.
///
/// These are the settings the reference encoder spells as `-l 0
/// --disable-constant-subframes --disable-fixed-subframes`, and the result is
/// still a FLAC file: frame headers, checksums and the MD5 of the audio are
/// all there, so a player reads it like any other and the samples inside are
/// bit for bit what came off the disc.
pub fn write_uncompressed(samples: &[i32], destination: &Path) -> Result<(), String> {
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

    // The minimum block size a FLAC file states is the smallest of its blocks
    // *excluding the last one*, which is short whenever the audio does not
    // divide evenly. flacenc counts that last block in anyway, and the file
    // then says the two differ, which by the specification is how a file
    // declares that its block size varies. Every frame in it still says
    // otherwise, and a decoder that believes the header rather than the frames
    // gives up: Apple's does, so nothing on macOS would play what came out.
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
