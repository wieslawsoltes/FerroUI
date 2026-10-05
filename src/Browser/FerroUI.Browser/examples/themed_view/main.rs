//! A view of themed controls in a web page: the end-to-end check of the
//! browser platform.
//!
//! The application adds the Fluent theme to its styles and shows, as the
//! main view of the single-view lifetime, a stack panel of a text block, a
//! button, a check box, a text box, a slider, a progress bar and a list box
//! that is too small for its items, so that it shows a scroll bar. It is the
//! page counterpart of the `themed_window` example of the themes.
//!
//! Build and assemble the site with `scripts/build-browser.sh themed_view`
//! and serve `target/browser/themed_view` with any static web server. The
//! query string selects the rendering mode and the theme variant:
//! `?RenderingMode=Software2D`, `?ThemeVariant=dark`.

#![cfg_attr(target_os = "emscripten", no_main)]

use ferroui_base::media::FontManagerOptions;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, Thickness};
use ferroui_browser::{BrowserAppBuilder, BrowserPlatformOptions, BrowserRenderingMode};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Button, CheckBox, Control, ListBox, NewApplication,
    ProgressBar, Slider, StackPanel, TextBlock, TextBox,
};
use ferroui_fonts_inter::AppBuilderExtension;
use ferroui_themes_fluent::FluentTheme;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// The controls of the view whose state the behaviour tests read.
struct Controls {
    button: Ref<Button>,
    check_box: Ref<CheckBox>,
    text_box: Ref<TextBox>,
    slider: Ref<Slider>,
    list_box: Ref<ListBox>,
    clicks: Rc<Cell<u32>>,
}

thread_local! {
    static THEME_VARIANT: RefCell<Option<String>> = const { RefCell::new(None) };
    static CONTROLS: RefCell<Option<Controls>> = const { RefCell::new(None) };
}

#[repr(C)]
pub struct App {
    base: Application,
}

ferro_class!(App: Application);
ferro_impl_classes!(App: FerroObjectImpl);

impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        instantiate(Self { base: Application::construct() })
    }
}

impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
        this.set_name(Some("FerroUI themed view".to_string()));
        match THEME_VARIANT.with(|variant| variant.borrow().clone()).as_deref() {
            Some("dark") => this.set_requested_theme_variant(Some(ThemeVariant::dark())),
            Some("light") => this.set_requested_theme_variant(Some(ThemeVariant::light())),
            _ => {}
        }
        this.styles().add(FluentTheme::new().as_style());
    }

    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(single_view) = lifetime.as_ref().and_then(|lifetime| lifetime.as_single_view_application_lifetime())
        {
            single_view.set_main_view(Some(create_main_view()));
        }

        Self::parent_on_framework_initialization_completed(this);
    }
}

fn text(value: &str) -> Option<BoxedValue> {
    Some(Rc::new(value.to_string()))
}

fn create_main_view() -> Ref<Control> {
    let title = TextBlock::new();
    title.set_text(Some("Fluent theme"));
    title.set_font_size(20.0);

    let button = Button::new();
    button.set_content(text("Button"));
    let clicks = Rc::new(Cell::new(0));
    button.click({
        let clicks = clicks.clone();
        move |_, _| clicks.set(clicks.get() + 1)
    });

    let check_box = CheckBox::new();
    check_box.set_content(text("Check box"));
    check_box.set_is_checked(Some(true));

    let text_box = TextBox::new();
    text_box.set_text(Some("Text box"));

    let slider = Slider::new();
    slider.set_minimum(0.0);
    slider.set_maximum(100.0);
    slider.set_range_value(40.0);

    let progress_bar = ProgressBar::new();
    progress_bar.set_minimum(0.0);
    progress_bar.set_maximum(100.0);
    progress_bar.set_range_value(65.0);
    progress_bar.set_height(16.0);

    let list_box = ListBox::new();
    for item in ["First item", "Second item", "Third item", "Fourth item", "Fifth item", "Sixth item", "Seventh item"] {
        list_box.items().add(text(item));
    }
    list_box.set_selected_index(1);
    // Smaller than its items: the scroll viewer of the list shows its scroll bar.
    list_box.set_height(96.0);

    let panel = StackPanel::new();
    panel.set_spacing(10.0);
    panel.set_margin(Thickness::uniform(16.0));
    panel.children().add(title);
    panel.children().add(button.clone());
    panel.children().add(check_box.clone());
    panel.children().add(text_box.clone());
    panel.children().add(slider.clone());
    panel.children().add(progress_bar);
    panel.children().add(list_box.clone());

    CONTROLS.with(|controls| {
        *controls.borrow_mut() = Some(Controls {
            button: button.clone(),
            check_box: check_box.clone(),
            text_box: text_box.clone(),
            slider: slider.clone(),
            list_box: list_box.clone(),
            clicks,
        })
    });

    panel.upcast()
}

/// The state of the controls as a line of `name=value` pairs, for the
/// behaviour tests that drive the page (`scripts/browser/tests`).
#[wasm_bindgen(js_name = themedViewState)]
pub fn themed_view_state() -> String {
    CONTROLS.with(|controls| {
        let controls = controls.borrow();
        let Some(c) = controls.as_ref() else { return String::new() };
        let focus = if c.text_box.is_focused() {
            "text_box"
        } else if c.button.is_focused() {
            "button"
        } else if c.check_box.is_focused() {
            "check_box"
        } else if c.slider.is_focused() {
            "slider"
        } else if c.list_box.is_focused() {
            "list_box"
        } else {
            "other"
        };
        format!(
            "clicks={};checked={:?};text={};caret={};slider={};selected={};focus={};button_over={}",
            c.clicks.get(),
            c.check_box.is_checked(),
            c.text_box.text().unwrap_or_default(),
            c.text_box.caret_index(),
            c.slider.value(),
            c.list_box.selected_index(),
            focus,
            c.button.is_pointer_over(),
        )
    })
}

/// The value of `name` in a query string (`?a=1&b=2`), ignoring the case of
/// the name.
fn query_value(query: &str, name: &str) -> Option<String> {
    query
        .trim_start_matches('?')
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.to_string())
}

fn parse_args(query: &str) -> BrowserPlatformOptions {
    let mut options = BrowserPlatformOptions::default();
    if let Some(rendering_mode_pairs) = query_value(query, "RenderingMode") {
        let modes: Vec<BrowserRenderingMode> = rendering_mode_pairs
            .split(|c| c == ';' || c == ',')
            .filter(|entry| !entry.is_empty())
            .filter_map(BrowserRenderingMode::parse)
            .collect();
        if !modes.is_empty() {
            options.rendering_mode = modes;
        }
    }
    options
}

/// The entry point the host page calls once the module is created and
/// registered with the script side. `query` is the query string of the page.
#[wasm_bindgen(js_name = runMain)]
pub fn run_main(query: &str) {
    let options = parse_args(query);
    THEME_VARIANT.with(|variant| *variant.borrow_mut() = query_value(query, "ThemeVariant"));

    AppBuilder::configure::<App>()
        .with_inter_font()
        .with(Rc::new(FontManagerOptions {
            default_family_name: Some("fonts:Inter#Inter".to_string()),
            ..Default::default()
        }))
        .start_browser_app("out", Some(options));
}

/// The example only does something in a web page; elsewhere it builds so
/// that the workspace checks cover it.
#[cfg(not(target_os = "emscripten"))]
fn main() {
    println!("themed_view runs in a browser: build it with scripts/build-browser.sh themed_view");
}
