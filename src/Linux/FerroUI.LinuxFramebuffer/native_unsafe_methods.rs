//! The kernel's structures, request numbers and codes of the framebuffer
//! and input interfaces (the port of `NativeUnsafeMethods.cs`), and the
//! system calls on files as safe functions.
//!
//! The functions of the C library are those of the `libc` crate. The
//! structures of `linux/fb.h` are declared here, since that crate does not
//! have them; the tests at the end hold them to the kernel's sizes.

#![allow(non_camel_case_types)]

/// The request numbers of the framebuffer device.
pub mod fb_io_ctl {
    pub const FBIOGET_VSCREENINFO: u32 = 0x4600;
    pub const FBIOPUT_VSCREENINFO: u32 = 0x4601;
    pub const FBIOGET_FSCREENINFO: u32 = 0x4602;
    pub const FBIOGET_VBLANK: u32 = 0x8020_4612;
    pub const FBIO_WAITFORVSYNC: u32 = 0x4004_4620;
    pub const FBIOPAN_DISPLAY: u32 = 0x4606;
}

/// The flags of [`fb_vblank`].
pub mod v_blank_flags {
    /// currently in a vertical blank
    pub const FB_VBLANK_VBLANKING: u32 = 0x001;
    /// currently in a horizontal blank
    pub const FB_VBLANK_HBLANKING: u32 = 0x002;
    /// vertical blanks can be detected
    pub const FB_VBLANK_HAVE_VBLANK: u32 = 0x004;
    /// horizontal blanks can be detected
    pub const FB_VBLANK_HAVE_HBLANK: u32 = 0x008;
    /// global retrace counter is available
    pub const FB_VBLANK_HAVE_COUNT: u32 = 0x010;
    /// the vcount field is valid
    pub const FB_VBLANK_HAVE_VCOUNT: u32 = 0x020;
    /// the hcount field is valid
    pub const FB_VBLANK_HAVE_HCOUNT: u32 = 0x040;
    /// currently in a vsync
    pub const FB_VBLANK_VSYNCING: u32 = 0x080;
    /// vertical syncs can be detected
    pub const FB_VBLANK_HAVE_VSYNC: u32 = 0x100;
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fb_vblank {
    /// FB_VBLANK flags
    pub flags: u32,
    /// counter of retraces since boot
    pub count: u32,
    /// current scanline position
    pub vcount: u32,
    /// current scandot position
    pub hcount: u32,
    /// reserved for future compatibility
    pub reserved: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fb_fix_screeninfo {
    /// identification string eg "TT Builtin"
    pub id: [u8; 16],
    /// Start of frame buffer mem (physical address)
    pub smem_start: usize,
    /// Length of frame buffer mem
    pub smem_len: u32,
    /// see FB_TYPE_*
    pub type_: u32,
    /// Interleave for interleaved Planes
    pub type_aux: u32,
    /// see FB_VISUAL_*
    pub visual: u32,
    /// zero if no hardware panning
    pub xpanstep: u16,
    /// zero if no hardware panning
    pub ypanstep: u16,
    /// zero if no hardware ywrap
    pub ywrapstep: u16,
    /// length of a line in bytes
    pub line_length: u32,
    /// Start of Memory Mapped I/O (physical address)
    pub mmio_start: usize,
    /// Length of Memory Mapped I/O
    pub mmio_len: u32,
    /// Type of acceleration available
    pub accel: u32,
    /// Reserved for future compatibility
    pub reserved: [u16; 3],
}

impl fb_fix_screeninfo {
    /// The identification string up to its terminator.
    pub fn id_string(&self) -> String {
        let idlen = self.id.iter().position(|&b| b == 0).unwrap_or(self.id.len());
        self.id[..idlen].iter().map(|&b| if b.is_ascii() { b as char } else { '?' }).collect()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fb_bitfield {
    /// beginning of bitfield
    pub offset: u32,
    /// length of bitfield
    pub length: u32,
    /// != 0 : Most significant bit is right
    pub msb_right: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fb_var_screeninfo {
    /// visible resolution
    pub xres: u32,
    pub yres: u32,
    /// virtual resolution
    pub xres_virtual: u32,
    pub yres_virtual: u32,
    /// offset from virtual to visible resolution
    pub xoffset: u32,
    pub yoffset: u32,
    /// guess what
    pub bits_per_pixel: u32,
    /// != 0 Graylevels instead of colors
    pub grayscale: u32,
    /// bitfield in fb mem if true color, else only length is significant
    pub red: fb_bitfield,
    pub green: fb_bitfield,
    pub blue: fb_bitfield,
    /// transparency
    pub transp: fb_bitfield,
    /// != 0 Non standard pixel format
    pub nonstd: u32,
    /// see FB_ACTIVATE_*
    pub activate: u32,
    /// height of picture in mm
    pub height: u32,
    /// width of picture in mm
    pub width: u32,
    /// acceleration flags (hints)
    pub accel_flags: u32,
    /* Timing: All values in pixclocks, except pixclock (of course) */
    /// pixel clock in ps (pico seconds)
    pub pixclock: u32,
    /// time from sync to picture
    pub left_margin: u32,
    /// time from picture to sync
    pub right_margin: u32,
    /// time from sync to picture
    pub upper_margin: u32,
    pub lower_margin: u32,
    /// length of horizontal sync
    pub hsync_len: u32,
    /// length of vertical sync
    pub vsync_len: u32,
    /// see FB_SYNC_*
    pub sync: u32,
    /// see FB_VMODE_*
    pub vmode: u32,
    /// Reserved for future compatibility
    pub reserved: [u32; 6],
}

/// The types of input events.
pub mod ev_type {
    pub const EV_SYN: u16 = 0x00;
    pub const EV_KEY: u16 = 0x01;
    pub const EV_REL: u16 = 0x02;
    pub const EV_ABS: u16 = 0x03;
    pub const EV_MSC: u16 = 0x04;
    pub const EV_SW: u16 = 0x05;
    pub const EV_LED: u16 = 0x11;
    pub const EV_SND: u16 = 0x12;
    pub const EV_REP: u16 = 0x14;
    pub const EV_FF: u16 = 0x15;
    pub const EV_PWR: u16 = 0x16;
    pub const EV_FF_STATUS: u16 = 0x17;

    /// Every type, in the order the reference enumerates them.
    pub const ALL: [u16; 12] =
        [EV_SYN, EV_KEY, EV_REL, EV_ABS, EV_MSC, EV_SW, EV_LED, EV_SND, EV_REP, EV_FF, EV_PWR, EV_FF_STATUS];
}

/// The codes of `EV_SYN` events. Not in the reference, which reads events
/// through a library that handles them.
pub mod syn_code {
    pub const SYN_REPORT: u16 = 0;
    pub const SYN_DROPPED: u16 = 3;
}

/// The codes of relative axes.
pub mod axis_event_code {
    pub const REL_X: u16 = 0x00;
    pub const REL_Y: u16 = 0x01;
    pub const REL_Z: u16 = 0x02;
    pub const REL_RX: u16 = 0x03;
    pub const REL_RY: u16 = 0x04;
    pub const REL_RZ: u16 = 0x05;
    pub const REL_HWHEEL: u16 = 0x06;
    pub const REL_DIAL: u16 = 0x07;
    pub const REL_WHEEL: u16 = 0x08;
    pub const REL_MISC: u16 = 0x09;
    pub const REL_MAX: u16 = 0x0f;
}

/// The codes of absolute axes.
pub mod abs_axis {
    pub const ABS_X: u16 = 0x00;
    pub const ABS_Y: u16 = 0x01;
    pub const ABS_Z: u16 = 0x02;
    pub const ABS_RX: u16 = 0x03;
    pub const ABS_RY: u16 = 0x04;
    pub const ABS_RZ: u16 = 0x05;
    pub const ABS_THROTTLE: u16 = 0x06;
    pub const ABS_RUDDER: u16 = 0x07;
    pub const ABS_WHEEL: u16 = 0x08;
    pub const ABS_GAS: u16 = 0x09;
    pub const ABS_BRAKE: u16 = 0x0a;
    pub const ABS_HAT0X: u16 = 0x10;
    pub const ABS_HAT0Y: u16 = 0x11;
    pub const ABS_HAT1X: u16 = 0x12;
    pub const ABS_HAT1Y: u16 = 0x13;
    pub const ABS_HAT2X: u16 = 0x14;
    pub const ABS_HAT2Y: u16 = 0x15;
    pub const ABS_HAT3X: u16 = 0x16;
    pub const ABS_HAT3Y: u16 = 0x17;
    pub const ABS_PRESSURE: u16 = 0x18;
    pub const ABS_DISTANCE: u16 = 0x19;
    pub const ABS_TILT_X: u16 = 0x1a;
    pub const ABS_TILT_Y: u16 = 0x1b;
    pub const ABS_TOOL_WIDTH: u16 = 0x1c;
    // Additions: the axes of the multi-touch protocol B.
    pub const ABS_MT_SLOT: u16 = 0x2f;
    pub const ABS_MT_POSITION_X: u16 = 0x35;
    pub const ABS_MT_POSITION_Y: u16 = 0x36;
    pub const ABS_MT_TRACKING_ID: u16 = 0x39;
}

/// The key codes the reference names: the buttons.
pub mod ev_key {
    pub const BTN_LEFT: u16 = 0x110;
    pub const BTN_RIGHT: u16 = 0x111;
    pub const BTN_MIDDLE: u16 = 0x112;
    pub const BTN_TOUCH: u16 = 0x14a;
}

/// An input event as the kernel writes it, with the time as two 64-bit
/// numbers on every architecture: the value the input thread hands to the
/// UI thread.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct input_event {
    pub tv_sec: i64,
    pub tv_usec: i64,
    pub type_: u16,
    pub code: u16,
    pub value: i32,
}

impl input_event {
    pub fn new(type_: u16, code: u16, value: i32) -> Self {
        Self { tv_sec: 0, tv_usec: 0, type_, code, value }
    }

    /// The time of the event in milliseconds.
    pub fn timestamp(&self) -> u64 {
        let ms = self.tv_usec as u64 / 1000;
        let s = (self.tv_sec as u64).wrapping_mul(1000);
        s.wrapping_add(ms)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct input_absinfo {
    pub value: i32,
    pub minimum: i32,
    pub maximum: i32,
    pub fuzz: i32,
    pub flat: i32,
    pub resolution: i32,
}

/// The number of an `ioctl` request in the encoding most architectures
/// use (x86, ARM, RISC-V): two bits of direction, fourteen of size, eight
/// of type and eight of number. The request numbers of the reference
/// (`FBIO_WAITFORVSYNC`, `FBIOGET_VBLANK`) are in this encoding too.
pub const fn ioc(dir: u32, type_: u8, nr: u32, size: usize) -> u32 {
    (dir << 30) | ((size as u32) << 16) | ((type_ as u32) << 8) | nr
}

pub const IOC_WRITE: u32 = 1;
pub const IOC_READ: u32 = 2;

/// The system calls, each as a safe function.
#[cfg(target_os = "linux")]
pub mod sys {
    use std::ffi::CString;
    use std::io;
    use std::os::fd::RawFd;

    /// `open(pathname, flags, 0)`.
    pub fn open(pathname: &str, flags: i32) -> io::Result<RawFd> {
        let path = CString::new(pathname).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        // SAFETY: `path` is a valid C string for the call.
        let fd = unsafe { libc::open(path.as_ptr(), flags, 0) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(fd)
    }

    pub fn close(fd: RawFd) {
        // SAFETY: closes a descriptor the caller owns and does not use again.
        unsafe {
            libc::close(fd);
        }
    }

    /// An `ioctl` that reads or writes one plain structure.
    ///
    /// The request must be one whose argument is a pointer to a `T`; every
    /// caller in this crate passes a request and the structure the kernel
    /// documents for it.
    pub fn ioctl_struct<T: Copy>(fd: RawFd, request: u32, arg: &mut T) -> io::Result<i32> {
        // SAFETY: `arg` is a valid, exclusive place for a `T`, and the
        // request reads or writes at most a `T` there (see above).
        let rv = unsafe { libc::ioctl(fd, request as _, arg as *mut T) };
        if rv == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(rv)
    }

    /// An `ioctl` that fills a buffer of bytes whose length is part of the
    /// request number.
    pub fn ioctl_buffer(fd: RawFd, request: u32, buffer: &mut [u8]) -> io::Result<i32> {
        // SAFETY: the callers encode `buffer.len()` in the request, so the
        // kernel writes at most that many bytes.
        let rv = unsafe { libc::ioctl(fd, request as _, buffer.as_mut_ptr()) };
        if rv == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(rv)
    }

    /// An `ioctl` without an argument (a null pointer), as the reference
    /// calls `FBIO_WAITFORVSYNC`.
    pub fn ioctl_null(fd: RawFd, request: u32) -> i32 {
        // SAFETY: the kernel rejects the null argument where it needs one.
        unsafe { libc::ioctl(fd, request as _, std::ptr::null_mut::<libc::c_void>()) }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file: the sizes
    // and offsets are those of `linux/fb.h` and `linux/input.h`.
    use super::*;
    use std::mem::{offset_of, size_of};

    #[test]
    fn the_framebuffer_structures_have_the_sizes_of_the_kernel() {
        assert_eq!(size_of::<fb_bitfield>(), 12);
        assert_eq!(size_of::<fb_var_screeninfo>(), 160);
        assert_eq!(size_of::<fb_vblank>(), 32);
        // 16 + long + 4 * 4 + 3 * 2 (+ 2) + 4 + long + 2 * 4 + 3 * 2 (+ 2).
        let expected = if size_of::<usize>() == 8 { 80 } else { 68 };
        assert_eq!(size_of::<fb_fix_screeninfo>(), expected);
        assert_eq!(size_of::<input_absinfo>(), 24);
    }

    #[test]
    fn the_fields_are_where_the_kernel_has_them() {
        assert_eq!(offset_of!(fb_var_screeninfo, bits_per_pixel), 24);
        assert_eq!(offset_of!(fb_var_screeninfo, red), 32);
        assert_eq!(offset_of!(fb_var_screeninfo, transp), 68);
        assert_eq!(offset_of!(fb_var_screeninfo, nonstd), 80);
        assert_eq!(offset_of!(fb_var_screeninfo, reserved), 136);
        if size_of::<usize>() == 8 {
            assert_eq!(offset_of!(fb_fix_screeninfo, smem_start), 16);
            assert_eq!(offset_of!(fb_fix_screeninfo, smem_len), 24);
            assert_eq!(offset_of!(fb_fix_screeninfo, line_length), 48);
            assert_eq!(offset_of!(fb_fix_screeninfo, mmio_start), 56);
            assert_eq!(offset_of!(fb_fix_screeninfo, accel), 68);
        }
    }

    #[test]
    fn the_request_numbers_are_the_encoded_ones() {
        // _IOW('F', 0x20, __u32) and _IOR('F', 0x12, struct fb_vblank).
        assert_eq!(ioc(IOC_WRITE, b'F', 0x20, 4), fb_io_ctl::FBIO_WAITFORVSYNC);
        assert_eq!(ioc(IOC_READ, b'F', 0x12, size_of::<fb_vblank>()), fb_io_ctl::FBIOGET_VBLANK);
        assert_eq!(ioc(0, b'F', 0x00, 0), fb_io_ctl::FBIOGET_VSCREENINFO);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_absolute_axis_information_is_that_of_the_c_library() {
        assert_eq!(size_of::<input_absinfo>(), size_of::<libc::input_absinfo>());
    }

    #[test]
    fn the_timestamp_is_in_milliseconds() {
        let ev = input_event { tv_sec: 12, tv_usec: 345_678, type_: 0, code: 0, value: 0 };
        assert_eq!(ev.timestamp(), 12_345);
    }

    #[test]
    fn the_identification_ends_at_its_terminator() {
        let mut fix = fb_fix_screeninfo::default();
        fix.id[..10].copy_from_slice(b"Virtual FB");
        assert_eq!(fix.id_string(), "Virtual FB");
        fix.id = *b"0123456789abcdef";
        assert_eq!(fix.id_string(), "0123456789abcdef");
    }
}
