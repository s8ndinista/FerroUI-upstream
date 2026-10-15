//! The declarations of `libgbm` and `libdrm` the dmabuf swapchain calls (the
//! port of `DrmGbmUnsafeNativeMethods.cs`): the flags of a buffer object,
//! the handle of a plane, the device record of `libdrm`, the format codes,
//! and the functions.
//!
//! The reference declares platform invokes of `libgbm.so.1` and
//! `libdrm.so.2`, which its runtime resolves when one is first called.
//! Here both libraries are opened at run time when the table is first
//! asked for ([`DrmGbmUnsafeNativeMethods::get`]), through the loader of
//! the X11 crate (`NativeLibrary` and `native_functions!`), which the
//! other libraries of the Linux backends are opened with: nothing links
//! them, a machine without them gets an error that names the library, and
//! the swapchain is then not created (the backend renders through a
//! `wl_egl_window` or in software, as it does when the reference's call
//! throws). The three functions of the C library are those of `libc`.

use ferroui_x11::interop::native_library::{native_functions, NativeLibrary, NativeLibraryError};
use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::OnceLock;

/// The uses of a buffer object (`enum gbm_bo_flags`), a set of flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GbmBoFlags(pub u32);

impl GbmBoFlags {
    pub const GBM_BO_USE_SCANOUT: Self = Self(1 << 0);
    pub const GBM_BO_USE_RENDERING: Self = Self(1 << 2);
    pub const GBM_BO_USE_LINEAR: Self = Self(1 << 4);
}

impl std::ops::BitOr for GbmBoFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// `union gbm_bo_handle`: the handle of a plane of a buffer object.
#[derive(Clone, Copy)]
#[repr(C)]
pub union GbmBoHandle {
    pub ptr: *mut c_void,
    pub s32: i32,
    pub u32: u32,
    pub s64: i64,
    pub u64: u64,
}

/// `drmDevice` of `xf86drm.h`: the nodes of a device and its bus.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DrmDevice {
    /// `char **`: an array of `DRM_NODE_MAX` paths.
    pub nodes: *mut *mut c_char,
    /// `DRM_NODE_*` bitmask.
    pub available_nodes: c_int,
    pub bus_type: c_int,
    /// Union of pointers.
    pub bus_info: *mut c_void,
    /// Union of pointers.
    pub device_info: *mut c_void,
}

native_functions! {
    /// The functions of the two libraries, resolved when the libraries are opened.
    pub struct DrmGbmFunctions(gbm, drm) {
        // GBM device
        gbm::gbm_create_device: unsafe extern "C" fn(c_int) -> *mut c_void;
        gbm::gbm_device_destroy: unsafe extern "C" fn(*mut c_void);
        // GBM buffer object - basic
        gbm::gbm_bo_create: unsafe extern "C" fn(*mut c_void, u32, u32, u32, u32) -> *mut c_void;
        gbm::gbm_bo_create_with_modifiers2:
            unsafe extern "C" fn(*mut c_void, u32, u32, u32, *const u64, u32, u32) -> *mut c_void;
        gbm::gbm_bo_destroy: unsafe extern "C" fn(*mut c_void);
        // GBM buffer object - properties
        gbm::gbm_bo_get_width: unsafe extern "C" fn(*mut c_void) -> u32;
        gbm::gbm_bo_get_height: unsafe extern "C" fn(*mut c_void) -> u32;
        gbm::gbm_bo_get_format: unsafe extern "C" fn(*mut c_void) -> u32;
        gbm::gbm_bo_get_modifier: unsafe extern "C" fn(*mut c_void) -> u64;
        gbm::gbm_bo_get_plane_count: unsafe extern "C" fn(*mut c_void) -> c_int;
        gbm::gbm_bo_get_handle_for_plane: unsafe extern "C" fn(*mut c_void, c_int) -> GbmBoHandle;
        gbm::gbm_bo_get_stride_for_plane: unsafe extern "C" fn(*mut c_void, c_int) -> u32;
        gbm::gbm_bo_get_offset: unsafe extern "C" fn(*mut c_void, c_int) -> u32;
        // DRM
        drm::drm_prime_handle_to_fd as "drmPrimeHandleToFD": unsafe extern "C" fn(c_int, u32, u32, *mut c_int) -> c_int;
        drm::drm_get_device_from_dev_id as "drmGetDeviceFromDevId":
            unsafe extern "C" fn(u64, u32, *mut *mut DrmDevice) -> c_int;
        drm::drm_free_device as "drmFreeDevice": unsafe extern "C" fn(*mut *mut DrmDevice);
    }
}

/// The declarations of the reference: the libraries, the format codes and the functions,
/// each a call of the table of its library.
pub struct DrmGbmUnsafeNativeMethods {
    functions: DrmGbmFunctions,
}

impl DrmGbmUnsafeNativeMethods {
    pub const LIB_GBM: &'static CStr = c"libgbm.so.1";
    pub const LIB_DRM: &'static CStr = c"libdrm.so.2";
    /// The C library: the functions are those of the crate `libc`.
    pub const LIB_C: &'static str = "libc";

    // DRM format constants
    pub const fn drm_format_code(a: char, b: char, c: char, d: char) -> u32 {
        (a as u32) | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24)
    }

    pub const DRM_FORMAT_ARGB8888: u32 = Self::drm_format_code('A', 'R', '2', '4');
    pub const DRM_FORMAT_XRGB8888: u32 = Self::drm_format_code('X', 'R', '2', '4');
    pub const DRM_FORMAT_MOD_INVALID: u64 = 0x00ff_ffff_ffff_ffff;

    pub const DRM_NODE_RENDER: c_int = 2;

    pub const O_RDWR: c_int = 2;

    /// The functions of the two libraries, opened the first time they are asked for. The
    /// error names the library or the function that is missing; it is kept, so a machine
    /// without the libraries is asked once.
    pub fn get() -> Result<&'static DrmGbmUnsafeNativeMethods, NativeLibraryError> {
        static METHODS: OnceLock<Result<DrmGbmUnsafeNativeMethods, NativeLibraryError>> = OnceLock::new();
        METHODS
            .get_or_init(|| {
                let gbm = NativeLibrary::open(Self::LIB_GBM)?;
                let drm = NativeLibrary::open(Self::LIB_DRM)?;
                Ok(DrmGbmUnsafeNativeMethods { functions: DrmGbmFunctions::load(&gbm, &drm)? })
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    // GBM device

    /// # Safety
    /// `fd` is an open descriptor of a DRM device that outlives the GBM device.
    pub unsafe fn gbm_create_device(&self, fd: c_int) -> *mut c_void {
        // SAFETY: the contract of the C function, which the caller upholds.
        unsafe { (self.functions.gbm_create_device)(fd) }
    }

    /// # Safety
    /// `gbm` is a device `gbm_create_device` returned, destroyed once.
    pub unsafe fn gbm_device_destroy(&self, gbm: *mut c_void) {
        // SAFETY: the contract of the C function, which the caller upholds.
        unsafe { (self.functions.gbm_device_destroy)(gbm) }
    }

    // GBM buffer object - basic

    /// # Safety
    /// `gbm` is a live device.
    pub unsafe fn gbm_bo_create(&self, gbm: *mut c_void, width: u32, height: u32, format: u32, flags: GbmBoFlags) -> *mut c_void {
        // SAFETY: the contract of the C function, which the caller upholds.
        unsafe { (self.functions.gbm_bo_create)(gbm, width, height, format, flags.0) }
    }

    /// # Safety
    /// `gbm` is a live device; `modifiers` is valid for `count` values.
    pub unsafe fn gbm_bo_create_with_modifiers2(
        &self,
        gbm: *mut c_void,
        width: u32,
        height: u32,
        format: u32,
        modifiers: *const u64,
        count: u32,
        flags: GbmBoFlags,
    ) -> *mut c_void {
        // SAFETY: the contract of the C function, which the caller upholds.
        unsafe { (self.functions.gbm_bo_create_with_modifiers2)(gbm, width, height, format, modifiers, count, flags.0) }
    }

    /// # Safety
    /// `bo` is a live buffer object, destroyed once.
    pub unsafe fn gbm_bo_destroy(&self, bo: *mut c_void) {
        // SAFETY: the contract of the C function, which the caller upholds.
        unsafe { (self.functions.gbm_bo_destroy)(bo) }
    }

    // GBM buffer object - properties

    /// # Safety
    /// `bo` is a live buffer object.
    pub unsafe fn gbm_bo_get_width(&self, bo: *mut c_void) -> u32 {
        // SAFETY: the contract of the C function, which the caller upholds.
        unsafe { (self.functions.gbm_bo_get_width)(bo) }
    }

    /// # Safety
    /// `bo` is a live buffer object.
    pub unsafe fn gbm_bo_get_height(&self, bo: *mut c_void) -> u32 {
        // SAFETY: the contract of the C function, which the caller upholds.
        unsafe { (self.functions.gbm_bo_get_height)(bo) }
    }

    /// # Safety
    /// `bo` is a live buffer object.
    pub unsafe fn gbm_bo_get_format(&self, bo: *mut c_void) -> u32 {
        // SAFETY: the contract of the C function, which the caller upholds.
        unsafe { (self.functions.gbm_bo_get_format)(bo) }
    }

    /// # Safety
    /// `bo` is a live buffer object.
    pub unsafe fn gbm_bo_get_modifier(&self, bo: *mut c_void) -> u64 {
        // SAFETY: the contract of the C function, which the caller upholds.
        unsafe { (self.functions.gbm_bo_get_modifier)(bo) }
    }

    /// # Safety
    /// `bo` is a live buffer object.
    pub unsafe fn gbm_bo_get_plane_count(&self, bo: *mut c_void) -> c_int {
        // SAFETY: the contract of the C function, which the caller upholds.
        unsafe { (self.functions.gbm_bo_get_plane_count)(bo) }
    }

    /// # Safety
    /// `bo` is a live buffer object and `plane` one of its planes.
    pub unsafe fn gbm_bo_get_handle_for_plane(&self, bo: *mut c_void, plane: c_int) -> GbmBoHandle {
        // SAFETY: the contract of the C function, which the caller upholds.
        unsafe { (self.functions.gbm_bo_get_handle_for_plane)(bo, plane) }
    }

    /// # Safety
    /// `bo` is a live buffer object and `plane` one of its planes.
    pub unsafe fn gbm_bo_get_stride_for_plane(&self, bo: *mut c_void, plane: c_int) -> u32 {
        // SAFETY: the contract of the C function, which the caller upholds.
        unsafe { (self.functions.gbm_bo_get_stride_for_plane)(bo, plane) }
    }

    /// # Safety
    /// `bo` is a live buffer object and `plane` one of its planes.
    pub unsafe fn gbm_bo_get_offset(&self, bo: *mut c_void, plane: c_int) -> u32 {
        // SAFETY: the contract of the C function, which the caller upholds.
        unsafe { (self.functions.gbm_bo_get_offset)(bo, plane) }
    }

    // DRM

    /// Exports a handle of a buffer as a descriptor of a dma-buf, written to `prime_fd`.
    ///
    /// # Safety
    /// `fd` is an open descriptor of the device the handle belongs to.
    pub unsafe fn drm_prime_handle_to_fd(&self, fd: c_int, handle: u32, flags: u32, prime_fd: &mut c_int) -> c_int {
        // SAFETY: the contract of the C function, which the caller upholds; the output is a
        // valid place for one descriptor.
        unsafe { (self.functions.drm_prime_handle_to_fd)(fd, handle, flags, prime_fd) }
    }

    /// The device of a device number, written to `device`, to be freed with
    /// [`drm_free_device`](Self::drm_free_device).
    pub fn drm_get_device_from_dev_id(&self, dev_id: u64, flags: u32, device: &mut *mut DrmDevice) -> c_int {
        // SAFETY: the output is a valid place for one pointer; the function allocates what it
        // writes there.
        unsafe { (self.functions.drm_get_device_from_dev_id)(dev_id, flags, device) }
    }

    /// # Safety
    /// `device` points to a device `drm_get_device_from_dev_id` returned, freed once.
    pub unsafe fn drm_free_device(&self, device: *mut *mut DrmDevice) {
        // SAFETY: the contract of the C function, which the caller upholds.
        unsafe { (self.functions.drm_free_device)(device) }
    }

    // libc

    /// `open(2)`: the descriptor, or -1.
    pub fn open(pathname: &CStr, flags: c_int) -> c_int {
        // SAFETY: the path is a terminated string that lives for the call.
        unsafe { libc::open(pathname.as_ptr(), flags) }
    }

    /// `close(2)`.
    ///
    /// # Safety
    /// `fd` is an open descriptor nobody else closes.
    pub unsafe fn close(fd: c_int) -> c_int {
        // SAFETY: the contract of the C function, which the caller upholds.
        unsafe { libc::close(fd) }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn the_format_codes_are_the_four_characters_of_drm_fourcc_h() {
        // DRM_FORMAT_ARGB8888 and DRM_FORMAT_XRGB8888 of drm_fourcc.h.
        assert_eq!(DrmGbmUnsafeNativeMethods::DRM_FORMAT_ARGB8888, 0x3432_5241);
        assert_eq!(DrmGbmUnsafeNativeMethods::DRM_FORMAT_XRGB8888, 0x3432_5258);
        assert_eq!(DrmGbmUnsafeNativeMethods::DRM_FORMAT_MOD_INVALID, (1u64 << 56) - 1);
        assert_eq!(DrmGbmUnsafeNativeMethods::O_RDWR, libc::O_RDWR);
    }

    #[test]
    fn the_records_have_the_layout_of_the_c_headers() {
        // union gbm_bo_handle: eight bytes; drmDevice: a pointer, two ints, two pointers.
        assert_eq!(std::mem::size_of::<GbmBoHandle>(), 8);
        assert_eq!(std::mem::size_of::<DrmDevice>(), 32);
        assert_eq!(std::mem::offset_of!(DrmDevice, available_nodes), 8);
        assert_eq!(std::mem::offset_of!(DrmDevice, bus_type), 12);
        assert_eq!(std::mem::offset_of!(DrmDevice, bus_info), 16);
        assert_eq!(std::mem::offset_of!(DrmDevice, device_info), 24);
    }

    #[test]
    fn the_flags_combine() {
        let flags = GbmBoFlags::GBM_BO_USE_RENDERING | GbmBoFlags::GBM_BO_USE_LINEAR;
        assert_eq!(flags.0, 0b1_0100);
        assert_eq!(GbmBoFlags::GBM_BO_USE_SCANOUT.0, 1);
    }
}
