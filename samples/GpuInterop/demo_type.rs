//! Port of `DemoType.cs`: which demo the main window shows.

/// Which demo the main window shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DemoType {
    #[default]
    Vulkan,
    D3D11,
    VulkanDmaBuf,
}
