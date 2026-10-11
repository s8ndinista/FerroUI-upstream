//! The Vulkan rendering mode of the platform: the loader of the system and
//! the surfaces of windows (`VK_KHR_xlib_surface`).

mod vulkan_native_interop;
mod vulkan_support;

pub(crate) use vulkan_support::VulkanSupport;
