//! Port of `D3DDemo/D3DContent.cs` (Windows): the shaders, the input layout and the vertices of
//! the coloured cube of the Direct3D demo, set on the immediate context of the device.
//!
//! Upstream compiles `D3DDemo\MiniCube.fx`, which the project copies next to the program, with
//! `D3DCompileFromFile`. The effect file is compiled into the program here and compiled from
//! memory with `D3DCompile`, under the same name.

use super::d3d11::{
    required, throw_h_result, D3DCompile, D3DGetInputSignatureBlob, ID3D10Blob, ID3D11Device, ID3D11DeviceContext,
    D3D10_PRIMITIVE_TOPOLOGY_TRIANGLELIST, D3D11_BIND_CONSTANT_BUFFER, D3D11_BIND_VERTEX_BUFFER, D3D11_BUFFER_DESC,
    D3D11_CPU_ACCESS_NONE, D3D11_INPUT_ELEMENT_DESC, D3D11_RESOURCE_MISC_NONE, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT,
    DXGI_FORMAT_R32G32B32A32_FLOAT,
};
use ferroui_base::numerics::{Matrix4x4, Vector4};
use ferroui_microcom::{ComPtr, HResult, IUnknown};
use std::ffi::{c_void, CStr};

/// The effect of the cube (`D3DDemo/MiniCube.fx`).
const MINI_CUBE_FX: &str = include_str!("MiniCube.fx");

pub struct D3DContent;

impl D3DContent {
    /// Builds the pipeline of the cube on the immediate context of `device` and returns the
    /// constant buffer of its transformation.
    pub fn create_mesh(device: &ComPtr<ID3D11Device>) -> Result<ComPtr<IUnknown>, String> {
        // Compile Vertex and Pixel shaders
        let vertex_shader_byte_code = Self::compile_shader(c"D3DDemo\\MiniCube.fx", c"VS", c"vs_4_0")?;
        // SAFETY: the byte code is the blob of the compiler, alive for the call.
        let vertex_shader = required(
            throw_h_result(unsafe {
                device.create_vertex_shader(
                    vertex_shader_byte_code.get_buffer_pointer(),
                    vertex_shader_byte_code.get_buffer_size(),
                    std::ptr::null_mut(),
                )
            })?,
            "ID3D11Device::CreateVertexShader",
        )?;

        let pixel_shader_byte_code = Self::compile_shader(c"D3DDemo\\MiniCube.fx", c"PS", c"ps_4_0")?;
        // SAFETY: as above.
        let pixel_shader = required(
            throw_h_result(unsafe {
                device.create_pixel_shader(
                    pixel_shader_byte_code.get_buffer_pointer(),
                    pixel_shader_byte_code.get_buffer_size(),
                    std::ptr::null_mut(),
                )
            })?,
            "ID3D11Device::CreatePixelShader",
        )?;

        let mut vertex_shader_signature = std::ptr::null_mut();
        // SAFETY: as above; the pointer is valid for the blob written.
        throw_h_result(HResult::check(unsafe {
            D3DGetInputSignatureBlob(
                vertex_shader_byte_code.get_buffer_pointer(),
                vertex_shader_byte_code.get_buffer_size(),
                &mut vertex_shader_signature,
            )
        }))?;
        // SAFETY: the call succeeded, so the pointer is null or a blob this call owns.
        let vertex_shader_signature = required(
            unsafe { ComPtr::<ID3D10Blob>::from_raw(vertex_shader_signature.cast()) },
            "D3DGetInputSignatureBlob",
        )?;

        // Layout from VertexShader input signature
        let position_name: &CStr = c"POSITION";
        let color_name: &CStr = c"COLOR";
        let mut inputs = [
            D3D11_INPUT_ELEMENT_DESC::new(position_name.as_ptr(), 0, DXGI_FORMAT_R32G32B32A32_FLOAT, 0, 0),
            D3D11_INPUT_ELEMENT_DESC::new(color_name.as_ptr(), 0, DXGI_FORMAT_R32G32B32A32_FLOAT, 0, 16),
        ];

        // SAFETY: the descriptions and their names, and the signature, are alive for the call.
        let input_layout = required(
            throw_h_result(unsafe {
                device.create_input_layout(
                    inputs.as_mut_ptr(),
                    inputs.len() as u32,
                    vertex_shader_signature.get_buffer_pointer(),
                    vertex_shader_signature.get_buffer_size(),
                )
            })?,
            "ID3D11Device::CreateInputLayout",
        )?;

        // Instantiate Vertex buffer from vertex data
        let vertices = [
            Vector4::new(-1.0, -1.0, -1.0, 1.0), Vector4::new(1.0, 0.0, 0.0, 1.0), // Front
            Vector4::new(-1.0, 1.0, -1.0, 1.0), Vector4::new(1.0, 0.0, 0.0, 1.0),
            Vector4::new(1.0, 1.0, -1.0, 1.0), Vector4::new(1.0, 0.0, 0.0, 1.0),
            Vector4::new(-1.0, -1.0, -1.0, 1.0), Vector4::new(1.0, 0.0, 0.0, 1.0),
            Vector4::new(1.0, 1.0, -1.0, 1.0), Vector4::new(1.0, 0.0, 0.0, 1.0),
            Vector4::new(1.0, -1.0, -1.0, 1.0), Vector4::new(1.0, 0.0, 0.0, 1.0),
            Vector4::new(-1.0, -1.0, 1.0, 1.0), Vector4::new(0.0, 1.0, 0.0, 1.0), // BACK
            Vector4::new(1.0, 1.0, 1.0, 1.0), Vector4::new(0.0, 1.0, 0.0, 1.0),
            Vector4::new(-1.0, 1.0, 1.0, 1.0), Vector4::new(0.0, 1.0, 0.0, 1.0),
            Vector4::new(-1.0, -1.0, 1.0, 1.0), Vector4::new(0.0, 1.0, 0.0, 1.0),
            Vector4::new(1.0, -1.0, 1.0, 1.0), Vector4::new(0.0, 1.0, 0.0, 1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0), Vector4::new(0.0, 1.0, 0.0, 1.0),
            Vector4::new(-1.0, 1.0, -1.0, 1.0), Vector4::new(0.0, 0.0, 1.0, 1.0), // Top
            Vector4::new(-1.0, 1.0, 1.0, 1.0), Vector4::new(0.0, 0.0, 1.0, 1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0), Vector4::new(0.0, 0.0, 1.0, 1.0),
            Vector4::new(-1.0, 1.0, -1.0, 1.0), Vector4::new(0.0, 0.0, 1.0, 1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0), Vector4::new(0.0, 0.0, 1.0, 1.0),
            Vector4::new(1.0, 1.0, -1.0, 1.0), Vector4::new(0.0, 0.0, 1.0, 1.0),
            Vector4::new(-1.0, -1.0, -1.0, 1.0), Vector4::new(1.0, 1.0, 0.0, 1.0), // Bottom
            Vector4::new(1.0, -1.0, 1.0, 1.0), Vector4::new(1.0, 1.0, 0.0, 1.0),
            Vector4::new(-1.0, -1.0, 1.0, 1.0), Vector4::new(1.0, 1.0, 0.0, 1.0),
            Vector4::new(-1.0, -1.0, -1.0, 1.0), Vector4::new(1.0, 1.0, 0.0, 1.0),
            Vector4::new(1.0, -1.0, -1.0, 1.0), Vector4::new(1.0, 1.0, 0.0, 1.0),
            Vector4::new(1.0, -1.0, 1.0, 1.0), Vector4::new(1.0, 1.0, 0.0, 1.0),
            Vector4::new(-1.0, -1.0, -1.0, 1.0), Vector4::new(1.0, 0.0, 1.0, 1.0), // Left
            Vector4::new(-1.0, -1.0, 1.0, 1.0), Vector4::new(1.0, 0.0, 1.0, 1.0),
            Vector4::new(-1.0, 1.0, 1.0, 1.0), Vector4::new(1.0, 0.0, 1.0, 1.0),
            Vector4::new(-1.0, -1.0, -1.0, 1.0), Vector4::new(1.0, 0.0, 1.0, 1.0),
            Vector4::new(-1.0, 1.0, 1.0, 1.0), Vector4::new(1.0, 0.0, 1.0, 1.0),
            Vector4::new(-1.0, 1.0, -1.0, 1.0), Vector4::new(1.0, 0.0, 1.0, 1.0),
            Vector4::new(1.0, -1.0, -1.0, 1.0), Vector4::new(0.0, 1.0, 1.0, 1.0), // Right
            Vector4::new(1.0, 1.0, 1.0, 1.0), Vector4::new(0.0, 1.0, 1.0, 1.0),
            Vector4::new(1.0, -1.0, 1.0, 1.0), Vector4::new(0.0, 1.0, 1.0, 1.0),
            Vector4::new(1.0, -1.0, -1.0, 1.0), Vector4::new(0.0, 1.0, 1.0, 1.0),
            Vector4::new(1.0, 1.0, -1.0, 1.0), Vector4::new(0.0, 1.0, 1.0, 1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0), Vector4::new(0.0, 1.0, 1.0, 1.0),
        ];

        let mut vertex_buffer_desc = D3D11_BUFFER_DESC {
            byte_width: std::mem::size_of_val(&vertices) as u32,
            usage: D3D11_USAGE_DEFAULT,
            bind_flags: D3D11_BIND_VERTEX_BUFFER,
            cpu_access_flags: D3D11_CPU_ACCESS_NONE,
            misc_flags: D3D11_RESOURCE_MISC_NONE,
            structure_byte_stride: 0,
        };
        let mut subresource_data = D3D11_SUBRESOURCE_DATA::new(vertices.as_ptr().cast());
        // SAFETY: the description and the vertices are alive for the call.
        let vertex_buffer = required(
            throw_h_result(unsafe { device.create_buffer(&mut vertex_buffer_desc, &mut subresource_data) })?,
            "ID3D11Device::CreateBuffer",
        )?;

        // Create Constant Buffer
        let mut constant_buffer_desc = D3D11_BUFFER_DESC {
            byte_width: std::mem::size_of::<Matrix4x4>() as u32,
            usage: D3D11_USAGE_DEFAULT,
            bind_flags: D3D11_BIND_CONSTANT_BUFFER,
            cpu_access_flags: D3D11_CPU_ACCESS_NONE,
            misc_flags: D3D11_RESOURCE_MISC_NONE,
            structure_byte_stride: 0,
        };
        // SAFETY: the description is alive for the call, and there is no initial data.
        let constant_buffer = required(
            throw_h_result(unsafe { device.create_buffer(&mut constant_buffer_desc, std::ptr::null_mut()) })?,
            "ID3D11Device::CreateBuffer",
        )?;

        // Prepare All the stages
        let mut context = std::ptr::null_mut();
        // SAFETY: the pointer is valid for the context written.
        unsafe { device.get_immediate_context(&mut context) };
        // SAFETY: the call returns the immediate context with a reference for the caller.
        let context = required(unsafe { ComPtr::<ID3D11DeviceContext>::from_raw(context.cast()) }, "GetImmediateContext")?;

        let mut stride = (std::mem::size_of::<Vector4>() * 2) as u32;
        let mut offset = 0u32;
        let mut vertex_buffers: [*mut c_void; 1] = [vertex_buffer.as_ptr().cast()];
        let mut constant_buffers: [*mut c_void; 1] = [constant_buffer.as_ptr().cast()];
        // SAFETY: every object passed is alive; the context keeps its own references.
        unsafe {
            context.ia_set_input_layout(input_layout.as_ptr().cast());
            context.ia_set_primitive_topology(D3D10_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            context.ia_set_vertex_buffers(0, 1, vertex_buffers.as_mut_ptr(), &mut stride, &mut offset);
            context.vs_set_constant_buffers(0, 1, constant_buffers.as_mut_ptr());
            context.vs_set_shader(vertex_shader.as_ptr().cast(), std::ptr::null_mut(), 0);
            context.ps_set_shader(pixel_shader.as_ptr().cast(), std::ptr::null_mut(), 0);
        }

        Ok(constant_buffer)
    }

    fn compile_shader(file_name: &CStr, entry_point: &CStr, profile: &CStr) -> Result<ComPtr<ID3D10Blob>, String> {
        let mut blob = std::ptr::null_mut();
        // SAFETY: the source and the names are alive for the call; the pointer is valid for the
        // blob written, and no error blob is asked for.
        throw_h_result(HResult::check(unsafe {
            D3DCompile(
                MINI_CUBE_FX.as_ptr().cast(),
                MINI_CUBE_FX.len(),
                file_name.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
                entry_point.as_ptr(),
                profile.as_ptr(),
                0,
                0,
                &mut blob,
                std::ptr::null_mut(),
            )
        }))?;
        // SAFETY: the call succeeded, so the pointer is null or a blob this call owns.
        required(unsafe { ComPtr::<ID3D10Blob>::from_raw(blob.cast()) }, "D3DCompile")
    }
}
