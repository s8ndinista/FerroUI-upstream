use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn visit(root: &Path, path: &Path, output: &mut Vec<(String, PathBuf)>) {
    let mut entries: Vec<_> = fs::read_dir(path)
        .expect("resource directory")
        .map(|e| e.expect("resource entry").path())
        .collect();
    entries.sort();
    for entry in entries {
        if entry.is_dir() {
            visit(root, &entry, output);
        } else if entry
            .extension()
            .is_some_and(|e| e == "json" || e == "code-snippets" || e == "tmLanguage")
        {
            output.push((
                entry
                    .strip_prefix(root)
                    .expect("resource path")
                    .to_string_lossy()
                    .replace('\\', "/"),
                entry,
            ));
        }
    }
}

fn main() {
    println!("cargo:rerun-if-changed=resources");
    println!("cargo:rerun-if-changed=browser");
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let root = manifest.join("resources");
    let mut resources = Vec::new();
    if env::var_os("CARGO_FEATURE_BUNDLED_GRAMMARS").is_some() {
        visit(&root, &root, &mut resources);
    }
    let mut generated = String::from("pub(crate) static RESOURCES: &[(&str, &str)] = &[\n");
    for (name, path) in resources {
        generated.push_str(&format!(
            "({name:?}, include_str!({:?})),\n",
            path.to_string_lossy()
        ));
    }
    generated.push_str("];\n");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").expect("output directory")).join("resources.rs"),
        generated,
    )
    .expect("write resource table");
    if env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        let vendor = fs::read_to_string(manifest.join("browser/vendor/oniguruma.js"))
            .expect("Oniguruma JavaScript");
        let adapter =
            fs::read_to_string(manifest.join("browser/adapter.js")).expect("Oniguruma adapter");
        let javascript = format!("const module = {{exports: {{}}}}; const exports = module.exports;\n{vendor}\n{adapter}");
        let generated = format!("#[wasm_bindgen::prelude::wasm_bindgen(inline_js = {javascript:?})]\nextern \"C\" {{\n#[wasm_bindgen(js_name = ferroOnigurumaReady)] pub fn ready() -> bool;\n#[wasm_bindgen(catch, js_name = ferroInitializeOniguruma)] pub fn initialize(data: &[u8]) -> std::result::Result<js_sys::Promise, wasm_bindgen::JsValue>;\n#[wasm_bindgen(catch, js_name = ferroCompileOniguruma)] pub fn compile(pattern: &str) -> std::result::Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;\n#[wasm_bindgen(catch, js_name = ferroSearchOniguruma)] pub fn search(scanner: &wasm_bindgen::JsValue, line: &str, start: u32) -> std::result::Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;\n#[wasm_bindgen(js_name = ferroDisposeOniguruma)] pub fn dispose(scanner: &wasm_bindgen::JsValue);\n}}\n");
        fs::write(
            PathBuf::from(env::var_os("OUT_DIR").expect("output directory"))
                .join("browser_bindings.rs"),
            generated,
        )
        .expect("write browser bindings");
    }
}
