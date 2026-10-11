//! The enumerations and flag sets of Vulkan under the names of the Vulkan
//! headers.
//!
//! Each name is the type of the bindings (`ash::vk`), which carries every
//! value of the enumeration as an associated constant: `VkFormat::B8G8R8A8_UNORM`
//! is `VK_FORMAT_B8G8R8A8_UNORM`. A flag set and the enumeration of its bits
//! are one type.

use ash::vk;

/// `VkResult`.
pub type VkResult = vk::Result;

/// `VkStructureType`.
pub type VkStructureType = vk::StructureType;

/// `VkPhysicalDeviceType`.
pub type VkPhysicalDeviceType = vk::PhysicalDeviceType;

/// `VkQueueFlags`.
pub type VkQueueFlags = vk::QueueFlags;

/// `VkDeviceQueueCreateFlags`.
pub type VkDeviceQueueCreateFlags = vk::DeviceQueueCreateFlags;

/// `VkFenceCreateFlags`.
pub type VkFenceCreateFlags = vk::FenceCreateFlags;

/// `VkCommandPoolCreateFlags`.
pub type VkCommandPoolCreateFlags = vk::CommandPoolCreateFlags;

/// `VkCommandBufferLevel`.
pub type VkCommandBufferLevel = vk::CommandBufferLevel;

/// `VkCommandBufferUsageFlags`.
pub type VkCommandBufferUsageFlags = vk::CommandBufferUsageFlags;

/// `VkPipelineStageFlags`.
pub type VkPipelineStageFlags = vk::PipelineStageFlags;

/// `VkFormat`.
pub type VkFormat = vk::Format;

/// `VkColorSpaceKHR`.
pub type VkColorSpaceKHR = vk::ColorSpaceKHR;

/// `VkMemoryPropertyFlags`.
pub type VkMemoryPropertyFlags = vk::MemoryPropertyFlags;

/// `VkMemoryHeapFlags`.
pub type VkMemoryHeapFlags = vk::MemoryHeapFlags;

/// `VkImageLayout`.
pub type VkImageLayout = vk::ImageLayout;

/// `VkAccessFlags`.
pub type VkAccessFlags = vk::AccessFlags;

/// `VkBufferUsageFlags`.
pub type VkBufferUsageFlags = vk::BufferUsageFlags;

/// `VkImageUsageFlags`.
pub type VkImageUsageFlags = vk::ImageUsageFlags;

/// `VkImageAspectFlags`.
pub type VkImageAspectFlags = vk::ImageAspectFlags;

/// `VkImageCreateFlags`.
pub type VkImageCreateFlags = vk::ImageCreateFlags;

/// `VkImageType`.
pub type VkImageType = vk::ImageType;

/// `VkSampleCountFlags`.
pub type VkSampleCountFlags = vk::SampleCountFlags;

/// `VkImageTiling`.
pub type VkImageTiling = vk::ImageTiling;

/// `VkSharingMode`.
pub type VkSharingMode = vk::SharingMode;

/// `VkImageViewCreateFlags`.
pub type VkImageViewCreateFlags = vk::ImageViewCreateFlags;

/// `VkImageViewType`.
pub type VkImageViewType = vk::ImageViewType;

/// `VkComponentSwizzle`.
pub type VkComponentSwizzle = vk::ComponentSwizzle;

/// `VkDependencyFlags`.
pub type VkDependencyFlags = vk::DependencyFlags;

/// `VkSurfaceTransformFlagsKHR`.
pub type VkSurfaceTransformFlagsKHR = vk::SurfaceTransformFlagsKHR;

/// `VkCompositeAlphaFlagsKHR`.
pub type VkCompositeAlphaFlagsKHR = vk::CompositeAlphaFlagsKHR;

/// `VkPresentModeKHR`.
pub type VkPresentModeKHR = vk::PresentModeKHR;

/// `VkSwapchainCreateFlagsKHR`.
pub type VkSwapchainCreateFlagsKHR = vk::SwapchainCreateFlagsKHR;

/// `VkFilter`.
pub type VkFilter = vk::Filter;

/// `VkDebugUtilsMessageSeverityFlagsEXT`.
pub type VkDebugUtilsMessageSeverityFlagsEXT = vk::DebugUtilsMessageSeverityFlagsEXT;

/// `VkDebugUtilsMessageTypeFlagsEXT`.
pub type VkDebugUtilsMessageTypeFlagsEXT = vk::DebugUtilsMessageTypeFlagsEXT;

/// `VkObjectType`.
pub type VkObjectType = vk::ObjectType;

/// `VkSemaphoreImportFlags`.
pub type VkSemaphoreImportFlags = vk::SemaphoreImportFlags;

/// `VkExternalSemaphoreHandleTypeFlags`.
pub type VkExternalSemaphoreHandleTypeFlags = vk::ExternalSemaphoreHandleTypeFlags;

/// `VkExternalMemoryHandleTypeFlagBits`.
pub type VkExternalMemoryHandleTypeFlagBits = vk::ExternalMemoryHandleTypeFlags;
