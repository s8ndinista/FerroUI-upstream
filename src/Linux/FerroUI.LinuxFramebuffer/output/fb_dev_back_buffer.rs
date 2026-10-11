//! The back buffer of the framebuffer output (the port of
//! `FbDevBackBuffer.cs`): a frame is drawn into memory of the process and
//! copied to the mapped memory of the device after the vertical sync,
//! optionally on a thread of its own.

use super::fb_device::{FbMapping, IFbDevice};
use crate::native_unsafe_methods::{fb_fix_screeninfo, fb_var_screeninfo};
use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormat};
use ferroui_base::{PixelSize, Vector};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What the output shares with its render targets and with the thread
/// that copies frames: the device and its mapped memory, both gone once
/// the output is disposed.
pub(crate) struct FbShared {
    pub device: Mutex<Option<Box<dyn IFbDevice>>>,
    pub mapping: Mutex<Option<Arc<FbMapping>>>,
}

impl FbShared {
    pub fn mapping(&self) -> Option<Arc<FbMapping>> {
        lock(&self.mapping).clone()
    }

    pub fn wait_for_vsync(&self) {
        if let Some(device) = lock(&self.device).as_ref() {
            device.wait_for_vsync();
        }
    }

    /// `BlitToDevice`.
    fn blit_to_device(&self, pixels: &[u8]) {
        self.wait_for_vsync();
        if let Some(mapping) = self.mapping() {
            mapping.copy_from(pixels);
        }
    }
}

/// The pixel format a device with this variable information is drawn in.
pub(crate) fn format_of(var_info: &fb_var_screeninfo) -> (PixelFormat, AlphaFormat) {
    if var_info.bits_per_pixel == 16 {
        (PixelFormat::RGB565, AlphaFormat::Opaque)
    } else if var_info.bits_per_pixel == 32 && var_info.blue.offset == 16 {
        (PixelFormat::RGBA8888, AlphaFormat::Premul)
    } else {
        (PixelFormat::BGRA8888, AlphaFormat::Premul)
    }
}

/// The memory a frame is drawn into.
enum Pixels {
    /// The back buffer, taken out of it for the frame.
    Owned(RefCell<Vec<u8>>),
    /// The mapped memory of the device.
    Mapped(Arc<FbMapping>),
}

/// A locked framebuffer of the output (`LockedFramebuffer` of the
/// reference, with the memory it points to).
pub(crate) struct FbLockedFramebuffer {
    pixels: Pixels,
    size: PixelSize,
    row_bytes: i32,
    dpi: Vector,
    format: PixelFormat,
    alpha_format: AlphaFormat,
    on_dispose: RefCell<Option<Box<dyn FnOnce(Vec<u8>, bool)>>>,
}

impl FbLockedFramebuffer {
    fn byte_length(&self) -> usize {
        self.row_bytes.max(0) as usize * self.size.height.max(0) as usize
    }

    fn finish(&self, present: bool) {
        let on_dispose = self.on_dispose.borrow_mut().take();
        if let Some(on_dispose) = on_dispose {
            let pixels = match &self.pixels {
                Pixels::Owned(pixels) => std::mem::take(&mut *pixels.borrow_mut()),
                Pixels::Mapped(_) => Vec::new(),
            };
            on_dispose(pixels, present);
        }
    }
}

impl ILockedFramebuffer for FbLockedFramebuffer {
    fn address(&self) -> *mut u8 {
        match &self.pixels {
            Pixels::Owned(pixels) => pixels.borrow_mut().as_mut_ptr(),
            Pixels::Mapped(mapping) => mapping.address(),
        }
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        match &self.pixels {
            Pixels::Owned(pixels) => access(&mut pixels.borrow_mut()),
            Pixels::Mapped(mapping) => {
                let length = self.byte_length().min(mapping.len());
                // SAFETY: the mapping is valid for `length` bytes while
                // this object holds it, and this frame is the one writer
                // (the output hands the memory to one renderer at a time).
                access(unsafe { std::slice::from_raw_parts_mut(mapping.address(), length) })
            }
        }
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    fn row_bytes(&self) -> i32 {
        self.row_bytes
    }

    fn dpi(&self) -> Vector {
        self.dpi
    }

    fn format(&self) -> PixelFormat {
        self.format
    }

    fn alpha_format(&self) -> AlphaFormat {
        self.alpha_format
    }

    fn dispose(&self) {
        self.finish(true);
    }
}

impl Drop for FbLockedFramebuffer {
    fn drop(&mut self) {
        // A frame that is dropped without having been disposed gives its
        // memory back and is not shown.
        self.finish(false);
    }
}

/// `LockFb` for the mapped memory of the device.
pub(crate) fn lock_mapped(
    mapping: Arc<FbMapping>,
    var_info: &fb_var_screeninfo,
    fixed_info: &fb_fix_screeninfo,
    dpi: Vector,
) -> Rc<dyn ILockedFramebuffer> {
    let (format, alpha_format) = format_of(var_info);
    Rc::new(FbLockedFramebuffer {
        pixels: Pixels::Mapped(mapping),
        size: PixelSize::new(var_info.xres as i32, var_info.yres as i32),
        row_bytes: fixed_info.line_length as i32,
        dpi,
        format,
        alpha_format,
        on_dispose: RefCell::new(None),
    })
}

#[derive(Default)]
struct BlitState {
    signal_to_thread: bool,
    exit: bool,
    transfer_in_progress: bool,
}

struct BlitShared {
    state: Mutex<BlitState>,
    changed: Condvar,
}

/// The thread that copies frames to the device.
struct AsyncFbBlitter {
    shared: Arc<BlitShared>,
    thread: Option<JoinHandle<()>>,
}

impl AsyncFbBlitter {
    fn new(fb: Arc<FbShared>, pixels: Arc<Mutex<Vec<u8>>>) -> Self {
        let shared = Arc::new(BlitShared { state: Mutex::new(BlitState::default()), changed: Condvar::new() });
        let worker = shared.clone();
        // The reference gives the thread the highest priority; the
        // standard library has no notion of one.
        let thread = thread::Builder::new()
            .name("FbDevBackBuffer::AsyncBlitter".to_owned())
            .spawn(move || loop {
                {
                    let mut state = lock(&worker.state);
                    while !state.signal_to_thread {
                        state = worker.changed.wait(state).unwrap_or_else(PoisonError::into_inner);
                    }
                    state.signal_to_thread = false;
                    if state.exit {
                        return;
                    }
                }
                fb.blit_to_device(&lock(&pixels));
                lock(&worker.state).transfer_in_progress = false;
                worker.changed.notify_all();
            })
            .unwrap_or_else(|error| panic!("Unable to start the framebuffer blit thread: {error}"));
        Self { shared, thread: Some(thread) }
    }

    fn wait_for_transfer(&self) {
        let mut state = lock(&self.shared.state);
        while state.transfer_in_progress {
            state = self.shared.changed.wait(state).unwrap_or_else(PoisonError::into_inner);
        }
    }

    fn begin_blit(shared: &BlitShared) {
        {
            let mut state = lock(&shared.state);
            state.transfer_in_progress = true;
            state.signal_to_thread = true;
        }
        shared.changed.notify_all();
    }
}

impl Drop for AsyncFbBlitter {
    fn drop(&mut self) {
        {
            let mut state = lock(&self.shared.state);
            state.exit = true;
            state.signal_to_thread = true;
        }
        self.shared.changed.notify_all();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub(crate) struct FbDevBackBuffer {
    fb: Arc<FbShared>,
    fixed_info: fb_fix_screeninfo,
    var_info: fb_var_screeninfo,
    /// The memory of the buffer (`Address`); empty while a frame holds it.
    pixels: Arc<Mutex<Vec<u8>>>,
    async_blit: Option<AsyncFbBlitter>,
}

impl FbDevBackBuffer {
    pub fn new(
        fb: Arc<FbShared>,
        fixed_info: fb_fix_screeninfo,
        var_info: fb_var_screeninfo,
        async_blit: bool,
    ) -> Self {
        let length = fixed_info.line_length as usize * var_info.yres as usize;
        let pixels = Arc::new(Mutex::new(vec![0u8; length]));
        let async_blit = async_blit.then(|| AsyncFbBlitter::new(fb.clone(), pixels.clone()));
        Self { fb, fixed_info, var_info, pixels, async_blit }
    }

    /// # Panics
    /// Panics when the previous frame of the buffer is still locked.
    pub fn lock(&self, dpi: Vector) -> Rc<dyn ILockedFramebuffer> {
        if let Some(async_blit) = &self.async_blit {
            async_blit.wait_for_transfer();
        }

        let taken = std::mem::take(&mut *lock(&self.pixels));
        if taken.is_empty() && self.row_bytes() * self.size().height != 0 {
            panic!("The framebuffer is already locked");
        }

        let fb = self.fb.clone();
        let pixels = self.pixels.clone();
        let blitter = self.async_blit.as_ref().map(|blitter| blitter.shared.clone());
        let on_dispose = move |frame: Vec<u8>, present: bool| {
            let mut pixels = lock(&pixels);
            *pixels = frame;
            if !present {
                return;
            }
            match &blitter {
                Some(blitter) => {
                    drop(pixels);
                    AsyncFbBlitter::begin_blit(blitter);
                }
                None => fb.blit_to_device(&pixels),
            }
        };

        let (format, alpha_format) = format_of(&self.var_info);
        Rc::new(FbLockedFramebuffer {
            pixels: Pixels::Owned(RefCell::new(taken)),
            size: self.size(),
            row_bytes: self.row_bytes(),
            dpi,
            format,
            alpha_format,
            on_dispose: RefCell::new(Some(Box::new(on_dispose))),
        })
    }

    pub fn size(&self) -> PixelSize {
        PixelSize::new(self.var_info.xres as i32, self.var_info.yres as i32)
    }

    pub fn row_bytes(&self) -> i32 {
        self.fixed_info.line_length as i32
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use crate::native_unsafe_methods::fb_bitfield;

    #[test]
    fn the_format_follows_the_depth_and_the_offset_of_blue() {
        let mut var_info = fb_var_screeninfo { bits_per_pixel: 16, ..Default::default() };
        assert_eq!(format_of(&var_info), (PixelFormat::RGB565, AlphaFormat::Opaque));

        var_info.bits_per_pixel = 32;
        var_info.blue = fb_bitfield { offset: 16, length: 8, msb_right: 0 };
        assert_eq!(format_of(&var_info), (PixelFormat::RGBA8888, AlphaFormat::Premul));

        var_info.blue.offset = 0;
        assert_eq!(format_of(&var_info), (PixelFormat::BGRA8888, AlphaFormat::Premul));

        // Anything else is taken to be BGRA, as the reference does.
        var_info.bits_per_pixel = 24;
        assert_eq!(format_of(&var_info), (PixelFormat::BGRA8888, AlphaFormat::Premul));
    }
}
