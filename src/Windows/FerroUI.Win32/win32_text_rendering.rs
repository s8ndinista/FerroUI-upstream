//! The text rendering parameters of Windows, which the windows of the
//! backend report to the render backend through their render surfaces
//! (`IPlatformRenderSurface::text_rendering_parameters`,
//! docs/porting/text-rendering.md).
//!
//! FerroUI's own addition: the reference renders text the same way on every
//! window (docs/porting/DEVIATIONS.md, area `Text rendering`). The values
//! are read the way Chromium reads them (`ui/gfx/font_render_params_win.cc`
//! and `ui/gfx/font_util_win.cc`):
//!
//! - subpixel antialiasing is on when font smoothing is on
//!   (`SPI_GETFONTSMOOTHING`) and its type is ClearType
//!   (`SPI_GETFONTSMOOTHINGTYPE`), for the subpixel order of
//!   `SPI_GETFONTSMOOTHINGORIENTATION` (RGB when it cannot be read);
//! - the contrast and gamma of text are the ones of DirectWrite
//!   (`IDWriteRenderingParams::GetEnhancedContrast` and `GetGamma`) when the
//!   user tuned ClearType (the ClearType Text Tuner writes the key
//!   `HKCU\SOFTWARE\Microsoft\Avalon.Graphics\<display>` of the primary
//!   display), clamped to what Skia accepts; otherwise contrast 1 and sRGB
//!   gamma, the values Chromium builds Skia with on Windows to match the
//!   weight of DirectWrite text.
//!
//! The values are read once and kept for the process until the system says
//! that a setting changed (`WM_SETTINGCHANGE`, [`invalidate`]).

use ferroui_base::platform::surfaces::{IPlatformRenderSurface, PlatformTextRenderingParameters, SubpixelGeometry};
use std::any::Any;
use std::sync::Mutex;

/// `FE_FONTSMOOTHINGCLEARTYPE`: the type of font smoothing that is ClearType.
const FE_FONTSMOOTHINGCLEARTYPE: u32 = 2;
/// `FE_FONTSMOOTHINGORIENTATIONBGR`.
const FE_FONTSMOOTHINGORIENTATIONBGR: u32 = 0;
/// `FE_FONTSMOOTHINGORIENTATIONRGB`.
const FE_FONTSMOOTHINGORIENTATIONRGB: u32 = 1;

/// The contrast Chromium builds Skia with on Windows (`SK_GAMMA_CONTRAST=1.0`
/// in `skia/BUILD.gn`): text as heavy as DirectWrite's.
pub(crate) const DEFAULT_TEXT_CONTRAST: f32 = 1.0;
/// The gamma Chromium builds Skia with on Windows (`SK_GAMMA_SRGB`): 0
/// means sRGB to Skia.
pub(crate) const DEFAULT_TEXT_GAMMA: f32 = 0.0;

/// The settings of the system the parameters are made of.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SystemTextSettings {
    /// `SPI_GETFONTSMOOTHING`, `None` when it cannot be read.
    pub font_smoothing: Option<bool>,
    /// `SPI_GETFONTSMOOTHINGTYPE`, `None` when it cannot be read.
    pub font_smoothing_type: Option<u32>,
    /// `SPI_GETFONTSMOOTHINGORIENTATION`, `None` when it cannot be read.
    pub font_smoothing_orientation: Option<u32>,
    /// The enhanced contrast and gamma of DirectWrite, when the user tuned
    /// ClearType for the primary display.
    pub tuned_contrast_and_gamma: Option<(f32, f32)>,
}

impl Default for SystemTextSettings {
    /// The settings of an untouched Windows installation: ClearType on an
    /// RGB display, not tuned.
    fn default() -> Self {
        Self {
            font_smoothing: Some(true),
            font_smoothing_type: Some(FE_FONTSMOOTHINGCLEARTYPE),
            font_smoothing_orientation: Some(FE_FONTSMOOTHINGORIENTATIONRGB),
            tuned_contrast_and_gamma: None,
        }
    }
}

impl SystemTextSettings {
    /// The text rendering parameters of a surface of a window with these
    /// settings. `composited`: the window is composed through
    /// Windows.UI.Composition or DirectComposition, or the surface is a
    /// swap chain panel of a WinUI host.
    pub(crate) fn parameters(&self, composited: bool) -> PlatformTextRenderingParameters {
        let subpixel_antialiasing =
            self.font_smoothing == Some(true) && self.font_smoothing_type == Some(FE_FONTSMOOTHINGCLEARTYPE);

        let subpixel_geometry = match self.font_smoothing_orientation {
            Some(FE_FONTSMOOTHINGORIENTATIONRGB) => SubpixelGeometry::RgbHorizontal,
            Some(FE_FONTSMOOTHINGORIENTATIONBGR) => SubpixelGeometry::BgrHorizontal,
            // Chromium: "No explicit ClearType settings, default to RGB".
            None => SubpixelGeometry::RgbHorizontal,
            Some(_) => SubpixelGeometry::Unknown,
        };

        let (text_contrast, text_gamma) =
            self.tuned_contrast_and_gamma.unwrap_or((DEFAULT_TEXT_CONTRAST, DEFAULT_TEXT_GAMMA));

        PlatformTextRenderingParameters {
            subpixel_antialiasing,
            subpixel_geometry,
            composited,
            text_contrast: Some(text_contrast),
            text_gamma: Some(text_gamma),
        }
    }
}

/// The settings of the system as they were last read; `None` until they
/// are read or after a setting changed.
static SETTINGS: Mutex<Option<SystemTextSettings>> = Mutex::new(None);

/// The text rendering parameters of the system for a surface, read on the
/// thread that asks (the render thread): from the cache, else from the
/// system.
pub(crate) fn system_parameters(composited: bool) -> PlatformTextRenderingParameters {
    let mut settings = SETTINGS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    settings.get_or_insert_with(read_system_settings).parameters(composited)
}

/// Forgets the settings that were read: the next frame reads them again.
/// Called for `WM_SETTINGCHANGE`.
pub(crate) fn invalidate() {
    *SETTINGS.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
}

/// A render surface that only reports the text rendering parameters of the
/// system for a composited surface: what a top-level over the swap chain
/// panel of a WinUI host adds to the surface of its host, so that its text
/// renders as WinUI's (greyscale, with the contrast and gamma of the
/// system).
pub(crate) struct CompositedTextRenderingSurface;

impl IPlatformRenderSurface for CompositedTextRenderingSurface {
    fn text_rendering_parameters(&self) -> Option<PlatformTextRenderingParameters> {
        Some(system_parameters(true))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(not(windows))]
fn read_system_settings() -> SystemTextSettings {
    SystemTextSettings::default()
}

#[cfg(windows)]
fn read_system_settings() -> SystemTextSettings {
    native::read()
}

#[cfg(windows)]
mod native {
    use super::SystemTextSettings;
    use crate::interop::unmanaged_methods::{get_proc_address, load_library, to_wide};
    use ferroui_microcom::Guid;
    use std::ffi::c_void;
    use windows_sys::Win32::Graphics::Gdi as gdi;
    use windows_sys::Win32::System::Registry as reg;
    use windows_sys::Win32::UI::WindowsAndMessaging as wm;

    /// `IID_IDWriteFactory`.
    const IID_IDWRITE_FACTORY: Guid = Guid::from_u128(0xB859EE5A_D838_4B5B_A2E8_1ADC7D93DB48);
    /// `DWRITE_FACTORY_TYPE_SHARED`.
    const DWRITE_FACTORY_TYPE_SHARED: i32 = 0;

    type DWriteCreateFactory =
        unsafe extern "system" fn(factory_type: i32, iid: *const Guid, factory: *mut *mut c_void) -> i32;

    /// The vtable of `IDWriteFactory` (`dwrite.h`) up to the method the
    /// backend calls: the three methods of `IUnknown`, then the methods of
    /// the interface in their order. The methods called have their types;
    /// the others are addresses.
    #[repr(C)]
    struct IDWriteFactoryVTable {
        query_interface: *const c_void,
        add_ref: *const c_void,
        release: unsafe extern "system" fn(this: *mut c_void) -> u32,
        get_system_font_collection: *const c_void,
        create_custom_font_collection: *const c_void,
        register_font_collection_loader: *const c_void,
        unregister_font_collection_loader: *const c_void,
        create_font_file_reference: *const c_void,
        create_custom_font_file_reference: *const c_void,
        create_font_face: *const c_void,
        create_rendering_params: unsafe extern "system" fn(this: *mut c_void, params: *mut *mut c_void) -> i32,
    }

    /// The vtable of `IDWriteRenderingParams` (`dwrite.h`).
    #[repr(C)]
    struct IDWriteRenderingParamsVTable {
        query_interface: *const c_void,
        add_ref: *const c_void,
        release: unsafe extern "system" fn(this: *mut c_void) -> u32,
        get_gamma: unsafe extern "system" fn(this: *mut c_void) -> f32,
        get_enhanced_contrast: unsafe extern "system" fn(this: *mut c_void) -> f32,
    }

    pub(super) fn read() -> SystemTextSettings {
        SystemTextSettings {
            font_smoothing: spi_u32(wm::SPI_GETFONTSMOOTHING).map(|value| value != 0),
            font_smoothing_type: spi_u32(wm::SPI_GETFONTSMOOTHINGTYPE),
            font_smoothing_orientation: spi_u32(wm::SPI_GETFONTSMOOTHINGORIENTATION),
            tuned_contrast_and_gamma: if text_is_tuned() { direct_write_contrast_and_gamma() } else { None },
        }
    }

    /// A setting of the system that is a number (the `BOOL` of
    /// `SPI_GETFONTSMOOTHING` is one too).
    fn spi_u32(action: u32) -> Option<u32> {
        let mut value: u32 = 0;
        // SAFETY: the three actions write a UINT (or a BOOL, the same size)
        // to the pointer, which points at a live u32.
        let ok = unsafe { wm::SystemParametersInfoW(action, 0, (&mut value as *mut u32).cast(), 0) };
        (ok != 0).then_some(value)
    }

    /// Whether the user tuned the text of the primary display with the
    /// ClearType Text Tuner: it writes the key
    /// `HKCU\SOFTWARE\Microsoft\Avalon.Graphics\<display>`. Chromium reads
    /// the values of DirectWrite only then.
    fn text_is_tuned() -> bool {
        let Some(display) = primary_display_name() else {
            return false;
        };
        let path = to_wide(&format!("SOFTWARE\\Microsoft\\Avalon.Graphics\\{display}"));
        let mut key: reg::HKEY = std::ptr::null_mut();
        // SAFETY: a null-terminated path and a live out pointer; the key is
        // closed below when it was opened.
        let status = unsafe { reg::RegOpenKeyExW(reg::HKEY_CURRENT_USER, path.as_ptr(), 0, reg::KEY_READ, &mut key) };
        if status != 0 {
            return false;
        }
        // SAFETY: the key was opened above and is not used again.
        unsafe { reg::RegCloseKey(key) };
        true
    }

    /// The name of the primary display device without its `\\.\` prefix
    /// (`DISPLAY1`).
    fn primary_display_name() -> Option<String> {
        let mut index = 0;
        loop {
            let mut device = gdi::DISPLAY_DEVICEW {
                cb: std::mem::size_of::<gdi::DISPLAY_DEVICEW>() as u32,
                ..Default::default()
            };
            // SAFETY: the structure is live and its size is set.
            if unsafe { gdi::EnumDisplayDevicesW(std::ptr::null(), index, &mut device, 0) } == 0 {
                return None;
            }
            if device.StateFlags & gdi::DISPLAY_DEVICE_PRIMARY_DEVICE != 0 {
                let length = device.DeviceName.iter().position(|&c| c == 0).unwrap_or(device.DeviceName.len());
                let name = String::from_utf16_lossy(&device.DeviceName[..length]);
                return Some(name.rsplit('\\').next().unwrap_or(&name).to_owned());
            }
            index += 1;
        }
    }

    /// The enhanced contrast and gamma of the default rendering parameters
    /// of DirectWrite (`IDWriteFactory::CreateRenderingParams`, the primary
    /// display).
    fn direct_write_contrast_and_gamma() -> Option<(f32, f32)> {
        let module = load_library("dwrite.dll");
        let create = get_proc_address(module, c"DWriteCreateFactory")?;
        // SAFETY: the export of `dwrite.dll` has this signature.
        let create: DWriteCreateFactory = unsafe { std::mem::transmute(create) };

        let mut factory: *mut c_void = std::ptr::null_mut();
        // SAFETY: a valid factory type, the identifier of the interface and
        // a live out pointer.
        if unsafe { create(DWRITE_FACTORY_TYPE_SHARED, &IID_IDWRITE_FACTORY, &mut factory) } < 0 || factory.is_null() {
            return None;
        }
        // SAFETY: `factory` is a live `IDWriteFactory`: a pointer to its
        // vtable pointer.
        let factory_vtable = unsafe { &**(factory as *mut *const IDWriteFactoryVTable) };

        let mut params: *mut c_void = std::ptr::null_mut();
        // SAFETY: the method of the factory with its own pointer and a live
        // out pointer.
        let result = unsafe { (factory_vtable.create_rendering_params)(factory, &mut params) };
        let values = if result >= 0 && !params.is_null() {
            // SAFETY: `params` is a live `IDWriteRenderingParams`.
            let params_vtable = unsafe { &**(params as *mut *const IDWriteRenderingParamsVTable) };
            // SAFETY: methods of the interface with its own pointer; the
            // reference the creation gave is released once.
            let values = unsafe {
                let values = ((params_vtable.get_enhanced_contrast)(params), (params_vtable.get_gamma)(params));
                (params_vtable.release)(params);
                values
            };
            Some(values)
        } else {
            None
        };
        // SAFETY: releases the reference the creation gave.
        unsafe { (factory_vtable.release)(factory) };
        values
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_type_on_an_rgb_display_is_subpixel_text_with_the_contrast_of_chromium() {
        let parameters = SystemTextSettings::default().parameters(false);
        assert!(parameters.subpixel_antialiasing);
        assert_eq!(SubpixelGeometry::RgbHorizontal, parameters.subpixel_geometry);
        assert!(!parameters.composited);
        assert_eq!(Some(1.0), parameters.text_contrast);
        assert_eq!(Some(0.0), parameters.text_gamma);
        assert!(SystemTextSettings::default().parameters(true).composited);
    }

    #[test]
    fn standard_or_disabled_font_smoothing_is_not_subpixel_text() {
        let standard = SystemTextSettings { font_smoothing_type: Some(1), ..Default::default() };
        assert!(!standard.parameters(false).subpixel_antialiasing);
        let off = SystemTextSettings { font_smoothing: Some(false), ..Default::default() };
        assert!(!off.parameters(false).subpixel_antialiasing);
        let unknown = SystemTextSettings { font_smoothing: None, ..Default::default() };
        assert!(!unknown.parameters(false).subpixel_antialiasing);
    }

    #[test]
    fn the_orientation_gives_the_subpixel_geometry() {
        let geometry = |orientation| {
            SystemTextSettings { font_smoothing_orientation: orientation, ..Default::default() }
                .parameters(false)
                .subpixel_geometry
        };
        assert_eq!(SubpixelGeometry::BgrHorizontal, geometry(Some(0)));
        assert_eq!(SubpixelGeometry::RgbHorizontal, geometry(Some(1)));
        assert_eq!(SubpixelGeometry::RgbHorizontal, geometry(None));
        assert_eq!(SubpixelGeometry::Unknown, geometry(Some(7)));
    }

    #[test]
    fn tuned_text_takes_the_values_of_direct_write() {
        let tuned = SystemTextSettings { tuned_contrast_and_gamma: Some((0.5, 1.8)), ..Default::default() };
        let parameters = tuned.parameters(false);
        assert_eq!(Some(0.5), parameters.text_contrast);
        assert_eq!(Some(1.8), parameters.text_gamma);
    }

    #[test]
    fn the_surface_of_a_host_reports_composited_parameters() {
        let parameters = CompositedTextRenderingSurface.text_rendering_parameters().unwrap();
        assert!(parameters.composited);
        assert!(CompositedTextRenderingSurface.as_framebuffer_surface().is_none());
    }

    #[test]
    fn the_settings_are_read_once_until_a_setting_changes() {
        // (Other tests read the cache too: only what holds whatever they do
        // is asserted.)
        invalidate();
        let first = system_parameters(false);
        assert!(SETTINGS.lock().unwrap().is_some());
        assert_eq!(first, system_parameters(false));
        invalidate();
        assert_eq!(first, system_parameters(false), "the system settings are read again");
    }
}
