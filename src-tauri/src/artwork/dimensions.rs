use super::PNG;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

pub fn measured(media_type: &str, image: &[u8]) -> Dimensions {
    match media_type {
        "image/jpeg" => jpeg(image),
        "image/png" => png(image),
        _ => Dimensions::default(),
    }
}

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

const FRAMES: [u8; 13] = [
    0xC0, 0xC1, 0xC2, 0xC3, 0xC5, 0xC6, 0xC7, 0xC9, 0xCA, 0xCB, 0xCD, 0xCE, 0xCF,
];

const SCAN: u8 = 0xDA;

const ALONE: std::ops::RangeInclusive<u8> = 0xD0..=0xD9;

fn jpeg(image: &[u8]) -> Dimensions {
    if !image.starts_with(&[0xFF, 0xD8]) {
        return Dimensions::default();
    }

    let mut at = 2;

    loop {
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
