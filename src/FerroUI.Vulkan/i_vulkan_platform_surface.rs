use crate::{IVulkanPlatformGraphicsContext, IVulkanRenderTarget};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::PixelSize;
use std::any::TypeId;
use std::rc::Rc;
use std::sync::Arc;

/// A surface that is presented to through a `VkSurfaceKHR` and a swap chain.
///
/// A render surface announces that it is one through
/// `try_get_surface_kind(TypeId::of::<dyn IVulkanKhrSurfacePlatformSurface>())`,
/// which returns an `Rc<dyn IVulkanKhrSurfacePlatformSurface>`; see
/// [`try_get_vulkan_khr_surface`].
pub trait IVulkanKhrSurfacePlatformSurface: IPlatformRenderSurface {
    /// The scaling from logical units to device pixels.
    fn scaling(&self) -> f64;

    /// The size of the surface in device pixels.
    fn size(&self) -> PixelSize;

    /// Creates the `VkSurfaceKHR` of the surface for the instance of
    /// `context`.
    ///
    /// # Panics
    /// Panics when the window system refuses the surface.
    fn create_surface(&self, context: &dyn IVulkanPlatformGraphicsContext) -> u64;

    /// Releases the surface.
    fn dispose(&self);
}

/// The render surface viewed as a surface with a `VkSurfaceKHR`, when it is
/// one.
pub fn try_get_vulkan_khr_surface(
    surface: &dyn IPlatformRenderSurface,
) -> Option<Rc<dyn IVulkanKhrSurfacePlatformSurface>> {
    surface
        .try_get_surface_kind(TypeId::of::<dyn IVulkanKhrSurfacePlatformSurface>())?
        .downcast_ref::<Rc<dyn IVulkanKhrSurfacePlatformSurface>>()
        .cloned()
}

/// A platform render surface that provides its own [`IVulkanRenderTarget`],
/// e.g. an offscreen image-based target that doesn't involve a swapchain.
///
/// A render surface announces that it is one through
/// `try_get_surface_kind(TypeId::of::<dyn IVulkanRenderTargetPlatformSurface>())`,
/// which returns an `Rc<dyn IVulkanRenderTargetPlatformSurface>`; see
/// [`try_get_vulkan_render_target_surface`].
///
/// This API is private to the framework and its backends.
pub trait IVulkanRenderTargetPlatformSurface: IPlatformRenderSurface {
    /// Creates the render target of the surface for `context`.
    fn create_render_target(&self, context: &Rc<dyn IVulkanPlatformGraphicsContext>) -> Rc<dyn IVulkanRenderTarget>;
}

/// The render surface viewed as a surface with its own Vulkan render
/// target, when it is one.
pub fn try_get_vulkan_render_target_surface(
    surface: &dyn IPlatformRenderSurface,
) -> Option<Rc<dyn IVulkanRenderTargetPlatformSurface>> {
    surface
        .try_get_surface_kind(TypeId::of::<dyn IVulkanRenderTargetPlatformSurface>())?
        .downcast_ref::<Rc<dyn IVulkanRenderTargetPlatformSurface>>()
        .cloned()
}

/// Lets a context present to surfaces that are not Vulkan surfaces
/// themselves (the window of a platform).
///
/// A platform registers the factory among the platform features of its
/// options ([`VulkanPlatformSpecificOptions`](crate::VulkanPlatformSpecificOptions)),
/// from where every context of the platform graphics takes it; a context
/// announces it through its optional features as an `Arc<dyn
/// IVulkanKhrSurfacePlatformSurfaceFactory>` (`try_get_shared`).
pub trait IVulkanKhrSurfacePlatformSurfaceFactory: Send + Sync {
    /// Whether the context can present to the surface.
    fn can_render_to_surface(
        &self,
        context: &dyn IVulkanPlatformGraphicsContext,
        surface: &Arc<dyn IPlatformRenderSurface>,
    ) -> bool;

    /// Creates the Vulkan view of the surface.
    fn create_surface(
        &self,
        context: &dyn IVulkanPlatformGraphicsContext,
        surface: &Arc<dyn IPlatformRenderSurface>,
    ) -> Rc<dyn IVulkanKhrSurfacePlatformSurface>;
}
