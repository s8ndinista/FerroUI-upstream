//! The Vulkan rendering mode of the platform: the loader of the system
//! (`vulkan-1.dll`) and the surfaces of windows (`VK_KHR_win32_surface`).
//!
//! Public for the probe of the example (`win32_window --vulkan-probe`),
//! which initialises the mode without the platform; an application names
//! the mode in `Win32PlatformOptions::rendering_mode` instead.

mod vulkan_native_interop;
#[cfg(windows)]
mod vulkan_support;

#[cfg(windows)]
pub use vulkan_support::VulkanSupport;
