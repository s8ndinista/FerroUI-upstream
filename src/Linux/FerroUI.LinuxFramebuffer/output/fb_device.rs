//! What the framebuffer output asks of its device.
//!
//! An addition (docs/porting/linux-framebuffer-platform.md, section 5):
//! the reference calls the kernel directly. With [`LinuxFbDevice`] the
//! calls are the reference's; [`FileFbDevice`] is a plain file that stands
//! in for a device in tests and on machines without one.

use crate::native_unsafe_methods::{fb_fix_screeninfo, fb_var_screeninfo};
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;

/// The memory of a device, mapped into the process. It is unmapped when
/// the last owner lets go of it.
pub struct FbMapping {
    address: *mut u8,
    length: usize,
}

// SAFETY: the mapping is memory shared with a device (or a file); nothing
// in it is tied to a thread. The output hands it to one renderer at a
// time, and a write that races another only mixes the pixels of two
// frames in memory that has no validity invariant.
unsafe impl Send for FbMapping {}
// SAFETY: as above.
unsafe impl Sync for FbMapping {}

impl FbMapping {
    /// `mmap(null, length, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0)`.
    fn map(fd: i32, length: usize) -> io::Result<Self> {
        // SAFETY: asks the kernel for a new shared mapping of `length`
        // bytes of the descriptor; no existing memory is affected.
        let address = unsafe {
            libc::mmap(std::ptr::null_mut(), length, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_SHARED, fd, 0)
        };
        if address == libc::MAP_FAILED {
            let error = io::Error::last_os_error();
            return Err(io::Error::new(error.kind(), format!("Unable to mmap {length} bytes, error {error}")));
        }
        Ok(Self { address: address.cast(), length })
    }

    pub fn address(&self) -> *mut u8 {
        self.address
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Copies `source` to the start of the mapping, as much as fits.
    pub fn copy_from(&self, source: &[u8]) {
        let count = source.len().min(self.length);
        // SAFETY: both ranges are valid for `count` bytes and do not
        // overlap (`source` is memory of the process, not of the mapping).
        unsafe {
            std::ptr::copy_nonoverlapping(source.as_ptr(), self.address, count);
        }
    }

    /// The content of the mapping.
    pub fn to_vec(&self) -> Vec<u8> {
        let mut rv = vec![0u8; self.length];
        // SAFETY: the mapping is valid for `length` bytes and `rv` is new
        // memory of that length.
        unsafe {
            std::ptr::copy_nonoverlapping(self.address, rv.as_mut_ptr(), self.length);
        }
        rv
    }
}

impl Drop for FbMapping {
    fn drop(&mut self) {
        // SAFETY: the range is the one `mmap` returned, unmapped once.
        unsafe {
            libc::munmap(self.address.cast(), self.length);
        }
    }
}

/// A framebuffer device.
pub trait IFbDevice: Send {
    /// `FBIOGET_VSCREENINFO`.
    fn get_var_screen_info(&self) -> io::Result<fb_var_screeninfo>;

    /// `FBIOPUT_VSCREENINFO`.
    fn put_var_screen_info(&mut self, info: &fb_var_screeninfo) -> io::Result<()>;

    /// `FBIOGET_FSCREENINFO`.
    fn get_fix_screen_info(&self) -> io::Result<fb_fix_screeninfo>;

    /// Maps `length` bytes of the device's memory for reading and writing.
    fn map(&self, length: usize) -> io::Result<FbMapping>;

    /// `FBIO_WAITFORVSYNC`; a device without it returns at once.
    fn wait_for_vsync(&self);
}

/// A framebuffer device of the kernel.
#[cfg(target_os = "linux")]
pub struct LinuxFbDevice {
    fd: std::os::fd::OwnedFd,
}

#[cfg(target_os = "linux")]
impl LinuxFbDevice {
    /// Opens the device for reading and writing.
    pub fn open(file_name: &str) -> io::Result<Self> {
        use std::os::fd::FromRawFd;
        let fd = crate::native_unsafe_methods::sys::open(file_name, libc::O_RDWR)
            .map_err(|error| io::Error::new(error.kind(), format!("Error: {error}")))?;
        // SAFETY: `fd` was just opened and is owned by nothing else.
        Ok(Self { fd: unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) } })
    }
}

#[cfg(target_os = "linux")]
impl IFbDevice for LinuxFbDevice {
    fn get_var_screen_info(&self) -> io::Result<fb_var_screeninfo> {
        use crate::native_unsafe_methods::{fb_io_ctl, sys};
        let mut info = fb_var_screeninfo::default();
        sys::ioctl_struct(self.fd.as_raw_fd(), fb_io_ctl::FBIOGET_VSCREENINFO, &mut info)
            .map_err(|error| io::Error::new(error.kind(), format!("FBIOGET_VSCREENINFO error: {error}")))?;
        Ok(info)
    }

    fn put_var_screen_info(&mut self, info: &fb_var_screeninfo) -> io::Result<()> {
        use crate::native_unsafe_methods::{fb_io_ctl, sys};
        let mut info = *info;
        sys::ioctl_struct(self.fd.as_raw_fd(), fb_io_ctl::FBIOPUT_VSCREENINFO, &mut info).map(|_| ())
    }

    fn get_fix_screen_info(&self) -> io::Result<fb_fix_screeninfo> {
        use crate::native_unsafe_methods::{fb_io_ctl, sys};
        let mut info = fb_fix_screeninfo::default();
        sys::ioctl_struct(self.fd.as_raw_fd(), fb_io_ctl::FBIOGET_FSCREENINFO, &mut info)
            .map_err(|error| io::Error::new(error.kind(), format!("FBIOGET_FSCREENINFO error: {error}")))?;
        Ok(info)
    }

    fn map(&self, length: usize) -> io::Result<FbMapping> {
        FbMapping::map(self.fd.as_raw_fd(), length)
    }

    fn wait_for_vsync(&self) {
        use crate::native_unsafe_methods::{fb_io_ctl, sys};
        sys::ioctl_null(self.fd.as_raw_fd(), fb_io_ctl::FBIO_WAITFORVSYNC);
    }
}

/// A plain file as a framebuffer: `line_length * yres` bytes, mapped as a
/// device's memory is.
pub struct FileFbDevice {
    file: File,
    var_info: fb_var_screeninfo,
    id: [u8; 16],
}

impl FileFbDevice {
    /// Creates (or truncates) the file for a framebuffer of the given
    /// variable information; the lines have no padding.
    pub fn create(path: &str, var_info: fb_var_screeninfo) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).create(true).truncate(true).open(path)?;
        let mut id = [0u8; 16];
        id[..4].copy_from_slice(b"file");
        let this = Self { file, var_info, id };
        this.resize()?;
        Ok(this)
    }

    fn line_length(&self) -> u32 {
        self.var_info.xres * self.var_info.bits_per_pixel.div_ceil(8)
    }

    fn resize(&self) -> io::Result<()> {
        self.file.set_len(self.line_length() as u64 * self.var_info.yres as u64)
    }
}

impl IFbDevice for FileFbDevice {
    fn get_var_screen_info(&self) -> io::Result<fb_var_screeninfo> {
        Ok(self.var_info)
    }

    fn put_var_screen_info(&mut self, info: &fb_var_screeninfo) -> io::Result<()> {
        if info.bits_per_pixel != 16 && info.bits_per_pixel != 32 {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        }
        self.var_info = *info;
        self.resize()
    }

    fn get_fix_screen_info(&self) -> io::Result<fb_fix_screeninfo> {
        let line_length = self.line_length();
        Ok(fb_fix_screeninfo {
            id: self.id,
            smem_len: line_length * self.var_info.yres,
            line_length,
            ..Default::default()
        })
    }

    fn map(&self, length: usize) -> io::Result<FbMapping> {
        FbMapping::map(self.file.as_raw_fd(), length)
    }

    fn wait_for_vsync(&self) {}
}
