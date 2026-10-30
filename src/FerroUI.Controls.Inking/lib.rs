//! ferroui-controls-inking
//!
//! The inking controls and services of WinUI on FerroUI: [`InkCanvas`] with its
//! [`InkPresenter`], the stroke services ([`InkStroke`], [`InkStrokeBuilder`],
//! [`InkStrokeContainer`], erasing and selection), the ruler and protractor stencils,
//! custom drying through [`InkSynchronizer`], [`InkToolbar`] with its buttons and pen
//! configuration, the portable FERROINK format and the clipboard. On Windows,
//! `WindowsInkServices` connects a stroke container to the ink services of the system (ISF
//! and GIF with ISF, the system ink clipboard, handwriting recognition).
//!
//! The toolbar's WinUI theme is part of this crate, not of the WinUI theme: an application
//! that uses the toolbar with the WinUI theme includes its styles,
//!
//! ```xml
//! <StyleInclude Source="ferres://FerroUI.Controls.Inking/Themes/WinUI.xaml" />
//! ```
//!
//! The theme documents are compiled by the build of the crate (`build.rs`);
//! [`register_types`] registers the types of the crate and the loader table of its compiled
//! markup. See `docs/porting/winui-inking.md` for the source mappings and the limits of
//! interoperability.

// The compiled markup names the types of the crate by the name of the crate, as the markup of any
// other crate does.
extern crate self as ferroui_controls_inking;

// `compiled_xaml` (the compiled markup of the theme documents) and `compiled_markup` (the loader
// table of the crate and `register()`), which the build script generates.
ferroui_markup_xaml::include_compiled_xaml!();

#[cfg(any(test, feature = "document-assets"))]
mod assets;
mod ink_events;
mod markup;
mod register_types;
mod rust_paths;
mod windows_ink_interop;
#[cfg(windows)]
mod windows_ink_services;
mod ink_platform_services;
mod ink_clipboard;
mod ink_point;
mod ink_drawing_attributes;
mod ink_stroke;
mod ink_stroke_builder;
mod ink_stroke_container;
mod persistence;
mod ink_input;
mod ink_presenter;
mod ink_presenter_stencil;
mod ink_stencil_input;
mod ink_synchronizer;
mod ink_canvas;
mod ink_canvas_automation_peer;
mod color_names;
mod ink_toolbar_types;
mod ink_toolbar_buttons;
mod ink_toolbar_configuration;
mod ink_toolbar;
mod ink_toolbar_automation_peer;

pub use ink_point::{InkError, InkPoint};
pub use ink_platform_services::*;
pub use ink_clipboard::InkClipboard;
pub use ink_drawing_attributes::*;
pub use ink_stroke::InkStroke;
pub use ink_stroke_builder::InkStrokeBuilder;
pub use ink_stroke_container::InkStrokeContainer;
pub use persistence::{InkPersistenceLimits, INK_MIME_TYPE};
pub use ink_input::*;
pub use ink_presenter::*;
pub use ink_presenter_stencil::*;
pub use ink_synchronizer::InkSynchronizer;
pub use ink_canvas::InkCanvas;
pub use ink_canvas_automation_peer::InkCanvasAutomationPeer;
pub use ink_toolbar_types::*;
pub use ink_toolbar_buttons::*;
pub use ink_toolbar_configuration::*;
pub use ink_toolbar::*;
pub use ink_toolbar_automation_peer::*;
pub use register_types::{register_types, ASSEMBLY};
#[cfg(windows)]
pub use windows_ink_services::{WindowsInkRecognitionResult, WindowsInkServices};

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod theme_tests;

#[cfg(test)]
mod inking_tests;

#[cfg(test)]
mod ink_canvas_tests;

#[cfg(test)]
mod ink_parity_tests;

#[cfg(test)]
mod ink_stencil_tests;
