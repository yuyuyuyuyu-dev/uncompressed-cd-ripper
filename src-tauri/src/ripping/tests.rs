use std::ffi::c_void;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use windows::Win32::Devices::Cdrom::{
    CDROM_TOC, CDROM_TOC_SESSION_DATA, IOCTL_CDROM_GET_LAST_SESSION, IOCTL_CDROM_RAW_READ,
    IOCTL_CDROM_READ_TOC, RAW_READ_INFO, TRACK_DATA,
};

use super::drive::SAMPLES_PER_SECTOR;
use super::win32::{Device, Drive};
use super::{rip, tracks, Flac};

const LEAD_IN: i32 = 150;
const LEAD_OUT: u8 = 0xAA;
const SECTORS_PER_SECOND: i32 = 75;
const BYTES_PER_ADDRESSED_SECTOR: i64 = 2048;
const SECTORS_BETWEEN_SESSIONS: i32 = 11_400;

const AUDIO: u8 = 0x10;
const DATA: u8 = 0x14;

const AUDIO_TRACKS_START: [i32; 11] = [
    150, 16320, 33855, 50175, 64515, 83657, 99050, 115787, 135232, 152885, 171725,
];
const AUDIO_SESSION_LEAD_OUT: i32 = 191212;
const RECORDED: Range<i32> = AUDIO_TRACKS_START[0]..AUDIO_SESSION_LEAD_OUT;
const DATA_TRACK_STARTS: i32 = AUDIO_SESSION_LEAD_OUT + SECTORS_BETWEEN_SESSIONS;
const DATA_TRACK_SECTORS: i32 = 30_000;
const DISC_LEAD_OUT: i32 = DATA_TRACK_STARTS + DATA_TRACK_SECTORS;

struct CdExtra;

impl Device for CdExtra {
    unsafe fn control(
        &self,
        code: u32,
        asked: *const c_void,
        _asked_size: usize,
        answer: *mut c_void,
        answer_size: usize,
    ) -> Result<usize, ()> {
        match code {
            IOCTL_CDROM_READ_TOC => unsafe { answered(answer, answer_size, table_of_contents()) },
            IOCTL_CDROM_GET_LAST_SESSION => unsafe { answered(answer, answer_size, sessions()) },
            IOCTL_CDROM_RAW_READ => {
                let asked = unsafe { asked.cast::<RAW_READ_INFO>().read_unaligned() };
                let address = (asked.DiskOffset / BYTES_PER_ADDRESSED_SECTOR) as i32 + LEAD_IN;

                if !RECORDED.contains(&address) {
                    return Err(());
                }

                unsafe { answered(answer, answer_size, recorded(address)) }
            }
            _ => Err(()),
        }
    }
}

unsafe fn answered<T>(answer: *mut c_void, answer_size: usize, value: T) -> Result<usize, ()> {
    if answer_size < size_of::<T>() {
        return Err(());
    }

    unsafe { answer.cast::<T>().write_unaligned(value) };

    Ok(size_of::<T>())
}

fn table_of_contents() -> CDROM_TOC {
    let mut toc = CDROM_TOC {
        Length: ((2 + 13 * size_of::<TRACK_DATA>()) as u16).to_be_bytes(),
        FirstTrack: 1,
        LastTrack: 12,
        ..Default::default()
    };

    for (at, address) in AUDIO_TRACKS_START.into_iter().enumerate() {
        toc.TrackData[at] = entry(at as u8 + 1, AUDIO, msf(address));
    }

    toc.TrackData[11] = entry(12, DATA, msf(DATA_TRACK_STARTS));
    toc.TrackData[12] = entry(LEAD_OUT, DATA, msf(DISC_LEAD_OUT));

    toc
}

fn sessions() -> CDROM_TOC_SESSION_DATA {
    CDROM_TOC_SESSION_DATA {
        Length: ((2 + size_of::<TRACK_DATA>()) as u16).to_be_bytes(),
        FirstCompleteSession: 1,
        LastCompleteSession: 2,
        TrackData: [entry(12, DATA, (DATA_TRACK_STARTS - LEAD_IN).to_be_bytes())],
    }
}

fn entry(number: u8, kind: u8, address: [u8; 4]) -> TRACK_DATA {
    TRACK_DATA {
        _bitfield: kind,
        TrackNumber: number,
        Address: address,
        ..Default::default()
    }
}

fn msf(address: i32) -> [u8; 4] {
    let seconds = address / SECTORS_PER_SECOND;

    [
        0,
        (seconds / 60) as u8,
        (seconds % 60) as u8,
        (address % SECTORS_PER_SECOND) as u8,
    ]
}

fn recorded(address: i32) -> [i16; SAMPLES_PER_SECTOR] {
    std::array::from_fn(|at| (address as usize * SAMPLES_PER_SECTOR + at) as i16)
}

fn destination(case: &str) -> PathBuf {
    let destination = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/ripped-by-the-tests")
        .join(case);

    let _ = fs::remove_dir_all(&destination);
    fs::create_dir_all(&destination).expect("the build directory can be written to");

    destination
}

fn decoded(file: &str) -> Vec<i32> {
    claxon::FlacReader::open(file)
        .expect("the ripped file is FLAC")
        .samples()
        .collect::<Result<_, _>>()
        .expect("the ripped file decodes")
}

#[test]
fn ripping_the_last_audio_track_of_a_cd_extra_leaves_out_the_gap_between_the_sessions() {
    // Arrange
    let drive = Drive::over(Box::new(CdExtra)).expect("the fake drive holds an audio CD");
    let destination = destination("the last audio track of a CD-Extra");
    let track: Vec<i32> = (AUDIO_TRACKS_START[10]..RECORDED.end)
        .flat_map(recorded)
        .map(i32::from)
        .collect();

    // Act
    let ripped = rip(
        &drive,
        11,
        &destination,
        None,
        0,
        &Flac,
        &crate::logging::Logger,
        |_| {},
    )
    .expect("the last audio track can be ripped");

    // Assert
    let file = decoded(&ripped.file);
    assert!(
        file == track,
        "the file holds {} samples where the track holds {}",
        file.len(),
        track.len()
    );
}

#[test]
fn listing_the_tracks_of_a_cd_extra_records_its_layout_for_an_error_report() {
    // Arrange
    let drive = Drive::over(Box::new(CdExtra)).expect("the fake drive holds an audio CD");

    // Act
    tracks(&drive, &crate::logging::Logger);

    // Assert
    let layout = "the disc's layout was read: 11 audio, then 1 data";
    assert!(crate::logging::breadcrumbs()
        .iter()
        .any(|breadcrumb| breadcrumb.message == layout));
}
