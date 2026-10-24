//! hello-world-component-host
//!
//! The HelloWorld sample as a XAML application whose window shows a component
//! (docs/declarative-ui.md, section 9.1): `App.xaml` (the application, with `<WinUITheme />`) and
//! `MainWindow.xaml`, whose content is `<view:ComponentHost Component="Hello" />`, compiled by the
//! build of the crate (`build.rs`); the component `Hello` (the text and a button that counts its
//! clicks, the count a signal) is written with `rsx!` and registered by [`register_types`].

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as hello_world_component_host;

mod app;
mod main_window;
pub mod markup;
mod register_types;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_view::prelude::*;

pub use app::App;
pub use main_window::MainWindow;
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};

/// The component the window shows, registered under the name `Hello`.
#[component]
pub fn Hello() -> Element {
    let clicks = signal(0);
    rsx! {
        StackPanel { horizontal_alignment: HorizontalAlignment::Center, vertical_alignment: VerticalAlignment::Center, spacing: 12.0,
            TextBlock { text: "Hello, World!", font_size: 28.0 }
            Button { name: "Count", horizontal_alignment: HorizontalAlignment::Center, on_click: move |_| clicks.update(|c| *c += 1),
                TextBlock { text: move || clicked_text(clicks.get()) }
            }
        }
    }
}

#[cfg(test)]
mod tests;

/// The text of the button after `clicks` clicks.
fn clicked_text(clicks: i32) -> String {
    match clicks {
        1 => "Clicked 1 time".to_string(),
        _ => format!("Clicked {clicks} times"),
    }
}
