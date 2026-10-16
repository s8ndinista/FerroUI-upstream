//! Port of `VulkanDemo/VulkanMemoryHelper.cs`: the memory type of an allocation and the layout
//! transitions of an image.

use ash::vk;

/// The index of the first memory type of `memory_type_bits` that has `flags`, or -1.
pub fn find_suitable_memory_type_index(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    memory_type_bits: u32,
    flags: vk::MemoryPropertyFlags,
) -> i32 {
    // SAFETY: the physical device belongs to the instance.
    let properties = unsafe { instance.get_physical_device_memory_properties(physical_device) };

    for i in 0..properties.memory_type_count {
        let type_ = properties.memory_types[i as usize];

        if (memory_type_bits & (1 << i)) != 0 && type_.property_flags.contains(flags) {
            return i as i32;
        }
    }

    -1
}

/// Records a barrier that moves `image` from one layout and access to another.
#[allow(clippy::too_many_arguments)]
pub fn transition_layout(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    image: vk::Image,
    source_layout: vk::ImageLayout,
    source_access_mask: vk::AccessFlags,
    destination_layout: vk::ImageLayout,
    destination_access_mask: vk::AccessFlags,
    mip_levels: u32,
) {
    let subresource_range = vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: mip_levels,
        base_array_layer: 0,
        layer_count: 1,
    };

    let barrier = vk::ImageMemoryBarrier::default()
        .src_access_mask(source_access_mask)
        .dst_access_mask(destination_access_mask)
        .old_layout(source_layout)
        .new_layout(destination_layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(subresource_range);

    // SAFETY: the command buffer is recording and the image belongs to its device.
    unsafe {
        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );
    }
}
