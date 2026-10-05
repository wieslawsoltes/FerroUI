//! A window of themed controls: the end-to-end check of the themes.
//!
//! The application adds [`SimpleTheme`] (or, with `--fluent`,
//! [`FluentTheme`]) to its styles and shows a window with a stack panel of
//! a text block, a button, a check box, a text box, a slider, a progress
//! bar and a list box that is too small for its items, so that it shows a
//! scroll bar.
//!
//! ```text
//! cargo run -p ferroui-themes-simple --example themed_window
//! cargo run -p ferroui-themes-simple --example themed_window -- --fluent
//! FERROUI_SMOKE_EXIT_MS=3000 cargo run -p ferroui-themes-simple --example themed_window
//! FERROUI_THEME_VARIANT=dark cargo run -p ferroui-themes-simple --example themed_window
//! ```
//!
//! With `FERROUI_SMOKE_EXIT_MS=<n>` the main window is closed after `n`
//! milliseconds, which ends the main loop; the process exits with the exit
//! code the lifetime returns. `FERROUI_THEME_VARIANT=dark|light` requests a
//! theme variant instead of following the system.

use ferroui_base::styling::{Styles, ThemeVariant};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, Thickness};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Button, CheckBox, Control, ListBox, NewApplication,
    ProgressBar, Slider, StackPanel, TextBlock, TextBox, Window,
};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_themes_fluent::FluentTheme;
use ferroui_themes_simple::SimpleTheme;
use std::rc::Rc;
use std::time::Duration;

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
        this.set_name(Some("FerroUI themed window".to_string()));
        match std::env::var("FERROUI_THEME_VARIANT").ok().as_deref() {
            Some("dark") => this.set_requested_theme_variant(Some(ThemeVariant::dark())),
            Some("light") => this.set_requested_theme_variant(Some(ThemeVariant::light())),
            _ => {}
        }
        if use_fluent_theme() {
            this.styles().add(FluentTheme::new().as_style());
        } else {
            this.styles().add(SimpleTheme::new().upcast::<Styles>());
        }
    }

    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(desktop) =
            lifetime.as_ref().and_then(|lifetime| lifetime.as_classic_desktop_style_application_lifetime())
        {
            desktop.set_main_window(Some(create_main_window()));
        }

        Self::parent_on_framework_initialization_completed(this);
    }
}

/// Whether the Fluent theme was asked for on the command line.
fn use_fluent_theme() -> bool {
    std::env::args().any(|argument| argument == "--fluent")
}

fn text(value: &str) -> Option<BoxedValue> {
    Some(Rc::new(value.to_string()))
}

fn create_main_window() -> Ref<Window> {
    let title = TextBlock::new();
    title.set_text(Some(if use_fluent_theme() { "Fluent theme" } else { "Simple theme" }));
    title.set_font_size(20.0);

    let button = Button::new();
    button.set_content(text("Button"));
    button.click(|_, _| println!("Button clicked"));

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
    panel.children().add(button);
    panel.children().add(check_box);
    panel.children().add(text_box);
    panel.children().add(slider);
    panel.children().add(progress_bar);
    panel.children().add(list_box);

    let window = Window::new();
    window.set_title(Some("FerroUI themed window".to_string()));
    window.set_width(420.0);
    window.set_height(460.0);
    window.set_content(Some(Control::boxed(&panel)));

    window.opened({
        let window = window.clone();
        let panel = panel.clone();
        move || {
            println!("Window opened: theme variant {:?}", window.actual_theme_variant());
            // What the theme gave each control: the smoke test reads these lines.
            println!("Window: template={} background={}", window.template().is_some(), window.background().is_some());
            for child in panel.children().snapshot().iter() {
                let name = child.get_type().name();
                match child.cast::<TemplatedControl>() {
                    Some(templated) => println!(
                        "{name}: template={} template children={} background={} bounds={:?}",
                        templated.template().is_some(),
                        templated.visual_children_count(),
                        templated.background().is_some(),
                        templated.bounds(),
                    ),
                    None => println!("{name}: bounds={:?}", child.bounds()),
                }
            }
        }
    });
    window.closed(|| println!("Window closed"));

    if let Some(ms) = std::env::var("FERROUI_SMOKE_EXIT_MS").ok().and_then(|v| v.parse::<u64>().ok()) {
        println!("Will close the main window after {ms} ms");
        let window = window.clone();
        // The timer stops itself after its only tick.
        let _timer = DispatcherTimer::run_once(
            move || {
                println!("Timer fired: closing the main window");
                window.close();
            },
            Duration::from_millis(ms),
            DispatcherPriority::NORMAL,
        );
    }

    window
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).filter(|argument| argument != "--fluent").collect();
    let exit_code = AppBuilder::configure::<App>().use_platform_detect().start_with_classic_desktop_lifetime(&args);
    println!("start_with_classic_desktop_lifetime returned {exit_code}");
    std::process::ExitCode::from(exit_code as u8)
}
