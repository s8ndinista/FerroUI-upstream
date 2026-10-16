//! Port of `VulkanDemo/VulkanImage.cs`: an image of the device with its memory and view, which
//! can be exported for the compositor (an opaque file descriptor or NT handle of Vulkan, a
//! Direct3D 11 texture on Windows, an IOSurface on macOS) and read back to a PNG file.

use super::vulkan_context::VulkanContext;
use super::vulkan_extensions::{ThrowOnError, VulkanDemoError, VulkanDemoResult};
use super::vulkan_memory_helper;
use ash::vk;
use ferroui_base::platform::{IPlatformHandle, KnownPlatformGraphicsExternalImageHandleTypes, PlatformHandle};
use ferroui_base::PixelSize;
use std::cell::Cell;
use std::rc::Rc;

/// The Direct3D 11 texture the memory of an image is imported from, on Windows where the
/// compositor takes Direct3D 11 textures (`ComPtr<ID3D11Texture2D>` upstream).
#[cfg(windows)]
type D3DTexture = Option<ferroui_microcom::ComPtr<crate::d3d_demo::d3d11::ID3D11Texture2D>>;

/// No Direct3D off Windows.
#[cfg(not(windows))]
type D3DTexture = ();

pub struct VulkanImage {
    vk: Rc<VulkanContext>,
    current_layout: Cell<vk::ImageLayout>,
    current_access_flags: Cell<vk::AccessFlags>,
    image_view: Cell<vk::ImageView>,
    image_memory: Cell<vk::DeviceMemory>,
    #[cfg_attr(not(windows), allow(dead_code))]
    d3d_texture2_d: D3DTexture,

    image_usage_flags: vk::ImageUsageFlags,

    internal_handle: Cell<vk::Image>,
    format: vk::Format,
    aspect_flags: vk::ImageAspectFlags,

    mip_levels: u32,
    size: PixelSize,
    memory_size: Cell<u64>,

    has_io_surface: bool,
    /// The tiling of the image (the virtual property `Tiling` upstream, which the dma-buf image
    /// overrides).
    tiling: vk::ImageTiling,
}

impl VulkanImage {
    /// The fields of an image without a handle, memory or view yet: what the constructor of a
    /// deriving class starts from (the protected constructor upstream).
    pub(super) fn base(vk: &Rc<VulkanContext>, format: vk::Format, size: PixelSize, tiling: vk::ImageTiling) -> Self {
        Self {
            vk: vk.clone(),
            current_layout: Cell::new(vk::ImageLayout::UNDEFINED),
            current_access_flags: Cell::new(vk::AccessFlags::empty()),
            image_view: Cell::new(vk::ImageView::null()),
            image_memory: Cell::new(vk::DeviceMemory::null()),
            d3d_texture2_d: Default::default(),
            image_usage_flags: vk::ImageUsageFlags::COLOR_ATTACHMENT
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::SAMPLED,
            internal_handle: Cell::new(vk::Image::null()),
            format,
            aspect_flags: vk::ImageAspectFlags::COLOR,
            mip_levels: 1, //mipLevels;
            size,
            memory_size: Cell::new(0),
            has_io_surface: false,
            tiling,
            //MipLevels = MipLevels != 0 ? MipLevels : (uint)Math.Floor(Math.Log(Math.Max(Size.Width, Size.Height), 2));
        }
    }

    /// An image of `format` and `size`; with `exportable` its memory can be handed to the
    /// compositor, which takes one of `supported_handle_types`.
    pub fn new(
        vk: &Rc<VulkanContext>,
        format: vk::Format,
        size: PixelSize,
        exportable: bool,
        supported_handle_types: &[String],
    ) -> VulkanDemoResult<Self> {
        let mut this = Self::base(vk, format, size, vk::ImageTiling::OPTIMAL);
        let device = vk.device();

        let handle_type = if cfg!(windows) {
            if supported_handle_types.iter().any(|t| t == KnownPlatformGraphicsExternalImageHandleTypes::D3D11_TEXTURE_NT_HANDLE)
                && !supported_handle_types
                    .iter()
                    .any(|t| t == KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_NT_HANDLE)
            {
                vk::ExternalMemoryHandleTypeFlags::D3D11_TEXTURE
            } else {
                vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32
            }
        } else {
            vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD
        };

        let external_memory_create_info = vk::ExternalMemoryImageCreateInfo::default().handle_types(handle_type);

        let io_surface_create_info = vk::ExportMetalObjectCreateInfoEXT::default()
            .export_object_type(vk::ExportMetalObjectTypeFlagsEXT::METAL_IOSURFACE);

        this.has_io_surface = exportable && cfg!(target_os = "macos");

        let mut image_create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(this.format)
            .extent(vk::Extent3D { width: size.width as u32, height: size.height as u32, depth: 1 })
            .mip_levels(this.mip_levels)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(this.tiling)
            .usage(this.image_usage_flags)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .flags(vk::ImageCreateFlags::MUTABLE_FORMAT);
        if exportable {
            image_create_info.p_next = if cfg!(target_os = "macos") {
                (&io_surface_create_info as *const vk::ExportMetalObjectCreateInfoEXT).cast()
            } else {
                (&external_memory_create_info as *const vk::ExternalMemoryImageCreateInfo).cast()
            };
        }

        // SAFETY: the structure and its chain live for the call.
        let image = unsafe { device.create_image(&image_create_info, None) }.throw_on_error()?;
        this.internal_handle.set(image);

        if !exportable || !cfg!(target_os = "macos") {
            // SAFETY: the image belongs to the device.
            let memory_requirements = unsafe { device.get_image_memory_requirements(image) };

            let dedicated_allocation = vk::MemoryDedicatedAllocateInfo::default().image(image);

            let mut fd_export = vk::ExportMemoryAllocateInfo::default().handle_types(handle_type);
            fd_export.p_next = (&dedicated_allocation as *const vk::MemoryDedicatedAllocateInfo).cast();

            let mut handle_import = vk::ImportMemoryWin32HandleInfoKHR::default();
            if handle_type == vk::ExternalMemoryHandleTypeFlags::D3D11_TEXTURE && exportable {
                this.import_d3d_texture(&mut handle_import, &dedicated_allocation)?;
            }

            let mut memory_allocate_info = vk::MemoryAllocateInfo::default()
                .allocation_size(memory_requirements.size)
                .memory_type_index(vulkan_memory_helper::find_suitable_memory_type_index(
                    vk.instance(),
                    vk.physical_device(),
                    memory_requirements.memory_type_bits,
                    vk::MemoryPropertyFlags::DEVICE_LOCAL,
                ) as u32);
            if exportable {
                memory_allocate_info.p_next = if handle_import.handle != 0 {
                    (&handle_import as *const vk::ImportMemoryWin32HandleInfoKHR).cast()
                } else {
                    (&fd_export as *const vk::ExportMemoryAllocateInfo).cast()
                };
            }

            // SAFETY: the structure and its chain live for the call.
            let image_memory = unsafe { device.allocate_memory(&memory_allocate_info, None) }.throw_on_error()?;

            this.attach_memory(image_memory, memory_requirements.size)?;
        }

        this.create_image_view()?;
        Ok(this)
    }

    /// Creates the Direct3D 11 texture the memory of the image is imported from and points
    /// `handle_import` at its shared handle.
    #[cfg(windows)]
    fn import_d3d_texture(
        &mut self,
        handle_import: &mut vk::ImportMemoryWin32HandleInfoKHR<'_>,
        dedicated_allocation: &vk::MemoryDedicatedAllocateInfo<'_>,
    ) -> VulkanDemoResult<()> {
        let texture = {
            let d3d_device = self.vk.d3d_device();
            let Some(d3d_device) = d3d_device.as_ref() else {
                return Err(VulkanDemoError::new("Vulkan D3DDevice wasn't created"));
            };
            super::d3d_memory_helper::D3DMemoryHelper::create_memory_handle(d3d_device, self.size, self.format)?
        };
        self.d3d_texture2_d = Some(texture);

        handle_import.p_next = (dedicated_allocation as *const vk::MemoryDedicatedAllocateInfo).cast();
        handle_import.handle_type = vk::ExternalMemoryHandleTypeFlags::D3D11_TEXTURE;
        handle_import.handle = self.create_dxgi_shared_handle()?;
        Ok(())
    }

    /// Off Windows the handle type of the memory is never a Direct3D 11 texture.
    #[cfg(not(windows))]
    fn import_d3d_texture(
        &mut self,
        _handle_import: &mut vk::ImportMemoryWin32HandleInfoKHR<'_>,
        _dedicated_allocation: &vk::MemoryDedicatedAllocateInfo<'_>,
    ) -> VulkanDemoResult<()> {
        Err(VulkanDemoError::new("Direct3D 11 textures exist on Windows only"))
    }

    pub fn handle(&self) -> vk::Image {
        self.internal_handle.get()
    }

    pub(super) fn internal_handle(&self) -> vk::Image {
        self.internal_handle.get()
    }

    pub(super) fn set_internal_handle(&self, image: vk::Image) {
        self.internal_handle.set(image);
    }

    pub fn format(&self) -> vk::Format {
        self.format
    }

    pub fn aspect_flags(&self) -> vk::ImageAspectFlags {
        self.aspect_flags
    }

    pub fn view_handle(&self) -> vk::ImageView {
        self.image_view.get()
    }

    pub fn usage_flags(&self) -> vk::ImageUsageFlags {
        self.image_usage_flags
    }

    pub fn memory_handle(&self) -> vk::DeviceMemory {
        self.image_memory.get()
    }

    pub fn device_memory(&self) -> vk::DeviceMemory {
        self.image_memory.get()
    }

    pub fn mip_levels(&self) -> u32 {
        self.mip_levels
    }

    pub fn context(&self) -> &Rc<VulkanContext> {
        &self.vk
    }

    pub fn size(&self) -> PixelSize {
        self.size
    }

    pub fn memory_size(&self) -> u64 {
        self.memory_size.get()
    }

    pub fn current_layout(&self) -> vk::ImageLayout {
        self.current_layout.get()
    }

    /// Records `memory` as the backing of the image and binds it.
    pub(super) fn attach_memory(&self, memory: vk::DeviceMemory, size: u64) -> VulkanDemoResult<()> {
        self.image_memory.set(memory);
        self.memory_size.set(size);
        // SAFETY: the memory was allocated for the requirements of the image.
        unsafe { self.vk.device().bind_image_memory(self.internal_handle.get(), memory, 0) }.throw_on_error()
    }

    /// Creates the image view over the image and moves the image into its initial layout.
    /// Must be called once by every constructor after the image and its memory exist.
    pub(super) fn create_image_view(&self) -> VulkanDemoResult<()> {
        let component_mapping = vk::ComponentMapping {
            r: vk::ComponentSwizzle::IDENTITY,
            g: vk::ComponentSwizzle::IDENTITY,
            b: vk::ComponentSwizzle::IDENTITY,
            a: vk::ComponentSwizzle::IDENTITY,
        };

        let subresource_range = vk::ImageSubresourceRange {
            aspect_mask: self.aspect_flags,
            base_mip_level: 0,
            level_count: self.mip_levels,
            base_array_layer: 0,
            layer_count: 1,
        };

        let image_view_create_info = vk::ImageViewCreateInfo::default()
            .image(self.internal_handle.get())
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(self.format)
            .components(component_mapping)
            .subresource_range(subresource_range);

        // SAFETY: the image belongs to the device and the structure is valid.
        let image_view = unsafe { self.vk.device().create_image_view(&image_view_create_info, None) }.throw_on_error()?;

        self.image_view.set(image_view);

        self.current_layout.set(vk::ImageLayout::UNDEFINED);

        self.transition_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::AccessFlags::NONE);
        Ok(())
    }

    #[cfg(windows)]
    fn create_dxgi_shared_handle(&self) -> VulkanDemoResult<isize> {
        use crate::d3d_demo::d3d11::{IDXGIResource1, DXGI_SHARED_RESOURCE_READ, DXGI_SHARED_RESOURCE_WRITE};

        let Some(texture) = &self.d3d_texture2_d else {
            return Err(VulkanDemoError::new("The image has no Direct3D 11 texture"));
        };
        let dxgi_resource = texture.cast::<IDXGIResource1>().map_err(|e| VulkanDemoError::new(e.to_string()))?;

        // SAFETY: no security attributes and no name: the two pointers may be null.
        unsafe {
            dxgi_resource.create_shared_handle(
                std::ptr::null_mut(),
                DXGI_SHARED_RESOURCE_READ | DXGI_SHARED_RESOURCE_WRITE,
                std::ptr::null(),
            )
        }
        .map_err(|e| VulkanDemoError::new(e.to_string()))
    }

    pub fn export_fd(&self) -> VulkanDemoResult<i32> {
        if !self.vk.has_device_extension(ash::khr::external_memory_fd::NAME) {
            return Err(VulkanDemoError::new("Operation is not valid due to the current state of the object."));
        }
        let ext = ash::khr::external_memory_fd::Device::new(self.vk.instance(), self.vk.device());
        let info = vk::MemoryGetFdInfoKHR::default()
            .memory(self.image_memory.get())
            .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
        // SAFETY: the memory was allocated exportable as an opaque file descriptor.
        unsafe { ext.get_memory_fd(&info) }.throw_on_error()
    }

    pub fn export_opaque_nt_handle(&self) -> VulkanDemoResult<isize> {
        if !self.vk.has_device_extension(ash::khr::external_memory_win32::NAME) {
            return Err(VulkanDemoError::new("Operation is not valid due to the current state of the object."));
        }
        let ext = ash::khr::external_memory_win32::Device::new(self.vk.instance(), self.vk.device());
        let info = vk::MemoryGetWin32HandleInfoKHR::default()
            .memory(self.image_memory.get())
            .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32);
        // SAFETY: the memory was allocated exportable as an opaque NT handle.
        unsafe { ext.get_memory_win32_handle(&info) }.throw_on_error()
    }

    pub fn export_io_surface(&self) -> VulkanDemoResult<isize> {
        if !self.vk.has_device_extension(ash::ext::metal_objects::NAME) {
            return Err(VulkanDemoError::new("Operation is not valid due to the current state of the object."));
        }
        let ext = ash::ext::metal_objects::Device::new(self.vk.instance(), self.vk.device());
        let mut surface_export = vk::ExportMetalIOSurfaceInfoEXT::default().image(self.internal_handle.get());
        let mut export = vk::ExportMetalObjectsInfoEXT::default().push_next(&mut surface_export);
        // SAFETY: the image was created with an IOSurface to export, and the chain is valid.
        unsafe { (ext.fp().export_metal_objects_ext)(self.vk.device().handle(), &mut export) };
        if surface_export.io_surface.is_null() {
            return Err(VulkanDemoError::new("Unable to export IOSurfaceRef"));
        }
        Ok(surface_export.io_surface as isize)
    }

    pub fn export(&self) -> VulkanDemoResult<Rc<dyn IPlatformHandle>> {
        if cfg!(windows) {
            #[cfg(windows)]
            if self.d3d_texture2_d.is_some() {
                return Ok(Rc::new(PlatformHandle::new(
                    self.create_dxgi_shared_handle()?,
                    Some(KnownPlatformGraphicsExternalImageHandleTypes::D3D11_TEXTURE_NT_HANDLE),
                )));
            }

            return Ok(Rc::new(PlatformHandle::new(
                self.export_opaque_nt_handle()?,
                Some(KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_NT_HANDLE),
            )));
        } else if cfg!(target_os = "macos") {
            return Ok(Rc::new(PlatformHandle::new(
                self.export_io_surface()?,
                Some(KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF),
            )));
        }
        Ok(Rc::new(PlatformHandle::new(
            self.export_fd()? as isize,
            Some(KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR),
        )))
    }

    pub fn tiling(&self) -> vk::ImageTiling {
        self.tiling
    }

    pub fn is_direct_x_backed(&self) -> bool {
        #[cfg(windows)]
        return self.d3d_texture2_d.is_some();
        #[cfg(not(windows))]
        return false;
    }

    pub fn transition_layout_from(
        &self,
        command_buffer: vk::CommandBuffer,
        from_layout: vk::ImageLayout,
        from_access_flags: vk::AccessFlags,
        destination_layout: vk::ImageLayout,
        destination_access_flags: vk::AccessFlags,
    ) {
        vulkan_memory_helper::transition_layout(
            self.vk.device(),
            command_buffer,
            self.internal_handle.get(),
            from_layout,
            from_access_flags,
            destination_layout,
            destination_access_flags,
            self.mip_levels,
        );

        self.current_layout.set(destination_layout);
        self.current_access_flags.set(destination_access_flags);
    }

    pub fn transition_layout_in(
        &self,
        command_buffer: vk::CommandBuffer,
        destination_layout: vk::ImageLayout,
        destination_access_flags: vk::AccessFlags,
    ) {
        self.transition_layout_from(
            command_buffer,
            self.current_layout.get(),
            self.current_access_flags.get(),
            destination_layout,
            destination_access_flags,
        );
    }

    pub fn transition_layout(&self, destination_layout: vk::ImageLayout, destination_access_flags: vk::AccessFlags) {
        let command_buffer = self.vk.pool().create_command_buffer();
        command_buffer.begin_recording();
        self.transition_layout_in(command_buffer.internal_handle(), destination_layout, destination_access_flags);
        command_buffer.end_recording();
        command_buffer.submit();
    }

    /// `TransitionLayout(uint, uint)`: the layout and the access flags as their numbers.
    pub fn transition_layout_raw(&self, destination_layout: u32, destination_access_flags: u32) {
        self.transition_layout(
            vk::ImageLayout::from_raw(destination_layout as i32),
            vk::AccessFlags::from_raw(destination_access_flags),
        );
    }

    pub fn dispose(&self) {
        let device = self.vk.device();
        // SAFETY: nothing uses the view, the image or the memory any more.
        unsafe {
            device.destroy_image_view(self.image_view.get(), None);
            device.destroy_image(self.internal_handle.get(), None);
            device.free_memory(self.image_memory.get(), None);
        }

        self.image_view.set(vk::ImageView::null());
        self.internal_handle.set(vk::Image::null());
        self.image_memory.set(vk::DeviceMemory::null());
    }

    /// Writes the content of the image to `path` as a PNG file: through the Skia context of the
    /// device, or, on macOS where there is none, from the IOSurface of the image.
    pub fn save_texture(&self, path: &str) -> VulkanDemoResult<()> {
        let gr_context = self.vk.gr_context().borrow();
        match gr_context.as_ref() {
            None => {
                if self.has_io_surface {
                    self.save_io_surface(path)
                } else {
                    Err(VulkanDemoError::new("Need skia to dump textures, sorry"))
                }
            }
            Some(gr_context) => self.save_with_skia(gr_context, path),
        }
    }

    #[cfg(target_os = "macos")]
    fn save_io_surface(&self, path: &str) -> VulkanDemoResult<()> {
        use crate::native_methods;
        use ferroui_base::media::imaging::{Bitmap, BitmapEncoderOptions, PngBitmapEncoderOptions};
        use ferroui_base::platform::{AlphaFormat, PixelFormat};
        use ferroui_base::Vector;

        let surf = self.export_io_surface()? as native_methods::IOSurfaceRef;
        // SAFETY: the IOSurface of the image is alive while the image is; it is locked while its
        // memory is read.
        unsafe {
            if native_methods::io_surface_lock(surf, native_methods::IOSurfaceLockOptions::NONE, std::ptr::null_mut()) != 0 {
                return Err(VulkanDemoError::new("IOSurfaceLock failed"));
            }
            let w = native_methods::io_surface_get_width(surf) as i32;
            let h = native_methods::io_surface_get_height(surf) as i32;
            let sstride = native_methods::io_surface_get_bytes_per_row(surf);

            let p_surface = native_methods::io_surface_get_base_address(surf);
            let data = std::slice::from_raw_parts(p_surface.cast::<u8>(), sstride * h as usize);
            let b = Bitmap::from_pixels(
                PixelFormat::BGRA8888,
                AlphaFormat::Premul,
                data,
                PixelSize::new(w, h),
                Vector::new(96.0, 96.0),
                sstride as i32,
            );
            let saved = b.save_to_file(path, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT));
            b.dispose();

            native_methods::io_surface_unlock(surf, native_methods::IOSurfaceLockOptions::NONE, std::ptr::null_mut());
            saved.map_err(|e| VulkanDemoError::new(e.to_string()))
        }
    }

    /// An image has an IOSurface on macOS only.
    #[cfg(not(target_os = "macos"))]
    fn save_io_surface(&self, _path: &str) -> VulkanDemoResult<()> {
        Err(VulkanDemoError::new("Need skia to dump textures, sorry"))
    }

    #[cfg(gpu_interop_skia_vulkan)]
    fn save_with_skia(&self, gr_context: &super::vulkan_context::GrContext, path: &str) -> VulkanDemoResult<()> {
        use skia_safe::gpu::{backend_render_targets, surfaces, vk as sk_vk, Protected, SurfaceOrigin};
        use skia_safe::{ColorSpace, ColorType, EncodedImageFormat};
        use vk::Handle;

        // Upstream hands the numbers of the tiling, the layout and the format to Skia as they
        // are; the bindings of Skia take the values of their own enumerations.
        let tiling = match self.tiling() {
            vk::ImageTiling::OPTIMAL => sk_vk::ImageTiling::OPTIMAL,
            vk::ImageTiling::LINEAR => sk_vk::ImageTiling::LINEAR,
            vk::ImageTiling::DRM_FORMAT_MODIFIER_EXT => sk_vk::ImageTiling::DRM_FORMAT_MODIFIER_EXT,
            other => return Err(VulkanDemoError::new(format!("Unsupported image tiling {other:?}"))),
        };
        let layout = match self.current_layout() {
            vk::ImageLayout::UNDEFINED => sk_vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::GENERAL => sk_vk::ImageLayout::GENERAL,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => sk_vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => sk_vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL => sk_vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL => sk_vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            other => return Err(VulkanDemoError::new(format!("Unsupported image layout {other:?}"))),
        };
        let format = match self.format() {
            vk::Format::R8G8B8A8_UNORM => sk_vk::Format::R8G8B8A8_UNORM,
            vk::Format::B8G8R8A8_UNORM => sk_vk::Format::B8G8R8A8_UNORM,
            other => return Err(VulkanDemoError::new(format!("Unsupported image format {other:?}"))),
        };

        let mut gr_context = gr_context.clone();
        gr_context.reset(None);
        let image = self;
        // SAFETY: the handles are those of the image and its memory, which outlive the Skia
        // objects made from them here.
        let image_info = unsafe {
            let alloc = sk_vk::Alloc::from_device_memory(
                image.memory_handle().as_raw() as usize as _,
                0,
                image.memory_size(),
                sk_vk::AllocFlag::empty(),
            );
            let mut image_info = sk_vk::ImageInfo::new(
                image.handle().as_raw() as usize as _,
                alloc,
                tiling,
                layout,
                format,
                image.mip_levels(),
                self.vk.queue_family_index(),
                None,
                Protected::No,
                None,
            );
            image_info.image_usage_flags = image.usage_flags().as_raw();
            image_info.sample_count = 1;
            image_info
        };

        let backend_texture =
            backend_render_targets::make_vk((image.size().width, image.size().height), &image_info);
        let mut surface = surfaces::wrap_backend_render_target(
            &mut gr_context,
            &backend_texture,
            SurfaceOrigin::TopLeft,
            ColorType::RGBA8888,
            ColorSpace::new_srgb(),
            None,
        )
        .ok_or_else(|| VulkanDemoError::new("Unable to wrap the image for Skia"))?;
        let snap = surface.image_snapshot();
        let encoded = snap
            .encode(Some(&mut gr_context), EncodedImageFormat::PNG, None)
            .ok_or_else(|| VulkanDemoError::new("Unable to encode the image"))?;
        std::fs::write(path, encoded.as_bytes()).map_err(|e| VulkanDemoError::new(e.to_string()))
    }

    /// No Skia context exists in a build without Skia with Vulkan.
    #[cfg(not(gpu_interop_skia_vulkan))]
    fn save_with_skia(&self, gr_context: &super::vulkan_context::GrContext, _path: &str) -> VulkanDemoResult<()> {
        match *gr_context {}
    }
}
