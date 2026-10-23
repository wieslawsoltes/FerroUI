use super::{
    ITextEditProvider, ITextProvider, ITextProvider2, ITextRangeProvider, ProviderAdapter,
    ProviderError, SupportedTextSelection, TextAttribute, TextAttributeValue, TextCaretRange,
    TextEditChangeType, TextEditTextChangedEventArgs, TextPatternRangeEndpoint, TextProviderError,
    TextUnit,
};
use crate::automation::peers::{
    AutomationPeer, ControlAutomationPeer, HyperlinkAutomationPeer, RichEditBoxAutomationPeer,
};
use crate::automation::ElementNotEnabledException;
use crate::documents::{
    CharacterFormat, Hyperlink, InlineCollection, InlineSnapshot, InlineSnapshotKind,
    InlineUIContainer, LineBreak, Paragraph, ParagraphFormat, RichTextDocument, Run, Span,
    TextGravity, WeakRichTextDocument,
};
use crate::rich_text::{
    RichEditTextDocument, RichEditTextRange, RichTextCompositionChange, TextFindOptions,
    UnicodeTextUnit,
};
use crate::{Control, RichEditBox, RichEditBoxTextPresenter, RichTextBlock};
use ferroui_base::input::IScrollable;
use ferroui_base::media::{
    BaselineAlignment, Color, FlowDirection, FontStyle, IBrush, TextAlignment,
    TextDecorationLocation,
};
use ferroui_base::reactive::{CompositeDisposable, IDisposable, SerialDisposable};
use ferroui_base::{Matrix, ObjectType, Point, Rect, Ref, Visual, WeakRef};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::ops::Range;
use std::rc::{Rc, Weak};

/// Both rich peers expose the same document/range implementation. The owner is
/// weak because accessibility clients can retain a peer after its control dies.
pub(crate) trait RichTextAutomationPeer: ObjectType {
    fn text_host(&self) -> Option<RichTextAutomationHost>;
    fn text_peer(&self) -> Ref<AutomationPeer>;
    fn text_automation_state(&self) -> &Rc<RichTextAutomationState>;
}

/// The semantic owner of text is distinct from its layout view. In particular,
/// the editor keeps its document, history, focus and automation identity while
/// its persistent presenter participates in different control templates.
#[derive(Clone)]
pub(crate) enum RichTextAutomationHost {
    Block(WeakRef<RichTextBlock>),
    Editor(WeakRef<RichEditBox>),
}

impl RichTextAutomationHost {
    pub(crate) fn control(&self) -> Result<Ref<Control>, ProviderError> {
        match self {
            Self::Block(view) => view.upgrade().map(Ref::upcast),
            Self::Editor(editor) => editor.upgrade().map(Ref::upcast),
        }
        .ok_or_else(|| TextProviderError::ElementNotAvailable.into())
    }

    pub(crate) fn view(&self) -> Result<Ref<RichTextBlock>, ProviderError> {
        match self {
            Self::Block(view) => view.upgrade(),
            Self::Editor(editor) => editor
                .upgrade()
                .map(|editor| editor.text_presenter().upcast()),
        }
        .ok_or_else(|| TextProviderError::ElementNotAvailable.into())
    }

    fn service(&self) -> Result<RichEditTextDocument, ProviderError> {
        match self {
            Self::Block(_) => Ok(RichEditTextDocument::new(self.view()?.document())),
            Self::Editor(editor) => Ok(editor
                .upgrade()
                .ok_or(TextProviderError::ElementNotAvailable)?
                .text_document()),
        }
    }

    fn selection(&self) -> Result<Range<usize>, ProviderError> {
        match self {
            Self::Block(_) => {
                let view = self.view()?;
                Ok(selection(&view))
            }
            Self::Editor(editor) => {
                let editor = editor
                    .upgrade()
                    .ok_or(TextProviderError::ElementNotAvailable)?;
                Ok(editor.selection_start().min(editor.selection_end())
                    ..editor.selection_start().max(editor.selection_end()))
            }
        }
    }

    fn caret_index(&self) -> Result<usize, ProviderError> {
        match self {
            Self::Block(_) => Ok(self.view()?.caret_index()),
            Self::Editor(editor) => Ok(editor
                .upgrade()
                .ok_or(TextProviderError::ElementNotAvailable)?
                .caret_index()),
        }
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.control()
            .is_ok_and(|owner| owner.is_effectively_enabled())
    }

    fn is_read_only(&self) -> Result<bool, ProviderError> {
        match self {
            Self::Block(_) => {
                self.view()?;
                Ok(true)
            }
            Self::Editor(editor) => Ok(editor
                .upgrade()
                .ok_or(TextProviderError::ElementNotAvailable)?
                .is_read_only()),
        }
    }

    pub(crate) fn has_focus(&self) -> bool {
        match self {
            Self::Block(_) => self.view().is_ok_and(|view| {
                text_views(&view)
                    .iter()
                    .any(|page| page.is_focused() && page.is_effectively_enabled())
            }),
            Self::Editor(editor) => editor
                .upgrade()
                .is_some_and(|editor| editor.is_focused() && editor.is_effectively_enabled()),
        }
    }

    fn select(&self, range: Range<usize>) -> Result<(), ProviderError> {
        self.control()?;
        if !self.is_enabled() {
            return Err(Box::new(ElementNotEnabledException::new()));
        }
        let view = self.view()?;
        if !view.is_text_selection_enabled() {
            return Err(TextProviderError::SelectionNotSupported.into());
        }
        match self {
            Self::Block(_) => view.select(range.start, range.end),
            Self::Editor(editor) => editor
                .upgrade()
                .ok_or(TextProviderError::ElementNotAvailable)?
                .select(range.start, range.end),
        }
        Ok(())
    }

    pub(crate) fn focus_hyperlink(
        &self,
        link: Ref<Hyperlink>,
        position: usize,
    ) -> Result<bool, ProviderError> {
        let view = self.view()?;
        let page = view.text_view_for_position(position).unwrap_or(view);
        let focused = match self {
            Self::Block(_) => page.focus(),
            Self::Editor(_) => self.control()?.focus(),
        };
        if focused {
            page.focus_hyperlink(Some(link));
        }
        Ok(focused)
    }
}

#[derive(Clone)]
struct TextContext {
    host: RichTextAutomationHost,
    peer: WeakRef<AutomationPeer>,
    state: Weak<RichTextAutomationState>,
}

impl TextContext {
    fn from_peer<T: RichTextAutomationPeer>(peer: &T) -> Result<Self, ProviderError> {
        let host = peer
            .text_host()
            .ok_or(TextProviderError::ElementNotAvailable)?;
        Ok(Self {
            host,
            peer: peer.text_peer().downgrade(),
            state: Rc::downgrade(peer.text_automation_state()),
        })
    }

    fn view(&self) -> Result<Ref<RichTextBlock>, ProviderError> {
        self.host.view()
    }

    fn peer(&self) -> Result<Ref<AutomationPeer>, ProviderError> {
        self.view()?;
        self.peer
            .upgrade()
            .ok_or_else(|| TextProviderError::ElementNotAvailable.into())
    }

    fn make_range(&self, range: Range<usize>) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        self.make_range_in_scope(range, false)
    }

    fn make_range_in_scope(
        &self,
        range: Range<usize>,
        document_scope: bool,
    ) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        let service = self.host.service()?;
        Ok(Rc::new(RichTextAutomationRange {
            context: self.clone(),
            range: service.get_range(range.start, range.end)?,
            document_scope: Cell::new(document_scope),
        }))
    }

    fn children(&self) -> Result<Vec<TextChildEntry>, ProviderError> {
        let state = self
            .state
            .upgrade()
            .ok_or(TextProviderError::ElementNotAvailable)?;
        state.entries(&self.host, &self.peer()?)
    }
}

impl ITextEditProvider for ProviderAdapter<RichEditBoxAutomationPeer> {
    fn raises_text_edit_events_through_peer(&self) -> bool {
        true
    }

    fn get_active_composition(&self) -> Result<Option<Rc<dyn ITextRangeProvider>>, ProviderError> {
        let context = TextContext::from_peer(&*self.0)?;
        let owner = self
            .0
            .try_owner()
            .ok_or(TextProviderError::ElementNotAvailable)?;
        owner
            .editor()
            .active_composition_range()
            .map(|range| context.make_range(range))
            .transpose()
    }

    fn get_conversion_target(&self) -> Result<Option<Rc<dyn ITextRangeProvider>>, ProviderError> {
        self.0
            .try_owner()
            .ok_or(TextProviderError::ElementNotAvailable)?;
        // TextInputMethodClient::set_preedit_text supplies a preview and caret,
        // but not TSF's TF_ATTR_TARGET_NOTCONVERTED/TF_ATTR_TARGET_CONVERTED
        // span. Neither the preview nor the selection identifies that span.
        Ok(None)
    }

    fn text_edit_text_changed(
        &self,
        handler: Rc<dyn Fn(&TextEditTextChangedEventArgs)>,
    ) -> Result<Rc<dyn IDisposable>, ProviderError> {
        let owner = self
            .0
            .try_owner()
            .ok_or(TextProviderError::ElementNotAvailable)?;
        let current = Rc::new(SerialDisposable::new());
        current.set(Some(observe_text_edit_changes(&owner, handler.clone())));
        let subscription = current.clone();
        let weak = owner.downgrade();
        let property_subscription = owner.property_changed(move |event| {
            if event.property() == RichEditBox::document_property().as_property() {
                if let Some(owner) = weak.upgrade() {
                    subscription.set(Some(observe_text_edit_changes(&owner, handler.clone())));
                }
            }
        });
        Ok(Rc::new(CompositeDisposable::from_disposables([
            property_subscription,
            current as Rc<dyn IDisposable>,
        ])))
    }
}

fn observe_text_edit_changes(
    owner: &RichEditBox,
    handler: Rc<dyn Fn(&TextEditTextChangedEventArgs)>,
) -> Rc<dyn IDisposable> {
    let editor = owner.editor();
    let weak_owner = owner.to_ref().downgrade();
    let weak_editor = Rc::downgrade(&editor);
    editor.composition_changed(move |change| {
        let (Some(owner), Some(editor)) = (weak_owner.upgrade(), weak_editor.upgrade()) else {
            return;
        };
        if !Rc::ptr_eq(&editor, &owner.editor()) {
            return;
        }
        let (change_type, text) = match change {
            RichTextCompositionChange::Preview { changed_text } => {
                (TextEditChangeType::Composition, changed_text)
            }
            RichTextCompositionChange::Finalized { text } => {
                (TextEditChangeType::CompositionFinalized, text)
            }
        };
        handler(&TextEditTextChangedEventArgs {
            change_type,
            changed_data: vec![text.clone()],
        });
    })
}

impl<T: RichTextAutomationPeer> ITextProvider2 for ProviderAdapter<T> {
    fn range_from_annotation(
        &self,
        _annotation: &Ref<AutomationPeer>,
    ) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        TextContext::from_peer(&*self.0)?.view()?;
        // RichTextDocument has no annotation objects yet. No peer can identify
        // an annotation of this document; do not reinterpret embedded controls
        // or hyperlinks as annotations (ITextProvider2.RangeFromAnnotation).
        Err(TextProviderError::InvalidArgument.into())
    }

    fn get_caret_range(&self) -> Result<TextCaretRange, ProviderError> {
        let context = TextContext::from_peer(&*self.0)?;
        let view = context.view()?;
        if !view.is_text_selection_enabled() {
            return Ok(TextCaretRange {
                is_active: false,
                range: None,
            });
        }
        let caret = context.host.caret_index()?;
        Ok(TextCaretRange {
            is_active: context.host.has_focus(),
            range: Some(context.make_range(caret..caret)?),
        })
    }
}

impl<T: RichTextAutomationPeer> ITextProvider for ProviderAdapter<T> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.text_peer()
    }

    fn document_range(&self) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        let context = TextContext::from_peer(&*self.0)?;
        let length = context.view()?.document().text_length();
        context.make_range_in_scope(0..length, true)
    }

    fn get_selection(&self) -> Result<Vec<Rc<dyn ITextRangeProvider>>, ProviderError> {
        let context = TextContext::from_peer(&*self.0)?;
        let view = context.view()?;
        if !view.is_text_selection_enabled() {
            return Ok(Vec::new());
        }
        Ok(vec![context.make_range(context.host.selection()?)?])
    }

    fn get_visible_ranges(&self) -> Result<Vec<Rc<dyn ITextRangeProvider>>, ProviderError> {
        let context = TextContext::from_peer(&*self.0)?;
        let view = context.view()?;
        let mut visible: Vec<Range<usize>> = Vec::new();
        if let Some(root) = view.visual_root() {
            for page in text_views(&view) {
                let Some(geometry) = ViewGeometry::in_root(&page, &root) else {
                    continue;
                };
                let sources = page.visible_text_ranges();
                for line in page.line_metrics() {
                    if geometry.visible(line.bounds).is_empty() {
                        continue;
                    }
                    for source in &sources {
                        let range =
                            source.start.max(line.start)..source.end.min(line.start + line.length);
                        if range.is_empty() {
                            continue;
                        }
                        if let Some(previous) =
                            visible.last_mut().filter(|last| last.end == range.start)
                        {
                            previous.end = range.end;
                        } else {
                            visible.push(range);
                        }
                    }
                }
            }
        }
        // UIA requires a degenerate range when the viewport has no visible text.
        if visible.is_empty() {
            let start = view.content_start().min(view.document().text_length());
            visible.push(start..start);
        }
        visible
            .into_iter()
            .map(|range| context.make_range(range))
            .collect()
    }

    fn range_from_child(
        &self,
        child: &Ref<AutomationPeer>,
    ) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        let context = TextContext::from_peer(&*self.0)?;
        let children = context.children()?;
        let mut candidate = Some(child.clone());
        while let Some(peer) = candidate {
            for child in &children {
                if child.peer == peer {
                    return context.make_range(child.range.clone());
                }
            }
            if peer == context.peer()? {
                break;
            }
            candidate = peer.get_parent();
        }
        Err(TextProviderError::InvalidArgument.into())
    }

    fn range_from_point(&self, point: Point) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(TextProviderError::InvalidArgument.into());
        }
        let context = TextContext::from_peer(&*self.0)?;
        let view = context.view()?;
        let root = view
            .visual_root()
            .ok_or(TextProviderError::GeometryUnavailable)?;
        let mut nearest = None;
        for page in text_views(&view) {
            let Some(geometry) = ViewGeometry::in_root(&page, &root) else {
                continue;
            };
            let bounds = geometry.visible(Rect::from_size(page.bounds().size()));
            if bounds.is_empty() {
                continue;
            }
            let dx = (bounds.x - point.x).max(0.0).max(point.x - bounds.right());
            let dy = (bounds.y - point.y).max(0.0).max(point.y - bounds.bottom());
            let distance = dx * dx + dy * dy;
            if nearest
                .as_ref()
                .is_none_or(|(previous, _)| distance < *previous)
            {
                if let Some(transform) = root.transform_to_visual(&page) {
                    nearest = Some((
                        distance,
                        page.hit_test_text_position(point.transform(transform)),
                    ));
                }
            }
        }
        let (_, position) = nearest.ok_or(TextProviderError::GeometryUnavailable)?;
        context.make_range(position..position)
    }

    fn supported_text_selection(&self) -> Result<SupportedTextSelection, ProviderError> {
        Ok(
            if TextContext::from_peer(&*self.0)?
                .view()?
                .is_text_selection_enabled()
            {
                SupportedTextSelection::Single
            } else {
                SupportedTextSelection::None
            },
        )
    }

    fn text_changed(&self, handler: Rc<dyn Fn()>) -> Result<Rc<dyn IDisposable>, ProviderError> {
        let context = TextContext::from_peer(&*self.0)?;
        let view = context.view()?;
        let owner = context.host.control()?;
        let document_subscription = Rc::new(SerialDisposable::new());
        let callback = handler.clone();
        document_subscription.set(Some(view.document().changed(move |_| callback())));
        let current = document_subscription.clone();
        let host = context.host;
        let property_subscription = owner.property_changed(move |event| {
            let document_property = match &host {
                RichTextAutomationHost::Block(_) => {
                    RichTextBlock::document_property().as_property()
                }
                RichTextAutomationHost::Editor(_) => RichEditBox::document_property().as_property(),
            };
            if event.property() == document_property {
                if let Ok(view) = host.view() {
                    let callback = handler.clone();
                    current.set(Some(view.document().changed(move |_| callback())));
                    handler();
                }
            }
        });
        Ok(Rc::new(CompositeDisposable::from_disposables([
            property_subscription,
            document_subscription as Rc<dyn IDisposable>,
        ])))
    }

    fn text_selection_changed(
        &self,
        handler: Rc<dyn Fn()>,
    ) -> Result<Rc<dyn IDisposable>, ProviderError> {
        let context = TextContext::from_peer(&*self.0)?;
        match &context.host {
            RichTextAutomationHost::Block(_) => {
                Ok(context.view()?.selection_changed(move |_| handler()))
            }
            RichTextAutomationHost::Editor(editor) => Ok(editor
                .upgrade()
                .ok_or(TextProviderError::ElementNotAvailable)?
                .selection_changed(move |_| handler())),
        }
    }
}

struct RichTextAutomationRange {
    context: TextContext,
    range: RichEditTextRange,
    // WinUI's document endpoints enclose structural positions outside a link.
    // UTF-16 offsets intentionally omit those positions, so retain the outer
    // document scope separately from a same-offset hyperlink content range.
    document_scope: Cell<bool>,
}

impl RichTextAutomationRange {
    fn view(&self) -> Result<Ref<RichTextBlock>, ProviderError> {
        let view = self.context.view()?;
        if view.document() != self.range.document() {
            return Err(TextProviderError::ElementNotAvailable.into());
        }
        Ok(view)
    }

    fn compatible<'a>(&self, other: &'a dyn ITextRangeProvider) -> Result<&'a Self, ProviderError> {
        self.view()?;
        let other = other
            .as_any()
            .downcast_ref::<Self>()
            .ok_or(TextProviderError::IncompatibleRange)?;
        other.view()?;
        if self.context.peer()? != other.context.peer()?
            || self.range.document() != other.range.document()
        {
            return Err(TextProviderError::IncompatibleRange.into());
        }
        Ok(other)
    }

    fn endpoint(&self, endpoint: TextPatternRangeEndpoint) -> usize {
        match endpoint {
            TextPatternRangeEndpoint::Start => self.range.start_position(),
            TextPatternRangeEndpoint::End => self.range.end_position(),
        }
    }

    fn set_endpoint(
        &self,
        endpoint: TextPatternRangeEndpoint,
        position: usize,
    ) -> Result<(), ProviderError> {
        match endpoint {
            TextPatternRangeEndpoint::Start => self
                .range
                .set_range(position, self.range.end_position().max(position))?,
            TextPatternRangeEndpoint::End => self
                .range
                .set_range(self.range.start_position().min(position), position)?,
        }
        Ok(())
    }

    fn select_range(&self, range: Range<usize>) -> Result<(), ProviderError> {
        self.view()?;
        self.context.host.select(range)
    }

    fn encloses_document(&self) -> bool {
        self.document_scope.get()
            && self.range.start_position() == 0
            && self.range.end_position() == self.range.document().text_length()
    }

    fn units(&self, unit: TextUnit) -> Result<Vec<Range<usize>>, ProviderError> {
        let view = self.view()?;
        let document = self.range.document();
        let text = document.text();
        Ok(match unit {
            TextUnit::Character => {
                crate::rich_text::rich_text_unit_ranges(&text, UnicodeTextUnit::Character)
            }
            TextUnit::Word => crate::rich_text::rich_text_unit_ranges(&text, UnicodeTextUnit::Word),
            TextUnit::Paragraph => paragraph_units(&text),
            TextUnit::Format => format_spans(&view)
                .into_iter()
                .map(|span| span.range)
                .collect(),
            TextUnit::Line | TextUnit::Page => {
                let mut lines = Vec::new();
                let mut pages = Vec::new();
                let mut next = Some(view);
                while let Some(page) = next {
                    let metrics = page.line_metrics();
                    if !metrics.is_empty() {
                        pages.push(page.content_start()..page.content_end());
                        lines.extend(
                            metrics
                                .into_iter()
                                .map(|line| line.start..line.start + line.length),
                        );
                    }
                    next = page.overflow_content_target().map(Ref::upcast);
                }
                let complete = pages.first().is_some_and(|page| page.start == 0)
                    && pages
                        .last()
                        .is_some_and(|page| page.end == document.text_length());
                if unit == TextUnit::Line && complete {
                    lines
                } else if unit == TextUnit::Page && complete && pages.len() > 1 {
                    pages
                } else if unit == TextUnit::Line {
                    // Text not laid out in a viewport has no line boundaries. UIA
                    // promotes that unit to the next larger supported unit.
                    paragraph_units(&text)
                } else {
                    vec![0..document.text_length()]
                }
            }
            TextUnit::Document => vec![0..document.text_length()],
        })
    }
}

impl ITextRangeProvider for RichTextAutomationRange {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn peer(&self) -> Result<Ref<AutomationPeer>, ProviderError> {
        self.view()?;
        self.context.peer()
    }

    fn clone_range(&self) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        self.view()?;
        Ok(Rc::new(Self {
            context: self.context.clone(),
            range: self.range.get_clone()?,
            document_scope: Cell::new(self.document_scope.get()),
        }))
    }

    fn compare(&self, range: &dyn ITextRangeProvider) -> Result<bool, ProviderError> {
        Ok(self.range.range() == self.compatible(range)?.range.range())
    }

    fn compare_endpoints(
        &self,
        endpoint: TextPatternRangeEndpoint,
        range: &dyn ITextRangeProvider,
        target_endpoint: TextPatternRangeEndpoint,
    ) -> Result<i32, ProviderError> {
        let left = self.endpoint(endpoint);
        let right = self.compatible(range)?.endpoint(target_endpoint);
        Ok((left as i128 - right as i128).clamp(i32::MIN as i128, i32::MAX as i128) as i32)
    }

    fn expand_to_enclosing_unit(&self, unit: TextUnit) -> Result<(), ProviderError> {
        let units = self.units(unit)?;
        if let Some(unit) = enclosing_unit(&units, self.range.start_position()) {
            self.range.set_range(unit.start, unit.end)?;
        }
        self.document_scope.set(unit == TextUnit::Document);
        Ok(())
    }

    fn find_attribute(
        &self,
        attribute: TextAttribute,
        value: &TextAttributeValue,
        backwards: bool,
    ) -> Result<Option<Rc<dyn ITextRangeProvider>>, ProviderError> {
        let view = self.view()?;
        let read_only = self.context.host.is_read_only()?;
        if matches!(attribute, TextAttribute::Unknown(_)) {
            return Err(TextProviderError::UnsupportedAttribute.into());
        }
        if matches!(
            value,
            TextAttributeValue::Mixed | TextAttributeValue::NotSupported
        ) {
            return Err(TextProviderError::InvalidArgument.into());
        }
        let range = self.range.range();
        let mut found: Vec<Range<usize>> = Vec::new();
        for span in format_spans(&view) {
            let selected = span.range.start.max(range.start)..span.range.end.min(range.end);
            if selected.is_empty() {
                continue;
            }
            let actual = attribute_value(&view, read_only, selected.start, false, attribute);
            if actual == TextAttributeValue::NotSupported {
                return Err(TextProviderError::UnsupportedAttribute.into());
            }
            if &actual == value {
                if let Some(previous) = found
                    .last_mut()
                    .filter(|previous| previous.end == selected.start)
                {
                    previous.end = selected.end;
                } else {
                    found.push(selected);
                }
            }
        }
        let result = if backwards {
            found.pop()
        } else {
            found.into_iter().next()
        };
        result
            .map(|range| self.context.make_range(range))
            .transpose()
    }

    fn find_text(
        &self,
        text: &str,
        backwards: bool,
        ignore_case: bool,
    ) -> Result<Option<Rc<dyn ITextRangeProvider>>, ProviderError> {
        self.view()?;
        if text.is_empty() {
            return Err(TextProviderError::InvalidArgument.into());
        }
        let mut options = TextFindOptions::empty();
        options.set(TextFindOptions::BACKWARDS, backwards);
        options.set(TextFindOptions::MATCH_CASE, !ignore_case);
        self.range
            .find(text, options)?
            .map(|range| {
                Ok(Rc::new(Self {
                    context: self.context.clone(),
                    range,
                    document_scope: Cell::new(false),
                }) as Rc<dyn ITextRangeProvider>)
            })
            .transpose()
    }

    fn get_attribute_value(
        &self,
        attribute: TextAttribute,
    ) -> Result<TextAttributeValue, ProviderError> {
        let view = self.view()?;
        let range = self.range.range();
        let read_only = self.context.host.is_read_only()?;
        let first = attribute_value(&view, read_only, range.start, range.is_empty(), attribute);
        if range.is_empty() || first == TextAttributeValue::NotSupported {
            return Ok(first);
        }
        for span in format_spans(&view) {
            if span.range.start > range.start && span.range.start < range.end {
                let next = attribute_value(&view, read_only, span.range.start, false, attribute);
                if next == TextAttributeValue::NotSupported {
                    return Ok(next);
                }
                if next != first {
                    return Ok(TextAttributeValue::Mixed);
                }
            }
        }
        Ok(first)
    }

    fn get_bounding_rectangles(&self) -> Result<Vec<Rect>, ProviderError> {
        let view = self.view()?;
        let Some(root) = view.visual_root() else {
            return Ok(Vec::new());
        };
        let mut rectangles = Vec::new();
        for page in text_views(&view) {
            if let Some(geometry) = ViewGeometry::in_root(&page, &root) {
                rectangles.extend(
                    page.get_rects_for_range(self.range.range())
                        .into_iter()
                        .map(|rect| geometry.visible(rect))
                        .filter(|rect| !rect.is_empty()),
                );
            }
        }
        Ok(rectangles)
    }

    fn get_enclosing_element(&self) -> Result<Ref<AutomationPeer>, ProviderError> {
        self.view()?;
        let selected = self.range.range();
        if !selected.is_empty() && !self.encloses_document() {
            // Descendants follow their ancestor in document order. Prefer the
            // deepest containing inline, including an embedded control inside
            // a hyperlink, without treating a boundary caret as that child.
            if let Some(child) = self.context.children()?.into_iter().rev().find(|child| {
                child.range.start <= selected.start && selected.end <= child.range.end
            }) {
                return Ok(child.peer);
            }
        }
        self.context.peer()
    }

    fn get_text(&self, max_length: Option<usize>) -> Result<String, ProviderError> {
        self.view()?;
        let text = self.range.text()?;
        let Some(max_length) = max_length else {
            return Ok(text);
        };
        let mut length = 0;
        Ok(text
            .chars()
            .take_while(|ch| {
                length += ch.len_utf16();
                length <= max_length
            })
            .collect())
    }

    fn move_by(&self, unit: TextUnit, count: i32) -> Result<i32, ProviderError> {
        let units = self.units(unit)?;
        if count == 0 || units.is_empty() {
            return Ok(0);
        }
        if self.range.is_degenerate() {
            let (position, moved) = move_position(&units, self.range.start_position(), count);
            self.range.set_range(position, position)?;
            return Ok(moved);
        }
        let nonempty: Vec<_> = units.into_iter().filter(|unit| !unit.is_empty()).collect();
        let Some(current) = enclosing_unit_index(&nonempty, self.range.start_position()) else {
            return Ok(0);
        };
        let target = (current as i128 + count as i128)
            .clamp(0, nonempty.len().saturating_sub(1) as i128) as usize;
        let moved = target as i128 - current as i128;
        if moved != 0 {
            self.range
                .set_range(nonempty[target].start, nonempty[target].end)?;
        }
        Ok(moved as i32)
    }

    fn move_endpoint_by_unit(
        &self,
        endpoint: TextPatternRangeEndpoint,
        unit: TextUnit,
        count: i32,
    ) -> Result<i32, ProviderError> {
        let units = self.units(unit)?;
        let (position, moved) = move_position(&units, self.endpoint(endpoint), count);
        self.set_endpoint(endpoint, position)?;
        Ok(moved)
    }

    fn move_endpoint_by_range(
        &self,
        endpoint: TextPatternRangeEndpoint,
        range: &dyn ITextRangeProvider,
        target_endpoint: TextPatternRangeEndpoint,
    ) -> Result<(), ProviderError> {
        let position = self.compatible(range)?.endpoint(target_endpoint);
        self.set_endpoint(endpoint, position)
    }

    fn select(&self) -> Result<(), ProviderError> {
        self.select_range(self.range.range())
    }

    fn add_to_selection(&self) -> Result<(), ProviderError> {
        self.view()?;
        let current = self.context.host.selection()?;
        let added = self.range.range();
        if current.is_empty() || added.is_empty() {
            return self.select_range(added);
        }
        if added.end < current.start || current.end < added.start {
            return Err(TextProviderError::MultipleSelectionNotSupported.into());
        }
        self.select_range(current.start.min(added.start)..current.end.max(added.end))
    }

    fn remove_from_selection(&self) -> Result<(), ProviderError> {
        self.view()?;
        let current = self.context.host.selection()?;
        let removed = self.range.range();
        if removed.is_empty() {
            return self.select_range(removed);
        }
        let remaining =
            if current.is_empty() || removed.end <= current.start || current.end <= removed.start {
                current
            } else if removed.start <= current.start && current.end <= removed.end {
                current.start..current.start
            } else if removed.start <= current.start {
                removed.end..current.end
            } else if current.end <= removed.end {
                current.start..removed.start
            } else {
                return Err(TextProviderError::MultipleSelectionNotSupported.into());
            };
        self.select_range(remaining)
    }

    fn scroll_into_view(&self, align_to_top: bool) -> Result<(), ProviderError> {
        let host = self.view()?;
        if host.visual_root().is_none() {
            return Err(TextProviderError::GeometryUnavailable.into());
        }
        let selected = self.range.range();
        let target = if selected.is_empty() {
            // A selection or scroll can invalidate arrangement without losing
            // the last viewport. Rebuild text in that established viewport;
            // a view that has never been arranged still has no scroll target.
            for view in text_views(&host) {
                if view.previous_arrange().is_some() && view.is_effectively_visible() {
                    view.page_layout();
                }
            }
            host.text_view_for_position(selected.start)
                .filter(|view| view.previous_arrange().is_some() && view.is_effectively_visible())
                .map(|view| {
                    let rectangle = view.get_rect_from_character_index(selected.start);
                    (view, rectangle)
                })
        } else {
            // Use selected glyph bounds. The exclusive end can be the start of
            // the next line/page; its forward caret is not the last selected
            // line. Unformatted or collapsed-only source has no scroll target.
            let mut target = None;
            for view in text_views(&host) {
                if view.previous_arrange().is_none() || !view.is_effectively_visible() {
                    continue;
                }
                let rectangles = view.get_rects_for_range(selected.clone());
                let rectangle = if align_to_top {
                    rectangles.into_iter().next()
                } else {
                    rectangles.into_iter().next_back()
                };
                if let Some(rectangle) = rectangle {
                    target = Some((view, rectangle));
                    if align_to_top {
                        break;
                    }
                }
            }
            target
        };
        let (view, mut rectangle) = target.ok_or(TextProviderError::GeometryUnavailable)?;
        if rectangle.height <= 0.0 {
            return Err(TextProviderError::GeometryUnavailable.into());
        }
        if let Some(presenter) = view.cast::<RichEditBoxTextPresenter>() {
            let old_offset = presenter.offset();
            presenter.bring_text_rect_into_view_aligned(rectangle, align_to_top);
            // The editor translates glyph geometry immediately when scrolling.
            // Forward its new local rectangle to outer scrolling ancestors.
            rectangle = rectangle.translate(old_offset - presenter.offset());
        } else {
            let mut ancestor = view.visual_parent();
            while let Some(parent) = ancestor {
                if let Some(presenter) = parent.cast::<crate::presenters::ScrollContentPresenter>()
                {
                    presenter.bring_descendant_into_view_aligned(&view, rectangle, align_to_top);
                    break;
                }
                ancestor = parent.visual_parent();
            }
        }
        view.bring_into_view_rect(rectangle);
        Ok(())
    }

    fn get_children(&self) -> Result<Vec<Ref<AutomationPeer>>, ProviderError> {
        self.view()?;
        let selected = self.range.range();
        let document_scope = self.encloses_document();
        let children = self.context.children()?;
        Ok(children
            .into_iter()
            .filter(|child| {
                !selected.is_empty()
                    && child.range.start < selected.end
                    && selected.start < child.range.end
                    // CTextRangeAdapter::IsLinkWithinRange excludes an object
                    // that contains the range. Do not expose the enclosing
                    // hyperlink/object (or an ancestor) as its own child.
                    && (document_scope
                        || !(child.range.start <= selected.start
                            && selected.end <= child.range.end))
            })
            .map(|child| child.peer)
            .collect())
    }
}

fn selection(view: &RichTextBlock) -> Range<usize> {
    view.selection_start().min(view.selection_end())
        ..view.selection_start().max(view.selection_end())
}

fn paragraph_units(text: &str) -> Vec<Range<usize>> {
    crate::rich_text::rich_text_unit_ranges(text, UnicodeTextUnit::Paragraph)
}

fn enclosing_unit_index(units: &[Range<usize>], position: usize) -> Option<usize> {
    units
        .iter()
        .position(|unit| unit.start <= position && (position < unit.end || unit.is_empty()))
        .or_else(|| units.len().checked_sub(1))
}

fn enclosing_unit(units: &[Range<usize>], position: usize) -> Option<&Range<usize>> {
    enclosing_unit_index(units, position).map(|index| &units[index])
}

fn move_position(units: &[Range<usize>], position: usize, count: i32) -> (usize, i32) {
    if count == 0 {
        return (position, 0);
    }
    let mut boundaries: Vec<_> = units
        .iter()
        .flat_map(|range| [range.start, range.end])
        .collect();
    boundaries.sort_unstable();
    boundaries.dedup();
    let remaining = count.unsigned_abs() as usize;
    if count > 0 {
        let first = boundaries.partition_point(|value| *value <= position);
        let moved = remaining.min(boundaries.len() - first);
        if moved == 0 {
            (position, 0)
        } else {
            (boundaries[first + moved - 1], moved as i32)
        }
    } else {
        let end = boundaries.partition_point(|value| *value < position);
        let moved = remaining.min(end);
        if moved == 0 {
            (position, 0)
        } else {
            (boundaries[end - moved], (-(moved as i64)) as i32)
        }
    }
}

struct ViewGeometry {
    transform: Matrix,
    clip: Rect,
}
impl ViewGeometry {
    fn in_root(view: &RichTextBlock, root: &Ref<Visual>) -> Option<Self> {
        if !view.is_effectively_visible() {
            return None;
        }
        // The portable provider reports one root coordinate space. Overflow
        // pages in a different presentation root cannot be expressed in it.
        let transform = view.transform_to_visual(root)?;
        let mut clip = Rect::from_size(root.bounds().size());
        let mut current = Some(view.to_ref().upcast::<Visual>());
        while let Some(visual) = current {
            let to_root = visual.transform_to_visual(root)?;
            if visual.clip_to_bounds() {
                clip = clip
                    .intersect(Rect::from_size(visual.bounds().size()).transform_to_aabb(to_root));
            }
            if let Some(geometry) = visual.clip() {
                clip = clip.intersect(geometry.bounds().transform_to_aabb(to_root));
            }
            current = visual.visual_parent();
        }
        Some(Self { transform, clip })
    }
    fn visible(&self, local: Rect) -> Rect {
        local.transform_to_aabb(self.transform).intersect(self.clip)
    }
}

fn text_views(view: &Ref<RichTextBlock>) -> Vec<Ref<RichTextBlock>> {
    let mut views = Vec::new();
    let mut next = Some(view.clone());
    while let Some(page) = next {
        next = page.overflow_content_target().map(Ref::upcast);
        views.push(page);
    }
    views
}

/// A peer owns one cache for the document's nonvisual children. The cache only
/// retains automation peers: document, control and hyperlink owners stay weak.
/// Revision checks avoid walking the inline tree for each native range call.
#[derive(Default)]
pub(crate) struct RichTextAutomationState {
    children: RefCell<Option<TextChildrenCache>>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

struct TextChildrenCache {
    document: WeakRichTextDocument,
    revision: u64,
    entries: Vec<TextChildEntry>,
}

#[derive(Clone)]
struct TextChildEntry {
    range: Range<usize>,
    parent: Option<usize>,
    peer: Ref<AutomationPeer>,
}

impl RichTextAutomationState {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub(crate) fn observe<T: RichTextAutomationPeer>(self: &Rc<Self>, peer: Ref<T>) {
        let weak_peer = peer.text_peer().downgrade();
        let host = peer.text_host();
        let weak_state = Rc::downgrade(self);
        let provider = ProviderAdapter(peer);
        let subscription = provider
            .text_changed(Rc::new(move || {
                let (Some(state), Some(peer), Some(host)) =
                    (weak_state.upgrade(), weak_peer.upgrade(), host.as_ref())
                else {
                    return;
                };
                let _ = state.entries(host, &peer);
            }))
            .ok();
        if let Some(previous) = self.subscription.replace(subscription) {
            previous.dispose();
        }
    }

    fn entries(
        self: &Rc<Self>,
        host: &RichTextAutomationHost,
        container: &Ref<AutomationPeer>,
    ) -> Result<Vec<TextChildEntry>, ProviderError> {
        let view = host.view()?;
        let document = view.document();
        let previous = {
            let current = self.children.borrow();
            if let Some(current) = current.as_ref() {
                if current.document.upgrade().as_ref() == Some(&document)
                    && current.revision == document.revision()
                {
                    return Ok(current.entries.clone());
                }
                current.entries.clone()
            } else {
                Vec::new()
            }
        };
        // No cache borrow is held while creating peers or assigning parents:
        // provider constructors and framework callbacks may reenter automation.
        let descriptors = document_children(&document);
        let mut entries = Vec::with_capacity(descriptors.len());
        let mut links = std::collections::HashMap::new();
        let mut retained_links: std::collections::HashMap<_, _> = previous
            .iter()
            .filter_map(|entry| {
                let peer = entry.peer.cast::<HyperlinkAutomationPeer>()?;
                let link = peer.try_owner()?;
                Some((hyperlink_key(&link), peer))
            })
            .collect();
        for child in descriptors {
            let peer = match child.owner {
                TextChildOwner::Hyperlink(link) => {
                    let peer = retained_links
                        .remove(&hyperlink_key(&link))
                        .unwrap_or_else(|| {
                            HyperlinkAutomationPeer::new(
                                host.clone(),
                                container,
                                &link,
                                Rc::downgrade(self),
                            )
                        });
                    let peer = peer.upcast::<AutomationPeer>();
                    links.insert(hyperlink_key(&link), peer.clone());
                    peer
                }
                TextChildOwner::Embedded(control) => {
                    ControlAutomationPeer::create_peer_for_element(&control)
                }
            };
            entries.push(TextChildEntry {
                range: child.range,
                parent: child.parent,
                peer,
            });
        }
        for child in &entries {
            let parent = child
                .parent
                .and_then(|key| links.get(&key))
                .unwrap_or(container)
                .clone();
            child.peer.try_set_parent(Some(parent));
        }
        let current_peers: HashSet<_> = entries
            .iter()
            .map(|entry| std::ptr::from_ref::<AutomationPeer>(&entry.peer) as usize)
            .collect();
        for removed in previous.iter().filter(|old| {
            !current_peers.contains(&(std::ptr::from_ref::<AutomationPeer>(&old.peer) as usize))
        }) {
            removed.peer.try_set_parent(None);
        }
        let changed = previous.len() != entries.len()
            || previous
                .iter()
                .zip(&entries)
                .any(|(old, new)| old.peer != new.peer || old.parent != new.parent);
        *self.children.borrow_mut() = Some(TextChildrenCache {
            document: document.downgrade(),
            revision: document.revision(),
            entries: entries.clone(),
        });
        if changed {
            container.raise_children_changed_event();
        }
        Ok(entries)
    }

    pub(crate) fn children(
        self: &Rc<Self>,
        host: &RichTextAutomationHost,
        container: &Ref<AutomationPeer>,
        parent: Option<&Ref<Hyperlink>>,
    ) -> Rc<Vec<Ref<AutomationPeer>>> {
        let parent = parent.map(hyperlink_key);
        let Ok(entries) = self.entries(host, container) else {
            return Rc::new(Vec::new());
        };
        let mut children: Vec<_> = entries
            .iter()
            .filter(|entry| entry.parent == parent)
            .map(|entry| entry.peer.clone())
            .collect();
        if parent.is_none() {
            // A rich editor may also contain framework header, description and
            // placeholder presenters. Retain those visual children in the raw
            // automation tree without duplicating any document child (including
            // a control nested inside a hyperlink).
            if let Some(visuals) = host
                .control()
                .ok()
                .and_then(|owner| owner.visual_children_snapshot())
            {
                for control in visuals.iter().filter_map(|visual| visual.cast::<Control>()) {
                    if !control.is_visible() {
                        continue;
                    }
                    let peer = ControlAutomationPeer::create_peer_for_element(&control);
                    if !entries.iter().any(|entry| entry.peer == peer) {
                        peer.try_set_parent(Some(container.clone()));
                        children.push(peer);
                    }
                }
            }
        }
        Rc::new(children)
    }

    pub(crate) fn range_for_child(
        self: &Rc<Self>,
        host: &RichTextAutomationHost,
        container: &Ref<AutomationPeer>,
        child: &Ref<AutomationPeer>,
    ) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        let range = self.range_bounds_for_child(host, container, child)?;
        TextContext {
            host: host.clone(),
            peer: container.downgrade(),
            state: Rc::downgrade(self),
        }
        .make_range(range)
    }

    pub(crate) fn range_bounds_for_child(
        self: &Rc<Self>,
        host: &RichTextAutomationHost,
        container: &Ref<AutomationPeer>,
        child: &Ref<AutomationPeer>,
    ) -> Result<Range<usize>, ProviderError> {
        let entry = self
            .entries(host, container)?
            .into_iter()
            .find(|entry| entry.peer == *child)
            .ok_or(TextProviderError::ElementNotAvailable)?;
        Ok(entry.range)
    }
}

impl Drop for RichTextAutomationState {
    fn drop(&mut self) {
        if let Some(subscription) = self.subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}

enum TextChildOwner {
    Hyperlink(Ref<Hyperlink>),
    Embedded(Ref<Control>),
}

struct DocumentChild {
    range: Range<usize>,
    parent: Option<usize>,
    owner: TextChildOwner,
}

fn hyperlink_key(link: &Ref<Hyperlink>) -> usize {
    std::ptr::from_ref::<Hyperlink>(link) as usize
}

fn document_children(document: &RichTextDocument) -> Vec<DocumentChild> {
    fn visit(
        inlines: &InlineCollection,
        offset: &mut usize,
        parent: Option<usize>,
        output: &mut Vec<DocumentChild>,
    ) {
        for inline in inlines.snapshot().iter() {
            if let Some(link) = inline.cast::<Hyperlink>() {
                let index = output.len();
                output.push(DocumentChild {
                    range: *offset..*offset,
                    parent,
                    owner: TextChildOwner::Hyperlink(link.clone()),
                });
                visit(&link.inlines(), offset, Some(hyperlink_key(&link)), output);
                output[index].range.end = *offset;
            } else if let Some(span) = inline.cast::<Span>() {
                visit(&span.inlines(), offset, parent, output);
            } else {
                let length = if let Some(run) = inline.cast::<Run>() {
                    run.text().map_or(0, |text| text.encode_utf16().count())
                } else if inline.is::<LineBreak>() || inline.is::<InlineUIContainer>() {
                    1
                } else {
                    let mut text = String::new();
                    inline.append_text(&mut text);
                    text.encode_utf16().count()
                };
                if let Some(control) = inline
                    .cast::<InlineUIContainer>()
                    .and_then(|item| item.child())
                {
                    output.push(DocumentChild {
                        range: *offset..*offset + length,
                        parent,
                        owner: TextChildOwner::Embedded(control),
                    });
                }
                *offset += length;
            }
        }
    }
    let mut output = Vec::new();
    let mut offset = 0;
    for (index, paragraph) in document.paragraphs().iter().enumerate() {
        if index > 0 {
            offset += 1
        }
        visit(&paragraph.inlines(), &mut offset, None, &mut output);
    }
    output
}

struct FormatSpan {
    range: Range<usize>,
    character: CharacterFormat,
    paragraph: ParagraphFormat,
}

fn format_spans(view: &RichTextBlock) -> Vec<FormatSpan> {
    fn visit(
        inlines: &[InlineSnapshot],
        offset: &mut usize,
        boundaries: &mut Vec<usize>,
        barriers: &mut HashSet<usize>,
    ) {
        for inline in inlines {
            boundaries.push(*offset);
            let barrier = matches!(
                inline.kind,
                InlineSnapshotKind::Embedded(_) | InlineSnapshotKind::Hyperlink { .. }
            );
            if barrier {
                barriers.insert(*offset);
            }
            if let Some(children) = inline.children() {
                visit(children, offset, boundaries, barriers)
            } else {
                *offset += inline.text_length()
            }
            if barrier {
                barriers.insert(*offset);
            }
            boundaries.push(*offset);
        }
    }
    let mut boundaries = vec![0];
    let mut barriers = HashSet::new();
    let mut offset = 0;
    let document = view.document();
    for (index, paragraph) in document.snapshot().paragraphs.iter().enumerate() {
        if index > 0 {
            boundaries.push(offset);
            offset += 1;
            boundaries.push(offset);
        }
        visit(
            &paragraph.inlines,
            &mut offset,
            &mut boundaries,
            &mut barriers,
        );
    }
    boundaries.push(document.text_length());
    boundaries.sort_unstable();
    boundaries.dedup();
    let mut spans: Vec<FormatSpan> = Vec::new();
    for pair in boundaries.windows(2) {
        let character = effective_format(view, pair[0], false);
        let paragraph = effective_paragraph(view, pair[0]);
        if let Some(previous) = spans.last_mut().filter(|span| {
            span.range.end == pair[0]
                && !barriers.contains(&pair[0])
                && span.character == character
                && span.paragraph == paragraph
        }) {
            previous.range.end = pair[1];
        } else {
            spans.push(FormatSpan {
                range: pair[0]..pair[1],
                character,
                paragraph,
            });
        }
    }
    if spans.is_empty() {
        spans.push(FormatSpan {
            range: 0..0,
            character: effective_format(view, 0, true),
            paragraph: effective_paragraph(view, 0),
        });
    }
    spans
}

fn effective_format(view: &RichTextBlock, position: usize, caret: bool) -> CharacterFormat {
    view.document()
        .element_at(
            position,
            if caret {
                TextGravity::Backward
            } else {
                TextGravity::Forward
            },
        )
        .map(|element| CharacterFormat::effective(&element))
        .unwrap_or_else(|| CharacterFormat {
            font_family: Some(view.font_family()),
            font_size: Some(view.font_size()),
            font_style: Some(view.font_style()),
            font_weight: Some(view.font_weight()),
            foreground: Some(view.foreground()),
            background: Some(view.background()),
            ..Default::default()
        })
}

fn effective_paragraph(view: &RichTextBlock, position: usize) -> ParagraphFormat {
    let mut offset = 0;
    for block in view.document().blocks().snapshot().iter() {
        if let Some(paragraph) = block.cast::<Paragraph>() {
            let length = paragraph.text_length();
            if offset <= position && position <= offset + length {
                return ParagraphFormat {
                    text_alignment: Some(paragraph.text_alignment()),
                    text_indent: Some(
                        if paragraph.is_set(Paragraph::text_indent_property().as_property()) {
                            paragraph.text_indent()
                        } else {
                            view.text_indent()
                        },
                    ),
                    margin: Some(paragraph.margin()),
                    flow_direction: Some(paragraph.flow_direction()),
                    left_indent: Some(paragraph.left_indent()),
                    right_indent: Some(paragraph.right_indent()),
                    ..Default::default()
                };
            }
            offset += length + 1;
        }
    }
    ParagraphFormat {
        text_alignment: Some(view.text_alignment()),
        flow_direction: Some(view.flow_direction()),
        ..Default::default()
    }
}

fn attribute_value(
    view: &RichTextBlock,
    read_only: bool,
    position: usize,
    caret: bool,
    attribute: TextAttribute,
) -> TextAttributeValue {
    use TextAttribute as A;
    use TextAttributeValue as V;
    let format = effective_format(view, position, caret);
    let paragraph = effective_paragraph(view, position);
    match attribute {
        A::FontName => format
            .font_family
            .map_or(V::NotSupported, |font| V::String(font.name().to_owned())),
        A::FontSize => format
            .font_size
            .map_or(V::NotSupported, |size| V::Number(size * 72.0 / 96.0)),
        A::FontWeight => format
            .font_weight
            .map_or(V::NotSupported, |weight| V::Integer(weight.0)),
        A::IsItalic => V::Bool(
            format
                .font_style
                .is_some_and(|style| style != FontStyle::Normal),
        ),
        A::IsReadOnly => V::Bool(read_only),
        A::IsHidden => V::Bool(!view.is_effectively_visible()),
        A::ForegroundColor => brush_color(format.foreground.as_ref().and_then(Option::as_ref)),
        A::BackgroundColor => brush_color(format.background.as_ref().and_then(Option::as_ref)),
        A::UnderlineStyle => decoration_style(&format, TextDecorationLocation::Underline),
        A::OverlineStyle => decoration_style(&format, TextDecorationLocation::Overline),
        A::StrikethroughStyle => decoration_style(&format, TextDecorationLocation::Strikethrough),
        A::UnderlineColor => decoration_color(&format, TextDecorationLocation::Underline),
        A::OverlineColor => decoration_color(&format, TextDecorationLocation::Overline),
        A::StrikethroughColor => decoration_color(&format, TextDecorationLocation::Strikethrough),
        A::IsSubscript => V::Bool(format.baseline_alignment == Some(BaselineAlignment::Subscript)),
        A::IsSuperscript => {
            V::Bool(format.baseline_alignment == Some(BaselineAlignment::Superscript))
        }
        A::HorizontalTextAlignment => {
            let rtl = paragraph.flow_direction == Some(FlowDirection::RightToLeft);
            V::Integer(match paragraph.text_alignment.unwrap_or_default() {
                TextAlignment::Center => 1,
                TextAlignment::Right => 2,
                TextAlignment::Justify => 3,
                TextAlignment::Start if rtl => 2,
                TextAlignment::End if !rtl => 2,
                _ => 0,
            })
        }
        A::IndentationFirstLine => {
            V::Number(paragraph.text_indent.unwrap_or_default() * 72.0 / 96.0)
        }
        A::IndentationLeading | A::IndentationTrailing => {
            let margin = paragraph.margin.unwrap_or_default();
            let rtl = paragraph.flow_direction == Some(FlowDirection::RightToLeft);
            let left = (attribute == A::IndentationLeading) != rtl;
            let indent = if left {
                margin.left + paragraph.left_indent.unwrap_or_default()
            } else {
                margin.right + paragraph.right_indent.unwrap_or_default()
            };
            V::Number(indent * 72.0 / 96.0)
        }
        A::MarginTop => V::Number(paragraph.margin.unwrap_or_default().top * 72.0 / 96.0),
        A::MarginBottom => V::Number(paragraph.margin.unwrap_or_default().bottom * 72.0 / 96.0),
        A::MarginLeading | A::MarginTrailing => {
            let margin = paragraph.margin.unwrap_or_default();
            let rtl = paragraph.flow_direction == Some(FlowDirection::RightToLeft);
            let left = (attribute == A::MarginLeading) != rtl;
            V::Number(if left { margin.left } else { margin.right } * 72.0 / 96.0)
        }
        A::TextFlowDirections => V::Integer(i32::from(
            paragraph.flow_direction == Some(FlowDirection::RightToLeft),
        )),
        A::Unknown(_) => V::NotSupported,
    }
}

fn decoration_style(
    format: &CharacterFormat,
    location: TextDecorationLocation,
) -> TextAttributeValue {
    let decorations: Vec<_> = format
        .text_decorations
        .as_ref()
        .and_then(Option::as_ref)
        .map(|collection| {
            collection
                .list()
                .snapshot()
                .iter()
                .filter(|item| item.location() == location)
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    match decorations.as_slice() {
        [] => TextAttributeValue::Integer(0),
        [line]
            if line
                .stroke_dash_array()
                .is_none_or(|array| array.list().is_empty()) =>
        {
            TextAttributeValue::Integer(1)
        }
        // Arbitrary stacked/dashed framework decorations do not have a unique
        // AutomationTextDecorationLineStyle. Preserve NotSupported instead of
        // misreporting every custom pattern as a single solid underline.
        _ => TextAttributeValue::NotSupported,
    }
}

fn decoration_color(
    format: &CharacterFormat,
    location: TextDecorationLocation,
) -> TextAttributeValue {
    let Some(decorations) = format.text_decorations.as_ref().and_then(Option::as_ref) else {
        return TextAttributeValue::NotSupported;
    };
    let mut result = None;
    for line in decorations
        .list()
        .snapshot()
        .iter()
        .filter(|line| line.location() == location)
    {
        let stroke = line.stroke();
        let color = brush_color(
            stroke
                .as_ref()
                .or_else(|| format.foreground.as_ref().and_then(Option::as_ref)),
        );
        if color == TextAttributeValue::NotSupported {
            return color;
        }
        if result.as_ref().is_some_and(|previous| *previous != color) {
            return TextAttributeValue::Mixed;
        }
        result = Some(color);
    }
    result.unwrap_or(TextAttributeValue::NotSupported)
}

fn brush_color(brush: Option<&Rc<dyn IBrush>>) -> TextAttributeValue {
    brush.and_then(|brush| brush.as_solid_color_brush()).map_or(
        TextAttributeValue::NotSupported,
        |brush| {
            let color = brush.color();
            TextAttributeValue::Color(Color::from_rgb(color.r, color.g, color.b))
        },
    )
}
