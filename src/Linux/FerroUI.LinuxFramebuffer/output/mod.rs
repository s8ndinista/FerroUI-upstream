//! The outputs of the platform: what the one top-level is drawn onto.

mod fb_dev_back_buffer;
mod fb_dev_output_options;
mod fb_device;
mod fbdev_output;
mod i_gl_output_backend;
mod i_output_backend;

pub use fb_dev_output_options::FbDevOutputOptions;
#[cfg(target_os = "linux")]
pub use fb_device::LinuxFbDevice;
pub use fb_device::{FbMapping, FileFbDevice, IFbDevice};
pub use fbdev_output::FbdevOutput;
pub use i_gl_output_backend::IGlOutputBackend;
pub use i_output_backend::IOutputBackend;
