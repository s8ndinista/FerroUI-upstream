//! The input backend that reads the kernel's event devices directly.

#[cfg(target_os = "linux")]
mod ev_dev_backend;
mod ev_dev_device;
mod ev_dev_device_description;
mod ev_dev_key_table;
mod ev_dev_keyboard;
mod ev_dev_mouse;
mod ev_dev_multi_touch_screen;
mod ev_dev_touch_screen;
mod ev_dev_touch_screen_device_description;

#[cfg(target_os = "linux")]
pub use ev_dev_backend::EvDevBackend;
#[cfg(target_os = "linux")]
pub use ev_dev_device::EvDevDevice;
pub use ev_dev_device::{EvDevAxisInfo, EvDevDeviceInfo};
pub use ev_dev_device_description::EvDevDeviceDescription;
pub use ev_dev_key_table::physical_key_from_ev_key;
pub use ev_dev_keyboard::{EvDevKeyboard, EvDevKeyboardDeviceDescription};
pub use ev_dev_mouse::{EvDevMouse, EvDevMouseDeviceDescription};
pub use ev_dev_multi_touch_screen::{EvDevMultiTouchScreen, EvDevMultiTouchScreenDeviceDescription};
pub use ev_dev_touch_screen::{EvDevDeviceHandler, EvDevRawEvent, EvDevSingleTouchScreen};
pub use ev_dev_touch_screen_device_description::EvDevTouchScreenDeviceDescription;
