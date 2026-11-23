//! The text pattern of a plain `TextBox`. Its text is one run of UTF-16 code units, so
//! a range is a pair of offsets into that text; lines and geometry come from the layout
//! the text presenter draws. Unit boundaries, moves and the single selection are the
//! ones of the rich text range (`text_range_support`).
//!
//! A password box discloses no text: the peer offers no text provider for it, and a
//! provider or range retained from before answers that its element is not available.

use super::text_range_support::{
    brush_color, endpoint_distance, enclosing_unit, horizontal_text_alignment, move_position,
    move_range_by, paragraph_units, selection_with, selection_without, truncate_utf16,
    ViewGeometry,
};
use super::{
    ITextProvider, ITextProvider2, ITextRangeProvider, ProviderAdapter, ProviderError,
    SupportedTextSelection, TextAttribute, TextAttributeValue, TextCaretRange,
    TextPatternRangeEndpoint, TextProviderError, TextUnit,
};
use crate::automation::peers::{AutomationPeer, TextBoxAutomationPeer};
use crate::automation::ElementNotEnabledError;
use crate::presenters::TextPresenter;
use crate::rich_text::{find_in_plain_text, rich_text_unit_ranges, UnicodeTextUnit};
use crate::TextBox;
use ferroui_base::input::text_input::TextSelection;
use ferroui_base::media::text_formatting::TextLayout;
use ferroui_base::media::{FlowDirection, FontStyle};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Point, Rect, Ref, WeakRef};
use std::any::Any;
use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;

#[derive(Clone)]
struct PlainTextContext {
    owner: WeakRef<TextBox>,
    peer: WeakRef<AutomationPeer>,
}

impl PlainTextContext {
    fn from_peer(peer: &TextBoxAutomationPeer) -> Result<Self, ProviderError> {
        let owner = peer
            .accessible_text_owner()
            .ok_or(TextProviderError::ElementNotAvailable)?;
        Ok(Self {
            owner: owner.downgrade(),
            peer: peer.to_ref().upcast::<AutomationPeer>().downgrade(),
        })
    }

    /// The text box while it discloses its text, which a password box never does.
    fn owner(&self) -> Result<Ref<TextBox>, ProviderError> {
        self.owner
            .upgrade()
            .filter(|owner| discloses_text(owner))
            .ok_or_else(|| TextProviderError::ElementNotAvailable.into())
    }

    fn peer(&self) -> Result<Ref<AutomationPeer>, ProviderError> {
        self.owner()?;
        self.peer
            .upgrade()
            .ok_or_else(|| TextProviderError::ElementNotAvailable.into())
    }

    fn text(&self) -> Result<Vec<u16>, ProviderError> {
        let owner = self.owner()?;
        Ok(utf16(&owner))
    }

    fn make_range(&self, range: Range<usize>) -> Rc<dyn ITextRangeProvider> {
        Rc::new(PlainTextAutomationRange {
            context: self.clone(),
            start: Cell::new(range.start),
            end: Cell::new(range.end.max(range.start)),
        })
    }
}

fn discloses_text(owner: &TextBox) -> bool {
    owner.password_char() == '\0'
}

fn utf16(owner: &TextBox) -> Vec<u16> {
    owner
        .text()
        .map(|text| text.encode_utf16().collect())
        .unwrap_or_default()
}

fn selection(owner: &TextBox) -> Range<usize> {
    let (start, end) = (owner.selection_start().max(0) as usize, owner.selection_end().max(0) as usize);
    start.min(end)..start.max(end)
}

/// The layout the presenter of a text box draws, once the presenter has been arranged.
/// While an input method composes, the layout also holds its preview at the caret; text
/// positions of the text box are mapped around it.
struct PresenterLayout {
    presenter: Ref<TextPresenter>,
    layout: Rc<TextLayout>,
    preview: Range<usize>,
}

impl PresenterLayout {
    fn of(owner: &TextBox) -> Option<Self> {
        let presenter = owner.presenter()?;
        presenter.previous_arrange()?;
        let caret = presenter.caret_index().max(0) as usize;
        let preview = presenter
            .preedit_text()
            .map_or(0, |preview| preview.encode_utf16().count());
        Some(Self {
            layout: presenter.text_layout(),
            presenter,
            preview: caret..caret + preview,
        })
    }

    /// The layout position of a text position; `after_preview` places the caret
    /// position after the preview (the start of a range) instead of before it.
    fn layout_position(&self, position: usize, after_preview: bool) -> usize {
        if position > self.preview.start || (after_preview && position == self.preview.start) {
            position + self.preview.len()
        } else {
            position
        }
    }

    fn text_position(&self, position: usize) -> usize {
        if position >= self.preview.end {
            position - self.preview.len()
        } else {
            position.min(self.preview.start)
        }
    }

    fn geometry(&self) -> Option<ViewGeometry> {
        let root = self.presenter.visual_root()?;
        ViewGeometry::in_root(&self.presenter, &root)
    }

    /// The lines of the text, when the layout holds exactly the text of the box.
    fn lines(&self, length: usize) -> Option<Vec<Range<usize>>> {
        if !self.preview.is_empty() {
            return None;
        }
        let lines: Vec<_> = self
            .layout
            .text_lines()
            .iter()
            .map(|line| {
                let start = (line.first_text_source_index().max(0) as usize).min(length);
                start..(start + line.length().max(0) as usize).min(length)
            })
            .collect();
        let complete = lines.first().is_some_and(|line| line.start == 0)
            && lines.last().is_some_and(|line| line.end == length);
        complete.then_some(lines)
    }

    /// The rectangles of a range of the text, in presenter coordinates.
    fn rectangles(&self, range: &Range<usize>) -> Vec<Rect> {
        let start = self.layout_position(range.start, true);
        let end = self.layout_position(range.end, false).max(start);
        self.layout
            .hit_test_text_range(start as i32, (end - start) as i32)
    }
}

impl ITextProvider for ProviderAdapter<TextBoxAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn document_range(&self) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        let context = PlainTextContext::from_peer(&self.0)?;
        let length = context.text()?.len();
        Ok(context.make_range(0..length))
    }

    fn get_selection(&self) -> Result<Vec<Rc<dyn ITextRangeProvider>>, ProviderError> {
        let context = PlainTextContext::from_peer(&self.0)?;
        let owner = context.owner()?;
        let length = utf16(&owner).len();
        let selected = selection(&owner);
        Ok(vec![context.make_range(selected.start.min(length)..selected.end.min(length))])
    }

    fn get_visible_ranges(&self) -> Result<Vec<Rc<dyn ITextRangeProvider>>, ProviderError> {
        let context = PlainTextContext::from_peer(&self.0)?;
        let owner = context.owner()?;
        let length = utf16(&owner).len();
        let mut visible: Vec<Range<usize>> = Vec::new();
        if let Some(layout) = PresenterLayout::of(&owner) {
            if let (Some(geometry), Some(lines)) = (layout.geometry(), layout.lines(length)) {
                for (index, (line, range)) in layout.layout.text_lines().iter().zip(lines).enumerate() {
                    let origin = layout.layout.line_origin(index);
                    let bounds = Rect::new(
                        origin.x + line.start(),
                        origin.y,
                        line.width_including_trailing_whitespace().max(1.0),
                        line.height(),
                    );
                    if range.is_empty() || geometry.visible(bounds).is_empty() {
                        continue;
                    }
                    if let Some(previous) = visible.last_mut().filter(|last| last.end == range.start) {
                        previous.end = range.end;
                    } else {
                        visible.push(range);
                    }
                }
            }
        }
        // UIA requires a degenerate range when the viewport has no visible text.
        if visible.is_empty() {
            visible.push(0..0);
        }
        Ok(visible.into_iter().map(|range| context.make_range(range)).collect())
    }

    fn range_from_child(
        &self,
        _child: &Ref<AutomationPeer>,
    ) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        PlainTextContext::from_peer(&self.0)?.owner()?;
        // Plain text has no embedded objects.
        Err(TextProviderError::InvalidArgument.into())
    }

    fn range_from_point(&self, point: Point) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(TextProviderError::InvalidArgument.into());
        }
        let context = PlainTextContext::from_peer(&self.0)?;
        let owner = context.owner()?;
        let layout = PresenterLayout::of(&owner).ok_or(TextProviderError::GeometryUnavailable)?;
        layout.geometry().ok_or(TextProviderError::GeometryUnavailable)?;
        let root = layout
            .presenter
            .visual_root()
            .ok_or(TextProviderError::GeometryUnavailable)?;
        let transform = root
            .transform_to_visual(&layout.presenter)
            .ok_or(TextProviderError::GeometryUnavailable)?;
        // The nearest position, also for a point outside the text.
        let hit = layout.layout.hit_test_point(point.transform(transform)).character_hit();
        let position = (hit.first_character_index() + hit.trailing_length()).max(0) as usize;
        let position = layout.text_position(position).min(utf16(&owner).len());
        Ok(context.make_range(position..position))
    }

    fn supported_text_selection(&self) -> Result<SupportedTextSelection, ProviderError> {
        PlainTextContext::from_peer(&self.0)?.owner()?;
        Ok(SupportedTextSelection::Single)
    }

    fn text_changed(&self, handler: Rc<dyn Fn()>) -> Result<Rc<dyn IDisposable>, ProviderError> {
        let owner = PlainTextContext::from_peer(&self.0)?.owner()?;
        let weak = owner.downgrade();
        Ok(owner.property_changed(move |event| {
            if event.property() == TextBox::text_property().as_property()
                && weak.upgrade().is_some_and(|owner| discloses_text(&owner))
            {
                handler();
            }
        }))
    }

    fn text_selection_changed(
        &self,
        handler: Rc<dyn Fn()>,
    ) -> Result<Rc<dyn IDisposable>, ProviderError> {
        let owner = PlainTextContext::from_peer(&self.0)?.owner()?;
        let weak = owner.downgrade();
        // Every move of the caret is a selection change of a text box: the caret is
        // an endpoint of the selection, and an empty selection moves with it.
        Ok(owner
            .selection_changed(move || {
                if weak.upgrade().is_some_and(|owner| discloses_text(&owner)) {
                    handler();
                }
            })
            .detach())
    }
}

impl ITextProvider2 for ProviderAdapter<TextBoxAutomationPeer> {
    fn range_from_annotation(
        &self,
        _annotation: &Ref<AutomationPeer>,
    ) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        PlainTextContext::from_peer(&self.0)?.owner()?;
        // Plain text has no annotations.
        Err(TextProviderError::InvalidArgument.into())
    }

    fn get_caret_range(&self) -> Result<TextCaretRange, ProviderError> {
        let context = PlainTextContext::from_peer(&self.0)?;
        let owner = context.owner()?;
        let caret = (owner.caret_index().max(0) as usize).min(utf16(&owner).len());
        Ok(TextCaretRange {
            is_active: owner.is_focused() && owner.is_effectively_enabled(),
            range: Some(context.make_range(caret..caret)),
        })
    }
}

/// A range of the text of a text box. Its offsets stay where they are when the text
/// changes, and are limited to the length of the current text.
struct PlainTextAutomationRange {
    context: PlainTextContext,
    start: Cell<usize>,
    end: Cell<usize>,
}

impl PlainTextAutomationRange {
    fn range(&self) -> Result<Range<usize>, ProviderError> {
        let length = self.context.text()?.len();
        let end = self.end.get().min(length);
        Ok(self.start.get().min(end)..end)
    }

    fn set(&self, range: Range<usize>) {
        self.start.set(range.start);
        self.end.set(range.end.max(range.start));
    }

    fn compatible<'a>(&self, other: &'a dyn ITextRangeProvider) -> Result<&'a Self, ProviderError> {
        let other = other
            .as_any()
            .downcast_ref::<Self>()
            .ok_or(TextProviderError::IncompatibleRange)?;
        if self.context.peer()? != other.context.peer()? {
            return Err(TextProviderError::IncompatibleRange.into());
        }
        Ok(other)
    }

    fn endpoint(&self, endpoint: TextPatternRangeEndpoint) -> Result<usize, ProviderError> {
        let range = self.range()?;
        Ok(match endpoint {
            TextPatternRangeEndpoint::Start => range.start,
            TextPatternRangeEndpoint::End => range.end,
        })
    }

    fn set_endpoint(
        &self,
        endpoint: TextPatternRangeEndpoint,
        position: usize,
    ) -> Result<(), ProviderError> {
        let range = self.range()?;
        let position = position.min(self.context.text()?.len());
        self.set(match endpoint {
            TextPatternRangeEndpoint::Start => position..range.end.max(position),
            TextPatternRangeEndpoint::End => range.start.min(position)..position,
        });
        Ok(())
    }

    fn select_range(&self, range: Range<usize>) -> Result<(), ProviderError> {
        let owner = self.context.owner()?;
        if !owner.is_effectively_enabled() {
            return Err(Box::new(ElementNotEnabledError::new()));
        }
        owner.set_selection_and_caret(
            TextSelection::new(range.start as i32, range.end as i32),
            range.end as i32,
        );
        Ok(())
    }

    fn units(&self, unit: TextUnit) -> Result<Vec<Range<usize>>, ProviderError> {
        let owner = self.context.owner()?;
        let text = owner.text().unwrap_or_default();
        let length = text.encode_utf16().count();
        Ok(match unit {
            TextUnit::Character => rich_text_unit_ranges(&text, UnicodeTextUnit::Character),
            TextUnit::Word => rich_text_unit_ranges(&text, UnicodeTextUnit::Word),
            // Text that is not laid out has no line boundaries. UIA promotes that
            // unit to the next larger supported unit.
            TextUnit::Line => PresenterLayout::of(&owner)
                .and_then(|layout| layout.lines(length))
                .unwrap_or_else(|| paragraph_units(&text)),
            TextUnit::Paragraph => paragraph_units(&text),
            // One format and one page: the whole text.
            TextUnit::Format | TextUnit::Page | TextUnit::Document => vec![0..length],
        })
    }

    fn attribute(&self, attribute: TextAttribute) -> Result<TextAttributeValue, ProviderError> {
        let owner = self.context.owner()?;
        Ok(attribute_value(&owner, attribute))
    }
}

impl ITextRangeProvider for PlainTextAutomationRange {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn peer(&self) -> Result<Ref<AutomationPeer>, ProviderError> {
        self.context.peer()
    }

    fn clone_range(&self) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        Ok(self.context.make_range(self.range()?))
    }

    fn compare(&self, range: &dyn ITextRangeProvider) -> Result<bool, ProviderError> {
        Ok(self.range()? == self.compatible(range)?.range()?)
    }

    fn compare_endpoints(
        &self,
        endpoint: TextPatternRangeEndpoint,
        range: &dyn ITextRangeProvider,
        target_endpoint: TextPatternRangeEndpoint,
    ) -> Result<i32, ProviderError> {
        let left = self.endpoint(endpoint)?;
        let right = self.compatible(range)?.endpoint(target_endpoint)?;
        Ok(endpoint_distance(left, right))
    }

    fn expand_to_enclosing_unit(&self, unit: TextUnit) -> Result<(), ProviderError> {
        let units = self.units(unit)?;
        if let Some(unit) = enclosing_unit(&units, self.range()?.start) {
            self.set(unit.clone());
        }
        Ok(())
    }

    fn find_attribute(
        &self,
        attribute: TextAttribute,
        value: &TextAttributeValue,
        _backwards: bool,
    ) -> Result<Option<Rc<dyn ITextRangeProvider>>, ProviderError> {
        if matches!(attribute, TextAttribute::Unknown(_)) {
            return Err(TextProviderError::UnsupportedAttribute.into());
        }
        if matches!(value, TextAttributeValue::Mixed | TextAttributeValue::NotSupported) {
            return Err(TextProviderError::InvalidArgument.into());
        }
        let actual = self.attribute(attribute)?;
        if actual == TextAttributeValue::NotSupported {
            return Err(TextProviderError::UnsupportedAttribute.into());
        }
        // The text has one format: the whole range matches, or nothing does.
        let range = self.range()?;
        Ok((!range.is_empty() && actual == *value).then(|| self.context.make_range(range)))
    }

    fn find_text(
        &self,
        text: &str,
        backwards: bool,
        ignore_case: bool,
    ) -> Result<Option<Rc<dyn ITextRangeProvider>>, ProviderError> {
        if text.is_empty() {
            return Err(TextProviderError::InvalidArgument.into());
        }
        let range = self.range()?;
        let source = String::from_utf16_lossy(&self.context.text()?[range.clone()]);
        Ok(
            find_in_plain_text(&source, text, !ignore_case, backwards, |_| true).map(|found| {
                self.context
                    .make_range(range.start + found.start..range.start + found.end)
            }),
        )
    }

    fn get_attribute_value(
        &self,
        attribute: TextAttribute,
    ) -> Result<TextAttributeValue, ProviderError> {
        self.attribute(attribute)
    }

    fn get_bounding_rectangles(&self) -> Result<Vec<Rect>, ProviderError> {
        let range = self.range()?;
        let owner = self.context.owner()?;
        if range.is_empty() {
            return Ok(Vec::new());
        }
        let Some(layout) = PresenterLayout::of(&owner) else {
            return Ok(Vec::new());
        };
        let Some(geometry) = layout.geometry() else {
            return Ok(Vec::new());
        };
        Ok(layout
            .rectangles(&range)
            .into_iter()
            .map(|rect| geometry.visible(rect))
            .filter(|rect| !rect.is_empty())
            .collect())
    }

    fn get_enclosing_element(&self) -> Result<Ref<AutomationPeer>, ProviderError> {
        self.context.peer()
    }

    fn get_text(&self, max_length: Option<usize>) -> Result<String, ProviderError> {
        let range = self.range()?;
        let text = String::from_utf16_lossy(&self.context.text()?[range]);
        Ok(truncate_utf16(text, max_length))
    }

    fn move_by(&self, unit: TextUnit, count: i32) -> Result<i32, ProviderError> {
        let units = self.units(unit)?;
        let (target, moved) = move_range_by(units, self.range()?, count);
        if let Some(target) = target {
            self.set(target);
        }
        Ok(moved)
    }

    fn move_endpoint_by_unit(
        &self,
        endpoint: TextPatternRangeEndpoint,
        unit: TextUnit,
        count: i32,
    ) -> Result<i32, ProviderError> {
        let units = self.units(unit)?;
        let (position, moved) = move_position(&units, self.endpoint(endpoint)?, count);
        self.set_endpoint(endpoint, position)?;
        Ok(moved)
    }

    fn move_endpoint_by_range(
        &self,
        endpoint: TextPatternRangeEndpoint,
        range: &dyn ITextRangeProvider,
        target_endpoint: TextPatternRangeEndpoint,
    ) -> Result<(), ProviderError> {
        let position = self.compatible(range)?.endpoint(target_endpoint)?;
        self.set_endpoint(endpoint, position)
    }

    fn select(&self) -> Result<(), ProviderError> {
        self.select_range(self.range()?)
    }

    fn add_to_selection(&self) -> Result<(), ProviderError> {
        let owner = self.context.owner()?;
        let current = selection(&owner);
        self.select_range(selection_with(current, self.range()?)?)
    }

    fn remove_from_selection(&self) -> Result<(), ProviderError> {
        let owner = self.context.owner()?;
        let current = selection(&owner);
        self.select_range(selection_without(current, self.range()?)?)
    }

    fn scroll_into_view(&self, align_to_top: bool) -> Result<(), ProviderError> {
        let range = self.range()?;
        let owner = self.context.owner()?;
        let layout = PresenterLayout::of(&owner).ok_or(TextProviderError::GeometryUnavailable)?;
        if !layout.presenter.is_effectively_visible() {
            return Err(TextProviderError::GeometryUnavailable.into());
        }
        let rectangle = if range.is_empty() {
            Some(layout.layout.hit_test_text_position(layout.layout_position(range.start, false) as i32))
        } else {
            let rectangles = layout.rectangles(&range);
            if align_to_top {
                rectangles.into_iter().next()
            } else {
                rectangles.into_iter().next_back()
            }
        };
        let rectangle = rectangle
            .filter(|rectangle| rectangle.height > 0.0)
            .ok_or(TextProviderError::GeometryUnavailable)?;
        layout.presenter.bring_into_view_rect(rectangle);
        Ok(())
    }

    fn get_children(&self) -> Result<Vec<Ref<AutomationPeer>>, ProviderError> {
        self.context.owner()?;
        Ok(Vec::new())
    }
}

/// The attributes of the text of a text box: the text has the format of the control.
fn attribute_value(owner: &TextBox, attribute: TextAttribute) -> TextAttributeValue {
    use TextAttribute as A;
    use TextAttributeValue as V;
    let rtl = owner.flow_direction() == FlowDirection::RightToLeft;
    match attribute {
        A::FontName => V::String(owner.font_family().name().to_owned()),
        A::FontSize => V::Number(owner.font_size() * 72.0 / 96.0),
        A::FontWeight => V::Integer(owner.font_weight().0),
        A::IsItalic => V::Bool(owner.font_style() != FontStyle::Normal),
        A::IsReadOnly => V::Bool(owner.is_read_only()),
        A::IsHidden => V::Bool(!owner.is_effectively_visible()),
        A::ForegroundColor => brush_color(owner.foreground().as_ref()),
        A::HorizontalTextAlignment => V::Integer(horizontal_text_alignment(owner.text_alignment(), rtl)),
        A::TextFlowDirections => V::Integer(i32::from(rtl)),
        _ => V::NotSupported,
    }
}
