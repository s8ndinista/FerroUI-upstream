#![cfg_attr(target_os = "emscripten", no_main)]
#[cfg(target_os = "emscripten")]
use wasm_bindgen::prelude::*;

#[cfg(target_os = "emscripten")]
#[wasm_bindgen(js_name = runMain)]
pub fn run_main(query: &str) {
    use ferroui_browser::{
        BrowserAppBuilder, BrowserPlatformOptions, BrowserRenderer, BrowserRenderingMode,
    };
    let mut options = BrowserPlatformOptions::default();
    options.renderer = BrowserRenderer::Vello;
    options.rendering_mode = if query.contains("Software2D") {
        vec![BrowserRenderingMode::Software2D]
    } else {
        vec![
            BrowserRenderingMode::WebGL2,
            BrowserRenderingMode::Software2D,
        ]
    };
    options.register_ferro_service_worker = false;
    ferrodesign_app::app::build_app().start_browser_app("out", Some(options));
}
#[cfg(target_os = "emscripten")]
#[wasm_bindgen(js_name = ferroDesignState)]
pub fn editor_state() -> String {
    ferrodesign_app::app::with_shell(|s| s.state_json()).unwrap_or_else(|| "null".into())
}
#[cfg(target_os = "emscripten")]
#[wasm_bindgen(js_name = ferroDesignCommand)]
pub fn editor_command(command: &str) -> bool {
    ferrodesign_app::app::with_shell(|s| s.command(command)).unwrap_or(false)
}
#[cfg(target_os = "emscripten")]
#[wasm_bindgen(js_name = ferroDesignExportJson)]
pub fn export_document() -> String {
    ferrodesign_app::app::with_shell(|s| s.document_json()).unwrap_or_default()
}
#[cfg(target_os = "emscripten")]
#[wasm_bindgen(js_name = ferroDesignImportJson)]
pub fn import_document(text: &str) -> bool {
    ferrodesign_app::app::with_shell(|s| s.import_document(text, "Imported design"))
        .unwrap_or(false)
}
#[cfg(target_os = "emscripten")]
#[wasm_bindgen(js_name = ferroDesignImportFile)]
pub fn import_file(text: &str, name: &str) -> bool {
    ferrodesign_app::app::with_shell(|s| s.import_document(text, name)).unwrap_or(false)
}
#[cfg(target_os = "emscripten")]
#[wasm_bindgen(js_name = ferroDesignHostError)]
pub fn host_error(message: &str) {
    ferrodesign_app::app::with_shell(|s| s.notify(message));
}
#[cfg(not(target_os = "emscripten"))]
fn main() {
    std::process::exit(ferrodesign_app::desktop::run());
}
