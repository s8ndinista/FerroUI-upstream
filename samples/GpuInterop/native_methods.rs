//! Port of `NativeMethods.cs`: the functions of IOSurface and Core Foundation the Vulkan demo
//! reads an IOSurface with on macOS, and `close` of the C library.

#![allow(dead_code)]

/// `IOSurfaceLockOptions`.
pub struct IOSurfaceLockOptions;

impl IOSurfaceLockOptions {
    pub const NONE: u32 = 0;
    pub const READ_ONLY: u32 = 1 << 0;
    pub const AVOID_SYNC: u32 = 1 << 1;
}

/// `IOSurfaceRef`.
#[cfg(target_os = "macos")]
pub type IOSurfaceRef = *mut std::ffi::c_void;

#[cfg(target_os = "macos")]
#[link(name = "IOSurface", kind = "framework")]
extern "C" {
    #[link_name = "IOSurfaceLock"]
    pub fn io_surface_lock(surface: IOSurfaceRef, options: u32, seed: *mut u32) -> i32;

    #[link_name = "IOSurfaceGetWidth"]
    pub fn io_surface_get_width(surface: IOSurfaceRef) -> usize;

    #[link_name = "IOSurfaceGetHeight"]
    pub fn io_surface_get_height(surface: IOSurfaceRef) -> usize;

    #[link_name = "IOSurfaceGetBytesPerRow"]
    pub fn io_surface_get_bytes_per_row(surface: IOSurfaceRef) -> usize;

    #[link_name = "IOSurfaceGetBaseAddress"]
    pub fn io_surface_get_base_address(surface: IOSurfaceRef) -> *mut std::ffi::c_void;

    #[link_name = "IOSurfaceUnlock"]
    pub fn io_surface_unlock(surface: IOSurfaceRef, options: u32, seed: *mut u32) -> i32;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    #[link_name = "CFRetain"]
    pub fn cf_retain(cf: *const std::ffi::c_void) -> *const std::ffi::c_void;

    #[link_name = "CFRelease"]
    pub fn cf_release(cf: *const std::ffi::c_void);

    #[link_name = "CFGetRetainCount"]
    pub fn cf_get_retain_count(cf: *const std::ffi::c_void) -> isize;
}

/// `close` of the C library (`_close` of the C runtime on Windows).
pub fn close(fd: i32) -> i32 {
    // SAFETY: closing a descriptor the caller owns; an invalid one gives -1.
    unsafe { libc::close(fd) }
}
