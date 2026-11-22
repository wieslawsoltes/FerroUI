//! Browser host for the same editor view used by the desktop sample.
//! The host initializes TextMate's real Oniguruma scanner before starting the
//! native FerroUI single-view lifetime. JavaScript only boots the application;
//! layout, text shaping, rendering, editing and popups remain Rust controls.

#![cfg_attr(target_os = "emscripten", no_main)]

use editor_demo::EditorDemo;
use ferroui_base::logging::LogEventLevel;
use ferroui_base::media::fonts::EmbeddedFontCollection;
use ferroui_base::media::{
    FontFallback, FontFamily, FontManager, FontManagerOptions, Typeface, UnicodeRange,
};
use ferroui_base::rendering::composition::transport::CompositionBatch;
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::utilities::UriRef;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Ref, Visual};
use ferroui_browser::rendering::RenderStatistics;
use ferroui_browser::{BrowserAppBuilder, BrowserPlatformOptions, BrowserRenderingMode};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Button, NewApplication,
    TextBlock, TextBox, TopLevel,
};
use ferroui_edit::{FoldingMargin, TextArea};
use ferroui_fonts_inter::AppBuilderExtension;
use serde_json::json;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

ferroui_base::ferro_thread_local! {
    static DARK: Cell<bool> = const { Cell::new(false) };
    static DEMO: RefCell<Option<Rc<EditorDemo>>> = const { RefCell::new(None) };
    static REQUESTED_FRAME: RefCell<Option<Arc<CompositionBatch>>> = const { RefCell::new(None) };
    static HOVER_STARTS: Cell<u32> = const { Cell::new(0) };
    static HOVER_STOPS: Cell<u32> = const { Cell::new(0) };
}

#[repr(C)]
struct BrowserApp {
    base: Application,
}
ferro_class!(BrowserApp:Application);
ferro_impl_classes!(BrowserApp:FerroObjectImpl);

impl NewApplication for BrowserApp {
    fn new_application() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
        })
    }
}

impl ApplicationImpl for BrowserApp {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
        ferroui_themes_fluent::register_types();
        this.styles()
            .add(ferroui_themes_fluent::FluentTheme::new().as_style());
    }

    fn on_framework_initialization_completed(this: &Self) {
        if let Some(lifetime) = this
            .application_lifetime()
            .as_ref()
            .and_then(|lifetime| lifetime.as_single_view_application_lifetime())
        {
            let demo = EditorDemo::new(DARK.with(Cell::get))
                .expect("The editor's XAML and bundled language resources must load");
            demo.editor().set_font_family(
                FontFamily::parse("fonts:EditorCode#Noto Mono")
                    .expect("valid embedded font family"),
            );
            let area = demo.editor().text_area();
            area.add_disposable_handler(TextArea::pointer_hover_event(), |_, _| {
                HOVER_STARTS.with(|count| count.set(count.get() + 1));
            })
            .detach();
            area.add_disposable_handler(TextArea::pointer_hover_stopped_event(), |_, _| {
                HOVER_STOPS.with(|count| count.set(count.get() + 1));
            })
            .detach();
            lifetime.set_main_view(Some(demo.view().upcast()));
            DEMO.with(|current| *current.borrow_mut() = Some(demo));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}

/// Awaited by the page before `runMain`. The scanner binary is embedded by the
/// TextMate crate and its flat module is packaged by the normal site builder.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = initializeScanner)]
pub async fn initialize_scanner() -> Result<(), JsValue> {
    ferroui_edit::textmate::initialize_browser_oniguruma()
        .await
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Starts the real editor once; subsequent calls leave the live document alone.
#[wasm_bindgen(js_name = runMain)]
pub fn run_main(href: &str) {
    if DEMO.with(|demo| demo.borrow().is_some()) {
        return;
    }
    #[cfg(target_arch = "wasm32")]
    assert!(
        ferroui_edit::textmate::browser_oniguruma_ready(),
        "Await initializeScanner before starting the editor"
    );
    DARK.with(|dark| {
        dark.set(query(href, "ThemeVariant").is_some_and(|v| v.eq_ignore_ascii_case("dark")))
    });
    let mut options = BrowserPlatformOptions::default();
    if let Some(mode) = query(href, "RenderingMode").and_then(BrowserRenderingMode::parse) {
        options.rendering_mode = vec![mode];
    }
    options.render_thread =
        !query(href, "RenderThread").is_some_and(|value| value.eq_ignore_ascii_case("false"));
    AppBuilder::configure::<BrowserApp>()
        .with_inter_font()
        .configure_fonts(|manager| {
            ferroui_base::platform::register_assets(
                "EditorDemo.Browser",
                &[
                    (
                        "/Assets/NotoMono-Regular.ttf",
                        include_bytes!("Assets/NotoMono-Regular.ttf"),
                    ),
                    (
                        "/Assets/WenQuanYiMicroHei-Catalog.ttf",
                        include_bytes!(
                            "../ControlCatalog/Assets/FallbackFonts/WenQuanYiMicroHei-Catalog.ttf"
                        ),
                    ),
                    (
                        "/Assets/NotoColorEmoji-Catalog.ttf",
                        include_bytes!(
                            "../ControlCatalog/Assets/FallbackFonts/NotoColorEmoji-Catalog.ttf"
                        ),
                    ),
                ],
            );
            manager.add_font_collection(Rc::new(EmbeddedFontCollection::new(
                UriRef::parse_absolute("fonts:EditorCode").expect("valid collection key"),
                UriRef::parse_absolute("ferres://EditorDemo.Browser/Assets")
                    .expect("valid font source"),
            )));
        })
        .with(Rc::new(FontManagerOptions {
            default_family_name: Some("fonts:Inter#Inter".into()),
            font_fallbacks: Some(vec![
                FontFallback {
                    font_family: FontFamily::new("fonts:EditorCode#WenQuanYi Micro Hei"),
                    unicode_range: UnicodeRange::parse("U+2E80-9FFF, U+F900-FAFF, U+FF00-FFEF")
                        .expect("valid CJK range"),
                },
                FontFallback {
                    font_family: FontFamily::new("fonts:EditorCode#Noto Color Emoji"),
                    unicode_range: UnicodeRange::parse(
                        "U+200D, U+2600-27BF, U+2B00-2BFF, U+FE0F, U+1F000-1FAFF",
                    )
                    .expect("valid emoji range"),
                },
            ]),
            ..Default::default()
        }))
        .log_to_text_writer(std::io::stdout(), LogEventLevel::Warning, &[])
        .start_browser_app("out", Some(options));
}

fn query<'a>(href: &'a str, name: &str) -> Option<&'a str> {
    href.split('#')
        .next()?
        .split_once('?')?
        .1
        .split('&')
        .find_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            key.eq_ignore_ascii_case(name).then_some(value)
        })
}

fn top_level() -> Option<Ref<TopLevel>> {
    Application::current()?
        .application_lifetime()?
        .as_single_top_level_application_lifetime()?
        .top_level()
}

/// Read-only state for browser behavior checks. Coordinates come from native
/// layout, including hit testing against the last committed composition frame.
/// Browser tests change the document exclusively through real input events.
#[wasm_bindgen(js_name = editorState)]
pub fn editor_state() -> String {
    let Some(top) = top_level() else {
        return "null".into();
    };
    let Some(demo) = DEMO.with(|current| current.borrow().clone()) else {
        return "null".into();
    };
    let editor = demo.editor();
    let document = editor.document();
    let installation = editor.textmate_installation();
    let token_count: usize = installation.as_ref().map_or(0, |installation| {
        (1..=document.line_count())
            .map(|line| installation.styled_tokens(line).len())
            .sum()
    });
    let mut elements = Vec::new();
    for visual in top.get_visual_descendants() {
        if visual.cast::<Button>().is_none()
            && visual.cast::<TextArea>().is_none()
            && visual.cast::<FoldingMargin>().is_none()
            && visual.cast::<TextBox>().is_none()
            && visual.cast::<TextBlock>().is_none()
        {
            continue;
        }
        let bounds = visual.bounds();
        if !visual.is_effectively_visible() || bounds.width <= 0.0 || bounds.height <= 0.0 {
            continue;
        }
        let Some(origin) = visual.translate_point(Point::new(0.0, 0.0), &top) else {
            continue;
        };
        let center = Point::new(
            origin.x + bounds.width / 2.0,
            origin.y + bounds.height / 2.0,
        );
        let hit = top.input_hit_test_with(center, false).is_some_and(|hit| {
            let hit: &Visual = &hit;
            std::ptr::eq(hit, &*visual)
                || hit.is_visual_ancestor_of(&visual)
                || visual.is_visual_ancestor_of(hit)
        });
        let text = visual
            .cast::<TextBlock>()
            .and_then(|block| block.text())
            .or_else(|| {
                visual
                    .cast::<TextBox>()
                    .and_then(|text_box| text_box.text())
            })
            .or_else(|| {
                visual
                    .cast::<Button>()
                    .and_then(|button| button.content())
                    .and_then(|value| value.downcast_ref::<String>().cloned())
            });
        let placeholder = visual
            .cast::<TextBox>()
            .and_then(|text_box| text_box.placeholder_text());
        elements.push(json!({
            "type": visual.get_type().name(), "name": visual.name(), "text": text, "hit": hit,
            "placeholder": placeholder,
            "x": origin.x, "y": origin.y, "width": bounds.width, "height": bounds.height,
        }));
    }
    let focused = top.focus_manager().get_focused_element();
    let statistics = RenderStatistics::current();
    let primary = editor.selection().primary();
    let search = editor.installed_search_panel();
    let font_manager = FontManager::current();
    let code_typeface = Typeface::new(editor.font_family());
    let sample_fallback_fonts = ['日', '本', '語', '😀'].map(|character| {
        font_manager
            .try_match_character(
                character as i32,
                code_typeface.style(),
                code_typeface.weight(),
                code_typeface.stretch(),
                Some(code_typeface.font_family()),
                None,
            )
            .and_then(|typeface| font_manager.try_get_glyph_typeface(&typeface))
            .map(|typeface| typeface.family_name().to_owned())
    });
    json!({
        "text": editor.text(), "language": format!("{:?}", demo.language()),
        "font": font_manager.try_get_glyph_typeface(&code_typeface)
            .map(|typeface| typeface.family_name().to_owned()),
        "sampleFallbackFonts": sample_fallback_fonts,
        "caret": editor.caret_offset(), "selection": {"anchor": primary.anchor, "caret": primary.caret},
        "selectedText": editor.selected_text().ok(), "wordWrap": editor.word_wrap(),
        "theme": installation.as_ref().and_then(|installation| installation.theme().name.clone()),
        "grammar": installation.as_ref().map(|installation| installation.grammar().scope_name().to_owned()),
        "grammarError": installation.as_ref().and_then(|installation| installation.last_error()),
        "editorError": editor.last_error(), "tokenCount": token_count,
        "cachedLines": editor.text_area().cached_visual_line_count(),
        "foldedCount": editor.folding_manager().map_or(0, |manager|
            manager.all_foldings().iter().filter(|fold| fold.is_folded()).count()),
        "searchOpen": search.as_ref().is_some_and(|search| search.is_open()),
        "searchPattern": search.as_ref().map(|search| search.search_pattern()),
        "focus": focused.map(|focus| focus.get_type().name().to_owned()),
        "hoverStarts": HOVER_STARTS.with(Cell::get), "hoverStops": HOVER_STOPS.with(Cell::get),
        "scrollY": editor.scroll_viewer().offset().y,
        "frames": statistics.frames, "renderPanics": statistics.render_thread_panics,
        "width": top.bounds().width, "height": top.bounds().height, "elements": elements,
    }).to_string()
}

/// Synchronizes pointer targeting with the compositor's committed hit-test tree.
#[wasm_bindgen(js_name = editorRequestFrame)]
pub fn editor_request_frame() -> bool {
    let Some(top) = top_level() else { return false };
    let Some(visual) = ElementComposition::get_element_visual(&top) else {
        return false;
    };
    REQUESTED_FRAME
        .with(|frame| *frame.borrow_mut() = Some(visual.compositor().request_commit_async()));
    true
}

#[wasm_bindgen(js_name = editorFrameDrawn)]
pub fn editor_frame_drawn() -> bool {
    REQUESTED_FRAME.with(|frame| {
        frame
            .borrow()
            .as_ref()
            .is_some_and(|batch| batch.rendered().is_completed())
    })
}

#[cfg(not(target_os = "emscripten"))]
fn main() {
    println!(
        "Build the editor browser application with scripts/build-browser.sh editor-demo-browser"
    );
}
