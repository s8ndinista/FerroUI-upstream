//! Build script of `ferroui-winui`: runs the MicroCom generator over the
//! interface definition file of the crate (`interop/winui.idl`) and writes
//! the bindings to `$OUT_DIR/winui.rs`, which `interop/mod.rs` includes. The
//! reference has no such file: it calls the Windows Runtime through the
//! projections of its runtime (docs/porting/winui.md, section 3).

use std::path::PathBuf;
use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let idl_path = manifest_dir.join("interop").join("winui.idl");
    println!("cargo:rerun-if-changed={}", idl_path.display());
    let source = fs::read_to_string(&idl_path).unwrap_or_else(|e| panic!("{}: {e}", idl_path.display()));
    let parsed = microcom_codegen::parse(&source).unwrap_or_else(|e| panic!("{}:{e}", idl_path.display()));
    let bindings = microcom_codegen::rust::generate(&parsed, &Default::default())
        .unwrap_or_else(|e| panic!("{}: {e}", idl_path.display()));
    let generated = out_dir.join("winui.rs");
    if fs::read_to_string(&generated).is_ok_and(|existing| existing == bindings) {
        return;
    }
    fs::write(&generated, bindings).unwrap_or_else(|e| panic!("{}: {e}", generated.display()));
}
