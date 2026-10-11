use super::VulkanCommandBuffer;
use crate::{IVulkanPlatformGraphicsContext, VulkanHelpers};
use ash::vk;

pub(crate) struct VulkanMemoryHelper;

impl VulkanMemoryHelper {
    /// The index of the first memory type of the physical device that is
    /// allowed by `memory_type_bits` and has all of `flags`; -1 when there
    /// is none.
    pub fn find_suitable_memory_type_index(
        context: &dyn IVulkanPlatformGraphicsContext,
        memory_type_bits: u32,
        flags: vk::MemoryPropertyFlags,
    ) -> i32 {
        let properties =
            context.instance_api().get_physical_device_memory_properties(context.physical_device_handle());
        Self::find_suitable_memory_type_index_in(&properties, memory_type_bits, flags)
    }

    /// [`find_suitable_memory_type_index`](Self::find_suitable_memory_type_index)
    /// over the memory properties of a physical device.
    pub fn find_suitable_memory_type_index_in(
        properties: &vk::PhysicalDeviceMemoryProperties,
        memory_type_bits: u32,
        flags: vk::MemoryPropertyFlags,
    ) -> i32 {
        let count = (properties.memory_type_count as usize).min(properties.memory_types.len());
        for (i, memory_type) in properties.memory_types[..count].iter().enumerate() {
            if (memory_type_bits & (1 << i)) != 0 && memory_type.property_flags.contains(flags) {
                return i as i32;
            }
        }

        -1
    }

    /// Records the change of the layout of the colour aspect of all
    /// `mip_levels` of an image.
    #[allow(clippy::too_many_arguments)]
    pub fn transition_layout(
        context: &dyn IVulkanPlatformGraphicsContext,
        command_buffer: &VulkanCommandBuffer,
        image: vk::Image,
        source_layout: vk::ImageLayout,
        source_access_mask: vk::AccessFlags,
        destination_layout: vk::ImageLayout,
        destination_access_mask: vk::AccessFlags,
        mip_levels: u32,
    ) {
        let subresource_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            level_count: mip_levels,
            layer_count: 1,
            ..Default::default()
        };

        let barrier = vk::ImageMemoryBarrier::default()
            .src_access_mask(source_access_mask)
            .dst_access_mask(destination_access_mask)
            .old_layout(source_layout)
            .new_layout(destination_layout)
            .src_queue_family_index(VulkanHelpers::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(VulkanHelpers::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(subresource_range);

        context.device_api().cmd_pipeline_barrier(
            command_buffer.handle(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::DependencyFlags::empty(),
            &[barrier],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn properties(types: &[vk::MemoryPropertyFlags]) -> vk::PhysicalDeviceMemoryProperties {
        let mut properties = vk::PhysicalDeviceMemoryProperties::default();
        properties.memory_type_count = types.len() as u32;
        for (i, flags) in types.iter().enumerate() {
            properties.memory_types[i].property_flags = *flags;
        }
        properties
    }

    #[test]
    fn the_first_allowed_type_with_all_flags_is_taken() {
        let local = vk::MemoryPropertyFlags::DEVICE_LOCAL;
        let host = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        let properties = properties(&[host, local, local | host]);

        assert_eq!(VulkanMemoryHelper::find_suitable_memory_type_index_in(&properties, 0b111, local), 1);
        // The type is not among the allowed ones: the next one with the flags.
        assert_eq!(VulkanMemoryHelper::find_suitable_memory_type_index_in(&properties, 0b101, local), 2);
        // All of the flags have to be there.
        assert_eq!(VulkanMemoryHelper::find_suitable_memory_type_index_in(&properties, 0b011, local | host), -1);
        assert_eq!(VulkanMemoryHelper::find_suitable_memory_type_index_in(&properties, 0b111, local | host), 2);
    }

    #[test]
    fn no_type_is_minus_one() {
        let properties = properties(&[]);
        assert_eq!(
            VulkanMemoryHelper::find_suitable_memory_type_index_in(
                &properties,
                u32::MAX,
                vk::MemoryPropertyFlags::DEVICE_LOCAL
            ),
            -1
        );
    }

    #[test]
    fn types_beyond_the_count_are_not_read() {
        let mut properties = properties(&[vk::MemoryPropertyFlags::HOST_VISIBLE]);
        properties.memory_types[1].property_flags = vk::MemoryPropertyFlags::DEVICE_LOCAL;
        assert_eq!(
            VulkanMemoryHelper::find_suitable_memory_type_index_in(
                &properties,
                u32::MAX,
                vk::MemoryPropertyFlags::DEVICE_LOCAL
            ),
            -1
        );
    }
}
