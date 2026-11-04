use std::ffi::c_void;
use std::ops::RangeInclusive;
use std::ptr;

use windows::core::HSTRING;
use windows::Win32::Devices::Cdrom::{
    CDDA, CDROM_TOC, CDROM_TOC_SESSION_DATA, IOCTL_CDROM_GET_LAST_SESSION, IOCTL_CDROM_RAW_READ,
    IOCTL_CDROM_READ_TOC, RAW_READ_INFO, TRACK_DATA,
};
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, GetDriveTypeW, GetLogicalDrives, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Ioctl::{
    PropertyStandardQuery, StorageDeviceProperty, IOCTL_STORAGE_EJECT_MEDIA,
    IOCTL_STORAGE_QUERY_PROPERTY, STORAGE_DEVICE_DESCRIPTOR, STORAGE_PROPERTY_QUERY,
};
use windows::Win32::System::WindowsProgramming::DRIVE_CDROM;
use windows::Win32::System::IO::DeviceIoControl;

use super::drive::{named, Hardware, ReportedTrack, BYTES_PER_SECTOR, SAMPLES_PER_SECTOR};
use super::LEAD_IN;

const SECONDS_PER_MINUTE: i32 = 60;
const SECTORS_PER_SECOND: i32 = 75;

const LEAD_OUT_SECONDS: i32 = 90;
const LEAD_IN_SECONDS: i32 = 60;
const PREGAP_SECONDS: i32 = 2;
const SECTORS_BETWEEN_SESSIONS: i32 =
    (LEAD_OUT_SECONDS + LEAD_IN_SECONDS + PREGAP_SECONDS) * SECTORS_PER_SECOND;

const LEAD_OUT: u8 = 0xAA;

const DATA: u8 = 0b0000_0100;

const BYTES_PER_ADDRESSED_SECTOR: i64 = 2048;

const ROOM_FOR_AN_ANSWER: usize = 1024;

pub fn holding_an_audio_disc() -> Vec<String> {
    let mounted = unsafe { GetLogicalDrives() };

    (b'A'..=b'Z')
        .enumerate()
        .filter(|(letter, _)| mounted & (1 << letter) != 0)
        .map(|(_, letter)| format!("{}:", char::from(letter)))
        .filter(|device| unsafe {
            GetDriveTypeW(&HSTRING::from(format!("{device}\\"))) == DRIVE_CDROM
        })
        .filter(|device| Drive::open(device).is_ok())
        .collect()
}

pub fn eject_disc(device: &str) -> Result<(), String> {
    let stuck = || format!("{device} would not eject its disc");

    let handle = Handle::open(device).map_err(|_| stuck())?;

    let ejected = unsafe {
        handle.control(
            IOCTL_STORAGE_EJECT_MEDIA,
            ptr::null(),
            0,
            ptr::null_mut(),
            0,
        )
    };

    ejected.map(|_| ()).map_err(|_| stuck())
}

pub(super) trait Device {
    unsafe fn control(
        &self,
        code: u32,
        asked: *const c_void,
        asked_size: usize,
        answer: *mut c_void,
        answer_size: usize,
    ) -> Result<usize, ()>;
}

struct Handle(HANDLE);

impl Handle {
    fn open(device: &str) -> windows::core::Result<Self> {
        unsafe {
            CreateFileW(
                &HSTRING::from(format!(r"\\.\{device}")),
                GENERIC_READ.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        }
        .map(Self)
    }
}

impl Device for Handle {
    unsafe fn control(
        &self,
        code: u32,
        asked: *const c_void,
        asked_size: usize,
        answer: *mut c_void,
        answer_size: usize,
    ) -> Result<usize, ()> {
        let mut answered = 0;

        unsafe {
            DeviceIoControl(
                self.0,
                code,
                (asked_size > 0).then_some(asked),
                asked_size as u32,
                (answer_size > 0).then_some(answer),
                answer_size as u32,
                Some(&mut answered),
                None,
            )
        }
        .map_err(|_| ())?;

        Ok(answered as usize)
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

pub struct Drive {
    device: Box<dyn Device>,
}

impl Drive {
    pub fn open(device: &str) -> Result<Self, String> {
        let absent = || format!("{device} is not a drive with an audio CD in it");

        let handle = Handle::open(device).map_err(|_| absent())?;

        Self::over(Box::new(handle)).map_err(|_| absent())
    }

    pub(super) fn over(device: Box<dyn Device>) -> Result<Self, String> {
        let drive = Self { device };

        if !drive
            .tracks()
            .is_ok_and(|tracks| tracks.iter().any(|track| track.audio))
        {
            return Err("the disc holds no audio".to_owned());
        }

        Ok(drive)
    }

    fn ask<Asked, Answer>(
        &self,
        code: u32,
        asked: &Asked,
        answer: &mut Answer,
    ) -> Result<usize, ()> {
        unsafe {
            self.device.control(
                code,
                ptr::from_ref(asked).cast(),
                size_of::<Asked>(),
                ptr::from_mut(answer).cast(),
                size_of::<Answer>(),
            )
        }
    }

    pub fn hardware(&self) -> Result<Hardware, String> {
        let unknown = || "the drive will not say what it is".to_owned();

        let asked = STORAGE_PROPERTY_QUERY {
            PropertyId: StorageDeviceProperty,
            QueryType: PropertyStandardQuery,
            AdditionalParameters: [0],
        };

        let mut answer = [0u8; ROOM_FOR_AN_ANSWER];

        let answered = self
            .ask(IOCTL_STORAGE_QUERY_PROPERTY, &asked, &mut answer)
            .map_err(|_| unknown())?;

        let answer = answer
            .get(..answered)
            .filter(|answer| answer.len() >= size_of::<STORAGE_DEVICE_DESCRIPTOR>())
            .ok_or_else(unknown)?;

        let described = unsafe {
            answer
                .as_ptr()
                .cast::<STORAGE_DEVICE_DESCRIPTOR>()
                .read_unaligned()
        };

        Ok(Hardware {
            vendor: named(&text_at(answer, described.VendorIdOffset)),
            model: named(&text_at(answer, described.ProductIdOffset)),
        })
    }

    pub(super) fn tracks(&self) -> Result<Vec<ReportedTrack>, String> {
        let unknown = || "the drive will not say what is on the disc".to_owned();
        let mut toc = CDROM_TOC::default();

        self.ask(IOCTL_CDROM_READ_TOC, &(), &mut toc)
            .map_err(|_| unknown())?;

        if toc.FirstTrack == 0 || toc.LastTrack < toc.FirstTrack {
            return Err(unknown());
        }

        let last = usize::from(toc.LastTrack - toc.FirstTrack) + 1;

        if last >= toc.TrackData.len() || toc.TrackData[last].TrackNumber != LEAD_OUT {
            return Err(unknown());
        }

        let mut tracks: Vec<ReportedTrack> = toc.TrackData[..=last]
            .windows(2)
            .map(|pair| ReportedTrack {
                number: pair[0].TrackNumber,
                audio: pair[0]._bitfield & DATA == 0,
                first: sector(pair[0]),
                last: sector(pair[1]) - 1,
            })
            .collect();

        if let Some(starts) = self.last_session() {
            let gap_begins = starts - SECTORS_BETWEEN_SESSIONS;
            let last_audio = tracks.windows(2).rposition(|pair| {
                pair[0].audio
                    && !pair[1].audio
                    && (pair[0].first..pair[1].first).contains(&gap_begins)
            });

            if let Some(at) = last_audio {
                tracks[at].last = gap_begins - 1;
            }
        }

        Ok(tracks)
    }

    fn last_session(&self) -> Option<i32> {
        let mut sessions = CDROM_TOC_SESSION_DATA::default();

        self.ask(IOCTL_CDROM_GET_LAST_SESSION, &(), &mut sessions)
            .ok()?;

        (sessions.FirstCompleteSession != sessions.LastCompleteSession)
            .then(|| i32::from_be_bytes(sessions.TrackData[0].Address))
    }

    pub(super) fn recorded(&self) -> Result<RangeInclusive<i32>, String> {
        let tracks = self.tracks()?;
        let nothing = || "the disc holds no audio".to_owned();

        let first = tracks.iter().find(|track| track.audio);
        let last = tracks.iter().rfind(|track| track.audio);

        match (first, last) {
            (Some(first), Some(last)) => Ok(first.first..=last.last),
            _ => Err(nothing()),
        }
    }

    pub(super) fn reading(&self) -> Result<Reading<'_>, String> {
        Ok(Reading {
            drive: self,
            samples: [0; SAMPLES_PER_SECTOR],
        })
    }
}

fn sector(entry: TRACK_DATA) -> i32 {
    let [_, minutes, seconds, sectors] = entry.Address;

    (i32::from(minutes) * SECONDS_PER_MINUTE + i32::from(seconds)) * SECTORS_PER_SECOND
        + i32::from(sectors)
        - LEAD_IN as i32
}

fn text_at(answer: &[u8], offset: u32) -> String {
    if offset == 0 {
        return String::new();
    }

    let Some(text) = answer.get(offset as usize..) else {
        return String::new();
    };

    let end = text
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(text.len());

    String::from_utf8_lossy(&text[..end]).into_owned()
}

pub(super) struct Reading<'drive> {
    drive: &'drive Drive,
    samples: [i16; SAMPLES_PER_SECTOR],
}

impl Reading<'_> {
    pub(super) fn read(&mut self, sector: i32) -> Result<&[i16], String> {
        let unreadable = || format!("sector {sector} could not be read");

        let asked = RAW_READ_INFO {
            DiskOffset: i64::from(sector) * BYTES_PER_ADDRESSED_SECTOR,
            SectorCount: 1,
            TrackMode: CDDA,
        };
        let answered = self
            .drive
            .ask(IOCTL_CDROM_RAW_READ, &asked, &mut self.samples)
            .map_err(|_| unreadable())?;

        if answered != BYTES_PER_SECTOR {
            return Err(unreadable());
        }

        Ok(&self.samples)
    }
}
