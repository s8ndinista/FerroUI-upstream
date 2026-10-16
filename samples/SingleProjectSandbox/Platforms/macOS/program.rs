//! Port of `Platforms/macOS/Program.cs`: the macOS entry point of the SingleProjectSandbox
//! sample, which selects the native platform of macOS and Skia instead of detecting the
//! platform.
//!
//! ```text
//! cargo run -p single-project-sandbox --bin single-project-sandbox-macos
//! ```
//!
//! The reference configures no text shaping system (the native platform of macOS and `UseSkia()`), so its
//! setup fails with "No text shaping system configured"; the port adds HarfBuzz, which the
//! platform detection of the desktop entry point adds too, and the application starts
//! (`GAPS.md`, SPS002).
//!
//! The target framework of the reference for macOS makes an application bundle with
//! `Info.plist` and `Entitlements.plist`; the port builds an executable, which runs as it is. The
//! smoke options of the desktop entry point apply (`sample_support::smoke_run`).

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("single-project-sandbox-macos: the macOS entry point of the sample; it needs macOS");
}

#[cfg(target_os = "macos")]
fn main() -> std::process::ExitCode {
    use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
    use ferroui_native::FerroNativePlatformExtensions;
    use ferroui_skia::SkiaApplicationExtensions;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let exit_code = single_project_sandbox::App::build_ferro_app()
        .use_ferro_native()
        .use_skia()
        .use_harfbuzz()
        .after_setup(|_| sample_support::smoke_run())
        .start_with_classic_desktop_lifetime(&args);
    std::process::ExitCode::from(exit_code as u8)
}
