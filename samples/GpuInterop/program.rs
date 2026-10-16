//! Port of `Program.cs`: the desktop entry point of the GpuInterop sample.
//!
//! ```text
//! cargo run -p gpu-interop                 # the Vulkan demo
//! cargo run -p gpu-interop -- --d3d        # the Direct3D 11 demo (Windows)
//! cargo run -p gpu-interop -- --dmabuf     # the dma-buf demo (Linux, the EGL compositor of X11)
//! cargo run -p gpu-interop -- --smoke      # the smoke run of the demo, see smoke.rs
//! ```
//!
//! On Windows the Vulkan demo needs the Vulkan rendering mode of the backend: build with the
//! feature `vulkan` (`cargo run -p gpu-interop --features vulkan`), with which Skia is compiled
//! from source (docs/porting/vulkan.md, section 2). On macOS the Vulkan demo draws through
//! MoltenVK and the loader of the Vulkan SDK (or of Homebrew).
//!
//! With `FERROUI_SMOKE_EXIT_MS=<n>` the main window is closed after `n` milliseconds, which
//! ends the main loop. On macOS `FERROUI_SMOKE_RENDERING=metal|opengl` asks for that rendering
//! mode of the compositor alone (the smoke options of the desktop entry point of the
//! ControlCatalog; `opengl` needs the Skia backend built with Ganesh on OpenGL,
//! docs/porting/apple-opengl.md).
//!
//! Not ported, because the tools it names do not exist in the port: the developer tools of a
//! debug build (GAPS.md, G001).

mod smoke;

use ferroui_base::logging::LogEventLevel;
use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_vulkan::{VulkanInstanceCreationOptions, VulkanOptions};
use gpu_interop::{App, DemoType};
use std::rc::Rc;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |arg: &str| args.iter().any(|a| a == arg);
    let demo_type = if cfg!(windows) && has("--d3d") {
        DemoType::D3D11
    } else if has("--dmabuf") {
        DemoType::VulkanDmaBuf
    } else {
        DemoType::Vulkan
    };
    let smoke = has("--smoke");
    let mut builder = build_ferro_app_core(demo_type);
    if smoke {
        builder = smoke::configure(builder, demo_type);
    }
    let exit_code = builder.start_with_classic_desktop_lifetime(&args);
    if smoke {
        return smoke::exit_code(exit_code);
    }
    std::process::ExitCode::from(exit_code as u8)
}

/// The application builder of the Vulkan demo: the method the designer of the upstream project
/// calls.
#[allow(dead_code)]
pub fn build_ferro_app() -> AppBuilder {
    build_ferro_app_core(DemoType::Vulkan)
}

fn build_ferro_app_core(demo_type: DemoType) -> AppBuilder {
    let builder = AppBuilder::configure_with(move || {
        let app = App::new();
        app.set_demo_type(demo_type);
        app
    })
    .use_platform_detect()
    .after_setup(|_| sample_support::smoke_run());
    let builder = with_platform_options(builder, demo_type);
    builder
        .with(Rc::new(VulkanOptions {
            vulkan_instance_creation_options: VulkanInstanceCreationOptions { use_debug: true, ..Default::default() },
            ..Default::default()
        }))
        .log_to_trace(LogEventLevel::Debug, &["Vulkan"])
}

/// `Win32PlatformOptions` of `Program.cs`: the D3D11 demo draws with ANGLE, the others with
/// Vulkan.
#[cfg(windows)]
fn with_platform_options(builder: AppBuilder, demo_type: DemoType) -> AppBuilder {
    use ferroui_win32::{Win32PlatformOptions, Win32RenderingMode};

    builder.with(Rc::new(Win32PlatformOptions {
        rendering_mode: vec![if demo_type == DemoType::D3D11 { Win32RenderingMode::AngleEgl } else { Win32RenderingMode::Vulkan }],
        ..Default::default()
    }))
}

/// `X11PlatformOptions` of `Program.cs`.
#[cfg(target_os = "linux")]
fn with_platform_options(builder: AppBuilder, demo_type: DemoType) -> AppBuilder {
    use ferroui_x11::{X11PlatformOptions, X11RenderingMode};

    let mut options = X11PlatformOptions::new();
    // The dma-buf demo imports its rendered frames through the EGL/OpenGL
    // compositor (EglExternalObjectsFeature), so it requires an EGL backend.
    options.rendering_mode =
        vec![if demo_type == DemoType::VulkanDmaBuf { X11RenderingMode::Egl } else { X11RenderingMode::Vulkan }];
    builder.with(Rc::new(options))
}

/// Upstream names no options of the macOS platform: the rendering mode of a smoke run is the
/// only one (see `smoke.rs`).
#[cfg(not(any(windows, target_os = "linux")))]
fn with_platform_options(builder: AppBuilder, _demo_type: DemoType) -> AppBuilder {
    builder
}
