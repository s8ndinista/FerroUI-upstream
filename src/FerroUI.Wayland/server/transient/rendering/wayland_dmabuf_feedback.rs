//! The default dmabuf feedback of the compositor (the port of
//! `WaylandDmabufFeedback.cs`): the format table, the main device, and the
//! tranches of format and modifier pairs that `zwp_linux_dmabuf_feedback_v1`
//! sends until `done`.
//!
//! The listener class of the reference is [`FeedbackListener`]: the user
//! data of the feedback object, which the `Dispatch` implementation of the
//! worker's state hands the events to. Its methods take the bytes of an
//! event, so the decoding runs (and is tested) without a compositor.

use std::sync::{Mutex, PoisonError};

/// A format and a modifier of a buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DmabufFormatModifierPair {
    pub format: u32,
    pub modifier: u64,
}

impl DmabufFormatModifierPair {
    pub fn new(format: u32, modifier: u64) -> Self {
        Self { format, modifier }
    }
}

/// A tranche of the feedback: the device it targets, its pairs, whether it is for scan-out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DmabufTranche {
    pub target_device: u64,
    pub formats: Vec<DmabufFormatModifierPair>,
    pub scanout: bool,
}

/// The feedback the compositor sent.
#[derive(Debug, Default)]
pub struct WaylandDmabufFeedback {
    format_table: Vec<DmabufFormatModifierPair>,

    pending_target_device: u64,
    pending_formats: Vec<DmabufFormatModifierPair>,
    pending_scanout: bool,

    main_device: u64,
    tranches: Vec<DmabufTranche>,
    is_complete: bool,
}

impl WaylandDmabufFeedback {
    pub const PROT_READ: i32 = 1;
    pub const MAP_PRIVATE: i32 = 2;
    pub const FORMAT_TABLE_ENTRY_SIZE: usize = 16;

    pub fn new() -> Self {
        Self::default()
    }

    /// The device number of the compositor's main device (`dev_t`).
    pub fn main_device(&self) -> u64 {
        self.main_device
    }

    pub fn tranches(&self) -> &[DmabufTranche] {
        &self.tranches
    }

    /// Whether `done` arrived.
    pub fn is_complete(&self) -> bool {
        self.is_complete
    }

    /// The listener the feedback object is created with: its user data.
    pub fn listener(self) -> FeedbackListener {
        FeedbackListener::new(self)
    }

    fn reset_pending_tranche(&mut self) {
        self.pending_target_device = 0;
        self.pending_formats = Vec::new();
        self.pending_scanout = false;
    }

    /// The bytes of a device number of an event (an array of the size of `dev_t`, in the
    /// native byte order); zero when there are fewer than eight.
    fn read_device(device: &[u8]) -> u64 {
        device.get(..8).and_then(|bytes| bytes.try_into().ok()).map_or(0, u64::from_ne_bytes)
    }
}

/// The events of `zwp_linux_dmabuf_feedback_v1` applied to the feedback (the
/// `FeedbackListener` of the reference). The user data of the feedback object, which every
/// thread may hold: the feedback is behind a lock.
#[derive(Debug)]
pub struct FeedbackListener {
    feedback: Mutex<WaylandDmabufFeedback>,
}

impl FeedbackListener {
    /// The listener of a feedback, which it owns from then on (the reference holds a reference
    /// to the feedback, whose properties it sets).
    pub fn new(feedback: WaylandDmabufFeedback) -> Self {
        Self { feedback: Mutex::new(feedback) }
    }

    /// The feedback, as it stands.
    pub fn with_feedback<R>(&self, f: impl FnOnce(&WaylandDmabufFeedback) -> R) -> R {
        f(&self.feedback.lock().unwrap_or_else(PoisonError::into_inner))
    }

    fn update(&self, f: impl FnOnce(&mut WaylandDmabufFeedback)) {
        f(&mut self.feedback.lock().unwrap_or_else(PoisonError::into_inner));
    }

    /// The format table: the mapped bytes of the descriptor of the event (the reference maps
    /// it here; the port maps it in the `Dispatch` implementation, which has the descriptor).
    pub fn format_table(&self, table: &[u8]) {
        self.update(|feedback| {
            // The table contains a tightly packed array of consecutive format + modifier
            // pairs. Each pair is 16 bytes wide. It contains a format as a 32-bit unsigned
            // integer, followed by 4 bytes of unused padding, and a modifier as a 64-bit
            // unsigned integer. The native endianness is used.
            let count = table.len() / WaylandDmabufFeedback::FORMAT_TABLE_ENTRY_SIZE;
            feedback.format_table.clear();
            for i in 0..count {
                let entry = &table[i * WaylandDmabufFeedback::FORMAT_TABLE_ENTRY_SIZE..];
                let format = u32::from_ne_bytes([entry[0], entry[1], entry[2], entry[3]]);
                let modifier = u64::from_ne_bytes([
                    entry[8], entry[9], entry[10], entry[11], entry[12], entry[13], entry[14], entry[15],
                ]);
                feedback.format_table.push(DmabufFormatModifierPair::new(format, modifier));
            }
        });
    }

    pub fn main_device(&self, device: &[u8]) {
        self.update(|feedback| feedback.main_device = WaylandDmabufFeedback::read_device(device));
    }

    pub fn tranche_target_device(&self, device: &[u8]) {
        self.update(|feedback| feedback.pending_target_device = WaylandDmabufFeedback::read_device(device));
    }

    pub fn tranche_formats(&self, indices: &[u8]) {
        self.update(|feedback| {
            for index in indices.chunks_exact(2) {
                let index = usize::from(u16::from_ne_bytes([index[0], index[1]]));
                if let Some(pair) = feedback.format_table.get(index) {
                    let pair = *pair;
                    feedback.pending_formats.push(pair);
                }
            }
        });
    }

    /// `scanout` is whether the flags have the scan-out flag.
    pub fn tranche_flags(&self, scanout: bool) {
        self.update(|feedback| feedback.pending_scanout = scanout);
    }

    pub fn tranche_done(&self) {
        self.update(|feedback| {
            let tranche = DmabufTranche {
                target_device: feedback.pending_target_device,
                formats: feedback.pending_formats.clone(),
                scanout: feedback.pending_scanout,
            };
            feedback.tranches.push(tranche);
            feedback.reset_pending_tranche();
        });
    }

    pub fn done(&self) {
        self.update(|feedback| feedback.is_complete = true);
    }
}

#[cfg(target_os = "linux")]
mod dispatch {
    use super::FeedbackListener;
    use crate::server::interop::unsafe_native_methods::MemoryMapping;
    use crate::server::wayland_worker::WaylandWorkerState;
    use std::os::fd::AsFd;
    use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
    use wayland_protocols::wp::linux_dmabuf::zv1::client::zwp_linux_dmabuf_feedback_v1::{
        self, TrancheFlags, ZwpLinuxDmabufFeedbackV1,
    };

    impl Dispatch<ZwpLinuxDmabufFeedbackV1, FeedbackListener> for WaylandWorkerState {
        fn event(
            _: &mut Self,
            _: &ZwpLinuxDmabufFeedbackV1,
            event: zwp_linux_dmabuf_feedback_v1::Event,
            listener: &FeedbackListener,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            match event {
                zwp_linux_dmabuf_feedback_v1::Event::FormatTable { fd, size } => {
                    // The descriptor is consumed: it is closed when the event is done with,
                    // after the mapping (which is unmapped first).
                    let Ok(mapping) = MemoryMapping::private_read(fd.as_fd(), size as usize) else {
                        return;
                    };
                    listener.format_table(mapping.as_slice());
                }
                zwp_linux_dmabuf_feedback_v1::Event::MainDevice { device } => listener.main_device(&device),
                zwp_linux_dmabuf_feedback_v1::Event::TrancheTargetDevice { device } => {
                    listener.tranche_target_device(&device);
                }
                zwp_linux_dmabuf_feedback_v1::Event::TrancheFormats { indices } => listener.tranche_formats(&indices),
                zwp_linux_dmabuf_feedback_v1::Event::TrancheFlags { flags } => {
                    let scanout = match flags {
                        WEnum::Value(flags) => flags.contains(TrancheFlags::Scanout),
                        WEnum::Unknown(bits) => bits & TrancheFlags::Scanout.bits() != 0,
                    };
                    listener.tranche_flags(scanout);
                }
                zwp_linux_dmabuf_feedback_v1::Event::TrancheDone => listener.tranche_done(),
                zwp_linux_dmabuf_feedback_v1::Event::Done => listener.done(),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    fn table(pairs: &[(u32, u64)]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (format, modifier) in pairs {
            bytes.extend_from_slice(&format.to_ne_bytes());
            bytes.extend_from_slice(&[0xAA; 4]);
            bytes.extend_from_slice(&modifier.to_ne_bytes());
        }
        bytes
    }

    fn indices(values: &[u16]) -> Vec<u8> {
        values.iter().flat_map(|value| value.to_ne_bytes()).collect()
    }

    #[test]
    fn the_tranches_take_their_pairs_from_the_format_table_by_index() {
        let listener = WaylandDmabufFeedback::new().listener();
        listener.format_table(&table(&[(0x3432_5241, 0), (0x3432_5258, 0x00ff_ffff_ffff_ffff), (7, 9)]));
        listener.main_device(&0xE280u64.to_ne_bytes());

        listener.tranche_target_device(&0xE280u64.to_ne_bytes());
        // Index 5 is beyond the table and is left out.
        listener.tranche_formats(&indices(&[1, 0, 5]));
        listener.tranche_flags(true);
        listener.tranche_done();

        listener.tranche_target_device(&0xE281u64.to_ne_bytes());
        listener.tranche_formats(&indices(&[2]));
        listener.tranche_done();
        listener.with_feedback(|feedback| assert!(!feedback.is_complete()));
        listener.done();

        listener.with_feedback(|feedback| {
            assert!(feedback.is_complete());
            assert_eq!(feedback.main_device(), 0xE280);
            assert_eq!(
                feedback.tranches(),
                &[
                    DmabufTranche {
                        target_device: 0xE280,
                        formats: vec![
                            DmabufFormatModifierPair::new(0x3432_5258, 0x00ff_ffff_ffff_ffff),
                            DmabufFormatModifierPair::new(0x3432_5241, 0),
                        ],
                        scanout: true,
                    },
                    // The pending tranche was reset after the first: no scan-out flag.
                    DmabufTranche { target_device: 0xE281, formats: vec![DmabufFormatModifierPair::new(7, 9)], scanout: false },
                ]
            );
        });
    }

    #[test]
    fn a_new_format_table_replaces_the_old_one_and_a_partial_entry_is_ignored() {
        let listener = WaylandDmabufFeedback::new().listener();
        listener.format_table(&table(&[(1, 1), (2, 2)]));
        let mut bytes = table(&[(3, 3)]);
        bytes.extend_from_slice(&[0; 7]);
        listener.format_table(&bytes);
        listener.tranche_formats(&indices(&[0, 1]));
        listener.tranche_done();
        listener.with_feedback(|feedback| {
            assert_eq!(feedback.tranches()[0].formats, vec![DmabufFormatModifierPair::new(3, 3)]);
        });
    }

    #[test]
    fn a_device_shorter_than_eight_bytes_is_zero() {
        let listener = WaylandDmabufFeedback::new().listener();
        listener.main_device(&[1, 2, 3]);
        listener.with_feedback(|feedback| assert_eq!(feedback.main_device(), 0));
    }
}
