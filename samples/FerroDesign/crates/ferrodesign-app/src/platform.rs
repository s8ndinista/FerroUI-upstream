//! Host services only. The editor UI and artwork are rendered by FerroUI.
#[cfg(target_os = "emscripten")]
mod browser {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen(inline_js = r#"
export function ferroLoad() { try { return localStorage.getItem('ferrodesign.document.v1') || ''; } catch { return ''; } }
export function ferroSave(text) { try { localStorage.setItem('ferrodesign.document.v1', text); return true; } catch { return false; } }
export function ferroDownload(name, text, mime) {
  const url = URL.createObjectURL(new Blob([text], {type:mime}));
  const a = document.createElement('a'); a.href=url; a.download=name; a.click();
  setTimeout(()=>URL.revokeObjectURL(url),10000);
}
export function ferroCopy(text) { navigator.clipboard?.writeText(text).catch(()=>{}); }
export function ferroPick() {
  const picker=document.createElement('input'); picker.type='file'; picker.accept='.ferrodesign,.json';
  picker.onchange=async()=>{const f=picker.files?.[0]; if(!f)return;
    const api=globalThis.ferroDesign; const text=await f.text();
    if(api?.ferroDesignImportJson) api.ferroDesignImportJson(text);
    else if(api?.importDocument) api.importDocument(text,f.name);
  }; picker.click();
}
export function ferroBytes(name, bytes, mime) {
  const url=URL.createObjectURL(new Blob([bytes],{type:mime}));
  const a=document.createElement('a');a.href=url;a.download=name;a.click();
  setTimeout(()=>URL.revokeObjectURL(url),10000);
}
"#)]
    extern "C" {
        #[wasm_bindgen(js_name=ferroLoad)]
        pub fn load() -> String;
        #[wasm_bindgen(js_name=ferroSave)]
        pub fn save(text: &str) -> bool;
        #[wasm_bindgen(js_name=ferroDownload)]
        pub fn download(name: &str, text: &str, mime: &str);
        #[wasm_bindgen(js_name=ferroCopy)]
        pub fn copy(text: &str);
        #[wasm_bindgen(js_name=ferroPick)]
        pub fn pick();
        #[wasm_bindgen(js_name=ferroBytes)]
        pub fn bytes(name: &str, bytes: &[u8], mime: &str);
    }
}
pub fn load_document() -> String {
    #[cfg(target_os = "emscripten")]
    {
        browser::load()
    }
    #[cfg(not(target_os = "emscripten"))]
    {
        std::fs::read_to_string("ferrodesign-autosave.json").unwrap_or_default()
    }
}
pub fn save_document(text: &str) -> bool {
    #[cfg(target_os = "emscripten")]
    {
        browser::save(text)
    }
    #[cfg(not(target_os = "emscripten"))]
    {
        std::fs::write("ferrodesign-autosave.json", text).is_ok()
    }
}
pub fn download(name: &str, text: &str, mime: &str) {
    #[cfg(target_os = "emscripten")]
    browser::download(name, text, mime);
    #[cfg(not(target_os = "emscripten"))]
    {
        let _ = mime;
        if let Err(e) = std::fs::write(name, text) {
            eprintln!("Cannot save {name}: {e}");
        }
    }
}
pub fn copy(text: &str) {
    #[cfg(target_os = "emscripten")]
    browser::copy(text);
    #[cfg(not(target_os = "emscripten"))]
    {
        use std::io::Write;
        let commands: &[(&str, &[&str])] = if cfg!(target_os = "macos") {
            &[("pbcopy", &[])]
        } else if cfg!(target_os = "windows") {
            &[("clip", &[])]
        } else {
            &[("wl-copy", &[]), ("xclip", &["-selection", "clipboard"])]
        };
        for (cmd, args) in commands {
            if let Ok(mut child) = std::process::Command::new(cmd)
                .args(*args)
                .stdin(std::process::Stdio::piped())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                if child.wait().is_ok_and(|s| s.success()) {
                    break;
                }
            }
        }
    }
}
pub fn pick_file() {
    #[cfg(target_os = "emscripten")]
    browser::pick();
    #[cfg(not(target_os = "emscripten"))]
    eprintln!("Use the Import JSON dialog to open a design on desktop.");
}
pub fn download_bytes(name: &str, bytes: &[u8], mime: &str) -> bool {
    #[cfg(target_os = "emscripten")]
    {
        browser::bytes(name, bytes, mime);
        true
    }
    #[cfg(not(target_os = "emscripten"))]
    {
        let _ = mime;
        std::fs::write(name, bytes).is_ok()
    }
}
