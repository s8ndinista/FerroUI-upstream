//! Compiles the markup documents of the sample and generates its per-document tests
//! (`sample-build`; docs/porting/xaml.md, 9.5.22), and generates the bindings of the COM
//! interfaces of Direct3D 11 and DXGI the Direct3D demo calls (`D3DDemo/d3d11.idl`, with the
//! MicroCom generator, as the Windows backend generates its own).

use std::path::PathBuf;
use std::{env, fs};

fn main() {
    sample_build::SampleBuild::new("GpuInterop").run();

    // Whether Skia has its Vulkan back end in this build, which the Skia context of
    // `VulkanContext` needs: always on Linux (the manifest of the Skia backend asks for it), on
    // Windows with the feature `vulkan`. Keep in step with the manifest.
    println!("cargo:rustc-check-cfg=cfg(gpu_interop_skia_vulkan)");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "linux" || (target_os == "windows" && env::var("CARGO_FEATURE_VULKAN").is_ok()) {
        println!("cargo:rustc-cfg=gpu_interop_skia_vulkan");
    }

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let idl_path = manifest_dir.join("D3DDemo").join("d3d11.idl");
    println!("cargo:rerun-if-changed={}", idl_path.display());
    let source = fs::read_to_string(&idl_path).unwrap_or_else(|e| panic!("{}: {e}", idl_path.display()));
    let parsed = microcom_codegen::parse(&source).unwrap_or_else(|e| panic!("{}:{e}", idl_path.display()));
    let bindings = microcom_codegen::rust::generate(&parsed, &Default::default())
        .unwrap_or_else(|e| panic!("{}: {e}", idl_path.display()));
    let generated = out_dir.join("d3d11.rs");
    if fs::read_to_string(&generated).ok().as_deref() != Some(bindings.as_str()) {
        fs::write(&generated, bindings).unwrap_or_else(|e| panic!("{}: {e}", generated.display()));
    }
}
