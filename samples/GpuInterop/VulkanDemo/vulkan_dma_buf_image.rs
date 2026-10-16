//! Port of `VulkanDemo/VulkanDmaBufImage.cs`: an image whose memory is a Linux dma-buf with an
//! explicit DRM format modifier, which the EGL compositor imports
//! (`EGL_EXT_image_dma_buf_import`).

use super::vulkan_context::VulkanContext;
use super::vulkan_extensions::{ThrowOnError, VulkanDemoError, VulkanDemoResult};
use super::vulkan_image::VulkanImage;
use super::vulkan_memory_helper;
use ash::vk;
use ferroui_base::platform::{
    IPlatformHandle, KnownPlatformGraphicsExternalImageHandleTypes, PlatformGraphicsExternalImageDmaBufProperties,
    PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties, PlatformHandle,
};
use ferroui_base::PixelSize;
use std::ops::Deref;
use std::rc::Rc;

pub struct VulkanDmaBufImage {
    base: VulkanImage,
    /// The DRM fourcc format of the dma-buf backing.
    drm_format: u32,
    /// The DRM format modifier chosen by the driver.
    drm_modifier: u64,
    /// Per-plane byte offsets into the dma-buf.
    plane_offsets: Vec<u64>,
    /// Per-plane row strides of the dma-buf.
    plane_strides: Vec<u64>,
}

impl Deref for VulkanDmaBufImage {
    type Target = VulkanImage;

    fn deref(&self) -> &VulkanImage {
        &self.base
    }
}

impl VulkanDmaBufImage {
    pub fn new(vk: &Rc<VulkanContext>, format: vk::Format, size: PixelSize) -> VulkanDemoResult<Self> {
        let base = VulkanImage::base(vk, format, size, vk::ImageTiling::DRM_FORMAT_MODIFIER_EXT);
        let drm_format = match base.format() {
            vk::Format::B8G8R8A8_UNORM => drm_fourcc('A', 'R', '2', '4'), // DRM_FORMAT_ARGB8888
            vk::Format::R8G8B8A8_UNORM => drm_fourcc('A', 'B', '2', '4'), // DRM_FORMAT_ABGR8888
            other => {
                return Err(VulkanDemoError::new(format!("Format {other:?} has no known DRM fourcc mapping")));
            }
        };

        if !vk.has_device_extension(ash::ext::image_drm_format_modifier::NAME) {
            return Err(VulkanDemoError::new("VK_EXT_image_drm_format_modifier is not available"));
        }
        let drm_ext = ash::ext::image_drm_format_modifier::Device::new(vk.instance(), vk.device());

        let instance = vk.instance();
        let physical_device = vk.physical_device();
        let mut modifier_list = vk::DrmFormatModifierPropertiesListEXT::default();
        {
            let mut format_properties = vk::FormatProperties2::default().push_next(&mut modifier_list);
            // SAFETY: the physical device belongs to the instance and the chain is valid.
            unsafe { instance.get_physical_device_format_properties2(physical_device, base.format(), &mut format_properties) };
        }

        let mut modifier_props =
            vec![vk::DrmFormatModifierPropertiesEXT::default(); modifier_list.drm_format_modifier_count as usize];
        modifier_list.p_drm_format_modifier_properties = modifier_props.as_mut_ptr();
        {
            let mut format_properties = vk::FormatProperties2::default().push_next(&mut modifier_list);
            // SAFETY: as above; the array has room for the count the first call gave.
            unsafe { instance.get_physical_device_format_properties2(physical_device, base.format(), &mut format_properties) };
        }

        let required_features = vk::FormatFeatureFlags::COLOR_ATTACHMENT | vk::FormatFeatureFlags::TRANSFER_DST;

        let single_plane_modifiers: Vec<u64> = modifier_props
            .iter()
            .filter(|p| {
                p.drm_format_modifier_tiling_features.contains(required_features) && p.drm_format_modifier_plane_count == 1
            })
            .map(|p| p.drm_format_modifier)
            .collect();
        if single_plane_modifiers.is_empty() {
            return Err(VulkanDemoError::new("No single-plane DRM format modifier supports rendering to this format"));
        }

        const DRM_FORMAT_MOD_LINEAR: u64 = 0;
        let candidate_modifiers = if single_plane_modifiers.contains(&DRM_FORMAT_MOD_LINEAR) {
            vec![DRM_FORMAT_MOD_LINEAR]
        } else {
            single_plane_modifiers
        };

        let image = {
            let mut modifier_list_create_info =
                vk::ImageDrmFormatModifierListCreateInfoEXT::default().drm_format_modifiers(&candidate_modifiers);
            let mut external_memory_create_info =
                vk::ExternalMemoryImageCreateInfo::default().handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);
            let image_create_info = vk::ImageCreateInfo::default()
                .push_next(&mut modifier_list_create_info)
                .push_next(&mut external_memory_create_info)
                .image_type(vk::ImageType::TYPE_2D)
                .format(base.format())
                .extent(vk::Extent3D { width: size.width as u32, height: size.height as u32, depth: 1 })
                .mip_levels(base.mip_levels())
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(base.tiling())
                .usage(base.usage_flags())
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED);
            // SAFETY: the structure and its chain live for the call.
            unsafe { vk.device().create_image(&image_create_info, None) }.throw_on_error()?
        };
        base.set_internal_handle(image);

        let mut image_modifier_props = vk::ImageDrmFormatModifierPropertiesEXT::default();
        // SAFETY: the image was created with a DRM format modifier.
        unsafe { drm_ext.get_image_drm_format_modifier_properties(image, &mut image_modifier_props) }.throw_on_error()?;
        let drm_modifier = image_modifier_props.drm_format_modifier;

        let mut plane_count = 1usize;
        for p in &modifier_props {
            if p.drm_format_modifier == drm_modifier {
                plane_count = p.drm_format_modifier_plane_count as usize;
            }
        }

        let device = vk.device();
        // SAFETY: the image belongs to the device.
        let memory_requirements = unsafe { device.get_image_memory_requirements(image) };

        let mut dedicated_allocation = vk::MemoryDedicatedAllocateInfo::default().image(image);
        let mut export_allocate_info =
            vk::ExportMemoryAllocateInfo::default().handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);
        let memory_allocate_info = vk::MemoryAllocateInfo::default()
            .push_next(&mut dedicated_allocation)
            .push_next(&mut export_allocate_info)
            .allocation_size(memory_requirements.size)
            .memory_type_index(vulkan_memory_helper::find_suitable_memory_type_index(
                instance,
                physical_device,
                memory_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ) as u32);
        // SAFETY: the structure and its chain live for the call.
        let image_memory = unsafe { device.allocate_memory(&memory_allocate_info, None) }.throw_on_error()?;
        base.attach_memory(image_memory, memory_requirements.size)?;

        let mut plane_offsets = vec![0u64; plane_count];
        let mut plane_strides = vec![0u64; plane_count];
        let plane_aspects = [
            vk::ImageAspectFlags::MEMORY_PLANE_0_EXT,
            vk::ImageAspectFlags::MEMORY_PLANE_1_EXT,
            vk::ImageAspectFlags::MEMORY_PLANE_2_EXT,
            vk::ImageAspectFlags::MEMORY_PLANE_3_EXT,
        ];
        for plane in 0..plane_count {
            let subresource = vk::ImageSubresource { aspect_mask: plane_aspects[plane], mip_level: 0, array_layer: 0 };
            // SAFETY: the image has the plane.
            let layout = unsafe { device.get_image_subresource_layout(image, subresource) };
            plane_offsets[plane] = layout.offset;
            plane_strides[plane] = layout.row_pitch;
        }

        base.create_image_view()?;

        Ok(Self { base, drm_format, drm_modifier, plane_offsets, plane_strides })
    }

    pub fn drm_format(&self) -> u32 {
        self.drm_format
    }

    pub fn drm_modifier(&self) -> u64 {
        self.drm_modifier
    }

    pub fn plane_offsets(&self) -> &[u64] {
        &self.plane_offsets
    }

    pub fn plane_strides(&self) -> &[u64] {
        &self.plane_strides
    }

    fn export_dma_buf_fd(&self) -> VulkanDemoResult<i32> {
        let vk = self.context();
        if !vk.has_device_extension(ash::khr::external_memory_fd::NAME) {
            return Err(VulkanDemoError::new("Operation is not valid due to the current state of the object."));
        }
        let ext = ash::khr::external_memory_fd::Device::new(vk.instance(), vk.device());
        let info = vk::MemoryGetFdInfoKHR::default()
            .memory(self.device_memory())
            .handle_type(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);
        // SAFETY: the memory was allocated exportable as a dma-buf.
        unsafe { ext.get_memory_fd(&info) }.throw_on_error()
    }

    /// The dma-buf of the image for the compositor: its handle, its properties, and the file
    /// descriptors the caller closes once the compositor has imported them.
    pub fn export_dma_buf(&self) -> VulkanDemoResult<(Rc<dyn IPlatformHandle>, PlatformGraphicsExternalImageProperties, Vec<i32>)> {
        let plane_count = self.plane_offsets.len();
        let fd = self.export_dma_buf_fd()?;

        let mut fds = vec![0i32; plane_count];
        fds[0] = fd;
        for plane_fd in fds.iter_mut().skip(1) {
            *plane_fd = libc_dup(fd);
        }

        let plane_offsets: Vec<u32> = self.plane_offsets.iter().map(|&offset| offset as u32).collect();
        let plane_strides: Vec<u32> = self.plane_strides.iter().map(|&stride| stride as u32).collect();

        let properties = PlatformGraphicsExternalImageProperties {
            width: self.size().width,
            height: self.size().height,
            format: if self.format() == vk::Format::B8G8R8A8_UNORM {
                PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm
            } else {
                PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm
            },
            memory_size: self.memory_size(),
            dma_buf_properties: Some(PlatformGraphicsExternalImageDmaBufProperties {
                drm_format: self.drm_format,
                drm_modifier: self.drm_modifier,
                plane_count: plane_count as i32,
                plane_fds: Some(fds.clone()),
                plane_offsets: Some(plane_offsets),
                plane_strides: Some(plane_strides),
            }),
            ..Default::default()
        };

        Ok((
            Rc::new(PlatformHandle::new(fd as isize, Some(KnownPlatformGraphicsExternalImageHandleTypes::DMA_BUF_FILE_DESCRIPTOR))),
            properties,
            fds,
        ))
    }
}

fn drm_fourcc(a: char, b: char, c: char, d: char) -> u32 {
    (a as u32) | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24)
}

/// `dup` of the C library (`_dup` of the C runtime on Windows, which has no dma-bufs).
fn libc_dup(fd: i32) -> i32 {
    // SAFETY: `dup` has no preconditions; an invalid descriptor gives -1.
    unsafe { libc::dup(fd) }
}
