use super::{EditorProvider, TextEditorAutomationPeer};
use crate::text_editor::EditTarget;
use crate::{
    ClipboardCopyOutcome, ClipboardPasteOutcome, EditorClipboard, EditorClipboardError, TextDocument,
    TextEditor,
};
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{Point, Rect, Ref, WeakRef};
use ferroui_controls::automation::peers::{AutomationPeer, ControlAutomationPeer};
use ferroui_controls::automation::provider::{
    ITextProvider, ITextProvider2, ITextRangeProvider, ProviderError, SupportedTextSelection,
    TextAttributeRun, TextAttributeSet, TextCaretRange, TextEditDelta, TextProviderError, ViewGeometry,
};
use ferroui_controls::{Control, TopLevel};
use ferroui_edit_core::document::{DocumentEdit, SimpleSegment};
use ferroui_edit_core::editing::EditError;
use std::ops::Range;
use std::rc::Rc;

mod links;
mod range;
mod units;

pub(super) use links::HyperlinkPeers;
use links::EditorHyperlinkAutomationPeer;
use range::EditorRange;

type TextRange = Rc<dyn ITextRangeProvider>;

#[derive(Clone)]
struct TextContext {
    peer: WeakRef<TextEditorAutomationPeer>,
    epoch: u64,
}

impl TextContext {
    fn editor(&self) -> Result<Ref<TextEditor>, TextProviderError> {
        self.peer
            .upgrade()
            .and_then(|peer| peer.try_editor())
            .filter(|editor| editor.document_epoch() == self.epoch)
            .ok_or(TextProviderError::ElementNotAvailable)
    }

    fn make_range(
        &self,
        document: &TextDocument,
        span: Range<usize>,
    ) -> Result<TextRange, ProviderError> {
        Ok(Rc::new(EditorRange::new(self.clone(), document, span, None)?))
    }

    fn make_inline_range(
        &self,
        document: &TextDocument,
        span: Range<usize>,
        control: &Ref<Control>,
    ) -> Result<TextRange, ProviderError> {
        let object = span.is_empty().then(|| control.downgrade());
        Ok(Rc::new(EditorRange::new(self.clone(), document, span, object)?))
    }
}

impl EditorProvider {
    fn text_context(&self) -> Result<(TextContext, Ref<TextEditor>), TextProviderError> {
        let editor = self.editor()?;
        let context = TextContext {
            peer: self.0.downgrade(),
            epoch: editor.document_epoch(),
        };
        Ok((context, editor))
    }
}

fn clipboard_result<T: PartialEq>(
    result: Result<T, EditorClipboardError>,
    success: T,
) -> Result<bool, ProviderError> {
    match result {
        Ok(outcome) => Ok(outcome == success),
        Err(EditorClipboardError::Edit(EditError::ReadOnly | EditError::StaleEditPlan)) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

impl ITextProvider for EditorProvider {
    fn is_multiline(&self) -> bool {
        true
    }

    fn default_text_attributes(&self) -> Result<TextAttributeSet, ProviderError> {
        let editor = self.editor()?;
        Ok(range::attributes::defaults(&editor))
    }

    fn text_attribute_run(&self, offset: usize) -> Result<Option<TextAttributeRun>, ProviderError> {
        let (context, editor) = self.text_context()?;
        range::attributes::run(&context, &editor, offset)
    }

    fn text_style_runs(&self, span: Range<usize>) -> Result<Option<Vec<TextAttributeRun>>, ProviderError> {
        let (context, editor) = self.text_context()?;
        range::attributes::runs(&context, &editor, span).map(Some)
    }

    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn document_range(&self) -> Result<TextRange, ProviderError> {
        let (context, _) = self.text_context()?;
        Ok(Rc::new(EditorRange::document(context)))
    }

    fn get_selection(&self) -> Result<Vec<TextRange>, ProviderError> {
        let (context, editor) = self.text_context()?;
        let document = editor.document();
        editor
            .selection()
            .segments()
            .iter()
            .map(|segment| {
                context.make_range(&document, segment.start_offset()..segment.end_offset())
            })
            .collect()
    }

    fn get_visible_ranges(&self) -> Result<Vec<TextRange>, ProviderError> {
        let (context, editor) = self.text_context()?;
        let document = editor.document();
        let area = editor.text_area();
        let mut visible = Vec::new();
        if let Some(clip) = area
            .visual_root()
            .and_then(|root| ViewGeometry::in_root(&area, &root))
            .and_then(|geometry| geometry.local_clip())
        {
            let version = document.version();
            let visibility = area.automation_visibility(&document)?;
            let ranges = area.automation_visible_text(clip)?;
            range::check_version(&document, version)?;
            context.editor()?;
            for span in subtract_ranges(merge_ranges(ranges), visibility.hidden) {
                visible.push(context.make_range(&document, span)?);
            }
        }
        if visible.is_empty() {
            visible.push(context.make_range(&document, 0..0)?);
        }
        Ok(visible)
    }

    fn range_from_child(&self, child: &Ref<AutomationPeer>) -> Result<TextRange, ProviderError> {
        if let Some(link) = child.cast::<EditorHyperlinkAutomationPeer>() {
            if !link.belongs_to(&self.0) {
                return Err(TextProviderError::InvalidArgument.into());
            }
            return link.text_range();
        }
        let (context, editor) = self.text_context()?;
        let document = editor.document();
        let version = document.version();
        let area = editor.text_area();
        let control = child
            .cast::<ControlAutomationPeer>()
            .and_then(|peer| peer.try_owner())
            .ok_or(TextProviderError::InvalidArgument)?;
        let (object, span) = area
            .automation_inline_objects()?
            .into_iter()
            .find(|(object, _)| object == &control || object.is_visual_ancestor_of(&control))
            .ok_or(TextProviderError::InvalidArgument)?;
        range::check_version(&document, version)?;
        context.editor()?;
        context.make_inline_range(&document, span, &object)
    }

    fn range_from_point(&self, point: Point) -> Result<TextRange, ProviderError> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(TextProviderError::InvalidArgument.into());
        }
        let (context, editor) = self.text_context()?;
        let area = editor.text_area();
        let transform = area
            .visual_root()
            .and_then(|root| root.transform_to_visual(&area))
            .filter(|_| area.is_effectively_visible() && !area.content_viewport().is_empty())
            .ok_or(TextProviderError::GeometryUnavailable)?;
        let offset = area.offset_at_point(point.transform(transform));
        context.editor()?;
        context.make_range(&editor.document(), offset..offset)
    }

    fn embedded_range_from_point(&self, point: Point) -> Result<Option<TextRange>, ProviderError> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(TextProviderError::InvalidArgument.into());
        }
        let (context, editor) = self.text_context()?;
        let area = editor.text_area();
        let Some(root) = area.visual_root() else { return Ok(None) };
        let Some(geometry) = ViewGeometry::in_root(&area, &root) else { return Ok(None) };
        let viewport = area.content_viewport();
        if !geometry.visible(viewport).contains_exclusive(point) {
            return Ok(None);
        }
        let Some(local_point) = root.translate_point(point, &area) else { return Ok(None) };
        let document = editor.document();
        let version = document.version();
        let line = area.automation_line_at_point(local_point);
        range::check_version(&document, version.clone())?;
        context.editor()?;
        let Some(line) = line else { return Ok(None) };
        let start = line.first_document_line().offset();
        for element in line.elements() {
            let Some(control) = element.inline_control().filter(|control| {
                area.is_visual_ancestor_of(control)
                    && ViewGeometry::in_root(control, &root).is_some_and(|geometry| {
                        geometry.visible(Rect::from_size(control.bounds().size())).contains_exclusive(point)
                    })
            }) else {
                continue;
            };
            let data = element.data();
            let span = start + data.relative_text_offset()..start + data.end_relative_text_offset();
            return context.make_inline_range(&document, span, &control).map(Some);
        }
        // Use each shaped rectangle. A wrapped link has gaps between its rows,
        // and a caret hit can land beyond the link at the right edge of a row.
        for span in links::link_spans_on_line(&line) {
            let rectangles = area.segment_rectangles(&SimpleSegment::new(span.start, span.len()), false)?;
            range::check_version(&document, version.clone())?;
            context.editor()?;
            if rectangles.into_iter().any(|rect| geometry.visible(rect.intersect(viewport)).contains_exclusive(point)) {
                return context.make_range(&document, span).map(Some);
            }
        }
        Ok(None)
    }

    fn range_from_offsets(&self, offsets: Range<usize>) -> Result<Option<TextRange>, ProviderError> {
        let (context, editor) = self.text_context()?;
        let document = editor.document();
        if offsets.start > offsets.end || offsets.end > document.text_length() {
            return Err(TextProviderError::InvalidArgument.into());
        }
        context.make_range(&document, offsets).map(Some)
    }

    fn logical_line_index(&self, offset: usize) -> Result<Option<usize>, ProviderError> {
        let line = self.editor()?.document().get_line_by_offset(offset)?;
        Ok(Some(line.line_number() - 1))
    }

    fn logical_line_range(&self, index: usize) -> Result<Option<Range<usize>>, ProviderError> {
        let number = index.checked_add(1).ok_or(TextProviderError::InvalidArgument)?;
        let line = self.editor()?.document().get_line_by_number(number)?;
        Ok(Some(line.offset()..line.offset() + line.total_length()))
    }

    fn utf16_offset_from_character(&self, offset: usize) -> Result<Option<usize>, ProviderError> {
        let rope = self.editor()?.document().create_snapshot().get_rope();
        Ok(rope.utf16_offset_from_character(offset))
    }

    fn character_offset_from_utf16(&self, offset: usize) -> Result<Option<usize>, ProviderError> {
        let rope = self.editor()?.document().create_snapshot().get_rope();
        Ok(rope.character_offset_from_utf16(offset))
    }

    fn text_character_count(&self) -> Result<Option<usize>, ProviderError> {
        let rope = self.editor()?.document().create_snapshot().get_rope();
        Ok(Some(rope.character_count()))
    }

    fn replace_text(&self, offsets: Range<usize>, text: &str) -> Result<bool, ProviderError> {
        let editor = self.editor()?;
        if !editor.is_effectively_enabled() || offsets.start > offsets.end {
            return Err(TextProviderError::InvalidArgument.into());
        }
        let edit = DocumentEdit::new(offsets.start, offsets.len(), text);
        EditTarget::new(&editor).apply(&editor, &[edit])?;
        Ok(true)
    }

    fn copy_text_to_clipboard(
        &self,
        offsets: Range<usize>,
        cut: bool,
    ) -> LocalBoxFuture<Result<bool, ProviderError>> {
        let editor = self.editor();
        Box::pin(async move {
            let editor = editor?;
            if !editor.is_effectively_enabled() {
                return Ok(false);
            }
            let Some(clipboard) = TopLevel::get_top_level(Some(&*editor)).and_then(|top| top.clipboard()) else {
                return Ok(false);
            };
            let operation = EditorClipboard::copy_range_to_clipboard(&editor, clipboard, offsets, cut);
            drop(editor);
            let success = if cut { ClipboardCopyOutcome::Cut } else { ClipboardCopyOutcome::Copied };
            clipboard_result(operation.await, success)
        })
    }

    fn paste_text_from_clipboard(&self, offset: usize) -> LocalBoxFuture<Result<bool, ProviderError>> {
        let editor = self.editor();
        Box::pin(async move {
            let editor = editor?;
            if !editor.is_effectively_enabled() || editor.is_read_only() || editor.document().is_read_only() {
                return Ok(false);
            }
            let Some(clipboard) = TopLevel::get_top_level(Some(&*editor)).and_then(|top| top.clipboard()) else {
                return Ok(false);
            };
            let operation = EditorClipboard::paste_at_from_clipboard(&editor, clipboard, offset);
            drop(editor);
            clipboard_result(operation.await, ClipboardPasteOutcome::Pasted)
        })
    }

    fn supported_text_selection(&self) -> Result<SupportedTextSelection, ProviderError> {
        self.editor()?;
        Ok(SupportedTextSelection::Multiple)
    }

    fn text_changed(&self, handler: Rc<dyn Fn()>) -> Result<Rc<dyn IDisposable>, ProviderError> {
        let editor = self.editor()?;
        let text = observe(&editor, TextEditor::text_changed_event(), handler.clone());
        let formatting = observe(
            &editor,
            TextEditor::automation_format_changed_event(),
            handler,
        );
        Ok(Disposable::create(move || {
            text.dispose();
            formatting.dispose();
        }))
    }

    fn text_attributes_changed(&self, handler: Rc<dyn Fn()>) -> Result<Option<Rc<dyn IDisposable>>, ProviderError> {
        let editor = self.editor()?;
        Ok(Some(observe(&editor, TextEditor::automation_format_changed_event(), handler)))
    }

    fn text_selection_changed(
        &self,
        handler: Rc<dyn Fn()>,
    ) -> Result<Rc<dyn IDisposable>, ProviderError> {
        let editor = self.editor()?;
        Ok(observe(
            &editor,
            TextEditor::selection_changed_event(),
            handler,
        ))
    }

    fn text_edited(
        &self,
        handler: Rc<dyn Fn(&TextEditDelta)>,
    ) -> Result<Option<Rc<dyn IDisposable>>, ProviderError> {
        let editor = self.editor()?;
        let token = editor.automation_text_edits.add(handler);
        let weak = editor.downgrade();
        Ok(Some(Disposable::create(move || {
            if let Some(editor) = weak.upgrade() {
                editor.automation_text_edits.remove(token);
            }
        })))
    }
}

impl ITextProvider2 for EditorProvider {
    fn range_from_annotation(&self, _: &Ref<AutomationPeer>) -> Result<TextRange, ProviderError> {
        self.editor()?;
        Err(TextProviderError::InvalidArgument.into())
    }

    fn get_caret_range(&self) -> Result<TextCaretRange, ProviderError> {
        let (context, editor) = self.text_context()?;
        let offset = editor.caret_offset();
        Ok(TextCaretRange {
            is_active: editor.text_area().is_focused() && editor.is_effectively_enabled(),
            range: Some(context.make_range(&editor.document(), offset..offset)?),
        })
    }
}

fn observe(
    editor: &TextEditor,
    event: &'static RoutedEvent<RoutedEventArgs>,
    handler: Rc<dyn Fn()>,
) -> Rc<dyn IDisposable> {
    let weak = editor.to_ref().downgrade();
    let source = weak.clone();
    let token = editor.add_handler(event, move |_, args| {
        if let (Some(owner), Some(sender)) = (source.upgrade(), args.source()) {
            if sender.ptr_eq(&owner) {
                handler();
            }
        }
    });
    Disposable::create(move || {
        if let Some(editor) = weak.upgrade() {
            editor.remove_handler(event, token);
        }
    })
}

pub(crate) fn merge_ranges(mut spans: Vec<Range<usize>>) -> Vec<Range<usize>> {
    spans.sort_unstable_by_key(|span| (span.start, span.end));
    let mut merged: Vec<Range<usize>> = Vec::new();
    for span in spans {
        if let Some(last) = merged.last_mut().filter(|last| last.end >= span.start) {
            last.end = last.end.max(span.end);
        } else {
            merged.push(span);
        }
    }
    merged
}

/// Both lists are ordered and disjoint. A cut can continue across multiple spans.
fn subtract_ranges(spans: Vec<Range<usize>>, cuts: Vec<Range<usize>>) -> Vec<Range<usize>> {
    let mut cuts = cuts.into_iter().peekable();
    let mut remaining = Vec::new();
    for mut span in spans {
        while let Some(cut) = cuts.peek().filter(|cut| cut.start < span.end) {
            if span.start < cut.start {
                remaining.push(span.start..cut.start);
            }
            span.start = span.start.max(cut.end);
            if cut.end > span.end {
                break;
            }
            cuts.next();
        }
        if !span.is_empty() {
            remaining.push(span);
        }
    }
    remaining
}

#[cfg(test)]
mod tests;
