//! FerroUI's code editor control and its framework integration.
//!
//! The editor uses the framework text formatter and drawing API. Document and
//! language services are reusable independently through the re-exported crates.

pub use ferroui_edit_core as core;
pub use ferroui_edit_core::TextViewPosition;
pub use ferroui_edit_core::document::{TextDocument, TextLocation};
pub use ferroui_textmate as textmate;

pub mod automation;
mod collapse_if_single_overload_converter;
mod commands;
mod completion_data;
mod completion_list;
mod completion_list_box;
mod completion_tip_content_control;
mod completion_window;
mod completion_window_base;
mod insight_window;
mod overload_insight_window;
mod overload_provider;
mod overload_viewer;
mod popup_with_custom_position;
mod register_types;
mod rendering;
mod rust_paths;
mod search_panel;
mod snippet_controller;
mod text_area;
mod text_editor;
mod text_input_method_client;
mod textmate_installation;

pub use collapse_if_single_overload_converter::CollapseIfSingleOverloadConverter;
pub use commands::{EditorCommand, EditorCommandKind};
pub use completion_data::{CompletionAction, CompletionData, CompletionDataRef, ICompletionData};
pub use completion_list::{
    CompletionAcceptAction, CompletionInsertionRequestedEventArgs, CompletionList,
};
pub use completion_list_box::CompletionListBox;
pub use completion_tip_content_control::CompletionTipContentControl;
pub use completion_window::CompletionWindow;
pub use completion_window_base::{
    CompletionWindowBase, CompletionWindowBaseImpl, CompletionWindowBaseImplExt,
};
pub use insight_window::InsightWindow;
pub use overload_insight_window::OverloadInsightWindow;
pub use overload_provider::{
    IOverloadProvider, OverloadItem, OverloadProvider, OverloadProviderRef,
};
pub use overload_viewer::OverloadViewer;
pub use popup_with_custom_position::PopupWithCustomPosition;
pub use register_types::{ASSEMBLY, register_types};
pub use search_panel::SearchPanel;
pub use snippet_controller::SnippetController;
pub use text_area::TextArea;
pub use text_editor::TextEditor;
pub use textmate_installation::TextMateInstallation;

#[cfg(test)]
mod tests;
