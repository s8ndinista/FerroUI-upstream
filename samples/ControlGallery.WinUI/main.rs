//! The ControlCatalog in a WinUI 3 window: the sample of the reference that
//! hosts the catalog's main view in the swap chain panel of the WinUI
//! integration (`ControlGallery.WinUI`).
//!
//! ```text
//! cargo run -p control-gallery-winui
//! cargo run -p control-gallery-winui -- --smoke
//! ```
//!
//! The reference's sample is a WinUI application in C# and XAML (`App`,
//! `MainWindow`, a generated `Main`); here `app.rs` and `main_window.rs` are
//! those two classes, and this file is the generated `Main` with the
//! bootstrap of the Windows App SDK that the SDK's build tasks generate for
//! an unpackaged application.
//!
//! What it needs on the machine: the Windows App Runtime of the release the
//! integration's bindings are generated from (1.6; another one with
//! `--windows-app-sdk <major.minor>` or `FERROUI_WINDOWSAPPSDK_RELEASE`), and
//! `Microsoft.WindowsAppRuntime.Bootstrap.dll` of that release beside the
//! executable (docs/porting/winui.md, section 7; `scripts/winui/fetch-windows-app-sdk.ps1`
//! puts it there).
//!
//! `--smoke` runs the checks of section 7.2 of that document, prints each
//! (`[ ok ]`, `[FAIL]`, `[info]`), closes the window and exits with 0 only if
//! every check passed.

#[cfg(windows)]
mod app;
#[cfg(windows)]
mod main_window;
#[cfg(windows)]
mod smoke;

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    eprintln!("control-gallery-winui: this sample hosts the catalog in a WinUI 3 window; it runs on Windows only");
    std::process::ExitCode::FAILURE
}

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    use ferroui_winui::{parse_major_minor, WindowsAppSdk, WindowsAppSdkOptions, XamlApplication};

    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let smoke = arguments.iter().any(|argument| argument == "--smoke");
    let release = arguments
        .iter()
        .position(|argument| argument == "--windows-app-sdk")
        .and_then(|index| arguments.get(index + 1).cloned())
        .or_else(|| std::env::var("FERROUI_WINDOWSAPPSDK_RELEASE").ok());

    let mut options = WindowsAppSdkOptions::default();
    if let Some(release) = release {
        match parse_major_minor(&release) {
            Some(major_minor) => options.major_minor_version = major_minor,
            None => {
                eprintln!("control-gallery-winui: the release {release:?} is not major.minor (1.6)");
                return std::process::ExitCode::FAILURE;
            }
        }
    }

    // MddBootstrapAutoInitializer of the Windows App SDK.
    let sdk = match WindowsAppSdk::initialize(&options) {
        Ok(sdk) => sdk,
        Err(error) => {
            smoke::report(false, &format!("Windows App SDK {}.{}: {error}", options.major_minor_version >> 16, options.major_minor_version & 0xFFFF));
            return std::process::ExitCode::FAILURE;
        }
    };
    smoke::report(
        true,
        &format!("Windows App SDK {}.{}: bootstrapped", options.major_minor_version >> 16, options.major_minor_version & 0xFFFF),
    );

    // Application.Start(p => new App()) of the generated Main.
    let started = XamlApplication::start(move |_| app::App::on_launched(smoke));
    drop(sdk);
    if let Err(error) = started {
        smoke::report(false, &format!("Application.Start failed: {error}"));
        return std::process::ExitCode::FAILURE;
    }

    if smoke && !smoke::passed() {
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}
