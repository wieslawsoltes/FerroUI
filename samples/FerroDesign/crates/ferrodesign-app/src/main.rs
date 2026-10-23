#![cfg_attr(target_os = "emscripten", no_main)]
#[cfg(target_os = "emscripten")]
use wasm_bindgen::prelude::*;

#[cfg(target_os = "emscripten")]
#[wasm_bindgen(js_name = runMain)]
pub fn run_main(query: &str) -> Result<(), JsValue> {
    use ferroui_browser::{
        BrowserAppBuilder, BrowserPlatformOptions, BrowserRenderer, BrowserRenderingMode,
    };
    let mut options = BrowserPlatformOptions::default();
    options.renderer = BrowserRenderer::Vello;
    options.rendering_mode = vec![
        BrowserRenderingMode::WebGL2,
        BrowserRenderingMode::Software2D,
    ];
    for item in query.trim_start_matches('?').split('&') {
        let Some((key, value)) = item.split_once('=') else {
            continue;
        };
        match key {
            "Renderer" if !value.eq_ignore_ascii_case("Vello") => {
                return Err(JsValue::from_str(
                    "This FerroDesign build uses the FerroUI Vello renderer.",
                ));
            }
            "RenderingMode" => {
                let mode = BrowserRenderingMode::parse(value)
                    .filter(|mode| matches!(mode, BrowserRenderingMode::Software2D | BrowserRenderingMode::WebGL2))
                    .ok_or_else(|| {
                        JsValue::from_str("RenderingMode must be Software2D or WebGL2 for Vello.")
                    })?;
                // An explicit surface request is exact. The default launch keeps a
                // raster fallback for systems where a GPU context is unavailable.
                options.rendering_mode = vec![mode];
            }
            _ => {}
        }
    }
    options.register_ferro_service_worker = false;
    ferrodesign_app::app::build_app().start_browser_app("out", Some(options));
    Ok(())
}
#[cfg(target_os = "emscripten")]
#[wasm_bindgen(js_name = ferroDesignState)]
pub fn editor_state() -> String {
    ferrodesign_app::app::with_shell(|s| s.state_json()).unwrap_or_else(|| "null".into())
}
#[cfg(target_os = "emscripten")]
#[wasm_bindgen(js_name = ferroDesignFrameState)]
pub fn editor_frame_state() -> String {
    ferrodesign_app::app::with_shell(|s| s.frame_state_json()).unwrap_or_else(|| "null".into())
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
