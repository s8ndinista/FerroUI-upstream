use super::vulkan_global_api::{proc_loader, read_into_vec};
use crate::IVulkanInstance;
use ash::vk;
use std::ffi::CStr;
use std::ptr;
use std::sync::Arc;

/// The entry points of Vulkan that are found through an instance.
///
/// The handles given to a member have to be handles of the instance the
/// table was made for (or of its physical devices and devices), which is
/// what makes the calls sound; the members take typed handles, and the
/// objects of this crate pass none but those they received from the same
/// instance.
pub struct VulkanInstanceApi {
    instance: Arc<dyn IVulkanInstance>,
    fp: ash::InstanceFnV1_0,
    fp_1_1: ash::InstanceFnV1_1,
    /// `vkDestroyDevice` and `vkGetDeviceQueue`, which the reference asks
    /// the instance for.
    device_fp: ash::DeviceFnV1_0,
    surface: ash::khr::surface::InstanceFn,
    debug_utils: ash::ext::debug_utils::InstanceFn,
}

impl VulkanInstanceApi {
    /// Resolves the entry points of `instance`. One the instance does not
    /// have (the surface functions without `VK_KHR_surface`, for instance)
    /// is replaced by a function that stops the program when it is called.
    pub fn new(instance: Arc<dyn IVulkanInstance>) -> Self {
        let handle = instance.handle();
        let lookup = |name: &str| instance.get_instance_proc_address(handle, name);
        let fp = ash::InstanceFnV1_0::load(proc_loader(lookup));
        let fp_1_1 = ash::InstanceFnV1_1::load(proc_loader(lookup));
        let device_fp = ash::DeviceFnV1_0::load(proc_loader(lookup));
        let surface = ash::khr::surface::InstanceFn::load(proc_loader(lookup));
        let debug_utils = ash::ext::debug_utils::InstanceFn::load(proc_loader(lookup));
        Self { instance, fp, fp_1_1, device_fp, surface, debug_utils }
    }

    /// The instance of the table.
    pub fn instance(&self) -> &Arc<dyn IVulkanInstance> {
        &self.instance
    }

    /// `vkCreateDebugUtilsMessengerEXT`.
    pub fn create_debug_utils_messenger_ext(
        &self,
        instance: vk::Instance,
        create_info: &vk::DebugUtilsMessengerCreateInfoEXT<'_>,
    ) -> Result<vk::DebugUtilsMessengerEXT, vk::Result> {
        let mut messenger = vk::DebugUtilsMessengerEXT::null();
        // SAFETY: the structure outlives the call and the result is written
        // to a local.
        let result = unsafe {
            (self.debug_utils.create_debug_utils_messenger_ext)(instance, create_info, ptr::null(), &mut messenger)
        };
        match result {
            vk::Result::SUCCESS => Ok(messenger),
            error => Err(error),
        }
    }

    /// `vkEnumeratePhysicalDevices`.
    pub fn enumerate_physical_devices(&self, instance: vk::Instance) -> Result<Vec<vk::PhysicalDevice>, vk::Result> {
        // SAFETY: the function writes at most `count` handles into the
        // buffer `read_into_vec` gives it.
        read_into_vec(|count, data| unsafe { (self.fp.enumerate_physical_devices)(instance, count, data) })
    }

    /// `vkGetPhysicalDeviceProperties`.
    pub fn get_physical_device_properties(&self, physical_device: vk::PhysicalDevice) -> vk::PhysicalDeviceProperties {
        let mut properties = vk::PhysicalDeviceProperties::default();
        // SAFETY: the result is written to a local.
        unsafe { (self.fp.get_physical_device_properties)(physical_device, &mut properties) };
        properties
    }

    /// `vkEnumerateDeviceExtensionProperties` without a layer.
    pub fn enumerate_device_extension_properties(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Vec<vk::ExtensionProperties>, vk::Result> {
        // SAFETY: as for the physical devices.
        read_into_vec(|count, data| unsafe {
            (self.fp.enumerate_device_extension_properties)(physical_device, ptr::null(), count, data)
        })
    }

    /// `vkGetPhysicalDeviceSurfaceSupportKHR`: whether the queue family can
    /// present to the surface.
    pub fn get_physical_device_surface_support_khr(
        &self,
        physical_device: vk::PhysicalDevice,
        queue_family_index: u32,
        surface: vk::SurfaceKHR,
    ) -> Result<bool, vk::Result> {
        let mut supported = vk::FALSE;
        // SAFETY: the result is written to a local.
        let result = unsafe {
            (self.surface.get_physical_device_surface_support_khr)(
                physical_device,
                queue_family_index,
                surface,
                &mut supported,
            )
        };
        match result {
            vk::Result::SUCCESS => Ok(supported != vk::FALSE),
            error => Err(error),
        }
    }

    /// `vkGetPhysicalDeviceQueueFamilyProperties`.
    pub fn get_physical_device_queue_family_properties(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Vec<vk::QueueFamilyProperties> {
        // SAFETY: as for the physical devices; the function has no result.
        read_into_vec(|count, data| unsafe {
            (self.fp.get_physical_device_queue_family_properties)(physical_device, count, data);
            vk::Result::SUCCESS
        })
        .unwrap_or_default()
    }

    /// `vkCreateDevice`.
    pub fn create_device(
        &self,
        physical_device: vk::PhysicalDevice,
        create_info: &vk::DeviceCreateInfo<'_>,
    ) -> Result<vk::Device, vk::Result> {
        let mut device = vk::Device::null();
        // SAFETY: the lifetime of the structure covers what it points to,
        // and the result is written to a local.
        let result = unsafe { (self.fp.create_device)(physical_device, create_info, ptr::null(), &mut device) };
        match result {
            vk::Result::SUCCESS => Ok(device),
            error => Err(error),
        }
    }

    /// `vkDestroyDevice`. The device and its objects must not be used
    /// afterwards.
    pub fn destroy_device(&self, device: vk::Device) {
        // SAFETY: the caller owns the device and gives it up.
        unsafe { (self.device_fp.destroy_device)(device, ptr::null()) };
    }

    /// `vkGetDeviceQueue`.
    pub fn get_device_queue(&self, device: vk::Device, queue_family_index: u32, queue_index: u32) -> vk::Queue {
        let mut queue = vk::Queue::null();
        // SAFETY: the result is written to a local.
        unsafe { (self.device_fp.get_device_queue)(device, queue_family_index, queue_index, &mut queue) };
        queue
    }

    /// `vkGetDeviceProcAddr`: the address of a device-level entry point,
    /// zero when the device does not have it.
    pub fn get_device_proc_addr(&self, device: vk::Device, name: &CStr) -> isize {
        // SAFETY: the name is a zero-terminated string.
        let address = unsafe { (self.fp.get_device_proc_addr)(device, name.as_ptr()) };
        address.map_or(0, |address| address as usize as isize)
    }

    /// `vkDestroySurfaceKHR`. The surface must not be used afterwards.
    pub fn destroy_surface_khr(&self, instance: vk::Instance, surface: vk::SurfaceKHR) {
        // SAFETY: the caller owns the surface and gives it up.
        unsafe { (self.surface.destroy_surface_khr)(instance, surface, ptr::null()) };
    }

    /// `vkGetPhysicalDeviceSurfaceFormatsKHR`.
    pub fn get_physical_device_surface_formats_khr(
        &self,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
    ) -> Result<Vec<vk::SurfaceFormatKHR>, vk::Result> {
        // SAFETY: as for the physical devices.
        read_into_vec(|count, data| unsafe {
            (self.surface.get_physical_device_surface_formats_khr)(physical_device, surface, count, data)
        })
    }

    /// `vkGetPhysicalDeviceMemoryProperties`.
    pub fn get_physical_device_memory_properties(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> vk::PhysicalDeviceMemoryProperties {
        let mut properties = vk::PhysicalDeviceMemoryProperties::default();
        // SAFETY: the result is written to a local.
        unsafe { (self.fp.get_physical_device_memory_properties)(physical_device, &mut properties) };
        properties
    }

    /// `vkGetPhysicalDeviceSurfaceCapabilitiesKHR`.
    pub fn get_physical_device_surface_capabilities_khr(
        &self,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
    ) -> Result<vk::SurfaceCapabilitiesKHR, vk::Result> {
        let mut capabilities = vk::SurfaceCapabilitiesKHR::default();
        // SAFETY: the result is written to a local.
        let result = unsafe {
            (self.surface.get_physical_device_surface_capabilities_khr)(physical_device, surface, &mut capabilities)
        };
        match result {
            vk::Result::SUCCESS => Ok(capabilities),
            error => Err(error),
        }
    }

    /// `vkGetPhysicalDeviceSurfacePresentModesKHR`.
    pub fn get_physical_device_surface_present_modes_khr(
        &self,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
    ) -> Result<Vec<vk::PresentModeKHR>, vk::Result> {
        // SAFETY: as for the physical devices.
        read_into_vec(|count, data| unsafe {
            (self.surface.get_physical_device_surface_present_modes_khr)(physical_device, surface, count, data)
        })
    }

    /// `vkGetPhysicalDeviceProperties2`: fills the structure and the
    /// structures chained to it.
    pub fn get_physical_device_properties2(
        &self,
        physical_device: vk::PhysicalDevice,
        properties: &mut vk::PhysicalDeviceProperties2<'_>,
    ) {
        // SAFETY: the lifetime of the structure covers its chain, and the
        // function writes to the structures of the chain only.
        unsafe { (self.fp_1_1.get_physical_device_properties2)(physical_device, properties) };
    }
}
