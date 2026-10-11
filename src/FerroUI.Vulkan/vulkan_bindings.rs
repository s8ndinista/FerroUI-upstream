use crate::Version;

/// Values of the Vulkan headers that are computed and not enumerated.
pub struct VulkanHelpers;

impl VulkanHelpers {
    /// `VK_QUEUE_FAMILY_IGNORED`.
    pub const QUEUE_FAMILY_IGNORED: u32 = 4294967295;

    /// The packed form of a version (`VK_MAKE_VERSION`).
    pub fn make_version(major: i32, minor: i32, patch: i32) -> u32 {
        ((major << 22) | (minor << 12) | patch) as u32
    }

    /// The packed form of `v`.
    pub fn make_version_of(v: Version) -> u32 {
        Self::make_version(v.major, v.minor, v.build)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ash::vk;

    #[test]
    fn make_version_packs_like_the_headers() {
        assert_eq!(VulkanHelpers::make_version(1, 0, 0), vk::make_api_version(0, 1, 0, 0));
        assert_eq!(VulkanHelpers::make_version(1, 1, 0), vk::API_VERSION_1_1);
        assert_eq!(VulkanHelpers::make_version(1, 3, 281), vk::make_api_version(0, 1, 3, 281));
        assert_eq!(VulkanHelpers::make_version_of(Version::new(1, 2, 7)), (1 << 22) | (2 << 12) | 7);
    }

    #[test]
    fn queue_family_ignored_is_the_constant_of_the_headers() {
        assert_eq!(VulkanHelpers::QUEUE_FAMILY_IGNORED, vk::QUEUE_FAMILY_IGNORED);
    }
}
