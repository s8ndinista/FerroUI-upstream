//! Compiles the markup documents of the sample and generates its per-document tests
//! (`sample-build`; docs/porting/xaml.md, 9.5.22), and links the executable with its application
//! manifest (`app.manifest`, the one of the reference's sample: Windows 10 and later, per-monitor
//! DPI awareness). The linker of the MSVC tools embeds a manifest it is given; no resource
//! compiler is needed.

use std::env;
use std::path::PathBuf;

fn main() {
    sample_build::SampleBuild::new("WinUIEmbedSample").run();

    let windows_msvc = env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if windows_msvc {
        let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("app.manifest");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}", manifest.display());
    }
}
