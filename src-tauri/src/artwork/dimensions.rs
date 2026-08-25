use super::PNG;

// What a picture block states about the image beside carrying the image
// itself. A player draws the artwork from the image; these are what a tag
// editor can show without opening it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
    // Every channel counted, so a colour photograph is twenty-four.
    pub depth: u32,
}

// Zero throughout for anything else, which is what the fields hold for an
// image that was not measured. Only the two the archive serves are read: a
// JPEG for every copy it makes itself, and a PNG for a scan uploaded as one.
pub fn measured(media_type: &str, image: &[u8]) -> Dimensions {
    match media_type {
        "image/jpeg" => jpeg(image),
        "image/png" => png(image),
        _ => Dimensions::default(),
    }
}

// A PNG says it is one, and its first chunk is the header: the size, how many
// bits a channel holds, and which channels there are.
fn png(image: &[u8]) -> Dimensions {
    if !image.starts_with(&PNG) || image.get(12..16) != Some(b"IHDR") {
        return Dimensions::default();
    }

    let (Some(width), Some(height), Some(&bits), Some(&colour)) = (
        be32(image.get(16..20)),
        be32(image.get(20..24)),
        image.get(24),
        image.get(25),
    ) else {
        return Dimensions::default();
    };

    // Grey, grey with an alpha channel, three colours, or three with an alpha
    // channel. An image drawn from a palette is left unmeasured instead: the
    // block would have to state how large the palette is as well, and the
    // archive serves no such image.
    let channels = match colour {
        0 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => return Dimensions::default(),
    };

    Dimensions {
        width,
        height,
        depth: u32::from(bits) * channels,
    }
}

// The marks that begin a frame. Three numbers in the run are missing because
// they mark something else: a table of Huffman codes, one the format reserved,
// and a table for arithmetic coding.
const FRAMES: [u8; 13] = [
    0xC0, 0xC1, 0xC2, 0xC3, 0xC5, 0xC6, 0xC7, 0xC9, 0xCA, 0xCB, 0xCD, 0xCE, 0xCF,
];

// The mark that ends the segments and begins the compressed image, which is
// not laid out in segments and is not searched.
const SCAN: u8 = 0xDA;

// The marks that carry nothing after them: the start and the end of the image,
// and the ones that break the compressed part into pieces.
const ALONE: std::ops::RangeInclusive<u8> = 0xD0..=0xD9;

// A JPEG is a run of segments, each marked and most of them measured, and the
// one that describes the frame carries the size. Which mark that segment
// begins with depends on how the image is coded, and all of them lay the frame
// out the same way, so any of them will do.
fn jpeg(image: &[u8]) -> Dimensions {
    if !image.starts_with(&[0xFF, 0xD8]) {
        return Dimensions::default();
    }

    let mut at = 2;

    loop {
        // A mark is one or more 0xFF and then the byte that names it.
        while image.get(at) == Some(&0xFF) {
            at += 1;
        }

        let Some(&mark) = image.get(at) else {
            return Dimensions::default();
        };

        if FRAMES.contains(&mark) {
            let (Some(&precision), Some(height), Some(width), Some(&channels)) = (
                image.get(at + 3),
                be16(image.get(at + 4..at + 6)),
                be16(image.get(at + 6..at + 8)),
                image.get(at + 8),
            ) else {
                return Dimensions::default();
            };

            return Dimensions {
                width,
                height,
                depth: u32::from(precision) * u32::from(channels),
            };
        }

        if mark == SCAN {
            return Dimensions::default();
        }

        at = if ALONE.contains(&mark) || mark == 0x01 {
            at + 1
        } else {
            let Some(length) = be16(image.get(at + 1..at + 3)) else {
                return Dimensions::default();
            };

            at + 1 + length as usize
        };
    }
}

fn be16(bytes: Option<&[u8]>) -> Option<u32> {
    bytes
        .and_then(|bytes| <[u8; 2]>::try_from(bytes).ok())
        .map(|bytes| u32::from(u16::from_be_bytes(bytes)))
}

fn be32(bytes: Option<&[u8]>) -> Option<u32> {
    bytes
        .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
        .map(u32::from_be_bytes)
}
