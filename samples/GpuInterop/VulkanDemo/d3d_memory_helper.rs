//! Port of `VulkanDemo/D3DMemoryHelper.cs` (Windows): the Direct3D 11 device on the adapter of
//! the Vulkan device, and the shared textures the memory of the images of the Vulkan demo is
//! imported from where the compositor takes Direct3D 11 textures.

use super::vulkan_extensions::{VulkanDemoError, VulkanDemoResult};
use crate::d3d_demo::d3d11::{self,
    CreateDXGIFactory1, D3D11CreateDevice, ID3D11Device, ID3D11Texture2D, IDXGIAdapter, IDXGIFactory1, D3D11_BIND_RENDER_TARGET,
    D3D11_BIND_SHADER_RESOURCE, D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX, D3D11_RESOURCE_MISC_SHARED_NTHANDLE,
    D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_10_0,
    D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_12_0, D3D_FEATURE_LEVEL_12_1,
    D3D_FEATURE_LEVEL_9_1, D3D_FEATURE_LEVEL_9_2, D3D_FEATURE_LEVEL_9_3, DXGI_ERROR_NOT_FOUND, DXGI_FORMAT_R8G8B8A8_UNORM,
    DXGI_SAMPLE_DESC, LUID,
};
use ash::vk;
use ferroui_base::PixelSize;
use ferroui_microcom::{ComPtr, HResult, Interface};

/// `ThrowHResult`: a failed call is an error.
fn throw_h_result<T>(result: Result<T, HResult>) -> VulkanDemoResult<T> {
    d3d11::throw_h_result(result).map_err(VulkanDemoError::new)
}

/// The object a call that succeeded returned.
fn required<T>(value: Option<T>, what: &str) -> VulkanDemoResult<T> {
    d3d11::required(value, what).map_err(VulkanDemoError::new)
}

pub struct D3DMemoryHelper;

impl D3DMemoryHelper {
    /// A Direct3D 11 device on the adapter whose LUID is `luid` (the bytes Vulkan reports).
    pub fn create_device_by_luid(luid: [u8; 8]) -> VulkanDemoResult<ComPtr<ID3D11Device>> {
        let luid = LUID {
            low_part: u32::from_le_bytes([luid[0], luid[1], luid[2], luid[3]]),
            high_part: i32::from_le_bytes([luid[4], luid[5], luid[6], luid[7]]),
        };
        let mut factory = std::ptr::null_mut();
        // SAFETY: the identifier is the one of the interface the result is read as, and the
        // pointer is valid for the one pointer written.
        throw_h_result(HResult::check(unsafe { CreateDXGIFactory1(&IDXGIFactory1::IID, &mut factory) }))?;
        // SAFETY: the call succeeded, so the pointer is null or a factory this call owns.
        let factory = required(unsafe { ComPtr::<IDXGIFactory1>::from_raw(factory.cast()) }, "CreateDXGIFactory1")?;
        let adapter = Self::get_adapter_by_luid(&factory, luid)?;

        let feature_levels = [
            D3D_FEATURE_LEVEL_12_1,
            D3D_FEATURE_LEVEL_12_0,
            D3D_FEATURE_LEVEL_11_1,
            D3D_FEATURE_LEVEL_11_0,
            D3D_FEATURE_LEVEL_10_0,
            D3D_FEATURE_LEVEL_9_3,
            D3D_FEATURE_LEVEL_9_2,
            D3D_FEATURE_LEVEL_9_1,
        ];

        let mut device = std::ptr::null_mut();
        let mut context = std::ptr::null_mut();
        let mut actual_feature_level = 0;
        // SAFETY: the adapter is alive; the feature levels are read from the array of the
        // length given; the out pointers are valid for the values written.
        throw_h_result(HResult::check(unsafe {
            D3D11CreateDevice(
                adapter.as_ptr().cast(),
                D3D_DRIVER_TYPE_UNKNOWN,
                std::ptr::null_mut(),
                0,
                feature_levels.as_ptr(),
                feature_levels.len() as u32,
                D3D11_SDK_VERSION,
                &mut device,
                &mut actual_feature_level,
                &mut context,
            )
        }))?;
        // The context is not kept, as upstream.
        // SAFETY: the call succeeded, so each pointer is null or an object this call owns.
        drop(unsafe { ComPtr::<crate::d3d_demo::d3d11::ID3D11DeviceContext>::from_raw(context.cast()) });
        // SAFETY: as above.
        required(unsafe { ComPtr::<ID3D11Device>::from_raw(device.cast()) }, "D3D11CreateDevice")
    }

    fn get_adapter_by_luid(factory: &ComPtr<IDXGIFactory1>, luid: LUID) -> VulkanDemoResult<ComPtr<IDXGIAdapter>> {
        let mut index = 0u32;

        loop {
            let adapter = match factory.enum_adapters(index) {
                Err(error) if error.0 == DXGI_ERROR_NOT_FOUND => break,
                result => required(throw_h_result(result)?, "IDXGIFactory::EnumAdapters")?,
            };
            if adapter.get_desc().is_ok_and(|desc| Self::are_luids_equal(desc.adapter_luid, luid)) {
                return Ok(adapter);
            }

            index += 1;
        }

        Err(VulkanDemoError::new("Device with the corresponding LUID not found"))
    }

    /// A texture of `size` that can be shared with an NT handle and has a keyed mutex.
    pub fn create_memory_handle(
        device: &ComPtr<ID3D11Device>,
        size: PixelSize,
        format: vk::Format,
    ) -> VulkanDemoResult<ComPtr<ID3D11Texture2D>> {
        if format != vk::Format::R8G8B8A8_UNORM {
            return Err(VulkanDemoError::new("Not supported format"));
        }

        let mut texture_desc = D3D11_TEXTURE2D_DESC {
            format: DXGI_FORMAT_R8G8B8A8_UNORM,
            width: size.width as u32,
            height: size.height as u32,
            array_size: 1,
            mip_levels: 1,
            sample_desc: DXGI_SAMPLE_DESC::new(1, 0),
            usage: D3D11_USAGE_DEFAULT,
            bind_flags: D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE,
            cpu_access_flags: 0,
            misc_flags: D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX | D3D11_RESOURCE_MISC_SHARED_NTHANDLE,
        };
        // SAFETY: the description is valid for the call, and there is no initial data.
        let texture = throw_h_result(unsafe { device.create_texture2d(&mut texture_desc, std::ptr::null_mut()) })?;

        required(texture, "ID3D11Device::CreateTexture2D")
    }

    fn are_luids_equal(x: LUID, y: LUID) -> bool {
        x.low_part == y.low_part && x.high_part == y.high_part
    }
}
