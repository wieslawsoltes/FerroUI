//! Schema-aware XAML text editing, navigation, refactoring and stdio LSP.
mod binding;
mod design_support;
#[cfg(test)]
mod dictionary_extraction_tests;
mod editing;
mod formatting;
mod handler_edit;
#[cfg(test)]
mod file_move_tests;
mod handler_project;
#[cfg(test)]
mod handler_tests;
mod line_index;
pub mod lsp;
mod project_schema;
#[cfg(test)]
mod project_navigation_tests;
#[cfg(test)]
mod registered_type_rename_tests;
#[cfg(test)]
mod registered_style_rename_tests;
#[cfg(test)]
mod registered_rsx_rename_tests;
mod references;
#[cfg(not(target_arch = "wasm32"))]
pub mod rust_analyzer;
mod schema;
mod service;
pub use formatting::FormattingOptions;
pub use editing::QuickFix;
pub use line_index::{LineIndex, Position, Range, TextChange, TextEdit};
pub use references::{
    identifier as is_xaml_identifier, name_occurrences, rewrite_names, scope_of as name_scope,
    NameOccurrence,
};
pub use schema::{ExtensionInfo, ModelType, PropertyInfo, Schema, TypeInfo, FRAMEWORK_NAMESPACE};
pub use service::*;
use std::fmt;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageError(pub String);
pub type Result<T> = std::result::Result<T, LanguageError>;
impl fmt::Display for LanguageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for LanguageError {}
impl From<ferroui_designer_core::DesignerError> for LanguageError {
    fn from(e: ferroui_designer_core::DesignerError) -> Self {
        Self(e.to_string())
    }
}
impl From<ferroui_designer_project::ProjectError> for LanguageError {
    fn from(e: ferroui_designer_project::ProjectError) -> Self {
        Self(e.to_string())
    }
}
fn require(value: bool, message: impl Into<String>) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(LanguageError(message.into()))
    }
}
#[cfg(test)]
mod component_schema_tests;
#[cfg(test)]
mod design_time_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod theme_resource_tests;
mod events;
pub use events::{event_handler_name, event_object_span, plan_event_handler, routed_events};
