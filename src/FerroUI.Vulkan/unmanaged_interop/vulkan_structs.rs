//! The handles and structures of Vulkan under the names of the Vulkan
//! headers.
//!
//! Each name is the type of the bindings (`ash::vk`). A structure that can
//! point to other data carries the lifetime of that data; its `sType` is set
//! by `Default`.

use ash::vk;

/// `VkInstance`.
pub type VkInstance = vk::Instance;

/// `VkPhysicalDevice`.
pub type VkPhysicalDevice = vk::PhysicalDevice;

/// `VkDevice`.
pub type VkDevice = vk::Device;

/// `VkSwapchainKHR`.
pub type VkSwapchainKHR = vk::SwapchainKHR;

/// `VkSemaphore`.
pub type VkSemaphore = vk::Semaphore;

/// `VkFence`.
pub type VkFence = vk::Fence;

/// `VkImage`.
pub type VkImage = vk::Image;

/// `VkImageView`.
pub type VkImageView = vk::ImageView;

/// `VkDeviceMemory`.
pub type VkDeviceMemory = vk::DeviceMemory;

/// `VkQueue`.
pub type VkQueue = vk::Queue;

/// `VkCommandPool`.
pub type VkCommandPool = vk::CommandPool;

/// `VkCommandBuffer`.
pub type VkCommandBuffer = vk::CommandBuffer;

/// `VkSurfaceKHR`.
pub type VkSurfaceKHR = vk::SurfaceKHR;

/// `VkDebugUtilsMessengerEXT`.
pub type VkDebugUtilsMessengerEXT = vk::DebugUtilsMessengerEXT;

/// `VkLayerProperties`.
pub type VkLayerProperties = vk::LayerProperties;

/// `VkDebugUtilsLabelEXT`.
pub type VkDebugUtilsLabelEXT<'a> = vk::DebugUtilsLabelEXT<'a>;

/// `VkDebugUtilsObjectNameInfoEXT`.
pub type VkDebugUtilsObjectNameInfoEXT<'a> = vk::DebugUtilsObjectNameInfoEXT<'a>;

/// `VkDebugUtilsMessengerCallbackDataEXT`.
pub type VkDebugUtilsMessengerCallbackDataEXT<'a> = vk::DebugUtilsMessengerCallbackDataEXT<'a>;

/// `VkDebugUtilsMessengerCallbackEXTDelegate`.
pub type VkDebugUtilsMessengerCallbackEXTDelegate = vk::PFN_vkDebugUtilsMessengerCallbackEXT;

/// `VkDebugUtilsMessengerCreateInfoEXT`.
pub type VkDebugUtilsMessengerCreateInfoEXT<'a> = vk::DebugUtilsMessengerCreateInfoEXT<'a>;

/// `VkInstanceCreateInfo`.
pub type VkInstanceCreateInfo<'a> = vk::InstanceCreateInfo<'a>;

/// `VkApplicationInfo`.
pub type VkApplicationInfo<'a> = vk::ApplicationInfo<'a>;

/// `VkPhysicalDeviceSparseProperties`.
pub type VkPhysicalDeviceSparseProperties = vk::PhysicalDeviceSparseProperties;

/// `VkPhysicalDeviceProperties`.
pub type VkPhysicalDeviceProperties = vk::PhysicalDeviceProperties;

/// `VkPhysicalDeviceProperties2`.
pub type VkPhysicalDeviceProperties2<'a> = vk::PhysicalDeviceProperties2<'a>;

/// `VkPhysicalDeviceIDProperties`.
pub type VkPhysicalDeviceIDProperties<'a> = vk::PhysicalDeviceIDProperties<'a>;

/// `VkPhysicalDeviceLimits`.
pub type VkPhysicalDeviceLimits = vk::PhysicalDeviceLimits;

/// `VkExtensionProperties`.
pub type VkExtensionProperties = vk::ExtensionProperties;

/// `VkQueueFamilyProperties`.
pub type VkQueueFamilyProperties = vk::QueueFamilyProperties;

/// `VkExtent2D`.
pub type VkExtent2D = vk::Extent2D;

/// `VkExtent3D`.
pub type VkExtent3D = vk::Extent3D;

/// `VkDeviceQueueCreateInfo`.
pub type VkDeviceQueueCreateInfo<'a> = vk::DeviceQueueCreateInfo<'a>;

/// `VkDeviceCreateInfo`.
pub type VkDeviceCreateInfo<'a> = vk::DeviceCreateInfo<'a>;

/// `VkFenceCreateInfo`.
pub type VkFenceCreateInfo<'a> = vk::FenceCreateInfo<'a>;

/// `VkCommandPoolCreateInfo`.
pub type VkCommandPoolCreateInfo<'a> = vk::CommandPoolCreateInfo<'a>;

/// `VkCommandBufferAllocateInfo`.
pub type VkCommandBufferAllocateInfo<'a> = vk::CommandBufferAllocateInfo<'a>;

/// `VkCommandBufferBeginInfo`.
pub type VkCommandBufferBeginInfo<'a> = vk::CommandBufferBeginInfo<'a>;

/// `VkSemaphoreCreateInfo`.
pub type VkSemaphoreCreateInfo<'a> = vk::SemaphoreCreateInfo<'a>;

/// `VkSubmitInfo`.
pub type VkSubmitInfo<'a> = vk::SubmitInfo<'a>;

/// `VkSurfaceFormatKHR`.
pub type VkSurfaceFormatKHR = vk::SurfaceFormatKHR;

/// `VkMemoryType`.
pub type VkMemoryType = vk::MemoryType;

/// `VkMemoryHeap`.
pub type VkMemoryHeap = vk::MemoryHeap;

/// `VkPhysicalDeviceMemoryProperties`.
pub type VkPhysicalDeviceMemoryProperties = vk::PhysicalDeviceMemoryProperties;

/// `VkMemoryTypesBuffer`.
pub type VkMemoryTypesBuffer = [vk::MemoryType; vk::MAX_MEMORY_TYPES];

/// `VkMemoryHeapsBuffer`.
pub type VkMemoryHeapsBuffer = [vk::MemoryHeap; vk::MAX_MEMORY_HEAPS];

/// `VkImageCreateInfo`.
pub type VkImageCreateInfo<'a> = vk::ImageCreateInfo<'a>;

/// `VkMemoryRequirements`.
pub type VkMemoryRequirements = vk::MemoryRequirements;

/// `VkMemoryAllocateInfo`.
pub type VkMemoryAllocateInfo<'a> = vk::MemoryAllocateInfo<'a>;

/// `VkComponentMapping`.
pub type VkComponentMapping = vk::ComponentMapping;

/// `VkImageSubresourceRange`.
pub type VkImageSubresourceRange = vk::ImageSubresourceRange;

/// `VkImageViewCreateInfo`.
pub type VkImageViewCreateInfo<'a> = vk::ImageViewCreateInfo<'a>;

/// `VkImageMemoryBarrier`.
pub type VkImageMemoryBarrier<'a> = vk::ImageMemoryBarrier<'a>;

/// `VkSurfaceCapabilitiesKHR`.
pub type VkSurfaceCapabilitiesKHR = vk::SurfaceCapabilitiesKHR;

/// `VkSwapchainCreateInfoKHR`.
pub type VkSwapchainCreateInfoKHR<'a> = vk::SwapchainCreateInfoKHR<'a>;

/// `VkOffset3D`.
pub type VkOffset3D = vk::Offset3D;

/// `VkImageSubresourceLayers`.
pub type VkImageSubresourceLayers = vk::ImageSubresourceLayers;

/// `VkImageBlit`.
pub type VkImageBlit = vk::ImageBlit;

/// `VkBuffer`.
pub type VkBuffer = vk::Buffer;

/// `VkBufferCreateInfo`.
pub type VkBufferCreateInfo<'a> = vk::BufferCreateInfo<'a>;

/// `VkBufferImageCopy`.
pub type VkBufferImageCopy = vk::BufferImageCopy;

/// `VkPresentInfoKHR`.
pub type VkPresentInfoKHR<'a> = vk::PresentInfoKHR<'a>;

/// `VkImportSemaphoreFdInfoKHR`.
pub type VkImportSemaphoreFdInfoKHR<'a> = vk::ImportSemaphoreFdInfoKHR<'a>;

/// `VkImportSemaphoreWin32HandleInfoKHR`.
pub type VkImportSemaphoreWin32HandleInfoKHR<'a> = vk::ImportSemaphoreWin32HandleInfoKHR<'a>;

/// `VkImportMemoryFdInfoKHR`.
pub type VkImportMemoryFdInfoKHR<'a> = vk::ImportMemoryFdInfoKHR<'a>;

/// `VkImportMemoryWin32HandleInfoKHR`.
pub type VkImportMemoryWin32HandleInfoKHR<'a> = vk::ImportMemoryWin32HandleInfoKHR<'a>;

/// `VkMemoryDedicatedAllocateInfo`.
pub type VkMemoryDedicatedAllocateInfo<'a> = vk::MemoryDedicatedAllocateInfo<'a>;

/// `VkExternalMemoryImageCreateInfo`.
pub type VkExternalMemoryImageCreateInfo<'a> = vk::ExternalMemoryImageCreateInfo<'a>;
