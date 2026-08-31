use std::cell::Cell;
use std::ffi::c_void;
use std::ptr;

use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW, CW_USEDEFAULT,
    DBTF_MEDIA, DBT_DEVICEARRIVAL, DBT_DEVICEREMOVECOMPLETE, DBT_DEVTYP_VOLUME, DEV_BROADCAST_HDR,
    DEV_BROADCAST_VOLUME, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WM_DEVICECHANGE, WNDCLASSW,
};

use super::Media;

const CLASS: &str = "UncompressedCdRipperWatchesTheDrives";

thread_local! {
    static REACTING: Cell<*mut c_void> = const { Cell::new(ptr::null_mut()) };
}

pub struct Drives;

impl Media for Drives {
    fn watch(&self, mut react: impl FnMut()) -> Result<(), String> {
        let unwatchable = || "the drives could not be watched".to_owned();

        let module = unsafe { GetModuleHandleW(None) }.map_err(|_| unwatchable())?;
        let name = HSTRING::from(CLASS);

        let class = WNDCLASSW {
            lpfnWndProc: Some(received),
            hInstance: module.into(),
            lpszClassName: PCWSTR(name.as_ptr()),
            ..Default::default()
        };

        if unsafe { RegisterClassW(&class) } == 0 {
            return Err(unwatchable());
        }

        let mut reacting: &mut dyn FnMut() = &mut react;
        REACTING.set(ptr::from_mut(&mut reacting).cast::<c_void>());

        unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR(name.as_ptr()),
                PCWSTR(name.as_ptr()),
                WINDOW_STYLE(0),
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                Some(module.into()),
                None,
            )
        }
        .map_err(|_| unwatchable())?;

        let mut message = MSG::default();

        while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
            unsafe { DispatchMessageW(&message) };
        }

        Ok(())
    }
}

unsafe extern "system" fn received(
    window: HWND,
    message: u32,
    carried: WPARAM,
    details: LPARAM,
) -> LRESULT {
    let arrived = carried.0 as u32 == DBT_DEVICEARRIVAL;
    let left = carried.0 as u32 == DBT_DEVICEREMOVECOMPLETE;

    if message == WM_DEVICECHANGE && (arrived || left) && media_changed(details) {
        let reacting = REACTING.get();

        if !reacting.is_null() {
            let react = unsafe { &mut *reacting.cast::<&mut dyn FnMut()>() };

            react();
        }
    }

    unsafe { DefWindowProcW(window, message, carried, details) }
}

fn media_changed(details: LPARAM) -> bool {
    let broadcast = details.0 as *const DEV_BROADCAST_HDR;

    if broadcast.is_null() {
        return false;
    }

    if unsafe { (*broadcast).dbch_devicetype } != DBT_DEVTYP_VOLUME {
        return false;
    }

    let volume = details.0 as *const DEV_BROADCAST_VOLUME;

    unsafe { (*volume).dbcv_flags.0 & DBTF_MEDIA.0 != 0 }
}
