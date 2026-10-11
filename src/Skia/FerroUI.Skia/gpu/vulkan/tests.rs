//! The value tables of the module.

use super::{sk_format, sk_image_layout, sk_image_tiling};
use ferroui_vulkan::ash::vk;

#[test]
fn the_values_of_vulkan_map_to_the_names_of_skia() {
    use skia_safe::gpu::vk as sk;
    assert!(matches!(sk_format(vk::Format::R8G8B8A8_UNORM.as_raw() as u32), Some(sk::Format::R8G8B8A8_UNORM)));
    assert!(matches!(sk_format(vk::Format::B8G8R8A8_UNORM.as_raw() as u32), Some(sk::Format::B8G8R8A8_UNORM)));
    assert!(sk_format(vk::Format::R8G8B8A8_SRGB.as_raw() as u32).is_none());
    assert!(matches!(
        sk_image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL.as_raw() as u32),
        Some(sk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
    ));
    assert!(matches!(
        sk_image_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL.as_raw() as u32),
        Some(sk::ImageLayout::TRANSFER_SRC_OPTIMAL)
    ));
    assert!(matches!(sk_image_layout(0), Some(sk::ImageLayout::UNDEFINED)));
    assert!(matches!(
        sk_image_layout(vk::ImageLayout::PRESENT_SRC_KHR.as_raw() as u32),
        Some(sk::ImageLayout::PRESENT_SRC_KHR)
    ));
    assert!(sk_image_layout(123456).is_none());
    assert!(matches!(sk_image_tiling(0), Some(sk::ImageTiling::OPTIMAL)));
    assert!(matches!(sk_image_tiling(1), Some(sk::ImageTiling::LINEAR)));
    assert!(sk_image_tiling(77).is_none());
}
