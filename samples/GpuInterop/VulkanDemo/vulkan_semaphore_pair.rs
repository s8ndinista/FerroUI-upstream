//! Port of `VulkanDemo/VulkanSemaphorePair.cs`: the two semaphores of a swapchain image (the
//! image is available, the rendering has finished), exportable for the compositor as opaque file
//! descriptors or NT handles of Vulkan.

use super::vulkan_context::VulkanContext;
use super::vulkan_extensions::{ThrowOnError, VulkanDemoError, VulkanDemoResult};
use ash::vk;
use ferroui_base::platform::{IPlatformHandle, KnownPlatformGraphicsExternalSemaphoreHandleTypes, PlatformHandle};
use std::rc::Rc;

pub struct VulkanSemaphorePair {
    resources: Rc<VulkanContext>,
    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
}

impl VulkanSemaphorePair {
    pub fn new(resources: &Rc<VulkanContext>, exportable: bool) -> VulkanDemoResult<Self> {
        let mut semaphore_export_info = vk::ExportSemaphoreCreateInfo::default().handle_types(if cfg!(windows) {
            vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_WIN32
        } else {
            vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD
        });

        let mut semaphore_create_info = vk::SemaphoreCreateInfo::default();
        if exportable {
            semaphore_create_info = semaphore_create_info.push_next(&mut semaphore_export_info);
        }

        let device = resources.device();
        // SAFETY: the structure and its chain live for the calls.
        let image_available_semaphore =
            unsafe { device.create_semaphore(&semaphore_create_info, None) }.throw_on_error()?;

        // SAFETY: as above.
        let render_finished_semaphore =
            unsafe { device.create_semaphore(&semaphore_create_info, None) }.throw_on_error()?;

        Ok(Self { resources: resources.clone(), image_available_semaphore, render_finished_semaphore })
    }

    pub fn export_fd(&self, render_finished: bool) -> VulkanDemoResult<i32> {
        if !self.resources.has_device_extension(ash::khr::external_semaphore_fd::NAME) {
            return Err(VulkanDemoError::new("Operation is not valid due to the current state of the object."));
        }
        let ext = ash::khr::external_semaphore_fd::Device::new(self.resources.instance(), self.resources.device());
        let info = vk::SemaphoreGetFdInfoKHR::default()
            .semaphore(if render_finished { self.render_finished_semaphore } else { self.image_available_semaphore })
            .handle_type(vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD);
        // SAFETY: the semaphore was created exportable as an opaque file descriptor.
        unsafe { ext.get_semaphore_fd(&info) }.throw_on_error()
    }

    pub fn export_win32(&self, render_finished: bool) -> VulkanDemoResult<isize> {
        if !self.resources.has_device_extension(ash::khr::external_semaphore_win32::NAME) {
            return Err(VulkanDemoError::new("Operation is not valid due to the current state of the object."));
        }
        let ext = ash::khr::external_semaphore_win32::Device::new(self.resources.instance(), self.resources.device());
        let info = vk::SemaphoreGetWin32HandleInfoKHR::default()
            .semaphore(if render_finished { self.render_finished_semaphore } else { self.image_available_semaphore })
            .handle_type(vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_WIN32);
        // SAFETY: the semaphore was created exportable as an opaque NT handle.
        unsafe { ext.get_semaphore_win32_handle(&info) }.throw_on_error()
    }

    pub fn export(&self, render_finished: bool) -> VulkanDemoResult<Rc<dyn IPlatformHandle>> {
        if cfg!(windows) {
            return Ok(Rc::new(PlatformHandle::new(
                self.export_win32(render_finished)?,
                Some(KnownPlatformGraphicsExternalSemaphoreHandleTypes::VULKAN_OPAQUE_NT_HANDLE),
            )));
        }
        Ok(Rc::new(PlatformHandle::new(
            self.export_fd(render_finished)? as isize,
            Some(KnownPlatformGraphicsExternalSemaphoreHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR),
        )))
    }

    pub fn image_available_semaphore(&self) -> vk::Semaphore {
        self.image_available_semaphore
    }

    pub fn render_finished_semaphore(&self) -> vk::Semaphore {
        self.render_finished_semaphore
    }

    pub fn dispose(&self) {
        let device = self.resources.device();
        // SAFETY: nothing waits for or signals the semaphores any more.
        unsafe {
            device.destroy_semaphore(self.image_available_semaphore, None);
            device.destroy_semaphore(self.render_finished_semaphore, None);
        }
    }
}
