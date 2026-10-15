//! The Vulkan rendering mode of the platform: the loader of the system and
//! the surfaces of the surface view (`VK_KHR_android_surface`).

mod vulkan_native_interop;
#[cfg(target_os = "android")]
mod vulkan_support;

#[cfg(target_os = "android")]
pub(crate) use vulkan_support::VulkanSupport;
