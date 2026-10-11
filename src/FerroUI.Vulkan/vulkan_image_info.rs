use ferroui_base::PixelSize;

/// A Vulkan image as a render backend needs to know it: the handles are
/// those of the device of the context that hands the value out, the other
/// members are the numeric values of the Vulkan enumerations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct VulkanImageInfo {
    /// The `VkFormat`.
    pub format: u32,
    /// The size in pixels.
    pub pixel_size: PixelSize,
    /// The `VkImage`.
    pub handle: u64,
    /// The `VkImageLayout` the image is in.
    pub layout: u32,
    /// The `VkImageTiling`.
    pub tiling: u32,
    /// The `VkImageUsageFlags` the image was created with.
    pub usage_flags: u32,
    /// The number of mip levels.
    pub level_count: u32,
    /// The number of samples per pixel.
    pub sample_count: u32,
    /// The `VkDeviceMemory` bound to the image.
    pub memory_handle: u64,
    /// The `VkImageView` of the image.
    pub view_handle: u64,
    /// The size of the memory in bytes.
    pub memory_size: u64,
    /// Whether the image is a protected one.
    pub is_protected: bool,
}
