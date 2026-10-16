//! The Vulkan demos (`VulkanDemo/` upstream, the namespace `GpuInterop.VulkanDemo`): the teapot
//! of the ControlCatalog drawn with Vulkan into images shared with the compositor, as opaque
//! handles of Vulkan, Direct3D 11 textures (Windows), IOSurfaces (macOS through MoltenVK) or
//! Linux dma-bufs.
//!
//! Upstream draws through the bindings of Silk.NET; here through `ash`, the bindings the Vulkan
//! crate of the framework uses.

mod byte_string;
#[cfg(windows)]
mod d3d_memory_helper;
mod vulkan_buffer_helper;
mod vulkan_command_buffer_pool;
mod vulkan_content;
mod vulkan_context;
mod vulkan_demo_control;
mod vulkan_dma_buf_demo_control;
mod vulkan_dma_buf_image;
mod vulkan_dma_buf_swapchain;
mod vulkan_extensions;
mod vulkan_image;
mod vulkan_memory_helper;
mod vulkan_semaphore_pair;
mod vulkan_swapchain;
mod vulkan_timeline_semaphore;

pub use byte_string::{ByteString, ByteStringList};
#[cfg(windows)]
pub use d3d_memory_helper::D3DMemoryHelper;
pub use vulkan_command_buffer_pool::{KeyedMutexSubmitInfo, VulkanCommandBuffer, VulkanCommandBufferPool};
pub use vulkan_content::VulkanContent;
pub use vulkan_context::{D3DDevice, GrContext, VulkanContext};
pub use vulkan_demo_control::VulkanDemoControl;
pub use vulkan_dma_buf_demo_control::VulkanDmaBufDemoControl;
pub use vulkan_dma_buf_image::VulkanDmaBufImage;
pub use vulkan_dma_buf_swapchain::{VulkanDmaBufSwapchain, VulkanDmaBufSwapchainImage};
pub use vulkan_extensions::{ThrowOnError, VulkanDemoError, VulkanDemoResult};
pub use vulkan_image::VulkanImage;
pub use vulkan_semaphore_pair::VulkanSemaphorePair;
pub use vulkan_swapchain::{VulkanSwapchain, VulkanSwapchainImage};
pub use vulkan_timeline_semaphore::VulkanTimelineSemaphore;

use ferroui_base::TypeInfo;

/// The classes of the namespace `GpuInterop.VulkanDemo` (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[VulkanDemoControl::TYPE, VulkanDmaBufDemoControl::TYPE];
