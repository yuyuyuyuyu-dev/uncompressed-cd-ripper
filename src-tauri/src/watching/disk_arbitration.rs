use std::ffi::c_void;
use std::ptr;

use core_foundation_sys::base::{kCFAllocatorDefault, CFAllocatorRef, CFRelease, CFTypeRef};
use core_foundation_sys::dictionary::CFDictionaryRef;
use core_foundation_sys::runloop::{
    kCFRunLoopDefaultMode, CFRunLoopGetCurrent, CFRunLoopRef, CFRunLoopRun,
};
use core_foundation_sys::string::CFStringRef;

use super::Media;

type DASessionRef = CFTypeRef;
type DADiskRef = CFTypeRef;
type DADiskCallback = extern "C" fn(disk: DADiskRef, context: *mut c_void);

#[link(name = "DiskArbitration", kind = "framework")]
extern "C" {
    fn DASessionCreate(allocator: CFAllocatorRef) -> DASessionRef;

    fn DASessionScheduleWithRunLoop(
        session: DASessionRef,
        run_loop: CFRunLoopRef,
        mode: CFStringRef,
    );

    fn DASessionUnscheduleFromRunLoop(
        session: DASessionRef,
        run_loop: CFRunLoopRef,
        mode: CFStringRef,
    );

    fn DARegisterDiskAppearedCallback(
        session: DASessionRef,
        matching: CFDictionaryRef,
        callback: DADiskCallback,
        context: *mut c_void,
    );

    fn DARegisterDiskDisappearedCallback(
        session: DASessionRef,
        matching: CFDictionaryRef,
        callback: DADiskCallback,
        context: *mut c_void,
    );
}

pub struct Drives;

impl Media for Drives {
    fn watch(&self, mut react: impl FnMut()) -> Result<(), String> {
        let session = unsafe { DASessionCreate(kCFAllocatorDefault) };

        if session.is_null() {
            return Err("the drives could not be watched".to_owned());
        }

        let mut reacting: &mut dyn FnMut() = &mut react;
        let context = ptr::from_mut(&mut reacting).cast::<c_void>();

        unsafe {
            DARegisterDiskAppearedCallback(session, ptr::null(), changed, context);
            DARegisterDiskDisappearedCallback(session, ptr::null(), changed, context);
            DASessionScheduleWithRunLoop(session, CFRunLoopGetCurrent(), kCFRunLoopDefaultMode);

            CFRunLoopRun();

            DASessionUnscheduleFromRunLoop(session, CFRunLoopGetCurrent(), kCFRunLoopDefaultMode);
            CFRelease(session);
        }

        Ok(())
    }
}

extern "C" fn changed(_disk: DADiskRef, context: *mut c_void) {
    let react = unsafe { &mut *context.cast::<&mut dyn FnMut()>() };

    react();
}
