//! The bindings of Direct3D 11, DXGI and the shader compiler the sample calls (Silk.NET.Direct3D11,
//! Silk.NET.DXGI and Silk.NET.Direct3D.Compilers upstream): the COM interfaces, generated from
//! `d3d11.idl` by the MicroCom generator in the build script of the crate; the structures and
//! constants of the system headers they take; and the four functions of the system the sample
//! calls, linked without import libraries.

#![allow(non_camel_case_types, non_snake_case, clippy::upper_case_acronyms)]

use ferroui_microcom::{Guid, HResult, RawHResult};
use std::ffi::{c_char, c_void};

#[allow(
    clippy::all,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    missing_docs,
    unused_unsafe,
    unsafe_op_in_unsafe_fn
)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/d3d11.rs"));
}

pub use generated::*;

/// `ThrowHResult` of Silk.NET: a failed call is an error, with the text of the exception the
/// runtime throws for it.
pub fn throw_h_result<T>(result: Result<T, HResult>) -> Result<T, String> {
    result.map_err(|error| format!("Error HRESULT {error} has been returned from a call to a COM component."))
}

/// The object a call that succeeded returned: the bindings give `None` for a null pointer,
/// which a call that succeeds does not return.
pub fn required<T>(value: Option<T>, what: &str) -> Result<T, String> {
    value.ok_or_else(|| format!("{what} returned no object"))
}

/// `LUID`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LUID {
    pub low_part: u32,
    pub high_part: i32,
}

/// `DXGI_ADAPTER_DESC`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DXGI_ADAPTER_DESC {
    pub description: [u16; 128],
    pub vendor_id: u32,
    pub device_id: u32,
    pub sub_sys_id: u32,
    pub revision: u32,
    pub dedicated_video_memory: usize,
    pub dedicated_system_memory: usize,
    pub shared_system_memory: usize,
    pub adapter_luid: LUID,
}

/// `DXGI_SAMPLE_DESC`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DXGI_SAMPLE_DESC {
    pub count: u32,
    pub quality: u32,
}

impl DXGI_SAMPLE_DESC {
    pub const fn new(count: u32, quality: u32) -> Self {
        Self { count, quality }
    }
}

/// `D3D11_TEXTURE2D_DESC`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct D3D11_TEXTURE2D_DESC {
    pub width: u32,
    pub height: u32,
    pub mip_levels: u32,
    pub array_size: u32,
    pub format: u32,
    pub sample_desc: DXGI_SAMPLE_DESC,
    pub usage: u32,
    pub bind_flags: u32,
    pub cpu_access_flags: u32,
    pub misc_flags: u32,
}

/// `D3D11_BUFFER_DESC`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct D3D11_BUFFER_DESC {
    pub byte_width: u32,
    pub usage: u32,
    pub bind_flags: u32,
    pub cpu_access_flags: u32,
    pub misc_flags: u32,
    pub structure_byte_stride: u32,
}

/// `D3D11_SUBRESOURCE_DATA`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct D3D11_SUBRESOURCE_DATA {
    pub p_sys_mem: *const c_void,
    pub sys_mem_pitch: u32,
    pub sys_mem_slice_pitch: u32,
}

impl D3D11_SUBRESOURCE_DATA {
    pub const fn new(p_sys_mem: *const c_void) -> Self {
        Self { p_sys_mem, sys_mem_pitch: 0, sys_mem_slice_pitch: 0 }
    }
}

/// `D3D11_INPUT_ELEMENT_DESC`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct D3D11_INPUT_ELEMENT_DESC {
    pub semantic_name: *const c_char,
    pub semantic_index: u32,
    pub format: u32,
    pub input_slot: u32,
    pub aligned_byte_offset: u32,
    pub input_slot_class: u32,
    pub instance_data_step_rate: u32,
}

impl D3D11_INPUT_ELEMENT_DESC {
    /// A per-vertex element (the constructor of Silk.NET the original calls).
    pub const fn new(semantic_name: *const c_char, semantic_index: u32, format: u32, input_slot: u32, aligned_byte_offset: u32) -> Self {
        Self {
            semantic_name,
            semantic_index,
            format,
            input_slot,
            aligned_byte_offset,
            input_slot_class: D3D11_INPUT_PER_VERTEX_DATA,
            instance_data_step_rate: 0,
        }
    }
}

/// `D3D11_VIEWPORT`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct D3D11_VIEWPORT {
    pub top_left_x: f32,
    pub top_left_y: f32,
    pub width: f32,
    pub height: f32,
    pub min_depth: f32,
    pub max_depth: f32,
}

impl D3D11_VIEWPORT {
    pub const fn new(top_left_x: f32, top_left_y: f32, width: f32, height: f32, min_depth: f32, max_depth: f32) -> Self {
        Self { top_left_x, top_left_y, width, height, min_depth, max_depth }
    }
}

// `DXGI_FORMAT`.
pub const DXGI_FORMAT_R32G32B32A32_FLOAT: u32 = 2;
pub const DXGI_FORMAT_D32_FLOAT_S8X24_UINT: u32 = 20;
pub const DXGI_FORMAT_R8G8B8A8_UNORM: u32 = 28;

// `D3D11_USAGE`.
pub const D3D11_USAGE_DEFAULT: u32 = 0;

// `D3D11_BIND_FLAG`.
pub const D3D11_BIND_VERTEX_BUFFER: u32 = 0x1;
pub const D3D11_BIND_CONSTANT_BUFFER: u32 = 0x4;
pub const D3D11_BIND_SHADER_RESOURCE: u32 = 0x8;
pub const D3D11_BIND_RENDER_TARGET: u32 = 0x20;
pub const D3D11_BIND_DEPTH_STENCIL: u32 = 0x40;

// `D3D11_CPU_ACCESS_FLAG`.
pub const D3D11_CPU_ACCESS_NONE: u32 = 0;

// `D3D11_RESOURCE_MISC_FLAG`.
pub const D3D11_RESOURCE_MISC_NONE: u32 = 0;
pub const D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX: u32 = 0x100;
pub const D3D11_RESOURCE_MISC_SHARED_NTHANDLE: u32 = 0x800;

// `D3D11_INPUT_CLASSIFICATION`.
pub const D3D11_INPUT_PER_VERTEX_DATA: u32 = 0;

// `D3D11_CLEAR_FLAG`.
pub const D3D11_CLEAR_DEPTH: u32 = 0x1;

// `D3D_PRIMITIVE_TOPOLOGY`.
pub const D3D10_PRIMITIVE_TOPOLOGY_TRIANGLELIST: u32 = 4;

// `D3D_DRIVER_TYPE`.
pub const D3D_DRIVER_TYPE_UNKNOWN: i32 = 0;

// `D3D_FEATURE_LEVEL`.
pub const D3D_FEATURE_LEVEL_9_1: i32 = 0x9100;
pub const D3D_FEATURE_LEVEL_9_2: i32 = 0x9200;
pub const D3D_FEATURE_LEVEL_9_3: i32 = 0x9300;
pub const D3D_FEATURE_LEVEL_10_0: i32 = 0xa000;
pub const D3D_FEATURE_LEVEL_10_1: i32 = 0xa100;
pub const D3D_FEATURE_LEVEL_11_0: i32 = 0xb000;
pub const D3D_FEATURE_LEVEL_11_1: i32 = 0xb100;
pub const D3D_FEATURE_LEVEL_12_0: i32 = 0xc000;
pub const D3D_FEATURE_LEVEL_12_1: i32 = 0xc100;

/// The name of a feature level as Silk.NET prints the value of its enumeration
/// (`D3DFeatureLevel.Level111`).
pub fn feature_level_name(level: i32) -> String {
    match level {
        D3D_FEATURE_LEVEL_9_1 => "Level91".to_string(),
        D3D_FEATURE_LEVEL_9_2 => "Level92".to_string(),
        D3D_FEATURE_LEVEL_9_3 => "Level93".to_string(),
        D3D_FEATURE_LEVEL_10_0 => "Level100".to_string(),
        D3D_FEATURE_LEVEL_10_1 => "Level101".to_string(),
        D3D_FEATURE_LEVEL_11_0 => "Level110".to_string(),
        D3D_FEATURE_LEVEL_11_1 => "Level111".to_string(),
        D3D_FEATURE_LEVEL_12_0 => "Level120".to_string(),
        D3D_FEATURE_LEVEL_12_1 => "Level121".to_string(),
        other => other.to_string(),
    }
}

/// `D3D11_SDK_VERSION`.
pub const D3D11_SDK_VERSION: u32 = 7;

// `DXGI_SHARED_RESOURCE_*`.
pub const DXGI_SHARED_RESOURCE_READ: u32 = 0x8000_0000;
pub const DXGI_SHARED_RESOURCE_WRITE: u32 = 1;

/// `DXGI_ERROR_NOT_FOUND`.
pub const DXGI_ERROR_NOT_FOUND: u32 = 0x887A_0002;

#[link(name = "d3d11", kind = "raw-dylib")]
extern "system" {
    pub fn D3D11CreateDevice(
        p_adapter: *mut c_void,
        driver_type: i32,
        software: *mut c_void,
        flags: u32,
        p_feature_levels: *const i32,
        feature_levels: u32,
        sdk_version: u32,
        pp_device: *mut *mut c_void,
        p_feature_level: *mut i32,
        pp_immediate_context: *mut *mut c_void,
    ) -> RawHResult;
}

#[link(name = "dxgi", kind = "raw-dylib")]
extern "system" {
    pub fn CreateDXGIFactory1(riid: *const Guid, pp_factory: *mut *mut c_void) -> RawHResult;
}

#[link(name = "d3dcompiler_47", kind = "raw-dylib")]
extern "system" {
    pub fn D3DCompile(
        p_src_data: *const c_void,
        src_data_size: usize,
        p_source_name: *const c_char,
        p_defines: *const c_void,
        p_include: *mut c_void,
        p_entrypoint: *const c_char,
        p_target: *const c_char,
        flags1: u32,
        flags2: u32,
        pp_code: *mut *mut c_void,
        pp_error_msgs: *mut *mut c_void,
    ) -> RawHResult;

    pub fn D3DGetInputSignatureBlob(
        p_src_data: *const c_void,
        src_data_size: usize,
        pp_signature_blob: *mut *mut c_void,
    ) -> RawHResult;
}
