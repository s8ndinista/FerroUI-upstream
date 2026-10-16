//! Port of `VulkanDemo/VulkanBufferHelper.cs`: buffers in memory the host sees, filled with
//! values.

use super::vulkan_context::VulkanContext;
use super::vulkan_extensions::{ThrowOnError, VulkanDemoResult};
use ash::vk;

/// Creates a buffer of `initial_data`'s size with `buffer_usage_flags` in memory the host sees
/// and copies `initial_data` into it.
pub fn allocate_buffer<T: Copy>(
    vk_context: &VulkanContext,
    buffer_usage_flags: vk::BufferUsageFlags,
    initial_data: &[T],
) -> VulkanDemoResult<(vk::Buffer, vk::DeviceMemory)> {
    let device = vk_context.device();

    let size = std::mem::size_of_val(initial_data);
    let buffer_info = vk::BufferCreateInfo::default()
        .size(size as u64)
        .usage(buffer_usage_flags)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    // SAFETY: the device is alive and the structure is valid.
    let buffer = unsafe { device.create_buffer(&buffer_info, None) }.throw_on_error()?;

    // SAFETY: the buffer belongs to the device.
    let memory_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

    let physical_device = vk_context.physical_device();

    let memory_allocate_info = vk::MemoryAllocateInfo::default().allocation_size(memory_requirements.size).memory_type_index(
        find_suitable_memory_type_index(
            vk_context.instance(),
            physical_device,
            memory_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        ) as u32,
    );

    // SAFETY: the structure is valid.
    let memory = unsafe { device.allocate_memory(&memory_allocate_info, None) }.throw_on_error()?;
    // Upstream ignores the result of the binding.
    // SAFETY: the memory was allocated for the requirements of the buffer.
    let _ = unsafe { device.bind_buffer_memory(buffer, memory, 0) };
    update_buffer_memory(vk_context, memory, initial_data);
    Ok((buffer, memory))
}

/// Copies `data` to the start of `memory`, which the host sees.
pub fn update_buffer_memory<T: Copy>(vk_context: &VulkanContext, memory: vk::DeviceMemory, data: &[T]) {
    let device = vk_context.device();

    let size = std::mem::size_of_val(data);
    // SAFETY: the memory is host visible and at least `size` bytes long; the copy writes `size`
    // bytes to the mapping, which is unmapped right after.
    unsafe {
        // Upstream ignores the result of the mapping.
        if let Ok(pointer) = device.map_memory(memory, 0, size as u64, vk::MemoryMapFlags::empty()) {
            std::ptr::copy_nonoverlapping(data.as_ptr().cast::<u8>(), pointer.cast::<u8>(), size);
        }

        device.unmap_memory(memory);
    }
}

fn find_suitable_memory_type_index(
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
