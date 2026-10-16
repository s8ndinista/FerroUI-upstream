//! Port of `VulkanDemo/VulkanContent.cs`: the teapot of the ControlCatalog drawn with a
//! pipeline of Vulkan into an image of its own, which is then copied into the image of the
//! swapchain the compositor shows.

use super::vulkan_buffer_helper;
use super::vulkan_context::VulkanContext;
use super::vulkan_extensions::{ThrowOnError, VulkanDemoResult};
use super::vulkan_image::VulkanImage;
use ash::vk;
use ferroui_base::numerics::{Matrix4x4, Vector3};
use ferroui_base::PixelSize;
use std::io::Read;
use std::rc::Rc;
use std::sync::LazyLock;
use std::time::Instant;

/// The model of the teapot, which the ControlCatalog shares with the sample (an embedded
/// resource of the assembly upstream, linked from the ControlCatalog by the project file).
const TEAPOT: &[u8] = include_bytes!("../../ControlCatalog/Pages/teapot.bin");

/// The compiled shaders (embedded resources upstream), from `Assets/Shaders/*.glsl` by the
/// `Makefile` beside them.
const VERT_SPIRV: &[u8] = include_bytes!("Assets/Shaders/vert.spirv");
const FRAG_SPIRV: &[u8] = include_bytes!("Assets/Shaders/frag.spirv");

static ST: LazyLock<Instant> = LazyLock::new(Instant::now);

pub struct VulkanContent {
    context: Rc<VulkanContext>,
    vert_shader: vk::ShaderModule,
    frag_shader: vk::ShaderModule,
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
    descriptor_set_layout: vk::DescriptorSetLayout,
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,
    uniform_buffer: vk::Buffer,
    uniform_buffer_memory: vk::DeviceMemory,
    framebuffer: vk::Framebuffer,

    depth_image: vk::Image,
    depth_image_memory: vk::DeviceMemory,
    depth_image_view: vk::ImageView,

    previous_image_size: Option<PixelSize>,

    points: Vec<Vertex>,
    indices: Vec<u16>,
    min_y: f32,
    max_y: f32,

    is_init: bool,
    color_attachment: Option<VulkanImage>,
    descriptor_set: vk::DescriptorSet,
}

fn read_block(sr: &mut impl Read) -> Vec<u8> {
    let mut length = [0u8; 4];
    sr.read_exact(&mut length).unwrap_or_else(|e| panic!("{e}"));
    let mut buf = vec![0u8; i32::from_le_bytes(length) as usize];
    sr.read_exact(&mut buf).unwrap_or_else(|e| panic!("{e}"));
    buf
}

impl VulkanContent {
    pub fn new(context: &Rc<VulkanContext>) -> VulkanDemoResult<Self> {
        LazyLock::force(&ST);

        let mut sr = TEAPOT;
        let buf = read_block(&mut sr);
        let points: Vec<f32> =
            buf.chunks_exact(4).map(|bytes| f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])).collect();
        let buf = read_block(&mut sr);
        let indices: Vec<u16> = buf.chunks_exact(2).map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]])).collect();
        let mut vertices = vec![Vertex::default(); points.len() / 3];
        for (primitive, vertex) in vertices.iter_mut().enumerate() {
            let srci = primitive * 3;
            vertex.position = Vector3::new(points[srci], points[srci + 1], points[srci + 2]);
        }

        let mut i = 0;
        while i < indices.len() {
            let a = vertices[indices[i] as usize].position;
            let b = vertices[indices[i + 1] as usize].position;
            let c = vertices[indices[i + 2] as usize].position;
            let normal = Vector3::normalize(Vector3::cross(c - b, a - b));

            vertices[indices[i] as usize].normal += normal;
            vertices[indices[i + 1] as usize].normal += normal;
            vertices[indices[i + 2] as usize].normal += normal;
            i += 3;
        }

        let mut max_y = 0.0f32;
        let mut min_y = 0.0f32;
        for vertex in vertices.iter_mut() {
            vertex.normal = Vector3::normalize(vertex.normal);
            max_y = f32::max(max_y, vertex.position.y);
            min_y = f32::min(min_y, vertex.position.y);
        }

        let device = context.device();
        let vert_shader_data = Self::get_shader(false);
        let frag_shader_data = Self::get_shader(true);

        // Upstream ignores the results of the two creations.
        let shader_create_info = vk::ShaderModuleCreateInfo::default().code(&vert_shader_data);
        // SAFETY: the code is SPIR-V, aligned to its words.
        let vert_shader = unsafe { device.create_shader_module(&shader_create_info, None) }.unwrap_or_default();

        let shader_create_info = vk::ShaderModuleCreateInfo::default().code(&frag_shader_data);
        // SAFETY: as above.
        let frag_shader = unsafe { device.create_shader_module(&shader_create_info, None) }.unwrap_or_default();

        let mut this = Self {
            context: context.clone(),
            vert_shader,
            frag_shader,
            pipeline_layout: vk::PipelineLayout::null(),
            render_pass: vk::RenderPass::null(),
            pipeline: vk::Pipeline::null(),
            descriptor_set_layout: vk::DescriptorSetLayout::null(),
            vertex_buffer: vk::Buffer::null(),
            vertex_buffer_memory: vk::DeviceMemory::null(),
            index_buffer: vk::Buffer::null(),
            index_buffer_memory: vk::DeviceMemory::null(),
            uniform_buffer: vk::Buffer::null(),
            uniform_buffer_memory: vk::DeviceMemory::null(),
            framebuffer: vk::Framebuffer::null(),
            depth_image: vk::Image::null(),
            depth_image_memory: vk::DeviceMemory::null(),
            depth_image_view: vk::ImageView::null(),
            previous_image_size: Some(PixelSize::default()),
            points: vertices,
            indices,
            min_y,
            max_y,
            is_init: false,
            color_attachment: None,
            descriptor_set: vk::DescriptorSet::null(),
        };
        this.create_buffers()?;
        Ok(this)
    }

    fn get_shader(fragment: bool) -> Vec<u32> {
        let data = if fragment { FRAG_SPIRV } else { VERT_SPIRV };
        ash::util::read_spv(&mut std::io::Cursor::new(data)).unwrap_or_else(|e| panic!("{e}"))
    }

    /// Draws the teapot turned by `yaw`, `pitch` and `roll`, coloured by `disco`, into `image`.
    ///
    /// # Panics
    /// Panics when the objects of a new size of the image cannot be created (the exception
    /// upstream, which nothing catches).
    pub fn render(&mut self, image: &VulkanImage, yaw: f64, pitch: f64, roll: f64, disco: f64) {
        let device = self.context.device().clone();

        if Some(image.size()) != self.previous_image_size {
            self.create_temporal_objects(image.size()).unwrap_or_else(|e| panic!("{e}"));
        }

        self.previous_image_size = Some(image.size());

        let model = Matrix4x4::create_from_yaw_pitch_roll(yaw as f32, pitch as f32, roll as f32);

        let elapsed = ST.elapsed();
        let ticks = elapsed.as_nanos() / 100;
        let ticks_per_second: u128 = 10_000_000;
        let vertex_constant = VertexPushConstant {
            disco: disco as f32,
            min_y: self.min_y,
            max_y: self.max_y,
            model,
            time: ((ticks % (ticks_per_second * 40)) as f64 / ticks_per_second as f64) as f32,
        };

        let command_buffer = self.context.pool().create_command_buffer();
        command_buffer.begin_recording();

        let color_attachment = self.color_attachment.as_ref().expect("the color attachment of the size");
        color_attachment.transition_layout_from(
            command_buffer.internal_handle(),
            vk::ImageLayout::UNDEFINED,
            vk::AccessFlags::NONE,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        );

        let command_buffer_handle = command_buffer.internal_handle();

        let viewport = vk::Viewport {
            width: image.size().width as f32,
            height: image.size().height as f32,
            max_depth: 1.0,
            min_depth: 0.0,
            x: 0.0,
            y: 0.0,
        };
        let scissor = vk::Rect2D {
            offset: vk::Offset2D::default(),
            extent: vk::Extent2D { width: image.size().width as u32, height: image.size().height as u32 },
        };

        let clear_values = [
            vk::ClearValue { color: vk::ClearColorValue { float32: [1.0, 0.0, 0.0, 0.1] } },
            vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 } },
        ];

        let begin_info = vk::RenderPassBeginInfo::default()
            .render_pass(self.render_pass)
            .framebuffer(self.framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D { width: image.size().width as u32, height: image.size().height as u32 },
            })
            .clear_values(&clear_values);

        let src_blit_region = vk::ImageBlit {
            src_offsets: [
                vk::Offset3D { x: 0, y: 0, z: 0 },
                vk::Offset3D { x: image.size().width, y: image.size().height, z: 1 },
            ],
            dst_offsets: [
                vk::Offset3D { x: 0, y: 0, z: 0 },
                vk::Offset3D { x: image.size().width, y: image.size().height, z: 1 },
            ],
            src_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_array_layer: 0,
                layer_count: 1,
                mip_level: 0,
            },
            dst_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_array_layer: 0,
                layer_count: 1,
                mip_level: 0,
            },
        };

        // SAFETY: the command buffer is recording; every object it is given belongs to the
        // device and lives until the buffer has run (the pool waits for its fence before the
        // objects of another size are destroyed).
        unsafe {
            device.cmd_set_viewport(command_buffer_handle, 0, &[viewport]);
            device.cmd_set_scissor(command_buffer_handle, 0, &[scissor]);

            device.cmd_begin_render_pass(command_buffer_handle, &begin_info, vk::SubpassContents::INLINE);

            device.cmd_bind_pipeline(command_buffer_handle, vk::PipelineBindPoint::GRAPHICS, self.pipeline);

            let dset = self.descriptor_set;
            device.cmd_bind_descriptor_sets(
                command_buffer_handle,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[dset],
                &[],
            );

            device.cmd_push_constants(
                command_buffer_handle,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                as_bytes(&vertex_constant),
            );
            let offsets = 0u64;
            device.cmd_bind_vertex_buffers(command_buffer_handle, 0, &[self.vertex_buffer], &[offsets]);
            device.cmd_bind_index_buffer(command_buffer_handle, self.index_buffer, 0, vk::IndexType::UINT16);

            device.cmd_draw_indexed(command_buffer_handle, self.indices.len() as u32, 1, 0, 0, 0);

            device.cmd_end_render_pass(command_buffer_handle);
        }

        color_attachment.transition_layout_in(
            command_buffer.internal_handle(),
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::AccessFlags::TRANSFER_READ,
        );
        image.transition_layout_in(
            command_buffer.internal_handle(),
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::AccessFlags::TRANSFER_WRITE,
        );

        // SAFETY: as above.
        unsafe {
            device.cmd_blit_image(
                command_buffer.internal_handle(),
                color_attachment.internal_handle(),
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                image.internal_handle(),
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[src_blit_region],
                vk::Filter::LINEAR,
            );
        }

        command_buffer.submit();
    }

    pub fn dispose(&mut self) {
        if self.is_init {
            let device = self.context.device().clone();

            self.destroy_temporal_objects();

            // SAFETY: nothing uses the shaders and the buffers any more.
            unsafe {
                device.destroy_shader_module(self.vert_shader, None);
                device.destroy_shader_module(self.frag_shader, None);

                device.destroy_buffer(self.vertex_buffer, None);
                device.free_memory(self.vertex_buffer_memory, None);

                device.destroy_buffer(self.index_buffer, None);
                device.free_memory(self.index_buffer_memory, None);
            }
        }

        self.is_init = false;
    }

    pub fn destroy_temporal_objects(&mut self) {
        if self.is_init && self.render_pass != vk::RenderPass::null() {
            let device = self.context.device().clone();
            // SAFETY: the objects belong to the device and nothing uses them any more: the pool
            // has waited for the command buffers that did.
            unsafe {
                let _ = device.free_descriptor_sets(self.context.descriptor_pool(), &[self.descriptor_set]);

                device.destroy_image_view(self.depth_image_view, None);
                device.destroy_image(self.depth_image, None);
                device.free_memory(self.depth_image_memory, None);

                device.destroy_framebuffer(self.framebuffer, None);
                device.destroy_pipeline(self.pipeline, None);
                device.destroy_pipeline_layout(self.pipeline_layout, None);
                device.destroy_render_pass(self.render_pass, None);
                device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);

                device.destroy_buffer(self.uniform_buffer, None);
                device.free_memory(self.uniform_buffer_memory, None);
            }
            if let Some(color_attachment) = self.color_attachment.take() {
                color_attachment.dispose();
            }

            self.depth_image = vk::Image::null();
            self.depth_image_view = vk::ImageView::null();
            self.framebuffer = vk::Framebuffer::null();
            self.pipeline = vk::Pipeline::null();
            self.render_pass = vk::RenderPass::null();
            self.pipeline_layout = vk::PipelineLayout::null();
            self.descriptor_set_layout = vk::DescriptorSetLayout::null();
            self.uniform_buffer = vk::Buffer::null();
            self.uniform_buffer_memory = vk::DeviceMemory::null();
        }
    }

    fn create_depth_attachment(&mut self, size: PixelSize) -> VulkanDemoResult<()> {
        let image_create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::D32_SFLOAT)
            .extent(vk::Extent3D { width: size.width as u32, height: size.height as u32, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .flags(vk::ImageCreateFlags::MUTABLE_FORMAT);

        let device = self.context.device().clone();
        // SAFETY: the structure is valid.
        self.depth_image = unsafe { device.create_image(&image_create_info, None) }.throw_on_error()?;

        // SAFETY: the image belongs to the device.
        let memory_requirements = unsafe { device.get_image_memory_requirements(self.depth_image) };

        let memory_allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(memory_requirements.size)
            .memory_type_index(Self::find_suitable_memory_type_index(
                self.context.instance(),
                self.context.physical_device(),
                memory_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ) as u32);

        // SAFETY: the structure is valid.
        self.depth_image_memory = unsafe { device.allocate_memory(&memory_allocate_info, None) }.throw_on_error()?;

        // Upstream ignores the result of the binding.
        // SAFETY: the memory was allocated for the requirements of the image.
        let _ = unsafe { device.bind_image_memory(self.depth_image, self.depth_image_memory, 0) };

        let component_mapping = vk::ComponentMapping {
            r: vk::ComponentSwizzle::R,
            g: vk::ComponentSwizzle::G,
            b: vk::ComponentSwizzle::B,
            a: vk::ComponentSwizzle::A,
        };

        let subresource_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::DEPTH,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };

        let image_view_create_info = vk::ImageViewCreateInfo::default()
            .image(self.depth_image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::D32_SFLOAT)
            .components(component_mapping)
            .subresource_range(subresource_range);

        // SAFETY: the structure is valid.
        self.depth_image_view = unsafe { device.create_image_view(&image_view_create_info, None) }.throw_on_error()?;
        Ok(())
    }

    fn create_temporal_objects(&mut self, size: PixelSize) -> VulkanDemoResult<()> {
        self.destroy_temporal_objects();

        let view = Matrix4x4::create_look_at(Vector3::new(25.0, 25.0, 25.0), Vector3::default(), Vector3::new(0.0, -1.0, 0.0));
        let projection = Matrix4x4::create_perspective_field_of_view(
            std::f32::consts::PI / 4.0,
            size.width as f32 / size.height as f32,
            0.01,
            1000.0,
        );

        self.color_attachment = Some(VulkanImage::new(&self.context, vk::Format::R8G8B8A8_UNORM, size, false, &[])?);
        self.create_depth_attachment(size)?;

        let device = self.context.device().clone();

        // create renderpasses
        let color_attachment = vk::AttachmentDescription {
            format: vk::Format::R8G8B8A8_UNORM,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            ..Default::default()
        };

        let depth_attachment = vk::AttachmentDescription {
            format: vk::Format::D32_SFLOAT,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            ..Default::default()
        };

        let subpass_dependency = vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::empty(),
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            ..Default::default()
        };

        let color_attachment_reference =
            vk::AttachmentReference { attachment: 0, layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL };

        let depth_attachment_reference =
            vk::AttachmentReference { attachment: 1, layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL };

        let color_attachments = [color_attachment_reference];
        let subpass_description = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachments)
            .depth_stencil_attachment(&depth_attachment_reference);

        let attachments = [color_attachment, depth_attachment];
        let subpasses = [subpass_description];
        let dependencies = [subpass_dependency];

        let render_pass_create_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        // SAFETY: the structure and what it points to live for the call.
        self.render_pass = unsafe { device.create_render_pass(&render_pass_create_info, None) }.throw_on_error()?;

        // create framebuffer
        let frame_buffer_attachments = [
            self.color_attachment.as_ref().map(VulkanImage::view_handle).unwrap_or_default(),
            self.depth_image_view,
        ];

        let framebuffer_create_info = vk::FramebufferCreateInfo::default()
            .render_pass(self.render_pass)
            .attachments(&frame_buffer_attachments)
            .width(size.width as u32)
            .height(size.height as u32)
            .layers(1);

        // SAFETY: as above.
        self.framebuffer = unsafe { device.create_framebuffer(&framebuffer_create_info, None) }.throw_on_error()?;

        // Create pipeline
        let pname = c"main";
        let vert_shader_stage_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(self.vert_shader)
            .name(pname);
        let frag_shader_stage_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(self.frag_shader)
            .name(pname);

        let stages = [vert_shader_stage_info, frag_shader_stage_info];

        let binding_description = [Vertex::vertex_input_binding_description()];
        let attribute_description = Vertex::vertex_input_attribute_description();

        let vertext_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&attribute_description)
            .vertex_binding_descriptions(&binding_description);

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: size.width as f32,
            height: size.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };

        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D { width: viewport.width as u32, height: viewport.height as u32 },
        };

        let viewports = [viewport];
        let scissors = [scissor];
        let pipeline_view_port_create_info =
            vk::PipelineViewportStateCreateInfo::default().viewports(&viewports).scissors(&scissors);

        let rasterizer_state_create_info = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE)
            .depth_bias_enable(false);

        let multisample_state_create_info = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let depth_stencil_create_info = vk::PipelineDepthStencilStateCreateInfo::default()
            .stencil_test_enable(false)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_bounds_test_enable(false);

        let color_blend_attachment_state = vk::PipelineColorBlendAttachmentState {
            color_write_mask: vk::ColorComponentFlags::A
                | vk::ColorComponentFlags::R
                | vk::ColorComponentFlags::G
                | vk::ColorComponentFlags::B,
            blend_enable: vk::FALSE,
            ..Default::default()
        };

        let color_blend_attachments = [color_blend_attachment_state];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(&color_blend_attachments);

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

        let dynamic_state_create_info = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let vertex_push_constant_range = vk::PushConstantRange {
            offset: 0,
            size: std::mem::size_of::<VertexPushConstant>() as u32,
            stage_flags: vk::ShaderStageFlags::VERTEX,
        };

        let frag_push_constant_range = vk::PushConstantRange {
            //Offset = vertexPushConstantRange.Size,
            offset: 0,
            size: std::mem::size_of::<VertexPushConstant>() as u32,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
        };

        let layout_binding_info = vk::DescriptorSetLayoutBinding {
            binding: 0,
            stage_flags: vk::ShaderStageFlags::VERTEX,
            descriptor_count: 1,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            ..Default::default()
        };

        let bindings = [layout_binding_info];
        let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

        // SAFETY: as above.
        self.descriptor_set_layout =
            unsafe { device.create_descriptor_set_layout(&layout_info, None) }.throw_on_error()?;

        let proj_view = view * projection;
        let (uniform_buffer, uniform_buffer_memory) = vulkan_buffer_helper::allocate_buffer::<UniformBuffer>(
            &self.context,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            &[UniformBuffer { projection: proj_view }],
        )?;
        self.uniform_buffer = uniform_buffer;
        self.uniform_buffer_memory = uniform_buffer_memory;

        let descriptor_set_layouts = [self.descriptor_set_layout];
        let descriptor_create_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.context.descriptor_pool())
            .set_layouts(&descriptor_set_layouts);
        // SAFETY: as above.
        self.descriptor_set =
            unsafe { device.allocate_descriptor_sets(&descriptor_create_info) }.throw_on_error()?[0];

        let descriptor_buffer_info = [vk::DescriptorBufferInfo {
            buffer: self.uniform_buffer,
            offset: 0,
            range: std::mem::size_of::<UniformBuffer>() as u64,
        }];
        let descriptor_write = vk::WriteDescriptorSet::default()
            .dst_set(self.descriptor_set)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&descriptor_buffer_info);
        // SAFETY: as above.
        unsafe { device.update_descriptor_sets(&[descriptor_write], &[]) };

        let constants = [vertex_push_constant_range, frag_push_constant_range];

        let set_layouts = [self.descriptor_set_layout];
        let pipeline_layout_create_info =
            vk::PipelineLayoutCreateInfo::default().push_constant_ranges(&constants).set_layouts(&set_layouts);

        // SAFETY: as above.
        self.pipeline_layout =
            unsafe { device.create_pipeline_layout(&pipeline_layout_create_info, None) }.throw_on_error()?;

        let pipeline_create_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertext_input_info)
            .input_assembly_state(&input_assembly)
            .viewport_state(&pipeline_view_port_create_info)
            .rasterization_state(&rasterizer_state_create_info)
            .multisample_state(&multisample_state_create_info)
            .depth_stencil_state(&depth_stencil_create_info)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state_create_info)
            .layout(self.pipeline_layout)
            .render_pass(self.render_pass)
            .subpass(0)
            .base_pipeline_handle(if self.pipeline != vk::Pipeline::null() { self.pipeline } else { vk::Pipeline::null() })
            .base_pipeline_index(if self.pipeline != vk::Pipeline::null() { 0 } else { -1 });

        // SAFETY: as above.
        self.pipeline = unsafe {
            device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_create_info], None)
        }
        .map_err(|(_, result)| result)
        .throw_on_error()?[0];

        self.is_init = true;
        Ok(())
    }

    fn create_buffers(&mut self) -> VulkanDemoResult<()> {
        let (vertex_buffer, vertex_buffer_memory) =
            vulkan_buffer_helper::allocate_buffer::<Vertex>(&self.context, vk::BufferUsageFlags::VERTEX_BUFFER, &self.points)?;
        self.vertex_buffer = vertex_buffer;
        self.vertex_buffer_memory = vertex_buffer_memory;
        let (index_buffer, index_buffer_memory) =
            vulkan_buffer_helper::allocate_buffer::<u16>(&self.context, vk::BufferUsageFlags::INDEX_BUFFER, &self.indices)?;
        self.index_buffer = index_buffer;
        self.index_buffer_memory = index_buffer_memory;
        Ok(())
    }

    fn find_suitable_memory_type_index(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        memory_type_bits: u32,
        flags: vk::MemoryPropertyFlags,
    ) -> i32 {
        // SAFETY: the physical device belongs to the instance.
        let properties = unsafe { instance.get_physical_device_memory_properties(physical_device) };

        for i in 0..properties.memory_type_count {
            let type_ = properties.memory_types[i as usize];

            if (memory_type_bits & (1 << i)) != 0 && type_.property_flags.contains(flags) {
                return i as i32;
            }
        }

        -1
    }
}

/// The bytes of a value of a `#[repr(C)]` type without padding, for the push constants.
fn as_bytes<T: Copy>(value: &T) -> &[u8] {
    // SAFETY: the types passed here are `#[repr(C)]` structures of `f32`s, which have no
    // padding, so every byte is initialized.
    unsafe { std::slice::from_raw_parts((value as *const T).cast::<u8>(), std::mem::size_of::<T>()) }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Vertex {
    position: Vector3,
    normal: Vector3,
}

impl Vertex {
    fn vertex_input_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }
    }

    fn vertex_input_attribute_description() -> [vk::VertexInputAttributeDescription; 2] {
        [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: std::mem::offset_of!(Vertex, position) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: std::mem::offset_of!(Vertex, normal) as u32,
            },
        ]
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct VertexPushConstant {
    max_y: f32,
    min_y: f32,
    time: f32,
    disco: f32,
    model: Matrix4x4,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct UniformBuffer {
    projection: Matrix4x4,
}
