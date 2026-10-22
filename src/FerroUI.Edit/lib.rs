//! FerroUI's code editor control and its framework integration.
//!
//! The editor uses the framework text formatter and drawing API. Document and
//! language services are reusable independently through the re-exported crates.

pub use ferroui_edit_core as core;
pub use ferroui_edit_core::document::{TextDocument, TextLocation};
pub use ferroui_textmate as textmate;

pub mod automation;
mod commands;
mod completion_window;
mod register_types;
mod rendering;
mod rust_paths;
mod search_panel;
mod snippet_controller;
mod text_area;
mod text_editor;
mod text_input_method_client;
mod textmate_installation;

pub use commands::{EditorCommand, EditorCommandKind};
pub use completion_window::CompletionWindow;
pub use register_types::{ASSEMBLY, register_types};
pub use search_panel::SearchPanel;
pub use snippet_controller::SnippetController;
pub use text_area::TextArea;
pub use text_editor::TextEditor;
pub use textmate_installation::TextMateInstallation;

#[cfg(test)]
mod tests;
