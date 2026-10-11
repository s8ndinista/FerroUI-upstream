use ferroui_vulkan::ash::{self, vk};
use ferroui_vulkan::IVulkanInstance;
use std::ffi::{c_void, CStr};
use std::ptr;

/// The entry point of `VK_KHR_xlib_surface`, found through an instance.
pub(crate) struct X11VulkanInterface {
    fp: ash::khr::xlib_surface::InstanceFn,
}

impl X11VulkanInterface {
    /// Resolves the entry point of `instance`. Without the extension it is
    /// replaced by a function that stops the program when it is called; the
    /// platform requires the extension of every instance it creates.
    pub fn new(instance: &dyn IVulkanInstance) -> Self {
        let handle = instance.handle();
        let fp = ash::khr::xlib_surface::InstanceFn::load(|name: &CStr| match name.to_str() {
            Ok(name) => instance.get_instance_proc_address(handle, name) as *const c_void,
            Err(_) => ptr::null(),
        });
        Self { fp }
    }

    /// `vkCreateXlibSurfaceKHR`: the result and the surface.
    ///
    /// The connection and the window of the structure have to be alive, and
    /// `instance` has to be the instance the table was made for.
    pub fn vk_create_xlib_surface_khr(
        &self,
        instance: isize,
        create_info: &VkXlibSurfaceCreateInfoKHR<'_>,
    ) -> (i32, u64) {
        let mut surface = vk::SurfaceKHR::null();
        // SAFETY: the structure outlives the call and names a connection and
        // a window of the platform (see `vulkan_support.rs`); the result is
        // written to a local.
        let result = unsafe {
            (self.fp.create_xlib_surface_khr)(
                vk::Handle::from_raw(instance as u64),
                create_info,
                ptr::null(),
                &mut surface,
            )
        };
        (result.as_raw(), vk::Handle::as_raw(surface))
    }
}

/// `VkXlibSurfaceCreateInfoKHR`: the structure of the bindings, whose
/// `sType` is set by `Default`.
pub(crate) type VkXlibSurfaceCreateInfoKHR<'a> = vk::XlibSurfaceCreateInfoKHR<'a>;

/// `VK_STRUCTURE_TYPE_XLIB_SURFACE_CREATE_INFO_KHR`.
#[allow(dead_code)] // Set by the bindings; kept for the tests.
pub(crate) const VK_STRUCTURE_TYPE_XLIB_SURFACE_CREATE_INFO_KHR: u32 = 1000004000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_structure_of_the_bindings_has_the_type_of_the_reference() {
        let info = VkXlibSurfaceCreateInfoKHR::default();
        assert_eq!(info.s_type.as_raw() as u32, VK_STRUCTURE_TYPE_XLIB_SURFACE_CREATE_INFO_KHR);
        assert!(info.p_next.is_null());
        assert!(info.dpy.is_null());
        assert_eq!(info.window, 0);
    }
}
