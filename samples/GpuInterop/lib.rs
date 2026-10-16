//! gpu-interop
//!
//! Port of the sample application `GpuInterop`: drawing with a graphics API of the platform
//! into a drawing surface of the compositor, through the GPU interop of the compositor (the
//! import of images and semaphores made outside of the framework). The main window shows one
//! demo with sliders that turn and colour what it draws:
//!
//! - the Vulkan demo (`vulkan_demo::VulkanDemoControl`): the teapot of the ControlCatalog drawn
//!   with Vulkan (`ash`) into images exported as opaque handles of Vulkan (Linux, Windows),
//!   Direct3D 11 textures (Windows, where the compositor takes those) or IOSurfaces with a
//!   timeline semaphore that is a Metal shared event (macOS, through MoltenVK);
//! - the Direct3D 11 demo (`d3d_demo::D3D11DemoControl`, Windows): a coloured cube drawn with
//!   Direct3D 11 into textures shared by their DXGI handles with keyed mutexes;
//! - the dma-buf demo (`vulkan_demo::VulkanDmaBufDemoControl`, Linux): the teapot exported as
//!   Linux dma-bufs to the EGL compositor.
//!
//! The entry point is `program.rs`. The documents (`App.xaml`, `MainWindow.xaml`,
//! `GpuDemo.xaml`) are converted from the upstream sample by `scripts/convert_catalog_xaml.py`
//! (names only) and compiled by the build of the crate (`build.rs`, docs/porting/xaml.md 9.5.22).

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as gpu_interop;

mod app;
mod demo_type;
mod drawing_surface_demo_base;
mod gpu_demo;
mod main_window;
pub mod markup;
pub mod native_methods;
mod register_types;
pub mod task_support;

#[path = "D3DDemo/mod.rs"]
pub mod d3d_demo;
#[path = "VulkanDemo/mod.rs"]
pub mod vulkan_demo;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use demo_type::DemoType;
pub use drawing_surface_demo_base::{DrawingSurfaceDemoBase, DrawingSurfaceDemoBaseImpl};
pub use gpu_demo::{GpuDemo, IGpuDemo};
pub use main_window::MainWindow;
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
