use std::ops::RangeInclusive;
use std::ptr;

use windows::core::HSTRING;
use windows::Win32::Devices::Cdrom::{
    CDDA, CDROM_TOC, IOCTL_CDROM_RAW_READ, IOCTL_CDROM_READ_TOC, RAW_READ_INFO, TRACK_DATA,
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

    let handle = unsafe {
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
    .map_err(|_| stuck())?;

    let ejected = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_EJECT_MEDIA,
            None,
            0,
            None,
            0,
            None,
            None,
        )
    };

    let _ = unsafe { CloseHandle(handle) };

    ejected.map_err(|_| stuck())
}

pub struct Drive {
    handle: HANDLE,
}

impl Drive {
    pub fn open(device: &str) -> Result<Self, String> {
        let absent = || format!("{device} is not a drive with an audio CD in it");

        let handle = unsafe {
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
        .map_err(|_| absent())?;

        let drive = Self { handle };

        if !drive
            .tracks()
            .is_ok_and(|tracks| tracks.iter().any(|track| track.audio))
        {
            return Err(absent());
        }

        Ok(drive)
    }

    pub fn hardware(&self) -> Result<Hardware, String> {
        let unknown = || "the drive will not say what it is".to_owned();

        let asked = STORAGE_PROPERTY_QUERY {
            PropertyId: StorageDeviceProperty,
            QueryType: PropertyStandardQuery,
            AdditionalParameters: [0],
        };

        let mut answer = [0u8; ROOM_FOR_AN_ANSWER];
        let mut answered = 0;

        unsafe {
            DeviceIoControl(
                self.handle,
                IOCTL_STORAGE_QUERY_PROPERTY,
                Some(ptr::from_ref(&asked).cast()),
                size_of::<STORAGE_PROPERTY_QUERY>() as u32,
                Some(answer.as_mut_ptr().cast()),
                answer.len() as u32,
                Some(&mut answered),
                None,
            )
        }
        .map_err(|_| unknown())?;

        let answer = answer
            .get(..answered as usize)
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

        unsafe {
            DeviceIoControl(
                self.handle,
                IOCTL_CDROM_READ_TOC,
                None,
                0,
                Some(ptr::from_mut(&mut toc).cast()),
                size_of::<CDROM_TOC>() as u32,
                None,
                None,
            )
        }
        .map_err(|_| unknown())?;

        if toc.FirstTrack == 0 || toc.LastTrack < toc.FirstTrack {
            return Err(unknown());
        }

        let last = usize::from(toc.LastTrack - toc.FirstTrack) + 1;

        if last >= toc.TrackData.len() || toc.TrackData[last].TrackNumber != LEAD_OUT {
            return Err(unknown());
        }

        Ok(toc.TrackData[..=last]
            .windows(2)
            .map(|pair| ReportedTrack {
                number: pair[0].TrackNumber,
                audio: pair[0]._bitfield & DATA == 0,
                first: sector(pair[0]),
                last: sector(pair[1]) - 1,
            })
            .collect())
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

impl Drop for Drive {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.handle) };
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
        let mut answered = 0;

        unsafe {
            DeviceIoControl(
                self.drive.handle,
                IOCTL_CDROM_RAW_READ,
                Some(ptr::from_ref(&asked).cast()),
                size_of::<RAW_READ_INFO>() as u32,
                Some(self.samples.as_mut_ptr().cast()),
                BYTES_PER_SECTOR as u32,
                Some(&mut answered),
                None,
            )
        }
        .map_err(|_| unreadable())?;

        if answered as usize != BYTES_PER_SECTOR {
            return Err(unreadable());
        }

        Ok(&self.samples)
    }
}
