//! ferroui-static-view-locator
//!
//! View locators generated at build time: a locator maps the type of a view model to the view
//! that shows it, through a table written by the build script of the crate, with no lookup of a
//! type by its name and no scan of the types of the program at run time. A port of
//! StaticViewLocator (<https://github.com/wieslawsoltes/StaticViewLocator>), a C# source
//! generator; the plan and the deviations are in `docs/porting/static-view-locator.md`.
//!
//! Three crates make the port:
//!
//! - this crate: the attributes, the tables a generated locator holds ([`ViewTable`],
//!   [`MissingViewTable`]), the run-time type of a view model ([`RuntimeType`]) and what the
//!   generated adapters call;
//! - `ferroui-static-view-locator-macros`: [`static_view_locator`] and [`static_view_mapping`],
//!   re-exported here;
//! - `ferroui-static-view-locator-build`: the generator, which the build script of the crate
//!   runs (the counterpart of the incremental generator of the reference).
//!
//! # Usage
//!
//! ```toml
//! [dependencies]
//! ferroui-static-view-locator = "0.4"
//!
//! [build-dependencies]
//! ferroui-static-view-locator-build = "0.4"
//! ```
//!
//! ```ignore
//! // build.rs
//! fn main() {
//!     ferroui_static_view_locator_build::StaticViewLocatorBuild::from_env().run();
//! }
//! ```
//!
//! ```ignore
//! use ferroui_static_view_locator::{static_view_locator, static_view_mapping};
//!
//! #[static_view_locator(generate_data_template = true, data_template_match_types = [ViewModelBase])]
//! #[static_view_mapping(LoginViewModel, LogInView)]
//! #[derive(Default)]
//! pub struct ViewLocator;
//! ```
//!
//! The attribute writes the struct and includes the file the build script generated for it:
//! the tables and the members the options ask for, in an `impl` block of the struct (the
//! `partial class` of the reference). By convention `crate::view_models::SettingsViewModel` is
//! shown by `crate::views::SettingsView`, and `crate::settings_view_model::SettingsViewModel` by
//! `crate::settings_view::SettingsView`; a view is a class of the object model that derives from
//! `UserControl` or `Window` and has a constructor without parameters (`new()`).

#![forbid(unsafe_code)]

mod runtime_type;
mod table;
#[cfg(test)]
mod tests;

#[cfg(feature = "reactiveui")]
pub mod reactiveui;

pub use ferroui_static_view_locator_macros::{static_view_locator, static_view_mapping};
pub use runtime_type::RuntimeType;
pub use table::{NameTable, TypeEntry, TypeTable};

use ferroui_base::{BoxedValue, ObjectType, Ref, Upcast};
use ferroui_controls::{Control, TextBlock};

/// The fallback precedence the generated adapters are compiled with
/// (`generated_adapter_resolution_mode` of [`static_view_locator`]).
///
/// The generator flattens the precedence into the table of the views: a view model type of the
/// crate that has no view of its own gets the view of its nearest mapped base class or of its
/// mapped trait, in the order the mode states.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ViewResolutionMode {
    /// The exact run-time type only.
    Exact,
    /// The exact type, then the nearest mapped base class.
    ExactThenBaseTypes,
    /// The exact type, then the nearest mapped trait.
    ExactThenInterfaces,
    /// The exact type, the mapped base classes, then the mapped traits. The default.
    #[default]
    ExactThenBaseTypesThenInterfaces,
    /// The exact type, the mapped traits, then the mapped base classes.
    ExactThenInterfacesThenBaseTypes,
}

/// Creates the view of a mapping (`Func<Control>` of `s_views`).
pub type ViewFactory = fn() -> Ref<Control>;

/// The views of the view model types (`Dictionary<Type, Func<Control>> s_views`).
pub type ViewTable = TypeTable<ViewFactory>;

/// The messages of the view model types whose view is not found
/// (`Dictionary<Type, string> s_missingViews`): `"Not Found: crate::views::MainWindowView"`.
pub type MissingViewTable = TypeTable<&'static str>;

/// A set of types (the resolved view model types a generated `match_` accepts).
pub type TypeSet = TypeTable<()>;

/// A view as a control (the conversion of `new View()` to `Control`).
pub fn upcast<V: ObjectType + Upcast<Control>>(view: Ref<V>) -> Ref<Control> {
    view.upcast()
}

/// A text block that shows `text` (`new TextBlock { Text = text }`): the view of a missing view
/// and of an invalid view model.
pub fn text_view(text: &str) -> Ref<Control> {
    let text_block = TextBlock::new();
    text_block.set_text(Some(text));
    text_block.upcast()
}

/// Sets `data` as the data context of `control` (`control.DataContext = data`).
pub fn set_data_context(control: &Ref<Control>, data: &BoxedValue) {
    control.set_data_context(Some(data.clone()));
}

/// The text of a view model type whose view is not found and that has no entry in the table of
/// missing views (`$"View for {viewModelType.FullName} is not found."`).
pub fn view_not_found_text(view_model_type: &RuntimeType) -> String {
    format!("View for {} is not found.", view_model_type.name())
}

/// The failure of the legacy `build` for a view model of a type without a view or a missing-view
/// entry (`throw new Exception($"Unable to create view for type: {type}")`).
#[track_caller]
pub fn unable_to_create_view(view_model_type: &RuntimeType) -> ! {
    panic!("Unable to create view for type: {}", view_model_type.name())
}

/// What the generated source names. Not a public API: the paths may change in any version.
#[doc(hidden)]
pub mod generated {
    pub use ferroui_base::{BoxedValue, Ref};
    pub use ferroui_controls::templates::{IDataTemplate, ITemplateWithParam};
    pub use ferroui_controls::Control;
    pub use std::any::{Any, TypeId};
    pub use std::option::Option;
    pub use std::rc::Rc;
}
