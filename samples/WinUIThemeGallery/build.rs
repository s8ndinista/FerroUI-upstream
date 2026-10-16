//! Compiles the markup documents of the gallery and generates its per-document tests
//! (`sample-build`; docs/porting/xaml.md, 9.5.22), and links the comparison host of Windows with
//! its application manifest (`app.manifest`: Windows 10 and later, per-monitor DPI awareness, as
//! the WinUI samples have it). The linker of the MSVC tools embeds a manifest it is given; no
//! resource compiler is needed.
//!
//! The native pages (`Pages/*.winui.xml`) are not markup of the framework: they are WinUI XAML,
//! which the comparison host loads at run time, and their extension keeps them out of the build
//! of the documents.

use std::env;
use std::path::PathBuf;

fn main() {
    sample_build::SampleBuild::new("WinUIThemeGallery").run();

    let windows_msvc = env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if windows_msvc {
        let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("app.manifest");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg-bin=winui-theme-compare=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-bin=winui-theme-compare=/MANIFESTINPUT:{}", manifest.display());
    }
}
