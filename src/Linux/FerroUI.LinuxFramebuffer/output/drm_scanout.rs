//! Reading back what a CRTC shows: the pixels of the framebuffer it scans
//! out. An addition, for the verification of the DRM output on a virtual
//! card (docs/porting/linux-framebuffer-platform.md, section 10): the smoke
//! example and the ControlCatalog compare and save the frame the card
//! shows, not the frame the renderer believes it drew.
//!
//! The framebuffer of the CRTC is found with `DRM_IOCTL_MODE_GETFB`, which
//! hands the caller a handle of its buffer object (the caller must be the
//! master of the card or have `CAP_SYS_ADMIN`); the buffer is mapped with
//! `DRM_IOCTL_MODE_MAP_DUMB`, which works for a buffer object a driver
//! keeps in memory the processor can map: the dumb buffers that Mesa's
//! software path on a virtual card (`vkms`) renders into. A buffer in the
//! memory of a GPU is not readable this way.

#![allow(non_camel_case_types)]

use crate::native_unsafe_methods::{ioc, IOC_READ, IOC_WRITE};

/// `struct drm_mode_crtc` of the kernel (`drm_mode.h`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct drm_mode_crtc {
    pub set_connectors_ptr: u64,
    pub count_connectors: u32,
    pub crtc_id: u32,
    pub fb_id: u32,
    pub x: u32,
    pub y: u32,
    pub gamma_size: u32,
    pub mode_valid: u32,
    /// `struct drm_mode_modeinfo`, the same 68 bytes as `drmModeModeInfo`.
    pub mode: [u8; 68],
}

/// `struct drm_mode_fb_cmd` of the kernel.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct drm_mode_fb_cmd {
    pub fb_id: u32,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u32,
    pub depth: u32,
    pub handle: u32,
}

/// `struct drm_mode_map_dumb` of the kernel.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct drm_mode_map_dumb {
    pub handle: u32,
    pub pad: u32,
    pub offset: u64,
}

/// `struct drm_gem_close` of the kernel.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct drm_gem_close {
    pub handle: u32,
    pub pad: u32,
}

/// `struct drm_version` of the kernel: the driver of a card.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct drm_version {
    pub version_major: libc::c_int,
    pub version_minor: libc::c_int,
    pub version_patchlevel: libc::c_int,
    pub name_len: libc::size_t,
    pub name: *mut libc::c_char,
    pub date_len: libc::size_t,
    pub date: *mut libc::c_char,
    pub desc_len: libc::size_t,
    pub desc: *mut libc::c_char,
}

const DRM_IOCTL_BASE: u8 = b'd';
pub const DRM_IOCTL_VERSION: u32 = ioc(IOC_READ | IOC_WRITE, DRM_IOCTL_BASE, 0x00, std::mem::size_of::<drm_version>());
pub const DRM_IOCTL_GEM_CLOSE: u32 = ioc(IOC_WRITE, DRM_IOCTL_BASE, 0x09, std::mem::size_of::<drm_gem_close>());
pub const DRM_IOCTL_MODE_GETCRTC: u32 =
    ioc(IOC_READ | IOC_WRITE, DRM_IOCTL_BASE, 0xA1, std::mem::size_of::<drm_mode_crtc>());
pub const DRM_IOCTL_MODE_GETFB: u32 =
    ioc(IOC_READ | IOC_WRITE, DRM_IOCTL_BASE, 0xAD, std::mem::size_of::<drm_mode_fb_cmd>());
pub const DRM_IOCTL_MODE_MAP_DUMB: u32 =
    ioc(IOC_READ | IOC_WRITE, DRM_IOCTL_BASE, 0xB3, std::mem::size_of::<drm_mode_map_dumb>());

/// The pixels a CRTC shows: rows of `pitch` bytes, `bits_per_pixel` per
/// pixel (32 for the `XRGB8888` of the DRM output: blue, green, red and an
/// unused byte in memory order).
#[derive(Clone, Debug)]
pub struct DrmScanout {
    pub fb_id: u32,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bits_per_pixel: u32,
    pub depth: u32,
    pub data: Vec<u8>,
}

impl DrmScanout {
    /// The red, green and blue of a pixel of a 32-bit frame.
    pub fn pixel(&self, x: u32, y: u32) -> Option<(u8, u8, u8)> {
        if self.bits_per_pixel != 32 || x >= self.width || y >= self.height {
            return None;
        }
        let offset = y as usize * self.pitch as usize + x as usize * 4;
        let px = self.data.get(offset..offset + 4)?;
        Some((px[2], px[1], px[0]))
    }
}

#[cfg(target_os = "linux")]
fn ioctl<T>(fd: i32, request: u32, argument: &mut T) -> std::io::Result<()> {
    // SAFETY: the request is one of the requests above, whose argument is a structure of the
    // type and size it encodes, alive for the call.
    let rv = unsafe { libc::ioctl(fd, request as _, argument as *mut T) };
    if rv < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// The name of the driver of the card `fd` (`vkms` for the virtual KMS
/// driver), so that a test refuses a card that is not the one it expects
/// before it sets a mode on it.
#[cfg(target_os = "linux")]
pub fn driver_name(fd: i32) -> std::io::Result<String> {
    let mut name = [0u8; 64];
    let mut version = drm_version {
        version_major: 0,
        version_minor: 0,
        version_patchlevel: 0,
        name_len: name.len(),
        name: name.as_mut_ptr().cast(),
        date_len: 0,
        date: std::ptr::null_mut(),
        desc_len: 0,
        desc: std::ptr::null_mut(),
    };
    // The kernel writes at most `name_len` bytes of the name and sets the length of the whole.
    ioctl(fd, DRM_IOCTL_VERSION, &mut version)?;
    let length = version.name_len.min(name.len());
    Ok(String::from_utf8_lossy(&name[..length]).into_owned())
}

/// Reads the framebuffer the CRTC `crtc_id` of the card `fd` scans out.
#[cfg(target_os = "linux")]
pub fn read_scanout(fd: i32, crtc_id: u32) -> std::io::Result<DrmScanout> {
    use std::io;

    let mut crtc = drm_mode_crtc {
        set_connectors_ptr: 0,
        count_connectors: 0,
        crtc_id,
        fb_id: 0,
        x: 0,
        y: 0,
        gamma_size: 0,
        mode_valid: 0,
        mode: [0; 68],
    };
    ioctl(fd, DRM_IOCTL_MODE_GETCRTC, &mut crtc)?;
    if crtc.fb_id == 0 {
        return Err(io::Error::other(format!("the CRTC {crtc_id} shows no framebuffer")));
    }

    let mut fb = drm_mode_fb_cmd { fb_id: crtc.fb_id, ..Default::default() };
    ioctl(fd, DRM_IOCTL_MODE_GETFB, &mut fb)?;
    if fb.handle == 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the framebuffer has no handle for this process (not the master of the card and no CAP_SYS_ADMIN)",
        ));
    }

    let read = || -> io::Result<Vec<u8>> {
        let mut map = drm_mode_map_dumb { handle: fb.handle, ..Default::default() };
        ioctl(fd, DRM_IOCTL_MODE_MAP_DUMB, &mut map)?;
        let length = fb.pitch as usize * fb.height as usize;
        // SAFETY: a shared read-only mapping of the buffer at the offset the kernel gave for it.
        let address =
            unsafe { libc::mmap(std::ptr::null_mut(), length, libc::PROT_READ, libc::MAP_SHARED, fd, map.offset as libc::off_t) };
        if address == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the mapping has `length` readable bytes; it is unmapped once, after the copy.
        let data = unsafe {
            let data = std::slice::from_raw_parts(address.cast::<u8>(), length).to_vec();
            libc::munmap(address, length);
            data
        };
        Ok(data)
    };
    let data = read();
    let mut close = drm_gem_close { handle: fb.handle, pad: 0 };
    let _ = ioctl(fd, DRM_IOCTL_GEM_CLOSE, &mut close);

    Ok(DrmScanout {
        fb_id: fb.fb_id,
        width: fb.width,
        height: fb.height,
        pitch: fb.pitch,
        bits_per_pixel: fb.bpp,
        depth: fb.depth,
        data: data?,
    })
}

#[cfg(test)]
mod tests {
    // An addition; the sizes, offsets and request numbers of the kernel
    // (drm.h, drm_mode.h).
    use super::*;
    use std::mem::{offset_of, size_of};

    #[test]
    fn the_structures_and_requests_are_those_of_the_kernel() {
        assert_eq!(size_of::<drm_mode_crtc>(), 104);
        assert_eq!(offset_of!(drm_mode_crtc, crtc_id), 12);
        assert_eq!(offset_of!(drm_mode_crtc, fb_id), 16);
        assert_eq!(offset_of!(drm_mode_crtc, mode), 36);
        assert_eq!(size_of::<drm_mode_fb_cmd>(), 28);
        assert_eq!(size_of::<drm_mode_map_dumb>(), 16);
        assert_eq!(offset_of!(drm_mode_map_dumb, offset), 8);
        assert_eq!(size_of::<drm_gem_close>(), 8);

        assert_eq!(DRM_IOCTL_MODE_GETCRTC, 0xC068_64A1);
        assert_eq!(DRM_IOCTL_MODE_GETFB, 0xC01C_64AD);
        assert_eq!(DRM_IOCTL_MODE_MAP_DUMB, 0xC010_64B3);
        assert_eq!(DRM_IOCTL_GEM_CLOSE, 0x4008_6409);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn the_version_has_the_layout_of_the_kernel_on_64_bits() {
        assert_eq!(size_of::<drm_version>(), 64);
        assert_eq!(offset_of!(drm_version, name_len), 16);
        assert_eq!(offset_of!(drm_version, desc), 56);
        assert_eq!(DRM_IOCTL_VERSION, 0xC040_6400);
    }

    #[test]
    fn a_pixel_of_xrgb8888_is_blue_green_red_in_memory() {
        let scanout = DrmScanout {
            fb_id: 1,
            width: 2,
            height: 1,
            pitch: 8,
            bits_per_pixel: 32,
            depth: 24,
            data: vec![0x99, 0x66, 0x33, 0, 1, 2, 3, 0],
        };
        assert_eq!(scanout.pixel(0, 0), Some((0x33, 0x66, 0x99)));
        assert_eq!(scanout.pixel(1, 0), Some((3, 2, 1)));
        assert_eq!(scanout.pixel(2, 0), None);
    }
}
