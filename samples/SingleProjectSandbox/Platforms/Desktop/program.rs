//! Port of `Platforms/Desktop/Program.cs`: the desktop entry point of the SingleProjectSandbox
//! sample.
//!
//! ```text
//! cargo run -p single-project-sandbox --bin single-project-sandbox
//! FERROUI_SMOKE_SCREENSHOT=/path/sandbox.png FERROUI_SMOKE_EXIT_MS=3000 cargo run -p single-project-sandbox --bin single-project-sandbox
//! ```
//!
//! Addition of the port, for the smoke runs (`sample_support::smoke_run`): with
//! `FERROUI_SMOKE_EXIT_MS=<n>` the main window is closed after `n` milliseconds, and with
//! `FERROUI_SMOKE_SCREENSHOT=<file.png>` a picture of the main window, drawn by the framework, is
//! written first. The process exits with the exit code the lifetime returns.

#[cfg(any(target_os = "android", target_os = "ios"))]
fn main() {
    eprintln!("single-project-sandbox: the desktop entry point of the sample; this target has its own");
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exit_code = build_ferro_app().start_with_classic_desktop_lifetime(&args);
    std::process::ExitCode::from(exit_code as u8)
}

/// The method of the reference that makes the application builder, with the platform
/// detection of the desktop package.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn build_ferro_app() -> ferroui_controls::AppBuilder {
    use ferroui_desktop::AppBuilderDesktopExtensions;
    single_project_sandbox::App::build_ferro_app().use_platform_detect().after_setup(|_| sample_support::smoke_run())
}
