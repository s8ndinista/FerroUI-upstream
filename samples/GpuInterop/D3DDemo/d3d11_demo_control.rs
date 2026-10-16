//! Port of `D3DDemo/D3D11DemoControl.cs`: the demo that draws a coloured cube with Direct3D 11
//! into textures it shares with the compositor by their DXGI shared handles.
//!
//! Direct3D exists on Windows only, where the class draws as upstream. Elsewhere the class
//! exists, as upstream, and its initialization ends where the compositor cannot take DXGI shared
//! handles, which no compositor off Windows can.

use crate::drawing_surface_demo_base::{DrawingSurfaceDemoBase, DrawingSurfaceDemoBaseImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::platform::KnownPlatformGraphicsExternalImageHandleTypes;
use ferroui_base::rendering::composition::{CompositionDrawingSurface, Compositor, ICompositionGpuInterop};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, PixelSize, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::ControlImpl;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

#[cfg(windows)]
use windows_impl::Resources;

#[repr(C)]
pub struct D3D11DemoControl {
    base: DrawingSurfaceDemoBase,
    #[cfg(windows)]
    resources: RefCell<Option<Resources>>,
    #[cfg(not(windows))]
    resources: RefCell<Option<std::convert::Infallible>>,
    st: Instant,
}

ferro_class!(D3D11DemoControl: DrawingSurfaceDemoBase);
ferro_class_info!(D3D11DemoControl { new: D3D11DemoControl::new });
ferro_impl_classes!(
    D3D11DemoControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl DrawingSurfaceDemoBaseImpl for D3D11DemoControl {
    fn initialize_graphics_resources(
        this: &Self,
        _compositor: &Rc<Compositor>,
        surface: &CompositionDrawingSurface,
        interop: &Rc<dyn ICompositionGpuInterop>,
    ) -> (bool, String) {
        if !interop
            .supported_image_handle_types()
            .iter()
            .any(|t| t == KnownPlatformGraphicsExternalImageHandleTypes::D3D11_TEXTURE_GLOBAL_SHARED_HANDLE)
        {
            return (false, "DXGI shared handle import is not supported by the current graphics backend".to_string());
        }

        #[cfg(windows)]
        {
            match Resources::new(surface, interop) {
                Ok((resources, info)) => {
                    *this.resources.borrow_mut() = Some(resources);
                    (true, info)
                }
                Err(error) => (false, error),
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (this, surface);
            (false, "Direct3D 11 is available on Windows only".to_string())
        }
    }

    fn free_graphics_resources(this: &Self) {
        let resources = this.resources.borrow_mut().take();
        #[cfg(windows)]
        if let Some(resources) = resources {
            resources.dispose();
        }
        #[cfg(not(windows))]
        let _ = resources;
    }

    fn supports_disco(_this: &Self) -> bool {
        true
    }

    fn render_frame(this: &Self, pixel_size: PixelSize) {
        if pixel_size == PixelSize::default() {
            return;
        }
        #[cfg(windows)]
        {
            let mut resources = this.resources.borrow_mut();
            if let Some(resources) = resources.as_mut() {
                let now = this.st.elapsed().as_secs_f64() * 5.0;
                resources.render_frame(pixel_size, now, this.yaw(), this.pitch(), this.roll(), this.disco());
            }
        }
        #[cfg(not(windows))]
        let _ = (this, pixel_size);
    }
}

impl D3D11DemoControl {
    pub fn construct() -> Self {
        Self { base: DrawingSurfaceDemoBase::construct(), resources: RefCell::new(None), st: Instant::now() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// `Matrix4x4.CreateLookAtLeftHanded` of .NET, which the numerics of the framework do not have:
/// a view matrix of a left-handed system.
pub fn create_look_at_left_handed(
    camera_position: ferroui_base::numerics::Vector3,
    camera_target: ferroui_base::numerics::Vector3,
    camera_up_vector: ferroui_base::numerics::Vector3,
) -> ferroui_base::numerics::Matrix4x4 {
    use ferroui_base::numerics::{Matrix4x4, Vector3};

    let zaxis = Vector3::normalize(camera_target - camera_position);
    let xaxis = Vector3::normalize(Vector3::cross(camera_up_vector, zaxis));
    let yaxis = Vector3::cross(zaxis, xaxis);

    Matrix4x4::new(
        xaxis.x,
        yaxis.x,
        zaxis.x,
        0.0,
        xaxis.y,
        yaxis.y,
        zaxis.y,
        0.0,
        xaxis.z,
        yaxis.z,
        zaxis.z,
        0.0,
        -Vector3::dot(xaxis, camera_position),
        -Vector3::dot(yaxis, camera_position),
        -Vector3::dot(zaxis, camera_position),
        1.0,
    )
}

/// `Matrix4x4.CreatePerspectiveFieldOfViewLeftHanded` of .NET, which the numerics of the
/// framework do not have: a perspective projection of a left-handed system with a depth range
/// of zero to one.
///
/// # Panics
/// Panics where .NET throws: when `field_of_view` is not between zero and pi, when a distance
/// is not above zero, and when the near plane is not before the far plane.
pub fn create_perspective_field_of_view_left_handed(
    field_of_view: f32,
    aspect_ratio: f32,
    near_plane_distance: f32,
    far_plane_distance: f32,
) -> ferroui_base::numerics::Matrix4x4 {
    use ferroui_base::numerics::Matrix4x4;

    if field_of_view <= 0.0 || field_of_view >= std::f32::consts::PI {
        panic!("field_of_view ('{field_of_view}') must be greater than '0' and less than '{}'.", std::f32::consts::PI);
    }
    if near_plane_distance <= 0.0 || far_plane_distance <= 0.0 {
        panic!("the plane distances must be greater than '0'.");
    }
    if near_plane_distance >= far_plane_distance {
        panic!("near_plane_distance ('{near_plane_distance}') must be less than '{far_plane_distance}'.");
    }

    let height = 1.0 / (field_of_view * 0.5).tan();
    let width = height / aspect_ratio;
    let range = if far_plane_distance == f32::INFINITY {
        1.0
    } else {
        far_plane_distance / (far_plane_distance - near_plane_distance)
    };

    Matrix4x4::new(
        width,
        0.0,
        0.0,
        0.0,
        0.0,
        height,
        0.0,
        0.0,
        0.0,
        0.0,
        range,
        1.0,
        0.0,
        0.0,
        -range * near_plane_distance,
        0.0,
    )
}

#[cfg(windows)]
mod windows_impl {
    use super::{create_look_at_left_handed, create_perspective_field_of_view_left_handed};
    use crate::d3d_demo::d3d11::{
        feature_level_name, required, throw_h_result, CreateDXGIFactory1, D3D11CreateDevice, ID3D11Device,
        ID3D11DeviceContext, ID3D11Texture2D, IDXGIFactory1, D3D11_BIND_DEPTH_STENCIL, D3D11_CLEAR_DEPTH,
        D3D11_CPU_ACCESS_NONE, D3D11_RESOURCE_MISC_NONE, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
        D3D11_VIEWPORT, D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_10_0, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1,
        D3D_FEATURE_LEVEL_12_0, D3D_FEATURE_LEVEL_12_1, D3D_FEATURE_LEVEL_9_1, D3D_FEATURE_LEVEL_9_2, D3D_FEATURE_LEVEL_9_3,
        DXGI_FORMAT_D32_FLOAT_S8X24_UINT, DXGI_SAMPLE_DESC,
    };
    use crate::d3d_demo::d3d11_swapchain::D3D11Swapchain;
    use crate::d3d_demo::d3d_content::D3DContent;
    use ferroui_base::numerics::{Matrix4x4, Vector3, Vector4};
    use ferroui_base::rendering::composition::{CompositionDrawingSurface, ICompositionGpuInterop};
    use ferroui_base::PixelSize;
    use ferroui_microcom::{ComPtr, HResult, IUnknown, Interface};
    use std::ffi::c_void;
    use std::rc::Rc;

    /// What the demo draws with (the fields of the class upstream).
    pub(super) struct Resources {
        device: ComPtr<ID3D11Device>,
        swapchain: Option<D3D11Swapchain>,
        context: ComPtr<ID3D11DeviceContext>,
        view: Matrix4x4,
        last_size: PixelSize,
        depth_buffer: Option<ComPtr<ID3D11Texture2D>>,
        depth_view: Option<ComPtr<IUnknown>>,
        proj: Matrix4x4,
        constant_buffer: ComPtr<IUnknown>,
    }

    impl Resources {
        /// `InitializeGraphicsResources` from the creation of the device: the resources and the
        /// text of the demo, or the text of the exception upstream.
        pub(super) fn new(
            surface: &CompositionDrawingSurface,
            interop: &Rc<dyn ICompositionGpuInterop>,
        ) -> Result<(Resources, String), String> {
            let mut factory = std::ptr::null_mut();
            // SAFETY: the identifier is the one of the interface the result is read as, and the
            // pointer is valid for the one pointer written.
            throw_h_result(HResult::check(unsafe { CreateDXGIFactory1(&IDXGIFactory1::IID, &mut factory) }))?;
            // SAFETY: the call succeeded, so the pointer is null or a factory this call owns.
            let factory = required(unsafe { ComPtr::<IDXGIFactory1>::from_raw(factory.cast()) }, "CreateDXGIFactory1")?;

            let adapter = required(throw_h_result(factory.enum_adapters(0))?, "IDXGIFactory::EnumAdapters")?;

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
            // SAFETY: the call succeeded, so each pointer is null or an object this call owns.
            let device = required(unsafe { ComPtr::<ID3D11Device>::from_raw(device.cast()) }, "D3D11CreateDevice")?;
            // SAFETY: as above.
            let context =
                required(unsafe { ComPtr::<ID3D11DeviceContext>::from_raw(context.cast()) }, "D3D11CreateDevice")?;

            let swapchain = D3D11Swapchain::new(&device, interop, surface);
            let constant_buffer = D3DContent::create_mesh(&device)?;
            let view = create_look_at_left_handed(Vector3::new(0.0, 0.0, -5.0), Vector3::new(0.0, 0.0, 0.0), Vector3::UNIT_Y);

            let adapter_desc = throw_h_result(adapter.get_desc())?;
            let length = adapter_desc.description.iter().position(|&c| c == 0).unwrap_or(adapter_desc.description.len());
            let description = String::from_utf16_lossy(&adapter_desc.description[..length]);

            Ok((
                Resources {
                    device,
                    swapchain: Some(swapchain),
                    context,
                    view,
                    last_size: PixelSize::default(),
                    depth_buffer: None,
                    depth_view: None,
                    proj: Matrix4x4::default(),
                    constant_buffer,
                },
                format!("D3D11 ({}) {description}", feature_level_name(actual_feature_level)),
            ))
        }

        /// `FreeGraphicsResources`. Upstream waits for the disposal of the swapchain here; the
        /// disposal is started here, and the textures are released once the compositor is done
        /// with them, holding references of their own.
        pub(super) fn dispose(mut self) {
            if let Some(swapchain) = self.swapchain.take() {
                drop(swapchain.dispose_async());
            }
            // The views, the buffers, the context and the device are released when dropped.
        }

        pub(super) fn render_frame(&mut self, pixel_size: PixelSize, now: f64, yaw: f32, pitch: f32, roll: f32, disco: f32) {
            if pixel_size != self.last_size {
                self.last_size = pixel_size;
                self.resize(pixel_size).unwrap_or_else(|e| panic!("{e}"));
            }
            let Some(swapchain) = &self.swapchain else {
                return;
            };
            let (session, render_view) = swapchain.begin_draw(pixel_size);
            let mut render_view_handle: [*mut c_void; 1] = [render_view.as_ptr().cast()];
            let depth_view = self.depth_view.as_ref().map_or(std::ptr::null_mut(), |view| view.as_ptr().cast());
            // SAFETY: the views are alive for the call.
            unsafe { self.context.om_set_render_targets(1, render_view_handle.as_mut_ptr(), depth_view) };
            let view_proj = self.view * self.proj;

            let disco = f64::from(disco);
            let scale_x = (1.0 + disco * (now.sin() + 1.0) / 6.0) as f32;
            let scale_y = (1.0 + disco * (now.cos() + 1.0) / 8.0) as f32;
            let color_off = ((now.sin() + 1.0) / 2.0 * disco) as f32;

            // Clear views
            let mut color = Vector4::new(1.0 - color_off, color_off, 0.5 + color_off * 0.5, 1.0);
            // SAFETY: the views are alive; the colour is four floats.
            unsafe {
                self.context.clear_depth_stencil_view(depth_view, D3D11_CLEAR_DEPTH, 1.0, 0);
                self.context.clear_render_target_view(render_view.as_ptr().cast(), (&mut color as *mut Vector4).cast());
            }

            // Update WorldViewProj Matrix
            let ypr = Matrix4x4::create_from_yaw_pitch_roll(yaw, pitch, roll);
            let world_view_proj = ypr * Matrix4x4::create_scale(Vector3::new(scale_x, scale_y, 1.0)) * view_proj;
            let mut world_view_proj = Matrix4x4::transpose(world_view_proj);

            // SAFETY: the constant buffer is alive and holds one matrix.
            unsafe {
                self.context.update_subresource(
                    self.constant_buffer.as_ptr().cast(),
                    0,
                    std::ptr::null_mut(),
                    (&mut world_view_proj as *mut Matrix4x4).cast(),
                    0,
                    0,
                );
            }

            // Draw the cube
            self.context.draw(36, 0);

            self.context.flush();
            session.dispose();
        }

        fn resize(&mut self, size: PixelSize) -> Result<(), String> {
            self.depth_buffer = None;

            let mut texture_desc = D3D11_TEXTURE2D_DESC {
                format: DXGI_FORMAT_D32_FLOAT_S8X24_UINT,
                array_size: 1,
                mip_levels: 1,
                width: size.width as u32,
                height: size.height as u32,
                sample_desc: DXGI_SAMPLE_DESC::new(1, 0),
                usage: D3D11_USAGE_DEFAULT,
                bind_flags: D3D11_BIND_DEPTH_STENCIL,
                cpu_access_flags: D3D11_CPU_ACCESS_NONE,
                misc_flags: D3D11_RESOURCE_MISC_NONE,
            };
            // SAFETY: the description is alive for the call, and there is no initial data.
            let depth_buffer = required(
                throw_h_result(unsafe { self.device.create_texture2d(&mut texture_desc, std::ptr::null_mut()) })?,
                "ID3D11Device::CreateTexture2D",
            )?;

            self.depth_view = None;
            // SAFETY: the texture is a resource of the device; no description: a view of the
            // whole texture.
            self.depth_view = Some(required(
                throw_h_result(unsafe { self.device.create_depth_stencil_view(Some(&**depth_buffer), std::ptr::null_mut()) })?,
                "ID3D11Device::CreateDepthStencilView",
            )?);
            self.depth_buffer = Some(depth_buffer);

            // Setup targets and viewport for rendering
            let mut viewport = D3D11_VIEWPORT::new(0.0, 0.0, size.width as f32, size.height as f32, 0.0, 1.0);
            // SAFETY: the viewport is alive for the call.
            unsafe { self.context.rs_set_viewports(1, &mut viewport) };

            // Setup new projection matrix with correct aspect ratio
            self.proj = create_perspective_field_of_view_left_handed(
                std::f32::consts::PI / 4.0,
                size.width as f32 / size.height as f32,
                0.1,
                100.0,
            );
            Ok(())
        }
    }
}
