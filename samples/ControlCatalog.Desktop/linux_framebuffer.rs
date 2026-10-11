//! The catalog on the Linux framebuffer platform, as the desktop entry
//! point of the reference starts it with `--fbdev` (and `--scaling`): one
//! full-screen view on the framebuffer device `FRAMEBUFFER` names, or
//! `/dev/fb0`.
//!
//! Additions for runs on a virtual framebuffer:
//!
//! - `--no-input` starts without an input backend (the default reads
//!   every event device of the machine through `libinput`).
//! - `FERROUI_LINUXFB_EXPECT_ID=<prefix>`: the catalog refuses a device
//!   whose identification does not begin with the prefix, before it
//!   writes to it (a virtual device must not be mistaken for a console).
//! - `FERROUI_LINUXFB_DUMPS=<directory>`: the pages of
//!   `FERROUI_SMOKE_SCREENSHOT_PAGES` are shown one after the other
//!   (`FERROUI_LINUXFB_PAGE_MS` milliseconds each; not `FERROUI_SMOKE_PAGES`,
//!   which starts the cycle through every page) and the content of the
//!   framebuffer device is written for each as raw bytes, which
//!   `scripts/linuxfb/fb-to-png.py` turns into pictures; then the
//!   application shuts down.

use ferroui_controls::{AppBuilder, Application};
use ferroui_linuxframebuffer::input::null_input::NullInputBackend;
use ferroui_linuxframebuffer::input::IInputBackend;
use ferroui_linuxframebuffer::output::{FbDevOutputOptions, FbdevOutput, IOutputBackend};
use ferroui_linuxframebuffer::LinuxFramebufferPlatformExtensions;
use std::io::Read;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

fn get_scaling(args: &[String]) -> f64 {
    let idx = args.iter().position(|arg| arg == "--scaling");
    idx.and_then(|idx| args.get(idx + 1)).and_then(|value| value.parse().ok()).unwrap_or(1.0)
}

fn shutdown() {
    let lifetime = Application::current().and_then(|application| application.application_lifetime());
    if let Some(controlled) = lifetime.as_ref().and_then(|lifetime| lifetime.as_controlled_application_lifetime()) {
        controlled.shutdown(0);
    }
}

/// Writes the content of the framebuffer device as raw bytes.
fn dump(output: &FbdevOutput, node: &str, path: &PathBuf) {
    let (var_info, fixed_info) = (output.var_screen_info(), output.fix_screen_info());
    let length = fixed_info.line_length as u64 * var_info.yres as u64;
    let mut content = Vec::new();
    let result = std::fs::File::open(node)
        .and_then(|file| file.take(length).read_to_end(&mut content))
        .and_then(|_| std::fs::write(path, &content));
    match result {
        Ok(()) => println!(
            "Dump: {} {}x{} {} {}",
            path.display(),
            var_info.xres,
            var_info.yres,
            var_info.bits_per_pixel,
            fixed_info.line_length
        ),
        Err(error) => println!("Dump: {} could not be written: {error}", path.display()),
    }
}

/// The run asked for with `FERROUI_LINUXFB_DUMPS`.
fn dump_run(output: Arc<FbdevOutput>, node: String, directory: PathBuf) {
    if let Err(error) = std::fs::create_dir_all(&directory) {
        println!("Dumps: the directory {} could not be created: {error}", directory.display());
        return;
    }
    let pages = std::env::var("FERROUI_SMOKE_SCREENSHOT_PAGES").unwrap_or_else(|_| crate::SCREENSHOT_PAGES.to_string());
    let headers: Vec<String> =
        pages.split(',').map(str::trim).filter(|header| !header.is_empty()).map(str::to_string).collect();
    let interval = Duration::from_millis(crate::environment_milliseconds("FERROUI_LINUXFB_PAGE_MS").unwrap_or(1500));
    println!("Dumps of {headers:?} to {}, {interval:?} a page", directory.display());
    control_catalog::show_pages(
        headers,
        interval,
        move |index, header| {
            let name = crate::screenshot_name(index, header, "").replace(".png", ".raw");
            dump(&output, &node, &directory.join(name));
        },
        || {
            println!("Dumps written: shutting down");
            shutdown();
        },
    );
}

/// Starts the catalog on the framebuffer device; returns the exit code.
pub fn start(builder: AppBuilder, args: &[String]) -> i32 {
    let node = std::env::var("FRAMEBUFFER").unwrap_or_else(|_| "/dev/fb0".to_owned());
    let options = FbDevOutputOptions { scaling: get_scaling(args), ..Default::default() };
    let output = match FbdevOutput::with_options(options) {
        Ok(output) => output,
        Err(error) => {
            eprintln!("ControlCatalog: the framebuffer device {node} cannot be used: {error}");
            return 1;
        }
    };
    if let Ok(expected) = std::env::var("FERROUI_LINUXFB_EXPECT_ID") {
        if !output.id().starts_with(&expected) {
            eprintln!(
                "ControlCatalog: {node} is \"{}\", not a device whose identification begins with \"{expected}\"; nothing was written to it",
                output.id()
            );
            return 1;
        }
    }
    let size = output.pixel_size();
    println!("ControlCatalog: the framebuffer device {node} \"{}\", {}x{}", output.id(), size.width, size.height);

    let input_backend: Option<Rc<dyn IInputBackend>> =
        args.iter().any(|arg| arg == "--no-input").then(|| Rc::new(NullInputBackend::new()) as Rc<dyn IInputBackend>);

    let builder = match std::env::var_os("FERROUI_LINUXFB_DUMPS").filter(|directory| !directory.is_empty()) {
        Some(directory) => {
            let output = output.clone();
            builder.after_setup(move |_| dump_run(output.clone(), node.clone(), PathBuf::from(&directory)))
        }
        None => builder,
    };
    builder.start_linux_direct(args, output, input_backend)
}
