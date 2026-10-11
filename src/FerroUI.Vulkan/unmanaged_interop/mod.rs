//! The entry points, structures and enumerations of Vulkan.
//!
//! The types and constants are those of the `ash` bindings, under the names
//! of the Vulkan headers in [`vulkan_enums`] and [`vulkan_structs`]. The
//! three tables of entry points are filled at run time through the lookup a
//! platform or an application provides: nothing of Vulkan is linked.

mod utf8_buffer_array;
mod vulkan_device_api;
pub mod vulkan_enums;
pub(crate) mod vulkan_global_api;
mod vulkan_instance_api;
pub mod vulkan_structs;

pub use utf8_buffer_array::Utf8BufferArray;
pub use vulkan_device_api::VulkanDeviceApi;
pub use vulkan_global_api::VulkanGlobalApi;
pub use vulkan_instance_api::VulkanInstanceApi;
