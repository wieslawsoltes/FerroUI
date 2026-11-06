#![cfg_attr(target_os = "emscripten", no_main)]

#[cfg(target_os = "emscripten")]
mod storage;

#[cfg(target_os = "emscripten")]
mod host {
    use ferroboard_sample::{App, sample_options::SampleOptions, sample_storage::SampleStorage};
    use ferroui_base::{Ref, media::FontManagerOptions, utilities::CultureInfo};
    use ferroui_browser::{BrowserAppBuilder, BrowserPlatformOptions};
    use ferroui_controls::{AppBuilder, Application};
    use ferroui_fonts_inter::AppBuilderExtension;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;

    fn app() -> Ref<App> {
        Application::current()
            .and_then(|app| app.cast::<App>())
            .expect("FerroBoard browser application is not running")
    }

    #[wasm_bindgen(js_name = runMain)]
    pub fn run_main(options: &str) -> Result<(), JsValue> {
        let locale = ferroboard_core::text::locale_text::system_locale_name()
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        CultureInfo::set_current_formatting(CultureInfo::get_culture_info(&locale));
        SampleOptions::shared().parse(Some(&options.into()));
        AppBuilder::configure::<App>()
            .with_inter_font()
            .with(Rc::new(FontManagerOptions {
                default_family_name: Some("fonts:Inter#Inter".into()),
                ..Default::default()
            }))
            .with(SampleStorage::new(Rc::new(crate::storage::BrowserStorage)))
            .start_browser_app("out", Some(BrowserPlatformOptions::default()));
        Ok(())
    }

    #[wasm_bindgen(js_name = saveSession)]
    pub fn save_session() {
        app().save_session();
    }

    #[wasm_bindgen(js_name = stopSession)]
    pub fn stop_session() {
        let app = app();
        app.release_session();
        if let Some(lifetime) = app.application_lifetime() {
            if let Some(single) = lifetime.as_single_view_application_lifetime() {
                single.set_main_view(None);
            }
            if let Some(browser) = lifetime
                .as_any()
                .downcast_ref::<ferroui_browser::BrowserSingleViewLifetime>()
                && let Some(view) = browser.view()
            {
                view.dispose();
            }
        }
    }
}

#[cfg(not(target_os = "emscripten"))]
fn main() -> std::process::ExitCode {
    eprintln!("FerroBoard browser requires wasm32-unknown-emscripten");
    std::process::ExitCode::FAILURE
}
