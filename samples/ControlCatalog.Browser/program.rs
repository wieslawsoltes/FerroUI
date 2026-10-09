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
//! `?RenderingMode=WebGL2;Software2D`, and whether the file dialogs use the
//! polyfill: `?PreferFileDialogPolyfill=true`.
//!
//! Addition of the port: built with threads
//! (`scripts/build-browser.sh control-catalog-browser --threads`, served
//! cross-origin isolated, or isolated by the service worker of the site) the
//! catalog is rendered by a render thread: the compositor and Skia run in a
//! worker that owns the canvas. `?RenderThread=false` keeps such a module on
//! the thread of the page. Without threads the option does nothing.
//!
//! Additions of the port: the application registers the embedded Inter font
//! and makes it the default family, because the browser has no system
//! fonts and the Skia build of the port has no default typeface. Trace
//! output goes to the standard output of the module (the console log of the
//! page), as the console trace listener of the managed original writes it.
//!
//! Addition of the port, for the behaviour tests of the site
//! (`scripts/browser/tests/control_catalog.test.mjs`): the export
//! [`catalog_state`] reports what the view shows, so that the tests can find
//! the controls they drive with real pointer and key events and check the
//! effect; [`catalog_rendering`] reports which thread drew the frames;
//! [`catalog_memory`] how much of the memory of the module is in use; and
//! [`catalog_panic_in_frame`] makes the next frame panic.
//! The native control demo of the browser (`EmbedSampleWeb`) is in
//! [`embed_sample_browser`].
//!
//! Addition of the port: the pictures and fonts of the catalog are plain
//! files next to the module, one per asset, listed with the pages that use
//! them; the host page registers the start-up files before it calls
//! [`run_main`], and the catalog asks
//! [`page_assets_browser::BrowserPageAssets`] for the files of a page before
//! it creates the page.

#![cfg_attr(target_os = "emscripten", no_main)]

mod embed_sample_browser;
mod page_assets_browser;

use control_catalog::pages::EmbedSample;
use control_catalog::view_models::MainWindowViewModel;
use control_catalog::{App, MainView, PageAssets};
use embed_sample_browser::EmbedSampleWeb;
use page_assets_browser::BrowserPageAssets;
use ferroui_base::logging::LogEventLevel;
use ferroui_base::media::{FontFamily, FontManager, FontManagerOptions, Typeface};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::rendering::RendererDebugOverlays;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Point, Ref, Visual};
use ferroui_browser::interop::canvas_helper::{RENDER_TARGET_KIND_SOFTWARE, RENDER_TARGET_KIND_WEB_GL};
use ferroui_browser::interop::thread_proxy;
use ferroui_browser::rendering::{BrowserSharedRenderLoop, RenderStatistics, RenderWorker};
use ferroui_browser::{BrowserAppBuilder, BrowserPlatformOptions, BrowserRenderingMode};
use ferroui_controls::{AppBuilder, Application, Button, Image, NavigationPage, TextBlock, TextBox, TopLevel};
use ferroui_fonts_inter::AppBuilderExtension;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wasm_bindgen::prelude::*;

/// `Main(args)`: the entry point the host page calls once the module is
/// created and registered with the script side. `href` is the address of the
/// page (`args[0]` of the managed original).
#[wasm_bindgen(js_name = runMain)]
pub fn run_main(href: &str) {
    let options = parse_args(&[href]).unwrap_or_default();

    build_ferro_app()
        .log_to_text_writer(std::io::stdout(), LogEventLevel::Warning, &[])
        .after_setup(|_| {
            EmbedSample::set_implementation(Some(Rc::new(EmbedSampleWeb)));
            PageAssets::set_implementation(Some(Rc::new(BrowserPageAssets)));
        })
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

/// The top level of the application, once it has started.
fn top_level() -> Option<Ref<TopLevel>> {
    Application::current()?
        .application_lifetime()?
        .as_single_top_level_application_lifetime()?
        .top_level()
}

/// The state of the view as JSON, for the behaviour tests that drive the
/// page (`scripts/browser/tests`): whether the drawer is open, the header of
/// the current page, whether the navigation page of the main view is running
/// a navigation (it ignores another one until it has finished), the focused
/// element (its class, and its text when it is a text box) and every visible
/// text block, text box, button and image with its class, its name, its text
/// (of a text block or a text box), its bounds in the coordinates of the view
/// (CSS pixels of the canvas) and whether the pointer reaches it at its
/// centre (`hit`: no other element covers it there); for a text block or a
/// text box also the family of the typeface the font manager finds for its
/// font family (`font`, `null` when it finds none), and for an image the
/// size of its source (`source`, `null` without one). Not a port.
#[wasm_bindgen(js_name = catalogState)]
pub fn catalog_state() -> String {
    let Some(top_level) = top_level() else { return "null".to_string() };
    let view = top_level.get_visual_descendants().find_map(|visual| visual.cast::<MainView>());
    let page = view
        .as_ref()
        .and_then(|view| from_markup_value::<Rc<MainWindowViewModel>>(&view.data_context()))
        .and_then(|view_model| view_model.current_page_item())
        .map(|item| item.header());
    let navigating = view
        .as_ref()
        .and_then(|view| view.get_visual_descendants().find_map(|visual| visual.cast::<NavigationPage>()))
        .is_some_and(|navigation| navigation.is_navigating());

    let focused = top_level.focus_manager().get_focused_element();
    let focus = match &focused {
        Some(element) => format!(
            "{{\"type\":{},\"text\":{}}}",
            json_string(element.get_type().name()),
            element.cast::<TextBox>().and_then(|text_box| text_box.text()).map_or("null".to_string(), |text| json_string(&text)),
        ),
        None => "null".to_string(),
    };

    let viewport = top_level.bounds();
    let mut elements = Vec::new();
    let font_manager = FontManager::current();
    let font = |family: FontFamily| {
        font_manager
            .try_get_glyph_typeface(&Typeface::new(family))
            .map_or("null".to_string(), |typeface| json_string(typeface.family_name()))
    };
    for visual in top_level.get_visual_descendants() {
        let (text, extra) = if let Some(text_block) = visual.cast::<TextBlock>() {
            (text_block.text(), format!(",\"font\":{}", font(text_block.font_family())))
        } else if let Some(text_box) = visual.cast::<TextBox>() {
            (text_box.text(), format!(",\"font\":{}", font(text_box.font_family())))
        } else if visual.cast::<Button>().is_some() {
            (None, String::new())
        } else if let Some(image) = visual.cast::<Image>() {
            let source = image.source().map(|source| source.size());
            let source = source.map_or("null".to_string(), |size| format!("{{\"width\":{:.1},\"height\":{:.1}}}", size.width, size.height));
            (None, format!(",\"source\":{source}"))
        } else {
            continue;
        };
        let size = visual.bounds();
        if !visual.is_effectively_visible() || size.width <= 0.0 || size.height <= 0.0 {
            continue;
        }
        let Some(origin) = visual.translate_point(Point::new(0.0, 0.0), &top_level) else { continue };
        if origin.x >= viewport.width || origin.y >= viewport.height || origin.x + size.width <= 0.0 || origin.y + size.height <= 0.0 {
            continue;
        }
        // The topmost element at the centre is this one, one inside it, or one it is inside of.
        let centre = Point::new(origin.x + size.width / 2.0, origin.y + size.height / 2.0);
        let hit = top_level.input_hit_test_with(centre, false).is_some_and(|hit| {
            let hit: &Visual = &hit;
            std::ptr::eq(hit, &*visual) || hit.is_visual_ancestor_of(&visual) || visual.is_visual_ancestor_of(hit)
        });
        elements.push(format!(
            "{{\"type\":{},\"name\":{},\"text\":{},\"hit\":{hit},\"x\":{:.1},\"y\":{:.1},\"width\":{:.1},\"height\":{:.1}{extra}}}",
            json_string(visual.get_type().name()),
            visual.name().map_or("null".to_string(), |name| json_string(&name)),
            text.map_or("null".to_string(), |text| json_string(&text)),
            origin.x,
            origin.y,
            size.width,
            size.height,
        ));
    }

    format!(
        "{{\"drawerOpen\":{},\"page\":{},\"navigating\":{navigating},\"focus\":{focus},\"elements\":[{}]}}",
        view.map_or("null".to_string(), |view| view.is_open().to_string()),
        page.map_or("null".to_string(), |page| json_string(&page)),
        elements.join(","),
    )
}

/// Where the frames of the view are rendered, as a line of `name=value`
/// pairs, for the behaviour tests (the fields of `themedViewRendering` of
/// the example `themed_view` of the platform):
///
/// - `frames`: the frames drawn to the canvas so far;
/// - `frame_thread`: the thread that drew the last one, and `page_thread`:
///   the thread of the page, which makes this call (both 0 in a module
///   built without threads); `other_thread`: whether they differ;
/// - `render_thread`: the render thread of the platform, 0 without one, and
///   `on_render_thread`: whether the render loop of the page ticks there;
/// - `kind` (`webgl`, `software`, `none` before the first frame), `gl` (the
///   major version of OpenGL ES: 2 for WebGL 1, 3 for WebGL 2, 0 in
///   software) and `size` (device pixels) of the last frame;
/// - `ticks`: the ticks of the frame loop of the render thread, and
///   `proxied`: the calls those ticks made the main thread of the page
///   serve (`unknown` when they are not counted), with `last_proxied`, the
///   index of the function of the last one in the script of the module (-1
///   for none);
/// - `released`: the canvases of closed views the thread that renders has
///   released, and `panics`: the panics of the render thread.
///
/// Not a port.
#[wasm_bindgen(js_name = catalogRendering)]
pub fn catalog_rendering() -> String {
    let statistics = RenderStatistics::current();
    let page_thread = thread_proxy::current_thread();
    let kind = match statistics.frame_kind {
        RENDER_TARGET_KIND_WEB_GL => "webgl",
        RENDER_TARGET_KIND_SOFTWARE => "software",
        _ => "none",
    };
    format!(
        "frames={};frame_thread={};page_thread={};other_thread={};render_thread={};on_render_thread={};kind={};gl={};size={}x{};ticks={};proxied={};last_proxied={};released={};panics={}",
        statistics.frames,
        statistics.frame_thread,
        page_thread,
        statistics.frames > 0 && statistics.frame_thread != page_thread,
        RenderWorker::thread_id(),
        BrowserSharedRenderLoop::renders_on_render_thread(),
        kind,
        statistics.frame_gl_major_version,
        statistics.frame_width,
        statistics.frame_height,
        statistics.ticks,
        statistics.tick_proxied_calls.map_or_else(|| "unknown".to_string(), |calls| calls.to_string()),
        statistics.last_proxied_function,
        statistics.canvases_released,
        statistics.render_thread_panics,
    )
}

/// Makes the next frame of the view panic, for the behaviour tests: a job
/// that panics is posted to the compositor of the view and runs on the
/// thread that renders, inside a frame. Returns whether the job was posted
/// (`false` before the view has a compositor). Not a port.
///
/// The render loop catches the panic and goes on with its next tick, as
/// upstream's loop does with the exception of a frame. On a render thread
/// the platform reports the panic to the page, which logs it as an error on
/// its console.
#[wasm_bindgen(js_name = catalogPanicInFrame)]
pub fn catalog_panic_in_frame() -> bool {
    let Some(top_level) = top_level() else { return false };
    let Some(visual) = ElementComposition::get_element_visual(&top_level) else { return false };
    visual.compositor().post_server_job(|_| panic!("catalogPanicInFrame: a frame panics on request"), false);
    true
}

/// The largest end of the dynamic memory [`catalog_memory`] has seen.
static MEMORY_PEAK: AtomicUsize = AtomicUsize::new(0);

/// How much of the memory of the module is in use, in bytes, as a line of
/// `name=value` pairs, for the measurement of what a site built with threads
/// needs (its memory is fixed: `FERROUI_BROWSER_THREAD_MEMORY_MB` of
/// `scripts/build-browser.sh`). Not a port.
///
/// - `top`: the end of the dynamic memory now. Everything the module uses
///   lies below it: its data, the stack of the page, and whatever the
///   allocator has taken from the system so far, in use or free again (the
///   stacks of the threads are allocations). The allocator gives nothing
///   back in the middle, so this is what the module needed so far;
/// - `peak`: the largest `top` any call of this function has seen (the host
///   page calls it ten times a second with `?MemoryReport=true`);
/// - `size`: the size of the memory: what a module built with threads was
///   linked with, or what the memory of one built without has grown to.
///
/// All 0 outside a web page.
#[wasm_bindgen(js_name = catalogMemory)]
pub fn catalog_memory() -> String {
    let (top, size) = module_memory();
    let peak = MEMORY_PEAK.fetch_max(top, Ordering::SeqCst).max(top);
    format!("top={top};peak={peak};size={size}")
}

/// The end of the dynamic memory and the size of the memory of the module.
#[cfg(target_os = "emscripten")]
fn module_memory() -> (usize, usize) {
    extern "C" {
        fn sbrk(increment: isize) -> *mut std::ffi::c_void;
        fn emscripten_get_heap_size() -> usize;
    }
    // SAFETY: `sbrk(0)` moves nothing and returns the current end of the
    // dynamic memory; `emscripten_get_heap_size` takes no argument and
    // returns the size of the memory. Neither touches memory.
    unsafe { (sbrk(0) as usize, emscripten_get_heap_size()) }
}

#[cfg(not(target_os = "emscripten"))]
fn module_memory() -> (usize, usize) {
    (0, 0)
}

/// `text` as a JSON string literal.
fn json_string(text: &str) -> String {
    let mut literal = String::with_capacity(text.len() + 2);
    literal.push('"');
    for character in text.chars() {
        match character {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            control if (control as u32) < 0x20 => literal.push_str(&format!("\\u{:04x}", control as u32)),
            other => literal.push(other),
        }
    }
    literal.push('"');
    literal
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

/// `bool.TryParse(value)`: `true` or `false` in any case, with surrounding
/// white space; `None` for anything else.
fn parse_bool(value: &str) -> Option<bool> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("true") {
        Some(true)
    } else if value.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

/// `ParseArgs(args)`: the options of the platform from the query string of
/// the address in `args[0]`; `None` when an option has an invalid value.
fn parse_args(args: &[&str]) -> Option<BrowserPlatformOptions> {
    let query = args.first().and_then(|uri| query_of(uri)).filter(|query| !query.is_empty()).unwrap_or_default();

    let mut options = BrowserPlatformOptions::default();

    if let Some(prefer_dialogs_polyfill) = query_value(query, "PreferFileDialogPolyfill").and_then(|value| parse_bool(&value)) {
        options.prefer_file_dialog_polyfill = prefer_dialogs_polyfill;
    }

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

    // Addition of the port: a module built with threads renders on a render
    // thread unless the page asks for one thread. A value `bool.TryParse`
    // rejects keeps the default, as for the option above.
    if let Some(render_thread) = query_value(query, "RenderThread").and_then(|value| parse_bool(&value)) {
        options.render_thread = render_thread;
    }

    println!("DemoBrowserPlatformOptions.PreferFileDialogPolyfill: {}", if options.prefer_file_dialog_polyfill { "True" } else { "False" });
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

    // Not from upstream: the probe of the behaviour tests is an addition of the port.
    #[test]
    fn json_strings_are_escaped() {
        assert_eq!(r#""a \"b\" \\ c\nd\u0001""#, json_string("a \"b\" \\ c\nd\u{1}"));
    }

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
    fn the_file_dialog_polyfill_is_read_from_the_query_string() {
        assert!(parse_args(&["http://localhost/?PreferFileDialogPolyfill=True"]).unwrap().prefer_file_dialog_polyfill);
        assert!(parse_args(&["http://localhost/?preferfiledialogpolyfill=%20true%20"]).unwrap().prefer_file_dialog_polyfill);
        assert!(!parse_args(&["http://localhost/?PreferFileDialogPolyfill=false"]).unwrap().prefer_file_dialog_polyfill);
        // A value bool.TryParse rejects keeps the default.
        assert!(!parse_args(&["http://localhost/?PreferFileDialogPolyfill=yes"]).unwrap().prefer_file_dialog_polyfill);
    }

    // Not from upstream: the render thread is an addition of the port.
    #[test]
    fn the_render_thread_is_switched_off_from_the_query_string() {
        assert!(parse_args(&["http://localhost/"]).unwrap().render_thread);
        assert!(!parse_args(&["http://localhost/?RenderThread=false"]).unwrap().render_thread);
        assert!(!parse_args(&["http://localhost/?RenderingMode=WebGL2&renderthread=False"]).unwrap().render_thread);
        assert!(parse_args(&["http://localhost/?RenderThread=true"]).unwrap().render_thread);
        assert!(parse_args(&["http://localhost/?RenderThread=no"]).unwrap().render_thread);
    }

    // Not from upstream: the probes of the behaviour tests are additions of the port.
    #[test]
    fn the_probes_answer_outside_a_web_page() {
        assert_eq!("top=0;peak=0;size=0", catalog_memory());
        assert!(catalog_rendering().starts_with("frames="));
        assert!(!catalog_panic_in_frame());
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
