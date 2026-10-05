//! Port of `Program.cs` of `ControlCatalog.Browser`: the browser entry point
//! of the ControlCatalog sample.
//!
//! The host page (`wwwroot/main.js`) creates the WebAssembly module, hands it
//! to the script side of the platform and calls [`run_main`] with the
//! address of the page; the application starts in the element `out` of the
//! page and the function returns, leaving the page to drive it. Build and
//! assemble the site with `scripts/build-browser.sh control-catalog-browser`
//! and serve `target/browser/control-catalog-browser` with any static web
//! server.
//!
//! The query string selects the rendering modes, in order of preference and
//! separated by `;`: `?RenderingMode=Software2D`,
//! `?RenderingMode=WebGL2;Software2D`.
//!
//! Additions of the port: the application registers the embedded Inter font
//! and makes it the default family, because the browser has no system
//! fonts and the Skia build of the port has no default typeface. Trace
//! output goes to the standard output of the module (the console log of the
//! page), as the console trace listener of the managed original writes it.
//!
//! Not ported, because the browser backend does not have them yet: the
//! option `PreferFileDialogPolyfill` of the query string (the storage
//! provider of the backend, `BrowserPlatformOptions.PreferFileDialogPolyfill`)
//! and the native control demo `EmbedSampleWeb` of `EmbedSample.Browser.cs`
//! with its script `wwwroot/embed.js` (the native control host of the
//! backend, `BrowserNativeControlHost`; until it exists the `EmbedSample`
//! of the catalog shows the default control of the platform).

#![cfg_attr(target_os = "emscripten", no_main)]

use control_catalog::App;
use ferroui_base::logging::LogEventLevel;
use ferroui_base::media::FontManagerOptions;
use ferroui_base::rendering::RendererDebugOverlays;
use ferroui_base::threading::Dispatcher;
use ferroui_browser::{BrowserAppBuilder, BrowserPlatformOptions, BrowserRenderingMode};
use ferroui_controls::{AppBuilder, Application};
use ferroui_fonts_inter::AppBuilderExtension;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// `Main(args)`: the entry point the host page calls once the module is
/// created and registered with the script side. `href` is the address of the
/// page (`args[0]` of the managed original).
#[wasm_bindgen(js_name = runMain)]
pub fn run_main(href: &str) {
    let options = parse_args(&[href]).unwrap_or_default();

    build_ferro_app()
        .log_to_text_writer(std::io::stdout(), LogEventLevel::Warning, &[])
        .start_browser_app("out", Some(options));

    let _ = Dispatcher::ui_thread().invoke_local(|| {
        let lifetime = Application::current().and_then(|application| application.application_lifetime());
        if let Some(top_level) = lifetime
            .as_ref()
            .and_then(|lifetime| lifetime.as_single_top_level_application_lifetime())
            .and_then(|lifetime| lifetime.top_level())
        {
            top_level.renderer_diagnostics().set_debug_overlays(RendererDebugOverlays::FPS);
        }
    });
}

/// The application builder of the catalog.
pub fn build_ferro_app() -> AppBuilder {
    AppBuilder::configure::<App>().with_inter_font().with(Rc::new(FontManagerOptions {
        default_family_name: Some("fonts:Inter#Inter".to_string()),
        ..Default::default()
    }))
}

/// The query string of an absolute address: what follows the first `?`, up
/// to the fragment.
fn query_of(uri: &str) -> Option<&str> {
    let uri = uri.split('#').next().unwrap_or_default();
    // An absolute address has a scheme.
    uri.find(':')?;
    uri.split_once('?').map(|(_, query)| query)
}

/// The value of `name` in a query string (`a=1&b=2`), ignoring the case of
/// the name, with `+` and percent escapes decoded.
fn query_value(query: &str, name: &str) -> Option<String> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| decode_query_component(value))
}

/// Decodes `+` and `%XX` escapes of a query component; an invalid escape is
/// kept as it is.
fn decode_query_component(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => decoded.push(b' '),
            b'%' => match bytes.get(i + 1..i + 3).filter(|hex| hex.iter().all(u8::is_ascii_hexdigit)) {
                Some(hex) => {
                    decoded.push(hex.iter().fold(0, |byte, digit| byte * 16 + (*digit as char).to_digit(16).unwrap_or(0) as u8));
                    i += 2;
                }
                None => decoded.push(b'%'),
            },
            byte => decoded.push(byte),
        }
        i += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// `ParseArgs(args)`: the options of the platform from the query string of
/// the address in `args[0]`; `None` when an option has an invalid value.
fn parse_args(args: &[&str]) -> Option<BrowserPlatformOptions> {
    let query = args.first().and_then(|uri| query_of(uri)).filter(|query| !query.is_empty()).unwrap_or_default();

    let mut options = BrowserPlatformOptions::default();

    if let Some(rendering_mode_pairs) = query_value(query, "RenderingMode") {
        let mut modes = Vec::new();
        for entry in rendering_mode_pairs.split(';').filter(|entry| !entry.is_empty()) {
            match BrowserRenderingMode::parse(entry) {
                Some(mode) => modes.push(mode),
                None => {
                    println!(
                        "ParseArgs of DemoBrowserPlatformOptions failed: \
                         Requested value '{entry}' was not found in BrowserRenderingMode."
                    );
                    return None;
                }
            }
        }
        options.rendering_mode = modes;
    }

    let rendering_mode: Vec<&str> = options.rendering_mode.iter().map(|mode| mode.name()).collect();
    println!("DemoBrowserPlatformOptions.RenderingMode: {}", rendering_mode.join(";"));
    Some(options)
}

/// The catalog only does something in a web page; elsewhere it builds so
/// that the workspace checks cover it.
#[cfg(not(target_os = "emscripten"))]
fn main() {
    println!("control-catalog-browser runs in a browser: build it with scripts/build-browser.sh control-catalog-browser");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendering_modes_are_read_from_the_query_string_in_order() {
        let options = parse_args(&["http://localhost:8080/?RenderingMode=Software2D;WebGL2"]).unwrap();

        assert_eq!(vec![BrowserRenderingMode::Software2D, BrowserRenderingMode::WebGL2], options.rendering_mode);
    }

    #[test]
    fn the_name_of_the_option_and_of_the_mode_ignore_case() {
        let options = parse_args(&["http://localhost/index.html?renderingmode=software2d"]).unwrap();

        assert_eq!(vec![BrowserRenderingMode::Software2D], options.rendering_mode);
    }

    #[test]
    fn an_address_without_options_gives_the_default_options() {
        assert_eq!(Some(BrowserPlatformOptions::default()), parse_args(&["http://localhost/"]));
        assert_eq!(Some(BrowserPlatformOptions::default()), parse_args(&["http://localhost/?"]));
        assert_eq!(Some(BrowserPlatformOptions::default()), parse_args(&[]));
    }

    #[test]
    fn a_relative_address_is_not_read() {
        assert_eq!(Some(BrowserPlatformOptions::default()), parse_args(&["index.html?RenderingMode=Software2D"]));
    }

    #[test]
    fn an_unknown_mode_fails_the_parse() {
        assert_eq!(None, parse_args(&["http://localhost/?RenderingMode=WebGL2;WebGPU"]));
    }

    #[test]
    fn empty_entries_are_skipped_and_escapes_decoded() {
        let options = parse_args(&["http://localhost/?RenderingMode=%3BWebGL1%3B%3BSoftware2D;#top"]).unwrap();

        assert_eq!(vec![BrowserRenderingMode::WebGL1, BrowserRenderingMode::Software2D], options.rendering_mode);
    }
}
