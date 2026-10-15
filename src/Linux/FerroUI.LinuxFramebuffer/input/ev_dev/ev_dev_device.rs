//! An event device of the kernel (the port of `EvDevDevice.cs`).
//!
//! The reference asks the `libevdev` library for the name, the event
//! types, the absolute axes and the next event. The same facts are read
//! from the device itself here (docs/porting/linux-framebuffer-platform.md,
//! section 3): `EVIOCGNAME`, `EVIOCGBIT`, `EVIOCGABS` and `read`.

use crate::native_unsafe_methods::{abs_axis, input_absinfo};
use std::collections::HashMap;

/// What is known of a device once it is open: the part of `EvDevDevice`
/// that is values.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EvDevDeviceInfo {
    pub name: Option<String>,
    /// The event types the device reports (`ev_type`).
    pub event_types: Vec<u16>,
    /// The absolute axes the device has, by code. An addition: the
    /// reference reads the two below only.
    pub abs: HashMap<u16, input_absinfo>,
}

impl EvDevDeviceInfo {
    pub fn abs_x(&self) -> Option<input_absinfo> {
        self.abs.get(&abs_axis::ABS_X).copied()
    }

    pub fn abs_y(&self) -> Option<input_absinfo> {
        self.abs.get(&abs_axis::ABS_Y).copied()
    }
}

/// The range of an axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EvDevAxisInfo {
    pub minimum: i32,
    pub maximum: i32,
}

#[cfg(target_os = "linux")]
pub use imp::EvDevDevice;

#[cfg(target_os = "linux")]
mod imp {
    use super::EvDevDeviceInfo;
    use crate::native_unsafe_methods::{abs_axis, ev_type, input_absinfo, input_event, ioc, sys, IOC_READ};
    use std::collections::HashMap;
    use std::io;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

    /// `EVIOCGNAME(len)`.
    const fn eviocgname(len: usize) -> u32 {
        ioc(IOC_READ, b'E', 0x06, len)
    }

    /// `EVIOCGBIT(ev, len)`.
    const fn eviocgbit(ev: u32, len: usize) -> u32 {
        ioc(IOC_READ, b'E', 0x20 + ev, len)
    }

    /// `EVIOCGABS(abs)`.
    const fn eviocgabs(abs: u32) -> u32 {
        ioc(IOC_READ, b'E', 0x40 + abs, std::mem::size_of::<input_absinfo>())
    }

    /// The axes that are asked for: those of the reference and those of
    /// the multi-touch protocol.
    const AXES: [u16; 5] = [
        abs_axis::ABS_X,
        abs_axis::ABS_Y,
        abs_axis::ABS_MT_SLOT,
        abs_axis::ABS_MT_POSITION_X,
        abs_axis::ABS_MT_POSITION_Y,
    ];

    pub struct EvDevDevice {
        fd: OwnedFd,
        info: EvDevDeviceInfo,
    }

    impl EvDevDevice {
        pub fn fd(&self) -> RawFd {
            self.fd.as_raw_fd()
        }

        pub fn info(&self) -> &EvDevDeviceInfo {
            &self.info
        }

        pub fn name(&self) -> Option<&str> {
            self.info.name.as_deref()
        }

        /// The event types the device reports (`ev_type`).
        pub fn event_types(&self) -> &[u16] {
            &self.info.event_types
        }

        pub fn abs_x(&self) -> Option<input_absinfo> {
            self.info.abs_x()
        }

        pub fn abs_y(&self) -> Option<input_absinfo> {
            self.info.abs_y()
        }

        /// Opens the device without blocking and reads what it is.
        pub fn open(device: &str) -> io::Result<Self> {
            let fd = sys::open(device, libc::O_RDONLY | libc::O_NONBLOCK)
                .map_err(|error| io::Error::new(error.kind(), format!("Unable to open {device} code {error}")))?;
            // SAFETY: `fd` was just opened and is owned by nothing else.
            let fd = unsafe { OwnedFd::from_raw_fd(fd) };
            // A failure drops the descriptor, which closes it.
            Self::new(fd).map_err(|error| {
                io::Error::new(error.kind(), format!("Unable to initialize evdev for {device} code {error}"))
            })
        }

        /// The device open on `fd`: reads its name, the event types it
        /// reports and its absolute axes, which the reference's constructor
        /// asks `libevdev` for. Fails when the descriptor is not an event
        /// device (its name cannot be read).
        pub fn new(fd: OwnedFd) -> io::Result<Self> {
            let raw = fd.as_raw_fd();

            let mut name = [0u8; 256];
            sys::ioctl_buffer(raw, eviocgname(name.len()), &mut name)?;
            let len = name.iter().position(|&b| b == 0).unwrap_or(name.len());
            let name = Some(String::from_utf8_lossy(&name[..len]).into_owned());

            // The bits of the event types the device has.
            let mut bits = [0u8; 4];
            let _ = sys::ioctl_buffer(raw, eviocgbit(0, bits.len()), &mut bits);
            let has = |bits: &[u8], bit: u16| bits.get(bit as usize / 8).is_some_and(|b| b & (1 << (bit % 8)) != 0);
            let event_types = ev_type::ALL.into_iter().filter(|&type_| has(&bits, type_)).collect();

            // The axes it has, and the range of each.
            let mut abs_bits = [0u8; 8];
            let _ = sys::ioctl_buffer(raw, eviocgbit(ev_type::EV_ABS as u32, abs_bits.len()), &mut abs_bits);
            let mut abs = HashMap::new();
            for code in AXES {
                if !has(&abs_bits, code) {
                    continue;
                }
                let mut absinfo = input_absinfo::default();
                if sys::ioctl_struct(raw, eviocgabs(code as u32), &mut absinfo).is_ok() {
                    abs.insert(code, absinfo);
                }
            }

            Ok(Self { fd, info: EvDevDeviceInfo { name, event_types, abs } })
        }

        /// The events that can be read now, in order.
        pub fn read_events(fd: RawFd) -> Vec<input_event> {
            let mut rv = Vec::new();
            loop {
                // SAFETY: an all-zero `input_event` is valid.
                let mut buffer: [libc::input_event; 64] = unsafe { std::mem::zeroed() };
                let size = std::mem::size_of::<libc::input_event>();
                // SAFETY: reads at most the size of the local buffer; the
                // kernel writes whole events.
                let read = unsafe { libc::read(fd, buffer.as_mut_ptr().cast(), size * buffer.len()) };
                if read <= 0 {
                    break;
                }
                for ev in &buffer[..read as usize / size] {
                    rv.push(input_event {
                        tv_sec: ev.time.tv_sec as i64,
                        tv_usec: ev.time.tv_usec as i64,
                        type_: ev.type_,
                        code: ev.code,
                        value: ev.value,
                    });
                }
            }
            rv
        }
    }

    #[cfg(test)]
    mod tests {
        // Not from the reference.
        use super::*;

        #[test]
        fn the_request_numbers_are_those_of_the_kernel() {
            // From linux/input.h, for a 64-bit and a 32-bit kernel alike.
            assert_eq!(eviocgname(256), 0x8100_4506);
            assert_eq!(eviocgbit(0, 4), 0x8004_4520);
            assert_eq!(eviocgabs(0), 0x8018_4540);
            assert_eq!(eviocgabs(0x35), 0x8018_4575);
        }
    }
}
