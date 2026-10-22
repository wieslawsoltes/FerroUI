use crate::clipboard_event_args::{
    ClipboardSelectionKind, CopyingToClipboardEventArgs, PastingFromClipboardEventArgs,
    TextCopiedEventArgs, TextPastedEventArgs,
};
use crate::rich_text_adapter::{NativeHighlightingStyle, RichTextDisplayError};
use crate::TextEditor;
use ferroui_base::input::platform::{ClipboardError, IClipboard};
use ferroui_base::input::{
    AsyncDataTransferExtensions, DataFormat, DataFormatOf, DataTransfer, DataTransferItem,
    IAsyncDataTransfer, LocalBoxFuture,
};
use ferroui_base::threading::Dispatcher;
use ferroui_controls::TopLevel;
use ferroui_edit_core::document::{DocumentEdit, DocumentError, SimpleSegment};
use ferroui_edit_core::editing::{EditError, Selection};
use ferroui_edit_core::highlighting::{
    HighlightingColor, HtmlClipboard, HtmlOptions, RichText, RichTextError, RichTextModel,
};
use std::fmt;
use std::rc::Rc;

#[derive(Debug)]
pub enum EditorClipboardError {
    Clipboard(ClipboardError),
    Edit(EditError),
    RichText(RichTextError),
    Display(RichTextDisplayError),
    Highlighting(String),
}

impl fmt::Display for EditorClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Clipboard(error) => error.fmt(f),
            Self::Edit(error) => error.fmt(f),
            Self::RichText(error) => error.fmt(f),
            Self::Display(error) => error.fmt(f),
            Self::Highlighting(error) => write!(f, "Clipboard highlighting failed: {error}"),
        }
    }
}

impl std::error::Error for EditorClipboardError {}

impl From<ClipboardError> for EditorClipboardError {
    fn from(error: ClipboardError) -> Self {
        Self::Clipboard(error)
    }
}
impl From<EditError> for EditorClipboardError {
    fn from(error: EditError) -> Self {
        Self::Edit(error)
    }
}
impl From<DocumentError> for EditorClipboardError {
    fn from(error: DocumentError) -> Self {
        Self::Edit(error.into())
    }
}
impl From<RichTextError> for EditorClipboardError {
    fn from(error: RichTextError) -> Self {
        Self::RichText(error)
    }
}
impl From<RichTextDisplayError> for EditorClipboardError {
    fn from(error: RichTextDisplayError) -> Self {
        Self::Display(error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardCopyOutcome {
    Empty,
    Cancelled,
    Copied,
    Cut,
    CopiedWithoutCut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardPasteOutcome {
    Empty,
    Cancelled,
    DocumentReplaced,
    Pasted,
}

/// Prepared native clipboard formats. Plain text is retained even when a rich
/// formatter fails; the failure is available to the copying event and host.
pub struct EditorClipboardData {
    pub data: Rc<DataTransfer>,
    pub text: String,
    pub kind: ClipboardSelectionKind,
    pub formatting_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipboardText {
    pub text: String,
    pub kind: ClipboardSelectionKind,
}

#[derive(Clone, Copy)]
enum HtmlPlatform {
    Mime,
    Windows,
    Apple,
}

impl HtmlPlatform {
    fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_vendor = "apple") {
            Self::Apple
        } else {
            Self::Mime
        }
    }

    fn format(self) -> DataFormat {
        match self {
            Self::Mime => DataFormat::create_string_platform_format("text/html").into(),
            Self::Windows => DataFormat::create_bytes_platform_format("HTML Format").into(),
            Self::Apple => DataFormat::create_string_platform_format("public.html").into(),
        }
    }

    fn set(self, item: &DataTransferItem, html: &str) -> Result<(), RichTextError> {
        match self {
            Self::Mime => item.set(
                &DataFormat::create_string_platform_format("text/html"),
                Some(html.to_owned()),
            ),
            Self::Apple => item.set(
                &DataFormat::create_string_platform_format("public.html"),
                Some(html.to_owned()),
            ),
            Self::Windows => {
                // The generic Win32 String adapter writes UTF-16. CF_HTML has
                // UTF-8 byte offsets and therefore uses the native byte format.
                let payload: Rc<[u8]> = Rc::from(HtmlClipboard::create_cf_html(html)?.into_bytes());
                item.set(
                    &DataFormat::create_bytes_platform_format("HTML Format"),
                    Some(payload),
                );
            }
        }
        Ok(())
    }
}

struct ReadTransfer(Rc<dyn IAsyncDataTransfer>);
impl Drop for ReadTransfer {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

/// Native rich copy, cut and paste using the framework's platform service.
pub struct EditorClipboard;

impl EditorClipboard {
    pub fn rectangular_selection_format() -> DataFormatOf<String> {
        DataFormat::create_string_application_format("FerroUI.Edit.RectangularSelection")
    }

    pub fn whole_line_selection_format() -> DataFormatOf<String> {
        DataFormat::create_string_application_format("FerroUI.Edit.WholeLineSelection")
    }

    pub fn html_format() -> DataFormat {
        HtmlPlatform::current().format()
    }

    /// Returns the current selection as rich text, preserving UTF-16 code units
    /// and the same separator used by the editor's plain-text selection API.
    pub fn selected_rich_text(editor: &TextEditor) -> Result<RichText, EditorClipboardError> {
        Self::rich_selection(editor, &editor.selection())
    }

    pub fn prepare_copy(
        editor: &TextEditor,
    ) -> Result<Option<EditorClipboardData>, EditorClipboardError> {
        let selection = editor.selection();
        if selection.is_empty() {
            return Ok(None);
        }
        let text = editor.selected_text()?;
        if text.is_empty() {
            return Ok(None);
        }
        let kind = if selection.is_rectangular() {
            ClipboardSelectionKind::Rectangular
        } else {
            ClipboardSelectionKind::Plain
        };
        let options = HtmlOptions {
            tab_size: editor.session().options().indentation_size,
        };
        let (html, formatting_error) = match Self::rich_selection(editor, &selection)
            .and_then(|rich| Ok(rich.to_html(&options)?))
        {
            Ok(html) => (Some(html), None),
            Err(error) => (None, Some(error.to_string())),
        };
        let data = DataTransfer::new();
        let item = DataTransferItem::create_text(Some(&text));
        let mut formatting_error = formatting_error;
        if let Some(html) = html {
            if let Err(error) = HtmlPlatform::current().set(&item, &html) {
                formatting_error = Some(error.to_string());
            }
        }
        if kind == ClipboardSelectionKind::Rectangular {
            item.set(&Self::rectangular_selection_format(), Some("1".to_owned()));
        }
        data.add(item);
        Ok(Some(EditorClipboardData {
            data,
            text,
            kind,
            formatting_error,
        }))
    }

    /// Prepares and raises customization synchronously; the returned future owns
    /// the asynchronous platform write. No editor/document borrow spans await.
    pub fn copy_to_clipboard(
        editor: &TextEditor,
        clipboard: Rc<dyn IClipboard>,
        cut: bool,
    ) -> LocalBoxFuture<Result<ClipboardCopyOutcome, EditorClipboardError>> {
        let prepared = (|| {
            let session = editor.session();
            let document = editor.document();
            let selection = editor.selection();
            let version = document.version();
            if cut {
                let edits = selection
                    .segments()
                    .iter()
                    .map(|segment| DocumentEdit::new(segment.start_offset(), segment.length(), ""))
                    .collect::<Vec<_>>();
                session.validate_document_edits(&edits)?;
            }
            let Some(data) = Self::prepare_copy(editor)? else {
                return Ok(None);
            };
            if let Some(error) = &data.formatting_error {
                editor.report_error(error.clone());
            }
            let args = CopyingToClipboardEventArgs::new(
                TextEditor::copying_to_clipboard_event(),
                data.data,
                cut,
                data.kind,
                data.formatting_error,
            );
            editor.raise_event(&args);
            if args.cancel() || args.handled() {
                return Ok(Some(Err(ClipboardCopyOutcome::Cancelled)));
            }
            let text = args.text();
            // A handler may inspect the aggregate format list before changing
            // an item's formats. Rebuild the collection after customization so
            // its cached union describes the final items sent to the platform.
            let data = DataTransfer::new();
            for item in args.data().items().iter() {
                data.add(item.clone());
            }
            Ok::<_, EditorClipboardError>(Some(Ok((document, version, selection, data, text))))
        })();
        let weak = editor.to_ref().downgrade();
        Box::pin(async move {
            let Some(prepared) = prepared? else {
                return Ok(ClipboardCopyOutcome::Empty);
            };
            let (document, version, selection, data, text) = match prepared {
                Ok(value) => value,
                Err(outcome) => return Ok(outcome),
            };
            clipboard.set_data_async(Some(data)).await?;
            let Some(editor) = weak.upgrade() else {
                return Ok(if cut {
                    ClipboardCopyOutcome::CopiedWithoutCut
                } else {
                    ClipboardCopyOutcome::Copied
                });
            };
            editor.raise_event(&TextCopiedEventArgs::new(
                TextEditor::text_copied_event(),
                text,
                cut,
            ));
            if !cut {
                return Ok(ClipboardCopyOutcome::Copied);
            }
            if editor.document() != document
                || document.version() != version
                || editor.selection() != selection
            {
                return Ok(ClipboardCopyOutcome::CopiedWithoutCut);
            }
            // The session rechecks the current read-only provider and applies
            // all selected ranges as one document transaction.
            editor.replace_selection_with_text("")?;
            Ok(ClipboardCopyOutcome::Cut)
        })
    }

    pub fn copy_or_cut(editor: &TextEditor, cut: bool) {
        let Some(clipboard) = TopLevel::get_top_level(Some(editor)).and_then(|top| top.clipboard())
        else {
            return;
        };
        let future = Self::copy_to_clipboard(editor, clipboard, cut);
        let weak = editor.to_ref().downgrade();
        drop(
            Dispatcher::ui_thread()
                .to_task_scheduler()
                .start_local(async move {
                    if let Err(error) = future.await {
                        if let Some(editor) = weak.upgrade() {
                            editor.report_error(error.to_string());
                        }
                    }
                }),
        );
    }

    /// Reads plain text and marker formats from one data-transfer snapshot. The
    /// platform transfer is disposed on success, failure or cancellation.
    pub fn read_clipboard(
        clipboard: Rc<dyn IClipboard>,
    ) -> LocalBoxFuture<Result<Option<ClipboardText>, ClipboardError>> {
        Box::pin(async move {
            let Some(data) = clipboard.try_get_data_async().await? else {
                return Ok(None);
            };
            let data = ReadTransfer(data);
            let Some(text) = data.0.try_get_value_async(&DataFormat::text()).await? else {
                return Ok(None);
            };
            let rectangle = data
                .0
                .try_get_value_async(&Self::rectangular_selection_format())
                .await?;
            let kind = if rectangle.as_deref() == Some("1") {
                ClipboardSelectionKind::Rectangular
            } else if data
                .0
                .try_get_value_async(&Self::whole_line_selection_format())
                .await?
                .as_deref()
                == Some("1")
            {
                ClipboardSelectionKind::WholeLine
            } else {
                ClipboardSelectionKind::Plain
            };
            Ok(Some(ClipboardText { text, kind }))
        })
    }

    pub fn paste_from_clipboard(
        editor: &TextEditor,
        clipboard: Rc<dyn IClipboard>,
    ) -> LocalBoxFuture<Result<ClipboardPasteOutcome, EditorClipboardError>> {
        let document = editor.document();
        let weak = editor.to_ref().downgrade();
        Box::pin(async move {
            let Some(payload) = Self::read_clipboard(clipboard).await? else {
                return Ok(ClipboardPasteOutcome::Empty);
            };
            let Some(editor) = weak.upgrade() else {
                return Ok(ClipboardPasteOutcome::DocumentReplaced);
            };
            if editor.document() != document {
                return Ok(ClipboardPasteOutcome::DocumentReplaced);
            }
            let args = PastingFromClipboardEventArgs::new(
                TextEditor::pasting_from_clipboard_event(),
                payload.text,
                payload.kind,
            );
            editor.raise_event(&args);
            if args.cancel() || args.handled() {
                return Ok(ClipboardPasteOutcome::Cancelled);
            }
            if editor.document() != document {
                return Ok(ClipboardPasteOutcome::DocumentReplaced);
            }
            let text = args.text();
            if text.is_empty() {
                return Ok(ClipboardPasteOutcome::Empty);
            }
            let kind = args.kind();
            if kind == ClipboardSelectionKind::Rectangular
                && editor.text_area().paste_rectangle(&text, false)?
            {
                editor.selection_changed();
            } else if kind == ClipboardSelectionKind::WholeLine
                && editor.selection().is_empty()
                && editor.selection().segments().len() == 1
            {
                Self::paste_whole_line(&editor, &text)?;
            } else {
                editor.replace_selection_with_text(&text)?;
            }
            editor.raise_event(&TextPastedEventArgs::new(
                TextEditor::text_pasted_event(),
                text,
                kind,
            ));
            Ok(ClipboardPasteOutcome::Pasted)
        })
    }

    pub fn paste(editor: &TextEditor) {
        if editor.is_read_only() {
            return;
        }
        let Some(clipboard) = TopLevel::get_top_level(Some(editor)).and_then(|top| top.clipboard())
        else {
            return;
        };
        let future = Self::paste_from_clipboard(editor, clipboard);
        let weak = editor.to_ref().downgrade();
        drop(
            Dispatcher::ui_thread()
                .to_task_scheduler()
                .start_local(async move {
                    if let Err(error) = future.await {
                        if let Some(editor) = weak.upgrade() {
                            editor.report_error(error.to_string());
                        }
                    }
                }),
        );
    }

    fn paste_whole_line(editor: &TextEditor, text: &str) -> Result<(), EditorClipboardError> {
        let session = editor.session();
        let document = editor.document();
        let version = document.version();
        let line = document.get_line_by_offset(session.caret_offset())?;
        let edits = [DocumentEdit::new(line.offset(), 0, text)];
        let validate = || -> Result<(), EditorClipboardError> {
            let policy = session.validate_document_edits(&edits);
            if document.version() != version || editor.document() != document {
                return Err(EditError::StaleEditPlan.into());
            }
            policy?;
            if session.is_read_only() || document.is_read_only() {
                return Err(EditError::ReadOnly.into());
            }
            Ok(())
        };
        validate()?;
        let guard = document.begin_update()?;
        // Host policy and update-start callbacks run outside document borrows.
        // Both can edit or replace the document while coordinates are prepared.
        validate()?;
        document.apply_edits(&edits)?;
        guard.finish()?;
        editor.selection_changed();
        Ok(())
    }

    fn rich_selection(
        editor: &TextEditor,
        selection: &Selection,
    ) -> Result<RichText, EditorClipboardError> {
        let document = editor.document();
        let snapshot = document.create_snapshot();
        let version = document.version();
        let newline = editor.session().options().newline;
        let installation = editor.textmate_installation();
        let highlighter = editor.document_highlighter();
        if let Some(installation) = &installation {
            if let Some(last) = selection
                .segments()
                .iter()
                .map(|segment| segment.end_offset())
                .max()
            {
                let target = document.get_line_by_offset(last)?.line_number();
                if !installation.advance_to(&document, target, target) {
                    return Err(EditorClipboardError::Highlighting(
                        installation
                            .last_error()
                            .unwrap_or_else(|| "Tokenization did not finish".into()),
                    ));
                }
            }
        }
        let mut pieces = Vec::new();
        for (index, selected) in selection.segments().iter().enumerate() {
            if index != 0 {
                pieces.push(RichText::new(&newline));
            }
            let segment = SimpleSegment::new(selected.start_offset(), selected.length());
            let units = snapshot.get_text_utf16(segment.offset, segment.length)?;
            let end = segment.end_offset();
            let mut model = RichTextModel::new();
            let mut line_number = document.get_line_by_offset(segment.offset)?.line_number();
            while segment.length != 0 && line_number <= document.line_count() {
                let line = document.get_line_by_number(line_number)?;
                if line.offset() >= end {
                    break;
                }
                let styles = if let Some(installation) = &installation {
                    installation
                        .styled_tokens(line_number)
                        .into_iter()
                        .map(|(start, end, style)| {
                            let native_color = |color: ferroui_textmate::Color| {
                                if color.a == 255 {
                                    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
                                } else {
                                    format!(
                                        "#{:02x}{:02x}{:02x}{:02x}",
                                        color.a, color.r, color.g, color.b
                                    )
                                }
                            };
                            let flags = style.font_style;
                            (
                                line.offset() + start,
                                line.offset() + end.min(line.length()),
                                HighlightingColor {
                                    foreground: style.foreground.map(native_color),
                                    background: style.background.map(native_color),
                                    bold: Some(flags.contains(ferroui_textmate::FontStyle::BOLD)),
                                    italic: Some(
                                        flags.contains(ferroui_textmate::FontStyle::ITALIC),
                                    ),
                                    underline: Some(
                                        flags.contains(ferroui_textmate::FontStyle::UNDERLINE),
                                    ),
                                    strikethrough: Some(
                                        flags.contains(ferroui_textmate::FontStyle::STRIKETHROUGH),
                                    ),
                                    ..HighlightingColor::default()
                                },
                            )
                        })
                        .collect::<Vec<_>>()
                } else if let Some(highlighter) = &highlighter {
                    let highlighted = highlighter
                        .highlight_line(line_number)
                        .map_err(|error| EditorClipboardError::Highlighting(error.to_string()))?;
                    highlighted.validate_invariants(&document)?;
                    highlighted
                        .sections
                        .into_iter()
                        .map(|section| {
                            (
                                section.offset,
                                section.offset + section.length,
                                section.color,
                            )
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                for (start, finish, color) in styles {
                    let start = start.max(segment.offset);
                    let finish = finish.min(end);
                    if start < finish {
                        let style = NativeHighlightingStyle::resolve(&color, Some(editor))?;
                        model.apply_highlighting(
                            start - segment.offset,
                            finish - start,
                            &style.export_color(&color),
                        )?;
                    }
                }
                line_number += 1;
            }
            pieces.push(RichText::from_utf16_with_model(&units, &model));
        }
        if document.version() != version || editor.document() != document {
            return Err(EditError::StaleEditPlan.into());
        }
        Ok(RichText::concat(&pieces)?)
    }
}

#[cfg(test)]
#[path = "editor_clipboard_tests.rs"]
mod tests;
