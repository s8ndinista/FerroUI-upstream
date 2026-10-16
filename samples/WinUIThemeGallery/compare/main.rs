//! The comparison host of the WinUI theme gallery (Windows only): a WinUI 3 window that shows
//! each page of the gallery as native WinUI 3 controls (the page's `.winui.xml`, loaded with
//! `XamlReader.Load`) next to the same page of the framework in the theme under test (in the swap
//! chain panel of `ferroui-winui`), at the same size, theme and flow direction, and writes for
//! each page and setting the two pictures, the picture of their difference and a line with the
//! share of the pixels that differ.
//!
//! ```text
//! cargo run -p winui-theme-gallery --bin winui-theme-compare -- --out target\winui-theme-compare
//! ```
//!
//! What it needs on the machine is what the WinUI samples need: the Windows App Runtime of the
//! release the bindings of `ferroui-winui` are generated from (1.6; another one with
//! `--windows-app-sdk <major.minor>` or `FERROUI_WINDOWSAPPSDK_RELEASE`), and
//! `Microsoft.WindowsAppRuntime.Bootstrap.dll` of that release beside the executable
//! (`scripts/winui/fetch-windows-app-sdk.ps1` puts it there; docs/porting/winui.md, section 7).
//!
//! The output directory is `--out <directory>` (or `FERROUI_WINUI_COMPARE_OUT`; by default
//! `winui-theme-compare` in the working directory). The pages, the settings, the size and the
//! waits are the options of the capture mode with the prefix `FERROUI_WINUI_COMPARE`
//! (`FERROUI_WINUI_COMPARE_PAGES`, `_VARIANTS`, `_DENSITIES`, `_FLOWS`, `_SIZE`, `_SETTLE_MS`,
//! `_TIMEOUT_S`; `winui_theme_gallery::capture`), and `FERROUI_WINUI_COMPARE_THRESHOLD` is the
//! difference of a colour channel up to which two pixels are the same (24 by default). The
//! pictures are at the scale of the window's screen. The process exits with 0 when every
//! comparison was made, and with 1 when a page could not be shown or captured on either side, or
//! when the framework's page had a problem the capture mode reports (a control without a
//! template, an error of a binding): a difference of pixels is a result, not a failure.
//! docs/porting/winui-theme-gallery.md describes the run and the report.

#[cfg(windows)]
mod app;
#[cfg(windows)]
mod compare_window;
#[cfg(windows)]
mod run;
#[cfg(windows)]
mod window_capture;

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    eprintln!(
        "winui-theme-compare: this host shows the pages of the gallery beside native WinUI 3 controls in a WinUI 3 window; it runs on Windows only (the gallery itself is the binary winui-theme-gallery)"
    );
    std::process::ExitCode::FAILURE
}

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    use ferroui_winui::{parse_major_minor, WindowsAppSdk, WindowsAppSdkOptions, XamlApplication};
    use std::path::PathBuf;

    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let value_of = |name: &str| arguments.iter().position(|argument| argument == name).and_then(|index| arguments.get(index + 1).cloned());
    let release = value_of("--windows-app-sdk").or_else(|| std::env::var("FERROUI_WINDOWSAPPSDK_RELEASE").ok());
    let out = value_of("--out")
        .or_else(|| std::env::var("FERROUI_WINUI_COMPARE_OUT").ok().filter(|out| !out.is_empty()))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("winui-theme-compare"));

    let mut options = WindowsAppSdkOptions::default();
    if let Some(release) = release {
        match parse_major_minor(&release) {
            Some(major_minor) => options.major_minor_version = major_minor,
            None => {
                eprintln!("winui-theme-compare: the release {release:?} is not major.minor (1.6)");
                return std::process::ExitCode::FAILURE;
            }
        }
    }

    // MddBootstrapAutoInitializer of the Windows App SDK.
    let sdk = match WindowsAppSdk::initialize(&options) {
        Ok(sdk) => sdk,
        Err(error) => {
            println!(
                "compare: FAILED: Windows App SDK {}.{}: {error}",
                options.major_minor_version >> 16,
                options.major_minor_version & 0xFFFF
            );
            return std::process::ExitCode::FAILURE;
        }
    };
    println!(
        "compare: Windows App SDK {}.{} bootstrapped",
        options.major_minor_version >> 16,
        options.major_minor_version & 0xFFFF
    );

    let started = XamlApplication::start(move |_| app::App::on_launched(out));
    // The window and the view, if the loop ended without closing the window, before the runtime
    // goes.
    app::App::shutdown();
    drop(sdk);
    let code = match started {
        Err(error) => {
            println!("compare: FAILED: Application.Start failed: {error}");
            1
        }
        Ok(()) if run::failed() => 1,
        Ok(()) => 0,
    };
    // The end of the process as the WinUI samples end it: the values of this thread (the
    // framework, the objects of the Windows App SDK it holds) are not destroyed while the system
    // ends the other threads (docs/porting/winui.md, section 7.7).
    ferroui_winui::end_process(code)
}
