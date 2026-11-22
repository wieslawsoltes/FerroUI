use crate::{TextArea, TextEditor};
use crate::clipboard_event_args::{
    ClipboardSelectionKind, CopyingToClipboardEventArgs, PastingFromClipboardEventArgs,
    TextCopiedEventArgs, TextPastedEventArgs,
};
use crate::rich_text_adapter::{NativeHighlightingStyle, RichTextDisplayError};
use crate::text_editor::EditTarget;
use ferroui_base::input::platform::{ClipboardError, IClipboard};
use ferroui_base::input::{
    AsyncDataTransferExtensions, DataFormat, DataFormatOf, DataTransfer, DataTransferExtensions,
    DataTransferItem, IAsyncDataTransfer, IDataTransfer, LocalBoxFuture,
};
use ferroui_base::threading::Dispatcher;
use ferroui_controls::TopLevel;
use ferroui_edit_core::document::{
    DocumentEdit, DocumentError, DocumentVersion, SimpleSegment, TextDocument, TextUtilities,
};
use ferroui_edit_core::editing::{EditError, Selection};
use ferroui_edit_core::highlighting::{
    HighlightingColor, HtmlClipboard, HtmlOptions, RichText, RichTextError, RichTextModel,
};
use std::fmt;
use std::ops::Range;
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

// The classic Windows editor's registered byte marker. Other platforms carry
// the rectangle through the FerroUI application format alone.
const RECTANGLE_PLATFORM_FORMATS: &[&str] = if cfg!(target_os = "windows") {
    &["AvalonEditRectangularSelection"]
} else {
    &[]
};

const WHOLE_LINE_PLATFORM_FORMATS: &[&str] = if cfg!(target_os = "windows") {
    &["MSDEVLineSelect"]
} else {
    &[]
};

fn is_native_selection_marker(value: &[u8]) -> bool {
    // OLE's HGLOBAL can include zero-filled allocation padding.
    matches!(value, [1, padding @ ..] if padding.is_empty()
        || cfg!(target_os = "windows") && padding.iter().all(|&byte| byte == 0))
}

async fn has_clipboard_selection_marker(
    data: &dyn IAsyncDataTransfer,
    format: DataFormatOf<String>,
    names: &[&str],
) -> Result<bool, ClipboardError> {
    if data.try_get_value_async(&format).await?.as_deref() == Some("1") {
        return Ok(true);
    }
    for name in names {
        if data
            .try_get_value_async(&DataFormat::create_bytes_platform_format(name))
            .await?
            .as_deref()
            .is_some_and(is_native_selection_marker)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

struct CopySelection {
    selection: Selection,
    kind: ClipboardSelectionKind,
}

impl CopySelection {
    fn deletion_edits(&self) -> Vec<DocumentEdit> {
        self.selection
            .segments()
            .iter()
            .map(|segment| DocumentEdit::new(segment.start_offset(), segment.length(), ""))
            .collect()
    }

    fn validate_cut(&self, editor: &TextEditor, fixed_range: bool) -> Result<(), EditError> {
        let session = editor.session();
        if self.kind == ClipboardSelectionKind::Rectangular {
            // The clipboard contains the whole selection. Find the writable
            // spans after a successful write, when the cut can be committed.
            session.validate_document_edits(&[])
        } else if !fixed_range && self.selection.segments().len() == 1 {
            session.writable_deletions(self.selection.primary().segment()).map(|_| ())
        } else {
            session.validate_document_edits(&self.deletion_edits())
        }
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

    pub(crate) fn has_rectangular_drop_marker(data: &dyn IDataTransfer) -> bool {
        data.try_get_value(&Self::rectangular_selection_format())
            .as_deref()
            == Some("1")
            || RECTANGLE_PLATFORM_FORMATS.iter().any(|name| {
                data.try_get_value(&DataFormat::create_bytes_platform_format(name))
                    .as_deref()
                    .is_some_and(is_native_selection_marker)
            })
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
        let prepared = Self::prepare_copy_for_event(editor)?;
        if let Some(prepared) = &prepared {
            for item in prepared.data.items().iter() {
                Self::add_native_selection_markers(item, prepared.kind);
            }
        }
        Ok(prepared)
    }

    pub(crate) fn prepare_copy_for_event(
        editor: &TextEditor,
    ) -> Result<Option<EditorClipboardData>, EditorClipboardError> {
        let Some(copy) = Self::copy_selection(editor)? else {
            return Ok(None);
        };
        Self::prepare_selection_copy(editor, &copy)
    }

    pub(crate) fn add_native_selection_markers(
        item: &DataTransferItem,
        kind: ClipboardSelectionKind,
    ) {
        let (format, names) = match kind {
            ClipboardSelectionKind::Rectangular => (
                Self::rectangular_selection_format(),
                RECTANGLE_PLATFORM_FORMATS,
            ),
            ClipboardSelectionKind::WholeLine => (
                Self::whole_line_selection_format(),
                WHOLE_LINE_PLATFORM_FORMATS,
            ),
            ClipboardSelectionKind::Plain => return,
        };
        // Enrich the final item only if the copying handler retained its marker.
        if names.is_empty() || item.try_get_value(&format).as_deref() != Some("1") {
            return;
        }
        for name in names {
            let format = DataFormat::create_bytes_platform_format(name);
            if !item.contains(&format) {
                item.set(&format, Some(Rc::from([1])));
            }
        }
    }

    fn copy_selection(editor: &TextEditor) -> Result<Option<CopySelection>, EditorClipboardError> {
        let selection = editor.selection();
        if selection.is_empty() {
            if !editor.session().options().cut_copy_whole_line || selection.segments().len() != 1 {
                return Ok(None);
            }
            // Keep the user's empty selection unchanged. The document range is
            // captured separately for formatting and a possible deferred cut.
            let line = editor
                .document()
                .get_line_by_offset(selection.primary().caret)?;
            return Ok(Some(CopySelection {
                selection: Selection::simple(line.offset(), line.offset() + line.total_length()),
                kind: ClipboardSelectionKind::WholeLine,
            }));
        }
        let kind = if selection.is_rectangular() {
            ClipboardSelectionKind::Rectangular
        } else {
            ClipboardSelectionKind::Plain
        };
        Ok(Some(CopySelection { selection, kind }))
    }

    pub(crate) fn can_copy(editor: &TextEditor) -> bool {
        Self::copy_selection(editor).is_ok_and(|selection| selection.is_some())
    }

    pub(crate) fn can_cut(editor: &TextEditor) -> bool {
        let epoch = editor.document_epoch();
        let document = editor.document();
        let version = document.version();
        let selection = editor.selection();
        let Ok(Some(copy)) = Self::copy_selection(editor) else {
            return false;
        };
        let policy = copy.validate_cut(editor, false);
        policy.is_ok()
            && editor.document_epoch() == epoch
            && Self::matches_prepared_state(editor, &document, &version, &selection)
    }

    fn prepare_selection_copy(
        editor: &TextEditor,
        copy: &CopySelection,
    ) -> Result<Option<EditorClipboardData>, EditorClipboardError> {
        let document = editor.document();
        let newline = editor.session().options().newline;
        let mut text = String::new();
        for (index, segment) in copy.selection.segments().iter().enumerate() {
            if index != 0 {
                text.push_str(&newline);
            }
            text.push_str(&document.get_text(segment.start_offset(), segment.length())?);
        }
        if text.is_empty() {
            // A delimiter-only line is copyable; the empty final line is not.
            return Ok(None);
        }
        if copy.kind == ClipboardSelectionKind::WholeLine {
            text = TextUtilities::normalize_new_lines(
                &text,
                if cfg!(windows) { "\r\n" } else { "\n" },
            )?;
        }
        let options = HtmlOptions {
            tab_size: editor.session().options().indentation_size,
        };
        let (html, formatting_error) = match Self::rich_selection(editor, &copy.selection)
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
        match copy.kind {
            ClipboardSelectionKind::Rectangular => {
                item.set(&Self::rectangular_selection_format(), Some("1".to_owned()));
            }
            ClipboardSelectionKind::WholeLine => {
                item.set(&Self::whole_line_selection_format(), Some("1".to_owned()));
            }
            ClipboardSelectionKind::Plain => {}
        }
        data.add(item);
        Ok(Some(EditorClipboardData {
            data,
            text,
            kind: copy.kind,
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
        Self::copy_clipboard_selection(editor, clipboard, cut, None)
    }

    pub(crate) fn copy_range_to_clipboard(
        editor: &TextEditor,
        clipboard: Rc<dyn IClipboard>,
        range: Range<usize>,
        cut: bool,
    ) -> LocalBoxFuture<Result<ClipboardCopyOutcome, EditorClipboardError>> {
        Self::copy_clipboard_selection(editor, clipboard, cut, Some(range))
    }

    fn copy_clipboard_selection(
        editor: &TextEditor,
        clipboard: Rc<dyn IClipboard>,
        cut: bool,
        range: Option<Range<usize>>,
    ) -> LocalBoxFuture<Result<ClipboardCopyOutcome, EditorClipboardError>> {
        let target = range.as_ref().map(|_| EditTarget::new(editor));
        let epoch = editor.document_epoch();
        let prepared = (|| {
            let document = editor.document();
            let selection = editor.selection();
            let version = document.version();
            let copy = match range {
                Some(range) if range.start > range.end || range.end > document.text_length() => {
                    return Err(EditError::InvalidSelection.into());
                }
                Some(range) if range.is_empty() => None,
                Some(range) => Some(CopySelection {
                    selection: Selection::simple(range.start, range.end),
                    kind: ClipboardSelectionKind::Plain,
                }),
                None => Self::copy_selection(editor)?,
            };
            let Some(copy) = copy else {
                return Ok(None);
            };
            if cut {
                let policy = copy.validate_cut(editor, target.is_some());
                let current = match &target {
                    Some(target) => target.is_current(editor),
                    None => {
                        editor.document_epoch() == epoch
                            && Self::matches_prepared_state(editor, &document, &version, &selection)
                    }
                };
                if !current {
                    return Err(EditError::StaleEditPlan.into());
                }
                policy?;
            }
            let Some(data) = Self::prepare_selection_copy(editor, &copy)? else {
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
                Self::add_native_selection_markers(item, copy.kind);
                data.add(item.clone());
            }
            Ok::<_, EditorClipboardError>(Some(Ok((
                document, version, selection, copy, data, text,
            ))))
        })();
        let weak = editor.to_ref().downgrade();
        Box::pin(async move {
            let Some(prepared) = prepared? else {
                return Ok(ClipboardCopyOutcome::Empty);
            };
            let (document, version, selection, copy, data, text) = match prepared {
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
            let event = TextArea::text_copied_event();
            if event.has_instance_handlers() || event.has_raised_subscriptions() {
                editor.text_area().raise_event(&TextCopiedEventArgs::new(
                    event,
                    text.clone(),
                    cut,
                ));
            }
            editor.raise_event(&TextCopiedEventArgs::new(
                TextEditor::text_copied_event(),
                text,
                cut,
            ));
            if !cut {
                return Ok(ClipboardCopyOutcome::Copied);
            }
            if let Some(target) = target {
                if !target.is_current(&editor) {
                    return Ok(ClipboardCopyOutcome::CopiedWithoutCut);
                }
                target.apply(&editor, &copy.deletion_edits())?;
                return Ok(ClipboardCopyOutcome::Cut);
            }
            if editor.document_epoch() != epoch
                || !Self::matches_prepared_state(&editor, &document, &version, &selection)
            {
                return Ok(ClipboardCopyOutcome::CopiedWithoutCut);
            }
            if copy.kind == ClipboardSelectionKind::WholeLine {
                Self::cut_whole_line(&editor, copy.selection.primary().segment())?;
            } else {
                Self::replace_clipboard_selection(&editor, "")?;
            }
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
        let _ = Dispatcher::ui_thread().start_local(async move {
                    if let Err(error) = future.await {
                        if let Some(editor) = weak.upgrade() {
                            editor.report_error(error.to_string());
                        }
                    }
        });
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
            let kind = if has_clipboard_selection_marker(
                data.0.as_ref(),
                Self::rectangular_selection_format(),
                RECTANGLE_PLATFORM_FORMATS,
            )
            .await?
            {
                ClipboardSelectionKind::Rectangular
            } else if has_clipboard_selection_marker(
                data.0.as_ref(),
                Self::whole_line_selection_format(),
                WHOLE_LINE_PLATFORM_FORMATS,
            )
            .await?
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
        Self::paste_clipboard_text(editor, clipboard, None)
    }

    pub(crate) fn paste_at_from_clipboard(
        editor: &TextEditor,
        clipboard: Rc<dyn IClipboard>,
        offset: usize,
    ) -> LocalBoxFuture<Result<ClipboardPasteOutcome, EditorClipboardError>> {
        Self::paste_clipboard_text(editor, clipboard, Some(offset))
    }

    fn paste_clipboard_text(
        editor: &TextEditor,
        clipboard: Rc<dyn IClipboard>,
        offset: Option<usize>,
    ) -> LocalBoxFuture<Result<ClipboardPasteOutcome, EditorClipboardError>> {
        let document = editor.document();
        let target = offset.map(|offset| (EditTarget::new(editor), offset));
        let weak = editor.to_ref().downgrade();
        Box::pin(async move {
            let is_current = |editor: &TextEditor| match &target {
                Some((target, _)) => target.is_current(editor),
                None => editor.document() == document,
            };
            let Some(payload) = Self::read_clipboard(clipboard).await? else {
                return Ok(ClipboardPasteOutcome::Empty);
            };
            let Some(editor) = weak.upgrade() else {
                return Ok(ClipboardPasteOutcome::DocumentReplaced);
            };
            if !is_current(&editor) {
                return Ok(ClipboardPasteOutcome::DocumentReplaced);
            }
            let args = PastingFromClipboardEventArgs::new(
                TextEditor::pasting_from_clipboard_event(),
                payload.text,
                if offset.is_some() { ClipboardSelectionKind::Plain } else { payload.kind },
            );
            editor.raise_event(&args);
            if args.cancel() || args.handled() {
                return Ok(ClipboardPasteOutcome::Cancelled);
            }
            if !is_current(&editor) {
                return Ok(ClipboardPasteOutcome::DocumentReplaced);
            }
            let mut text = args.text();
            if text.is_empty() {
                return Ok(ClipboardPasteOutcome::Empty);
            }
            let kind = if offset.is_some() { ClipboardSelectionKind::Plain } else { args.kind() };
            if let Some((target, offset)) = &target {
                target.apply(&editor, &[DocumentEdit::new(*offset, 0, &text)])?;
            } else if kind == ClipboardSelectionKind::Rectangular
                && editor.text_area().paste_rectangle(&text, false)?
            {
                editor.selection_changed();
            } else if kind == ClipboardSelectionKind::WholeLine
                && editor.selection().is_empty()
                && editor.selection().segments().len() == 1
            {
                text = Self::paste_whole_line(&editor, &text)?;
            } else {
                let adjusted = if kind == ClipboardSelectionKind::Plain {
                    crate::language_indent_paste::paste(&editor, &text).await?
                } else {
                    None
                };
                if let Some(adjusted) = adjusted {
                    text = adjusted;
                } else {
                    Self::replace_clipboard_selection(&editor, &text)?;
                }
            }
            let event = TextArea::text_pasted_event();
            if event.has_instance_handlers() || event.has_raised_subscriptions() {
                editor.text_area().raise_event(&TextPastedEventArgs::new(
                    event,
                    text.clone(),
                    kind,
                ));
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
        let _ = Dispatcher::ui_thread().start_local(async move {
                    if let Err(error) = future.await {
                        if let Some(editor) = weak.upgrade() {
                            editor.report_error(error.to_string());
                        }
                    }
        });
    }

    fn paste_whole_line(editor: &TextEditor, text: &str) -> Result<String, EditorClipboardError> {
        let session = editor.session();
        let document = editor.document();
        let version = document.version();
        let selection = editor.selection();
        let line = document.get_line_by_offset(session.caret_offset())?;
        let newline = TextUtilities::get_new_line_from_document(&document, line.line_number())?;
        let mut text = TextUtilities::normalize_new_lines(text, &newline)?;
        let options = session.options();
        if options.convert_tabs_to_spaces {
            text = text.replace('\t', &" ".repeat(options.indentation_size));
        }
        let edits = [DocumentEdit::new(line.offset(), 0, &text)];
        Self::apply_clipboard_edits(editor, &document, &version, &selection, &edits)?;
        Ok(text)
    }

    fn matches_prepared_state(
        editor: &TextEditor,
        document: &TextDocument,
        version: &DocumentVersion,
        selection: &Selection,
    ) -> bool {
        editor.document() == *document
            && document.version() == *version
            && editor.selection() == *selection
    }

    fn replace_clipboard_selection(editor: &TextEditor, text: &str) -> Result<(), EditError> {
        let target = EditTarget::new(editor);
        editor
            .session()
            .replace_writable_selection_with_text_checked(text, || target.is_current(editor))?;
        editor.selection_changed();
        Ok(())
    }

    fn cut_whole_line(editor: &TextEditor, range: SimpleSegment) -> Result<(), EditError> {
        let target = EditTarget::new(editor);
        let selection = editor.selection();
        let session = editor.session();
        let provider = session.read_only_section_provider();
        let edits = session.writable_deletions(range);
        if !target.is_current(editor) || editor.selection() != selection {
            return Err(EditError::StaleEditPlan);
        }
        // Later spans are removed first, so the original coordinates of each
        // remaining span still apply. Each removal has its own undo entry.
        for (removed, edit) in edits?.iter().rev().enumerate() {
            let selection = editor.selection();
            editor.apply_checked_document_edits(session.document(), std::slice::from_ref(edit), || {
                target.is_current_after(editor, removed)
                    && editor.selection() == selection
                    && Rc::ptr_eq(&provider, &session.read_only_section_provider())
            })?;
        }
        Ok(())
    }

    pub(crate) fn apply_clipboard_edits(
        editor: &TextEditor,
        document: &TextDocument,
        version: &DocumentVersion,
        selection: &Selection,
        edits: &[DocumentEdit],
    ) -> Result<(), EditorClipboardError> {
        editor.apply_checked_document_edits(document, edits, || {
            Self::matches_prepared_state(editor, document, version, selection)
        })?;
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
                        .token_style_overrides(line_number)
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
                            use ferroui_textmate::FontStyle;
                            let flags = |flag| style.font_style.map(|value| value.contains(flag));
                            (
                                line.offset() + start,
                                line.offset() + end.min(line.length()),
                                HighlightingColor {
                                    foreground: style.foreground.map(native_color),
                                    background: style.background.map(native_color),
                                    bold: flags(FontStyle::BOLD),
                                    italic: flags(FontStyle::ITALIC),
                                    underline: flags(FontStyle::UNDERLINE),
                                    strikethrough: flags(FontStyle::STRIKETHROUGH),
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
