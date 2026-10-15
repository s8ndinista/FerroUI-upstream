//! The outputs of the platform: what the one top-level is drawn onto.

pub mod drm;
mod drm_bindings;
mod drm_output;
pub mod drm_scanout;
mod fb_dev_back_buffer;
mod fb_dev_output_options;
mod fb_device;
mod fbdev_output;
mod i_gl_output_backend;
mod i_output_backend;

pub use drm::{DrmModeConnection, DrmModeSubPixel};
pub use drm_bindings::{DrmCard, DrmConnector, DrmModeInfo, DrmResources};
#[cfg(target_os = "linux")]
pub use drm_output::DrmOutput;
pub use fb_dev_output_options::FbDevOutputOptions;
#[cfg(target_os = "linux")]
pub use fb_device::LinuxFbDevice;
pub use fb_device::{FbMapping, FileFbDevice, IFbDevice};
pub use fbdev_output::FbdevOutput;
pub use i_gl_output_backend::IGlOutputBackend;
pub use i_output_backend::IOutputBackend;
