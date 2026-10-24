//! FerroUI's code editor control and its framework integration.
//!
//! The editor uses the framework text formatter and drawing API. Document and
//! language services are reusable independently through the re-exported crates.

pub use ferroui_edit_core as core;
pub use ferroui_edit_core::TextViewPosition;
pub use ferroui_edit_core::document::{TextDocument, TextLocation};
pub use ferroui_textmate as textmate;

pub mod automation;
mod background_geometry_builder;
mod background_geometry_rects;
mod background_renderer;
mod background_renderer_connections;
mod clipboard_event_args;
mod collapse_if_single_overload_converter;
mod column_ruler_renderer;
mod commands;
mod completion_data;
mod completion_list;
mod completion_list_box;
mod completion_tip_content_control;
mod completion_window;
mod completion_window_base;
mod current_line_highlight_renderer;
mod editor_clipboard;
mod folding_element_generator;
mod formatted_text_element;
mod highlighting_brush;
mod i_text_run_construction_context;
mod i_text_view_connect;
mod insight_window;
mod link_element_generator;
mod overload_insight_window;
mod overload_provider;
mod overload_viewer;
mod popup_with_custom_position;
mod preedit_element_generator;
mod register_types;
mod rendering;
mod rich_text_adapter;
mod rust_paths;
mod search_panel;
mod single_character_element_generator;
mod snippet_background_renderer;
mod snippet_controller;
mod tab_element_generator;
mod text_area;
mod text_editor;
mod text_input_method_client;
mod textmate_installation;
mod visual_line;
mod visual_line_construction_start_event_args;
mod visual_line_element;
mod visual_line_element_generator;
mod visual_line_element_text_run_properties;
mod visual_line_error;
mod visual_line_link_text;
mod visual_line_text;
mod visual_line_transformer;

pub use background_geometry_builder::{BackgroundGeometryBuilder, BackgroundGeometryError};
pub use background_geometry_rects::{
    BackgroundGeometryLine, BackgroundGeometryViewport, get_rects_for_segment,
    get_rects_for_selection, get_rects_from_visual_segment,
};
pub use background_renderer::{BackgroundRendererRef, IBackgroundRenderer, KnownLayer};
pub use clipboard_event_args::{
    ClipboardSelectionKind, CopyingToClipboardEventArgs, PastingFromClipboardEventArgs,
    TextCopiedEventArgs, TextPastedEventArgs,
};
pub use collapse_if_single_overload_converter::CollapseIfSingleOverloadConverter;
pub use column_ruler_renderer::ColumnRulerRenderer;
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
pub use current_line_highlight_renderer::CurrentLineHighlightRenderer;
pub use editor_clipboard::{
    ClipboardCopyOutcome, ClipboardPasteOutcome, ClipboardText, EditorClipboard,
    EditorClipboardData, EditorClipboardError,
};
pub use folding_element_generator::{FoldingElementGenerator, FoldingLineElement};
pub use formatted_text_element::{FormattedTextContent, FormattedTextElement, FormattedTextRun};
pub use highlighting_brush::{
    HighlightingBrush, HighlightingBrushError, ResourceHighlightingBrush, SimpleHighlightingBrush,
    resolve_highlighting_brush,
};
pub use i_text_run_construction_context::{
    EndOfLineMarkerOptions, ITextRunConstructionContext, VisualLineBuildOptions,
};
pub use i_text_view_connect::ITextViewConnect;
pub use insight_window::InsightWindow;
pub use link_element_generator::{
    LinkElementFactory, LinkElementGenerator, LinkMatch, LinkUriResolver, MailLinkElementGenerator,
};
pub use overload_insight_window::OverloadInsightWindow;
pub use overload_provider::{
    IOverloadProvider, OverloadItem, OverloadProvider, OverloadProviderRef,
};
pub use overload_viewer::OverloadViewer;
pub use popup_with_custom_position::PopupWithCustomPosition;
pub use preedit_element_generator::{PreeditElementGenerator, PreeditTextElement};
pub use register_types::{ASSEMBLY, register_types};
pub use rich_text_adapter::{NativeHighlightingStyle, RichTextDisplayError, RichTextDisplayExt};
pub use search_panel::SearchPanel;
pub use single_character_element_generator::{
    IBuiltinElementGenerator, SingleCharacterElementGenerator,
};
pub use snippet_background_renderer::SnippetBackgroundRenderer;
pub use snippet_controller::SnippetController;
pub use tab_element_generator::{TabElementGenerator, TabTextElement};
pub use text_area::TextArea;
pub use text_editor::TextEditor;
pub use textmate_installation::TextMateInstallation;
pub use visual_line::{
    VISUAL_LINE_LENGTH_LIMIT, VisualLine, VisualLineBuilder, VisualLineTextSource,
};
pub use visual_line_construction_start_event_args::VisualLineConstructionStartEventArgs;
pub use visual_line_element::{VisualLineElement, VisualLineElementData};
pub use visual_line_element_generator::{
    VisualLineElementGenerator, VisualLineElementGeneratorRef,
};
pub use visual_line_element_text_run_properties::VisualLineElementTextRunProperties;
pub use visual_line_error::VisualLineError;
pub use visual_line_link_text::{LinkTextStyle, OpenUriRoutedEventArgs, VisualLineLinkText};
pub use visual_line_text::VisualLineText;
pub use visual_line_transformer::{
    ColorizeDocumentLine, ColorizeVisualLine, ColorizingTransformer, DocumentColorizingTransformer,
    DocumentLineColorizer, IVisualLineTransformer, TextViewConnectionHandler,
    VisualLineTransformerRef,
};

#[cfg(test)]
mod background_geometry_tests;
#[cfg(test)]
mod builtin_background_renderer_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod visual_line_tests;
