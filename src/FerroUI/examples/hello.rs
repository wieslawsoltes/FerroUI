//! A minimal application written against the facade alone: an application
//! class with the Fluent theme whose main window holds a text block and a
//! button, started through the application builder with the classic desktop
//! lifetime. Everything it names comes from `ferroui::prelude`, except the
//! timer of the run without a person, which comes from `ferroui::base`.
//!
//! ```text
//! cargo run -p ferroui --example hello
//! FERROUI_SMOKE_EXIT_MS=2000 cargo run -p ferroui --example hello
//! ```
//!
//! With `FERROUI_SMOKE_EXIT_MS=<n>` the main window is closed after `n`
//! milliseconds, which ends the main loop; the process exits with the exit
//! code the lifetime returns.

use ferroui::base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui::prelude::*;
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
        this.set_name(Some("FerroUI hello".to_string()));
        this.styles().add(FluentTheme::new().as_style());
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

fn create_main_window() -> Ref<Window> {
    let text = TextBlock::new();
    text.set_text(Some("Hello from FerroUI"));
    text.set_font_size(24.0);

    let clicks = Rc::new(std::cell::Cell::new(0u32));
    let button = Button::new();
    button.set_content(Some(Rc::new("Click me".to_string())));
    button.click({
        let text = text.clone();
        move |_, _| {
            clicks.set(clicks.get() + 1);
            println!("Button clicked");
            text.set_text(Some(format!("Clicked {} times", clicks.get()).as_str()));
        }
    });

    let panel = StackPanel::new();
    panel.set_spacing(12.0);
    panel.set_horizontal_alignment(HorizontalAlignment::Center);
    panel.set_vertical_alignment(VerticalAlignment::Center);
    panel.children().add(text);
    panel.children().add(button);

    let window = Window::new();
    window.set_title(Some("FerroUI hello".to_string()));
    window.set_width(480.0);
    window.set_height(320.0);
    window.set_padding(Thickness::uniform(16.0));
    window.set_content(Some(Control::boxed(panel)));

    window.opened(|| println!("Window opened"));
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
        )
        .detach();
    }

    window
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exit_code = AppBuilder::configure::<App>().use_platform_detect().start_with_classic_desktop_lifetime(&args);
    println!("start_with_classic_desktop_lifetime returned {exit_code}");
    std::process::ExitCode::from(exit_code as u8)
}
