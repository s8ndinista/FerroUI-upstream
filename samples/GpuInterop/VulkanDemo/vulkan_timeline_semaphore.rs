//! Port of `VulkanDemo/VulkanTimelineSemaphore.cs`: the timeline semaphore of a swapchain image
//! on macOS, exported for the compositor as a Metal shared event (`VK_EXT_metal_objects` of
//! MoltenVK).

use super::vulkan_context::VulkanContext;
use super::vulkan_extensions::{ThrowOnError, VulkanDemoError, VulkanDemoResult};
use ash::vk;
use ferroui_base::platform::{IPlatformHandle, KnownPlatformGraphicsExternalSemaphoreHandleTypes, PlatformHandle};
use std::rc::Rc;

pub struct VulkanTimelineSemaphore {
    resources: Rc<VulkanContext>,
    handle: vk::Semaphore,
}

impl VulkanTimelineSemaphore {
    pub fn new(resources: &Rc<VulkanContext>) -> VulkanDemoResult<Self> {
        let mut mtl_event = vk::ExportMetalObjectCreateInfoEXT::default()
            .export_object_type(vk::ExportMetalObjectTypeFlagsEXT::METAL_SHARED_EVENT);

        let mut semaphore_type_info =
            vk::SemaphoreTypeCreateInfo::default().semaphore_type(vk::SemaphoreType::TIMELINE);
        semaphore_type_info.p_next = (&mut mtl_event as *mut vk::ExportMetalObjectCreateInfoEXT).cast();

        let semaphore_create_info = vk::SemaphoreCreateInfo::default().push_next(&mut semaphore_type_info);
        // SAFETY: the structure and its chain live for the call.
        let handle = unsafe { resources.device().create_semaphore(&semaphore_create_info, None) }.throw_on_error()?;
        Ok(Self { resources: resources.clone(), handle })
    }

    pub fn handle(&self) -> vk::Semaphore {
        self.handle
    }

    pub fn dispose(&self) {
        // SAFETY: nothing waits for or signals the semaphore any more.
        unsafe { self.resources.device().destroy_semaphore(self.handle, None) };
    }

    pub fn export_shared_event(&self) -> VulkanDemoResult<isize> {
        if !self.resources.has_device_extension(ash::ext::metal_objects::NAME) {
            return Err(VulkanDemoError::new("Operation is not valid due to the current state of the object."));
        }
        let ext = ash::ext::metal_objects::Device::new(self.resources.instance(), self.resources.device());
        let mut event_export = vk::ExportMetalSharedEventInfoEXT::default().semaphore(self.handle);
        let mut export = vk::ExportMetalObjectsInfoEXT::default().push_next(&mut event_export);
        // SAFETY: the semaphore was created with a shared event to export, and the chain is valid.
        unsafe { (ext.fp().export_metal_objects_ext)(self.resources.device().handle(), &mut export) };
        if event_export.mtl_shared_event.is_null() {
            return Err(VulkanDemoError::new("Unable to export IOSurfaceRef"));
        }
        Ok(event_export.mtl_shared_event as isize)
    }

    pub fn export(&self) -> VulkanDemoResult<Rc<dyn IPlatformHandle>> {
        if cfg!(target_os = "macos") {
            return Ok(Rc::new(PlatformHandle::new(
                self.export_shared_event()?,
                Some(KnownPlatformGraphicsExternalSemaphoreHandleTypes::METAL_SHARED_EVENT),
            )));
        }
        Err(VulkanDemoError::new("Operation is not supported on this platform."))
    }
}
