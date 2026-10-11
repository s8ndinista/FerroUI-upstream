use crate::VulkanImageInfo;
use ferroui_base::platform::{IPlatformHandle, PlatformGraphicsExternalImageProperties};
use ferroui_base::rendering::composition::CompositionGpuImportedImageSynchronizationCapabilities;
use std::rc::Rc;

/// Lets a Vulkan context import images and semaphores that were created
/// outside of it (by another device or another process).
///
/// A graphics context that is a Vulkan context announces the feature through
/// its optional features: `try_get_feature(TypeId::of::<dyn
/// IVulkanContextExternalObjectsFeature>())` returns an `Rc<dyn
/// IVulkanContextExternalObjectsFeature>`.
///
/// This API is unstable.
pub trait IVulkanContextExternalObjectsFeature {
    /// The kinds of image handles the device can import.
    fn supported_image_handle_types(&self) -> Vec<String>;

    /// The kinds of semaphore handles the device can import.
    fn supported_semaphore_types(&self) -> Vec<String>;

    /// The universally unique identifier of the device, when it has one.
    fn device_uuid(&self) -> Option<Vec<u8>>;

    /// The locally unique identifier of the device, when it has one.
    fn device_luid(&self) -> Option<Vec<u8>>;

    /// How access to an imported image of the given handle type can be
    /// synchronized.
    ///
    /// # Panics
    /// Panics when the handle type is not one of
    /// [`supported_image_handle_types`](Self::supported_image_handle_types).
    fn get_synchronization_capabilities(
        &self,
        image_handle_type: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities;

    /// Imports an image.
    ///
    /// # Panics
    /// Panics when the kind of the handle is not supported, and when the
    /// device refuses the image.
    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IVulkanExternalImage>;

    /// Imports a semaphore.
    ///
    /// # Panics
    /// Panics when the kind of the handle is not supported, and when the
    /// device refuses the semaphore.
    fn import_semaphore(&self, handle: Rc<dyn IPlatformHandle>) -> Rc<dyn IVulkanExternalSemaphore>;
}

/// A semaphore of a Vulkan device that was imported from a handle.
///
/// This API is unstable.
pub trait IVulkanExternalSemaphore {
    /// The `VkSemaphore`.
    ///
    /// # Panics
    /// Panics when the semaphore has been disposed.
    fn handle(&self) -> u64;

    /// Submits an empty batch of commands that waits for the semaphore.
    fn submit_wait_semaphore(&self);

    /// Submits an empty batch of commands that signals the semaphore.
    fn submit_signal_semaphore(&self);

    /// Destroys the semaphore.
    fn dispose(&self);
}

/// An image of a Vulkan device that was imported from an external image.
///
/// This API is unstable.
pub trait IVulkanExternalImage {
    /// The image.
    fn info(&self) -> VulkanImageInfo;

    /// Destroys the image and frees its memory.
    fn dispose(&self);
}
