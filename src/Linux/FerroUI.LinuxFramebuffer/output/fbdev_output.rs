//! The framebuffer device as an output (the port of `FbdevOutput.cs`):
//! frames are drawn in software into the memory of the device.

use super::fb_dev_back_buffer::{lock_mapped, FbDevBackBuffer, FbShared};
use super::fb_device::IFbDevice;
use super::{FbDevOutputOptions, IOutputBackend};
use crate::native_unsafe_methods::{fb_bitfield, fb_fix_screeninfo, fb_var_screeninfo};
use ferroui_base::platform::surfaces::{
    FramebufferLockProperties, FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget,
    IPlatformRenderSurface,
};
use ferroui_base::platform::{ILockedFramebuffer, PixelFormat, RenderTargetSceneInfo};
use ferroui_base::{PixelSize, Vector};
use std::any::Any;
use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

/// What the output and its render targets share.
struct Inner {
    fb: Arc<FbShared>,
    fixed_info: fb_fix_screeninfo,
    var_info: fb_var_screeninfo,
    options: FbDevOutputOptions,
    locked_at_least_once: AtomicBool,
    /// The bits of the scaling, a floating point number.
    scaling: AtomicU64,
}

impl Inner {
    fn scaling(&self) -> f64 {
        f64::from_bits(self.scaling.load(Ordering::Relaxed))
    }
}

/// A framebuffer device as the output of the platform.
pub struct FbdevOutput {
    inner: Arc<Inner>,
    id: String,
}

/// Writes the description of `format` into the variable information
/// (`SetBpp`).
pub(crate) fn set_bpp(var_info: &mut fb_var_screeninfo, format: PixelFormat) -> io::Result<()> {
    let eight = fb_bitfield { offset: 0, length: 8, msb_right: 0 };
    if format == PixelFormat::RGBA8888 {
        var_info.bits_per_pixel = 32;
        var_info.grayscale = 0;
        var_info.red = eight;
        var_info.blue = eight;
        var_info.green = eight;
        var_info.transp = eight;
        var_info.green.offset = 8;
        var_info.blue.offset = 16;
        var_info.transp.offset = 24;
    } else if format == PixelFormat::BGRA8888 {
        var_info.bits_per_pixel = 32;
        var_info.grayscale = 0;
        var_info.red = eight;
        var_info.blue = eight;
        var_info.green = eight;
        var_info.transp = eight;
        var_info.green.offset = 8;
        var_info.red.offset = 16;
        var_info.transp.offset = 24;
    } else if format == PixelFormat::RGB565 {
        var_info.bits_per_pixel = 16;
        var_info.grayscale = 0;
        var_info.red = fb_bitfield::default();
        var_info.blue = fb_bitfield::default();
        var_info.green = fb_bitfield::default();
        var_info.transp = fb_bitfield::default();
        var_info.red.length = 5;
        var_info.green.offset = 5;
        var_info.green.length = 6;
        var_info.blue.offset = 11;
        var_info.blue.length = 5;
    } else {
        return Err(io::Error::new(io::ErrorKind::Unsupported, format!("Pixel format {format:?} is not supported")));
    }
    Ok(())
}

impl FbdevOutput {
    /// Opens the framebuffer device `file_name`, or the one the
    /// environment variable `FRAMEBUFFER` names, or `/dev/fb0`.
    #[cfg(target_os = "linux")]
    pub fn new(file_name: Option<&str>) -> io::Result<Arc<Self>> {
        Self::with_format(file_name, None)
    }

    /// Opens a framebuffer device and asks it for `format`.
    #[cfg(target_os = "linux")]
    pub fn with_format(file_name: Option<&str>, format: Option<PixelFormat>) -> io::Result<Arc<Self>> {
        Self::with_options(FbDevOutputOptions {
            file_name: file_name.map(str::to_owned),
            pixel_format: format,
            ..Default::default()
        })
    }

    /// Opens the framebuffer device of the options.
    #[cfg(target_os = "linux")]
    pub fn with_options(options: FbDevOutputOptions) -> io::Result<Arc<Self>> {
        let file_name = options
            .file_name
            .clone()
            .or_else(|| std::env::var("FRAMEBUFFER").ok())
            .unwrap_or_else(|| "/dev/fb0".to_owned());
        let device = super::LinuxFbDevice::open(&file_name)?;
        Self::with_device(Box::new(device), options)
    }

    /// The output over a device that is already open. An addition: with a
    /// [`FileFbDevice`](super::FileFbDevice) the output runs without a
    /// framebuffer device.
    pub fn with_device(mut device: Box<dyn IFbDevice>, options: FbDevOutputOptions) -> io::Result<Arc<Self>> {
        // `Init`. A failure drops the device, which closes it.
        let mut var_info = device.get_var_screen_info()?;

        if let Some(format) = options.pixel_format {
            set_bpp(&mut var_info, format)?;

            if device.put_var_screen_info(&var_info).is_err() {
                var_info.transp = fb_bitfield::default();
            }

            let _ = device.put_var_screen_info(&var_info);

            var_info = device.get_var_screen_info()?;

            if var_info.bits_per_pixel != 32 {
                return Err(io::Error::other("Unable to set 32-bit display mode"));
            }
        }

        let fixed_info = device.get_fix_screen_info()?;

        let mapped_length = fixed_info.line_length as usize * var_info.yres as usize;
        let mapping = device.map(mapped_length)?;

        let id = fixed_info.id_string();
        let scaling = options.scaling;
        Ok(Arc::new(Self {
            inner: Arc::new(Inner {
                fb: Arc::new(FbShared {
                    device: Mutex::new(Some(device)),
                    mapping: Mutex::new(Some(Arc::new(mapping))),
                }),
                fixed_info,
                var_info,
                options,
                locked_at_least_once: AtomicBool::new(false),
                scaling: AtomicU64::new(scaling.to_bits()),
            }),
            id,
        }))
    }

    /// The identification string of the device.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Releases the mapped memory (once the frame that uses it has ended)
    /// and closes the device.
    pub fn dispose(&self) {
        self.inner.fb.mapping.lock().unwrap_or_else(PoisonError::into_inner).take();
        self.inner.fb.device.lock().unwrap_or_else(PoisonError::into_inner).take();
    }
}

/// What a render target of the output keeps between frames: the back
/// buffer, which the reference keeps in the output (`_backBuffer`).
struct TargetState {
    inner: Arc<Inner>,
    back_buffer: RefCell<Option<FbDevBackBuffer>>,
}

impl TargetState {
    fn lock(&self) -> (Rc<dyn ILockedFramebuffer>, FramebufferLockProperties) {
        let inner = &self.inner;
        let Some(mapping) = inner.fb.mapping() else {
            panic!("Cannot access a disposed object: LinuxFramebuffer");
        };

        let dpi = Vector::new(96.0, 96.0) * inner.scaling();

        if inner.options.render_directly_to_mapped_memory {
            let properties = FramebufferLockProperties {
                previous_frame_is_retained: inner.locked_at_least_once.swap(true, Ordering::Relaxed),
            };
            inner.fb.wait_for_vsync();
            return (lock_mapped(mapping, &inner.var_info, &inner.fixed_info, dpi), properties);
        }

        let retained = inner.locked_at_least_once.swap(true, Ordering::Relaxed) && self.back_buffer.borrow().is_some();
        let properties = FramebufferLockProperties { previous_frame_is_retained: retained };
        let mut back_buffer = self.back_buffer.borrow_mut();
        let back_buffer = back_buffer.get_or_insert_with(|| {
            FbDevBackBuffer::new(
                inner.fb.clone(),
                inner.fixed_info,
                inner.var_info,
                inner.options.use_async_front_buffer_blit == Some(true),
            )
        });
        (back_buffer.lock(dpi), properties)
    }
}

impl IPlatformRenderSurface for FbdevOutput {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for FbdevOutput {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        let state = TargetState { inner: self.inner.clone(), back_buffer: RefCell::new(None) };
        Rc::new(FuncFramebufferRenderTarget::with_scene_info(move |_: &RenderTargetSceneInfo| state.lock(), true))
    }
}

impl IOutputBackend for FbdevOutput {
    fn pixel_size(&self) -> PixelSize {
        let device = self.inner.fb.device.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(device) = device.as_ref() else {
            panic!("Cannot access a disposed object: LinuxFramebuffer");
        };
        match device.get_var_screen_info() {
            Ok(nfo) => PixelSize::new(nfo.xres as i32, nfo.yres as i32),
            Err(error) => panic!("{error}"),
        }
    }

    fn scaling(&self) -> f64 {
        self.inner.scaling()
    }

    fn set_scaling(&self, value: f64) {
        self.inner.scaling.store(value.to_bits(), Ordering::Relaxed);
    }

    fn as_render_surface(self: Arc<Self>) -> Arc<dyn IPlatformRenderSurface> {
        self
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file. The
    // device is a plain file.
    use super::*;
    use crate::output::FileFbDevice;

    fn scene() -> RenderTargetSceneInfo {
        RenderTargetSceneInfo::new(
            PixelSize::new(1, 1),
            1.0,
            ferroui_base::rendering::composition::CompositionTransparencyLevel::None,
        )
    }

    fn temp_file(name: &str) -> String {
        let dir = std::env::temp_dir().join(format!("ferroui-linuxfb-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name).to_string_lossy().into_owned()
    }

    fn var_info(width: u32, height: u32, format: PixelFormat) -> fb_var_screeninfo {
        let mut var_info = fb_var_screeninfo { xres: width, yres: height, ..Default::default() };
        set_bpp(&mut var_info, format).unwrap();
        var_info
    }

    #[test]
    fn set_bpp_writes_the_bit_fields_of_the_three_formats() {
        let rgba = var_info(1, 1, PixelFormat::RGBA8888);
        assert_eq!((rgba.bits_per_pixel, rgba.red.offset, rgba.green.offset, rgba.blue.offset), (32, 0, 8, 16));
        assert_eq!((rgba.transp.offset, rgba.transp.length, rgba.red.length), (24, 8, 8));

        let bgra = var_info(1, 1, PixelFormat::BGRA8888);
        assert_eq!((bgra.bits_per_pixel, bgra.red.offset, bgra.green.offset, bgra.blue.offset), (32, 16, 8, 0));

        // The fields as the reference writes them: red in the low bits.
        let rgb565 = var_info(1, 1, PixelFormat::RGB565);
        assert_eq!(rgb565.bits_per_pixel, 16);
        assert_eq!((rgb565.red.offset, rgb565.red.length), (0, 5));
        assert_eq!((rgb565.green.offset, rgb565.green.length), (5, 6));
        assert_eq!((rgb565.blue.offset, rgb565.blue.length), (11, 5));
        assert_eq!(rgb565.transp, fb_bitfield::default());

        let mut other = fb_var_screeninfo::default();
        assert!(set_bpp(&mut other, ferroui_base::platform::PixelFormats::GRAY8).is_err());
    }

    fn draw(output: &FbdevOutput, target: &Rc<dyn IFramebufferRenderTarget>, value: u8) -> FramebufferLockProperties {
        let (fb, properties) = target.lock(&scene());
        assert_eq!(fb.size(), output.pixel_size());
        fb.with_data(&mut |data| data.fill(value));
        fb.dispose();
        properties
    }

    #[test]
    fn a_frame_arrives_in_the_file_when_it_is_disposed() {
        let path = temp_file("back-buffer.fb");
        let device = FileFbDevice::create(&path, var_info(8, 4, PixelFormat::BGRA8888)).unwrap();
        let output = FbdevOutput::with_device(Box::new(device), FbDevOutputOptions::default()).unwrap();
        assert_eq!(output.id(), "file");
        assert_eq!(output.pixel_size(), PixelSize::new(8, 4));

        let target = output.create_framebuffer_render_target();
        assert!(target.retains_frame_contents());

        let (fb, properties) = target.lock(&scene());
        assert!(!properties.previous_frame_is_retained);
        assert_eq!((fb.row_bytes(), fb.format()), (32, PixelFormat::BGRA8888));
        assert_eq!(fb.dpi(), Vector::new(96.0, 96.0));
        fb.with_data(&mut |data| data.fill(0x5a));
        // Nothing is on the device before the frame ends.
        assert!(std::fs::read(&path).unwrap().iter().all(|&b| b == 0));
        fb.dispose();
        let content = std::fs::read(&path).unwrap();
        assert_eq!(content.len(), 8 * 4 * 4);
        assert!(content.iter().all(|&b| b == 0x5a));

        // The second frame finds the first one in the back buffer.
        let (fb, properties) = target.lock(&scene());
        assert!(properties.previous_frame_is_retained);
        fb.with_data(&mut |data| assert!(data.iter().all(|&b| b == 0x5a)));
        // A frame that is dropped without being disposed is not shown.
        fb.with_data(&mut |data| data.fill(0x11));
        drop(fb);
        assert!(std::fs::read(&path).unwrap().iter().all(|&b| b == 0x5a));

        output.set_scaling(2.0);
        let (fb, _) = target.lock(&scene());
        assert_eq!(fb.dpi(), Vector::new(192.0, 192.0));
        fb.dispose();
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_frame_drawn_directly_is_in_the_file_at_once() {
        let path = temp_file("direct.fb");
        let device = FileFbDevice::create(&path, var_info(4, 2, PixelFormat::RGBA8888)).unwrap();
        let options = FbDevOutputOptions { render_directly_to_mapped_memory: true, ..Default::default() };
        let output = FbdevOutput::with_device(Box::new(device), options).unwrap();
        let target = output.create_framebuffer_render_target();

        let (fb, properties) = target.lock(&scene());
        assert!(!properties.previous_frame_is_retained);
        assert_eq!(fb.format(), PixelFormat::RGBA8888);
        fb.with_data(&mut |data| data.fill(0x33));
        assert!(std::fs::read(&path).unwrap().iter().all(|&b| b == 0x33));
        fb.dispose();
        assert!(draw(&output, &target, 0x44).previous_frame_is_retained);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn the_blit_thread_copies_the_frame_before_the_next_one_is_locked() {
        let path = temp_file("async.fb");
        let device = FileFbDevice::create(&path, var_info(16, 16, PixelFormat::BGRA8888)).unwrap();
        let options = FbDevOutputOptions { use_async_front_buffer_blit: Some(true), ..Default::default() };
        let output = FbdevOutput::with_device(Box::new(device), options).unwrap();
        let target = output.create_framebuffer_render_target();
        for value in 1..=5u8 {
            draw(&output, &target, value);
            // Locking waits for the transfer of the frame before.
            let (fb, _) = target.lock(&scene());
            assert!(std::fs::read(&path).unwrap().iter().all(|&b| b == value));
            drop(fb);
        }
        drop(target);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_requested_format_is_put_and_one_of_16_bits_is_refused_as_the_reference_does() {
        let path = temp_file("format.fb");
        let device = FileFbDevice::create(&path, var_info(4, 4, PixelFormat::BGRA8888)).unwrap();
        let options = FbDevOutputOptions { pixel_format: Some(PixelFormat::RGBA8888), ..Default::default() };
        let output = FbdevOutput::with_device(Box::new(device), options).unwrap();
        let (fb, _) = output.create_framebuffer_render_target().lock(&scene());
        assert_eq!(fb.format(), PixelFormat::RGBA8888);
        drop(fb);

        let device = FileFbDevice::create(&path, var_info(4, 4, PixelFormat::BGRA8888)).unwrap();
        let options = FbDevOutputOptions { pixel_format: Some(PixelFormat::RGB565), ..Default::default() };
        let error = FbdevOutput::with_device(Box::new(device), options).err().unwrap();
        assert_eq!(error.to_string(), "Unable to set 32-bit display mode");

        // A device that already has 16 bits is drawn in them.
        let device = FileFbDevice::create(&path, var_info(4, 4, PixelFormat::RGB565)).unwrap();
        let output = FbdevOutput::with_device(Box::new(device), FbDevOutputOptions::default()).unwrap();
        let (fb, _) = output.create_framebuffer_render_target().lock(&scene());
        assert_eq!((fb.format(), fb.row_bytes()), (PixelFormat::RGB565, 8));
        drop(fb);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    #[should_panic(expected = "Cannot access a disposed object: LinuxFramebuffer")]
    fn a_disposed_output_cannot_be_locked() {
        let path = temp_file("disposed.fb");
        let device = FileFbDevice::create(&path, var_info(2, 2, PixelFormat::BGRA8888)).unwrap();
        let output = FbdevOutput::with_device(Box::new(device), FbDevOutputOptions::default()).unwrap();
        let target = output.create_framebuffer_render_target();
        output.dispose();
        let _ = std::fs::remove_file(&path);
        let _ = target.lock(&scene());
    }
}
