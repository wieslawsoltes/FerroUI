//! The port of `AtSpiTextHandler.cs`: `org.a11y.atspi.Text` over the
//! ranges of a text provider or the value of a value provider.
//!
//! The wire interface counts Unicode scalar values. Native text providers
//! use UTF-16 code units for their ranges.

use super::at_spi_coordinate_helper::AtSpiCoordinateHelper;
use super::at_spi_event_object_handler::AtSpiEventObjectHandler;
use super::node_of;
use crate::at_spi::at_spi_constants::TEXT_VERSION;
use crate::at_spi::at_spi_coord_type::AtSpiCoordType;
use crate::at_spi::at_spi_node::AtSpiNode;
use crate::at_spi::dbus::descriptions::TEXT;
use crate::at_spi::dbus::interface::{args, reply, CallResult, DBusError, DBusInterface, InterfaceDescription};
use crate::at_spi::dbus::types::{AtSpiAttributeSet, AtSpiTextRange};
use ferroui_base::media::text_formatting::unicode::SentenceBreakEnumerator;
use ferroui_base::reactive::Subscription;
use ferroui_base::{Point, Rect};
use ferroui_controls::automation::provider::{
    ITextProvider, ITextProvider2, ITextRangeProvider, IValueProvider, TextPatternRangeEndpoint,
    TextScrollPlacement, TextUnit,
};
use std::cell::{OnceCell, RefCell};
use std::ops::Range;
use std::rc::{Rc, Weak};
use zbus::{OwnedVariant, Variant};

mod attributes;
mod boundaries;
mod bounds;
use boundaries::Direction;

// `TextGranularity`
const GRANULARITY_CHAR: u32 = 0;
const GRANULARITY_WORD: u32 = 1;
const GRANULARITY_SENTENCE: u32 = 2;
const GRANULARITY_LINE: u32 = 3;
const GRANULARITY_PARAGRAPH: u32 = 4;

// `TextBoundaryType`
const BOUNDARY_CHAR: u32 = 0;
const BOUNDARY_WORD_START: u32 = 1;
const BOUNDARY_WORD_END: u32 = 2;
const BOUNDARY_SENTENCE_START: u32 = 3;
const BOUNDARY_SENTENCE_END: u32 = 4;
const BOUNDARY_LINE_START: u32 = 5;
const BOUNDARY_LINE_END: u32 = 6;

/// A piece of text with the offsets of its start and its end.
pub(crate) type TextSegment = (String, i32, i32);

struct AccessibleText {
    units: Vec<u16>,
    offsets: Vec<usize>,
}

impl AccessibleText {
    fn new(text: &str) -> Self {
        let mut units = Vec::new();
        let mut offsets = Vec::new();
        for character in text.chars() {
            offsets.push(units.len());
            let character = if character == '\0' { '\u{fffd}' } else { character };
            units.extend_from_slice(character.encode_utf16(&mut [0; 2]));
        }
        offsets.push(units.len());
        Self { units, offsets }
    }

    fn len(&self) -> i32 {
        to_offset(self.offsets.len() - 1)
    }

    fn utf16_offset(&self, offset: usize) -> Option<usize> {
        self.offsets.get(offset).copied()
    }

    fn character_offset(&self, offset: usize) -> Option<usize> {
        (offset <= self.units.len())
            .then(|| self.offsets.partition_point(|&unit| unit <= offset) - 1)
    }

    fn segment(&self, start: i32, end: i32) -> TextSegment {
        let start_unit = self.offsets[start as usize];
        let end_unit = self.offsets[end as usize];
        (String::from_utf16_lossy(&self.units[start_unit..end_unit]), start, end)
    }

    fn unit_segment(&self, start: i32, end: i32) -> TextSegment {
        let start = self.offsets.partition_point(|&unit| unit <= start as usize) - 1;
        let end = self.offsets.partition_point(|&unit| unit < end as usize);
        self.segment(to_offset(start), to_offset(end))
    }
}

fn empty() -> TextSegment {
    (String::new(), 0, 0)
}

fn get_string_at_offset(text: &AccessibleText, offset: i32, granularity: u32) -> TextSegment {
    let len = text.len();
    if len == 0 {
        return empty();
    }
    let offset = offset.clamp(0, len - 1);
    match granularity {
        GRANULARITY_CHAR => text.segment(offset, offset + 1),
        GRANULARITY_WORD => boundaries::segment(text, offset, BOUNDARY_WORD_START, Direction::At),
        GRANULARITY_SENTENCE => sentence_segment(text, offset),
        GRANULARITY_LINE | GRANULARITY_PARAGRAPH => line_segment(text, offset),
        _ => text.segment(0, len),
    }
}

fn sentence_segment(text: &AccessibleText, offset: i32) -> TextSegment {
    let position = text.offsets[offset as usize];
    let mut sentences = SentenceBreakEnumerator::new(&text.units);
    while let Some(sentence) = sentences.move_next() {
        let end = sentence.offset() + sentence.text().len();
        if position < end {
            return text.unit_segment(to_offset(sentence.offset()), to_offset(end));
        }
    }
    empty()
}

fn line_segment(text: &AccessibleText, offset: i32) -> TextSegment {
    let units = &text.units;
    let mut position = text.offsets[offset as usize];
    if position > 0 && units.get(position) == Some(&10) && units[position - 1] == 13 {
        position -= 1;
    }
    let is_break = |&unit: &u16| matches!(unit, 10 | 13 | 0x85 | 0x2028 | 0x2029);
    let start = units[..position].iter().rposition(is_break).map_or(0, |index| index + 1);
    let end = units[position..].iter().position(is_break).map_or(units.len(), |offset| {
        let end = position + offset;
        end + 1 + usize::from(units[end] == 13 && units.get(end + 1) == Some(&10))
    });
    text.unit_segment(to_offset(start), to_offset(end))
}

fn requested_range(length: i32, start: i32, end: i32) -> Range<i32> {
    start.clamp(0, length)..if end < 0 { length } else { end.min(length) }
}

fn scalar_range(length: i32, offset: i32) -> Range<i32> {
    if (0..length).contains(&offset) { offset..offset + 1 } else { length..length }
}

fn get_text(text: &AccessibleText, start_offset: i32, end_offset: i32) -> String {
    let span = requested_range(text.len(), start_offset, end_offset);
    if span.is_empty() {
        return String::new();
    }
    text.segment(span.start, span.end).0
}


/// The Unicode scalar at an offset, or -1 for an offset outside the text.
fn get_character_at_offset(text: &AccessibleText, offset: i32) -> i32 {
    let Ok(start) = usize::try_from(offset) else { return -1 };
    let Some(&end) = text.offsets.get(start + 1) else { return -1 };
    char::decode_utf16(text.units[text.offsets[start]..end].iter().copied())
        .next().and_then(Result::ok).map_or(-1, |character| character as i32)
}

pub(crate) struct AtSpiTextHandler {
    node: Weak<AtSpiNode>,
    selection_subscription: OnceCell<Subscription>,
    edit_subscription: OnceCell<Subscription>,
    attributes_subscription: OnceCell<Subscription>,
}

type Extents = (i32, i32, i32, i32);
type AttributeRun = (AtSpiAttributeSet, i32, i32);

impl AtSpiTextHandler {
    pub(crate) fn new(node: Weak<AtSpiNode>) -> Self {
        Self {
            node,
            selection_subscription: OnceCell::new(),
            edit_subscription: OnceCell::new(),
            attributes_subscription: OnceCell::new(),
        }
    }

    pub(crate) fn observe_text(self: &Rc<Self>) {
        let Some(provider) = self.text_provider() else { return };
        let previous = RefCell::new(self.selection_state());
        let weak = Rc::downgrade(self);
        let handler = Rc::new(move || {
            let Some(text) = weak.upgrade() else { return };
            let Some(events) = text.event_handler() else {
                return;
            };
            let current = text.selection_state();
            let (caret, selection_changed) = {
                let mut last = previous.borrow_mut();
                let caret = current.0.filter(|position| last.0 != Some(*position));
                let selection_changed = current.1 != last.1;
                *last = current;
                (caret, selection_changed)
            };
            if let Some(caret) = caret {
                events.emit_text_caret_moved_signal(caret);
            }
            if selection_changed {
                events.emit_text_selection_changed_signal();
            }
        });
        if let Ok(subscription) = provider.text_selection_changed(handler) {
            let _ = self.selection_subscription.set(Subscription::new(subscription));
        }

        let weak = Rc::downgrade(self);
        if let Ok(Some(subscription)) = provider.text_edited(Rc::new(move |edit| {
            let events = weak.upgrade().and_then(|text| text.event_handler());
            let Some(events) = events else { return };
            let offset = to_offset(edit.character_offset);
            for (detail, text) in [("delete", &edit.removed), ("insert", &edit.inserted)] {
                if !text.is_empty() {
                    let length = to_offset(text.chars().count());
                    events.emit_text_changed_signal(detail, offset, length, text);
                }
            }
        })) {
            let _ = self.edit_subscription.set(Subscription::new(subscription));
        }

        let weak = Rc::downgrade(self);
        if let Ok(Some(subscription)) = provider.text_attributes_changed(Rc::new(move || {
            if let Some(events) = weak.upgrade().and_then(|text| text.event_handler()) {
                events.emit_text_attributes_changed_signal();
            }
        })) {
            let _ = self.attributes_subscription.set(Subscription::new(subscription));
        }
    }

    fn event_handler(&self) -> Option<Rc<AtSpiEventObjectHandler>> {
        self.node.upgrade()
            .filter(|node| node.is_attached())
            .and_then(|node| node.event_object_handler())
    }

    pub(crate) fn version(&self) -> u32 {
        TEXT_VERSION
    }

    pub(crate) fn character_count(&self) -> i32 {
        self.indexed_provider().map_or_else(|| self.get_text().len(), |(_, length)| length)
    }

    pub(crate) fn caret_offset(&self) -> i32 {
        self.current_caret_offset().unwrap_or(0)
    }

    fn current_caret_offset(&self) -> Option<i32> {
        self.node
            .upgrade()
            .and_then(|node| node.peer().get_provider::<dyn ITextProvider2>())
            .and_then(|provider| provider.get_caret_range().ok()?.range)
            .and_then(|range| range.text_offsets().ok().flatten())
            .and_then(|offsets| self.character_offset(offsets.start))
            .map(to_offset)
    }

    pub(crate) fn get_string_at_offset_async(&self, offset: i32, granularity: u32) -> TextSegment {
        let nearest = |length: i32| offset.clamp(0, (length - 1).max(0));
        if granularity == GRANULARITY_CHAR {
            if let Some(segment) = self.indexed_segment(|length| {
                scalar_range(length, nearest(length))
            }) {
                return segment;
            }
        }
        let boundary = match granularity {
            GRANULARITY_WORD => Some(BOUNDARY_WORD_START),
            GRANULARITY_SENTENCE => Some(BOUNDARY_SENTENCE_START),
            _ => None,
        };
        if let Some(segment) = boundary
            .and_then(|boundary| self.indexed_boundary_segment(nearest, boundary, Direction::At))
        {
            return segment;
        }
        let unit = match granularity {
            GRANULARITY_LINE => Some(TextUnit::Line),
            GRANULARITY_PARAGRAPH => Some(TextUnit::Paragraph),
            _ => None,
        };
        if let Some(segment) = unit.and_then(|unit| self.text_unit_at_offset(offset, unit)) {
            return segment;
        }
        get_string_at_offset(&self.get_text(), offset, granularity)
    }

    fn text_unit_at_offset(&self, offset: i32, unit: TextUnit) -> Option<TextSegment> {
        let range = self.range(offset, offset)?;
        range.expand_to_enclosing_unit(unit).ok()?;
        let offsets = range.text_offsets().ok().flatten()?;
        let start = self.character_offset(offsets.start)?;
        let end = self.character_offset(offsets.end)?;
        let text = range.get_text(None).ok()?.replace('\0', "\u{fffd}");
        Some((text, to_offset(start), to_offset(end)))
    }

    pub(crate) fn get_text_async(&self, start_offset: i32, end_offset: i32) -> String {
        if let Some((text, _, _)) = self.indexed_segment(|length| {
            requested_range(length, start_offset, end_offset)
        }) {
            return text;
        }
        get_text(&self.get_text(), start_offset, end_offset)
    }

    pub(crate) fn set_caret_offset_async(&self, offset: i32) -> bool {
        self.range(offset, offset).is_some_and(|range| range.select().is_ok())
    }

    pub(crate) fn get_text_before_offset_async(&self, offset: i32, boundary_type: u32) -> TextSegment {
        self.legacy_text_segment(offset, boundary_type, Direction::Before)
    }

    pub(crate) fn get_text_at_offset_async(&self, offset: i32, boundary_type: u32) -> TextSegment {
        self.legacy_text_segment(offset, boundary_type, Direction::At)
    }

    pub(crate) fn get_text_after_offset_async(&self, offset: i32, boundary_type: u32) -> TextSegment {
        self.legacy_text_segment(offset, boundary_type, Direction::After)
    }

    fn legacy_text_segment(&self, offset: i32, boundary_type: u32, direction: Direction) -> TextSegment {
        if boundary_type == BOUNDARY_CHAR {
            if let Some(segment) = self.indexed_segment(|length| {
                boundaries::character_range(length, offset, direction)
            }) {
                return segment;
            }
        }
        if matches!(boundary_type, BOUNDARY_WORD_START | BOUNDARY_WORD_END | BOUNDARY_SENTENCE_START | BOUNDARY_SENTENCE_END) {
            if let Some(segment) = self.indexed_boundary_segment(|_| offset, boundary_type, direction) {
                return segment;
            }
        }
        if !matches!(boundary_type, BOUNDARY_LINE_START | BOUNDARY_LINE_END) {
            return boundaries::segment(&self.get_text(), offset, boundary_type, direction);
        }
        let span = boundaries::line_range(
            offset,
            self.character_count(),
            boundary_type == BOUNDARY_LINE_END,
            direction,
            |offset| self.get_string_at_offset_async(offset, GRANULARITY_LINE),
        );
        let text = self.range(span.start, span.end)
            .and_then(|range| range.get_text(None).ok())
            .map(|text| text.replace('\0', "\u{fffd}"))
            .unwrap_or_else(|| self.get_text_async(span.start, span.end));
        (text, span.start, span.end)
    }

    pub(crate) fn get_character_at_offset_async(&self, offset: i32) -> i32 {
        if let Some((text, _, _)) = self.indexed_segment(|length| scalar_range(length, offset)) {
            return text.chars().next().map_or(-1, |character| character as i32);
        }
        get_character_at_offset(&self.get_text(), offset)
    }

    pub(crate) fn get_attribute_value_async(&self, offset: i32, attribute_name: &str) -> String {
        self.get_attributes_async(offset).0.remove(attribute_name).unwrap_or_default()
    }

    pub(crate) fn get_attributes_async(&self, offset: i32) -> AttributeRun {
        self.get_attribute_run_async(offset, false)
    }

    pub(crate) fn get_default_attributes_async(&self) -> AtSpiAttributeSet {
        self.text_provider()
            .and_then(|provider| provider.default_text_attributes().ok())
            .map(attributes::serialize)
            .unwrap_or_default()
    }

    pub(crate) fn get_character_extents_async(&self, offset: i32, coord_type: u32) -> Extents {
        offset.checked_add(1).map_or((0, 0, 0, 0), |end| {
            self.get_range_extents_async(offset, end, coord_type)
        })
    }

    pub(crate) fn get_offset_at_point_async(&self, x: i32, y: i32, coord_type: u32) -> i32 {
        self.offset_at_point(x, y, coord_type)
            .and_then(|offset| self.character_offset(offset))
            .map_or(-1, to_offset)
    }

    fn offset_at_point(&self, x: i32, y: i32, coord_type: u32) -> Option<usize> {
        let node = self.node.upgrade()?;
        let screen = AtSpiCoordinateHelper::screen_point(&node, x, y, coord_type);
        let bounds = AtSpiCoordinateHelper::get_screen_extents(&node);
        if !bounds.contains_exclusive(Point::new(f64::from(screen.x), f64::from(screen.y))) {
            return None;
        }
        let point = AtSpiCoordinateHelper::to_root_point(&node, screen)?;
        let provider = node.peer().get_provider::<dyn ITextProvider>()?;
        let caret = provider.range_from_point(point).ok()?;
        let character = caret.clone_range().ok()?;
        character.expand_to_enclosing_unit(TextUnit::Character).ok()?;

        // The nearest caret can be the trailing edge of the character under the point.
        character_offset_at_point(&*character, point).or_else(|| {
            let moved = caret
                .move_endpoint_by_unit(TextPatternRangeEndpoint::Start, TextUnit::Character, -1)
                .ok()?;
            if moved == -1 { character_offset_at_point(&*caret, point) } else { None }
        })
    }

    pub(crate) fn get_n_selections_async(&self) -> i32 {
        to_offset(self.selections().count())
    }

    pub(crate) fn get_selection_async(&self, selection_num: i32) -> (i32, i32) {
        self.selection(selection_num).map_or((0, 0), |(_, offsets)| {
            (to_offset(offsets.start), to_offset(offsets.end))
        })
    }

    pub(crate) fn add_selection_async(&self, start_offset: i32, end_offset: i32) -> bool {
        start_offset < end_offset
            && self
                .range(start_offset, end_offset)
                .is_some_and(|range| range.add_to_selection().is_ok())
    }

    pub(crate) fn remove_selection_async(&self, selection_num: i32) -> bool {
        self.selection(selection_num)
            .is_some_and(|(range, _)| range.remove_from_selection().is_ok())
    }

    pub(crate) fn set_selection_async(&self, selection_num: i32, start_offset: i32, end_offset: i32) -> bool {
        self.selection(selection_num)
            .zip(self.range(start_offset, end_offset))
            .is_some_and(|((current, _), replacement)| {
                current.replace_in_selection(&*replacement).unwrap_or(false)
            })
    }

    pub(crate) fn get_range_extents_async(&self, start_offset: i32, end_offset: i32, coord_type: u32) -> Extents {
        let Some(node) = self.node.upgrade() else {
            return (0, 0, 0, 0);
        };
        let rect = self
            .range(start_offset, end_offset)
            .and_then(|range| range.get_bounding_rectangles().ok())
            .and_then(|rectangles| rectangles.into_iter().reduce(|a, b| a.union(b)));
        rect.map_or((0, 0, 0, 0), |rect| {
            let screen = AtSpiCoordinateHelper::to_screen(&node, rect);
            let bounds = AtSpiCoordinateHelper::translate_rect(&node, screen, coord_type);
            (bounds.x as i32, bounds.y as i32, bounds.width as i32, bounds.height as i32)
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn get_bounded_ranges_async(
        &self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        coord_type: u32,
        x_clip_type: u32,
        y_clip_type: u32,
    ) -> Vec<AtSpiTextRange> {
        let (Some(node), Some(provider), Some(x_clip), Some(y_clip)) = (
            self.node.upgrade(), self.text_provider(), bounds::Clip::new(x_clip_type), bounds::Clip::new(y_clip_type),
        ) else { return Vec::new() };
        if width <= 0 || height <= 0 || AtSpiCoordType::from_u32(coord_type).is_none() {
            return Vec::new();
        }
        let origin = AtSpiCoordinateHelper::screen_point(&node, x, y, coord_type);
        let area = Rect::new(f64::from(origin.x), f64::from(origin.y), f64::from(width), f64::from(height));
        let query = bounds::BoundsQuery { node: &node, provider: &*provider, area, x_clip, y_clip };
        query.offsets().and_then(|spans| {
            spans.into_iter().map(|span| {
                let start = self.character_offset(span.start)?;
                let end = self.character_offset(span.end - 1)? + 1;
                let range = provider.range_from_offsets(span).ok().flatten()?;
                let contents = range.get_text(None).ok()?.replace('\0', "\u{fffd}");
                Some((to_offset(start), to_offset(end), contents, OwnedVariant::from(0_i32)))
            }).collect()
        }).unwrap_or_default()
    }

    pub(crate) fn get_attribute_run_async(&self, offset: i32, include_defaults: bool) -> AttributeRun {
        self.attribute_run(offset, include_defaults).unwrap_or_default()
    }

    fn attribute_run(&self, offset: i32, include_defaults: bool) -> Option<AttributeRun> {
        let index = usize::try_from(offset).ok()?;
        let provider = self.text_provider()?;
        let native = provider.utf16_offset_from_character(index).ok()?
            .or_else(|| self.get_text().utf16_offset(index))?;
        let Some(run) = provider.text_attribute_run(native).ok()? else {
            let defaults = if include_defaults { self.get_default_attributes_async() } else { AtSpiAttributeSet::new() };
            return Some((defaults, 0, self.character_count()));
        };
        // If formatting changes between surrogate halves, the scalar retains
        // the attributes of its first unit. Project both boundaries upwards.
        let boundary = |native: usize| native.checked_sub(1)
            .map_or(Some(0), |previous| self.character_offset(previous).map(|index| index + 1));
        let start = to_offset(boundary(run.range.start)?);
        let end = to_offset(boundary(run.range.end)?);
        let mut values = attributes::serialize(run.attributes);
        if !include_defaults {
            let defaults = attributes::serialize(provider.default_text_attributes().ok()?);
            values.retain(|name, value| defaults.get(name) != Some(value));
        }
        Some((values, start, end))
    }

    pub(crate) fn get_default_attribute_set_async(&self) -> AtSpiAttributeSet {
        self.get_default_attributes_async()
    }

    pub(crate) fn scroll_substring_to_async(&self, start_offset: i32, end_offset: i32, scroll_type: u32) -> bool {
        let placement = match scroll_type {
            0 => TextScrollPlacement::TopLeft,
            1 => TextScrollPlacement::BottomRight,
            2 => TextScrollPlacement::Top,
            3 => TextScrollPlacement::Bottom,
            4 => TextScrollPlacement::Left,
            5 => TextScrollPlacement::Right,
            6 => TextScrollPlacement::Nearest,
            _ => return false,
        };
        self.scroll_range(start_offset, end_offset, placement)
    }

    pub(crate) fn scroll_substring_to_point_async(
        &self,
        start_offset: i32,
        end_offset: i32,
        coord_type: u32,
        x: i32,
        y: i32,
    ) -> bool {
        let Some(node) = self.node.upgrade() else { return false };
        if AtSpiCoordType::from_u32(coord_type).is_none() {
            return false;
        }
        let screen = AtSpiCoordinateHelper::screen_point(&node, x, y, coord_type);
        let bounds = AtSpiCoordinateHelper::get_screen_extents(&node);
        if !bounds.contains_exclusive(Point::new(f64::from(screen.x), f64::from(screen.y))) {
            return false;
        }
        AtSpiCoordinateHelper::to_root_point(&node, screen).is_some_and(|point| {
            self.scroll_range(start_offset, end_offset, TextScrollPlacement::Point(point))
        })
    }

    fn scroll_range(&self, start: i32, end: i32, placement: TextScrollPlacement) -> bool {
        self.range(start, end).is_some_and(|range| range.scroll_to(placement).unwrap_or(false))
    }

    fn text_provider(&self) -> Option<Rc<dyn ITextProvider>> {
        self.node.upgrade()?.peer().get_provider::<dyn ITextProvider>()
    }

    fn indexed_provider(&self) -> Option<(Rc<dyn ITextProvider>, i32)> {
        let provider = self.text_provider()?;
        let length = to_offset(provider.text_character_count().ok().flatten()?);
        Some((provider, length))
    }

    fn indexed_segment(&self, select: impl FnOnce(i32) -> Range<i32>) -> Option<TextSegment> {
        let (provider, length) = self.indexed_provider()?;
        read_indexed_segment(&*provider, select(length))
    }

    fn indexed_boundary_segment(
        &self,
        request: impl FnOnce(i32) -> i32,
        boundary: u32,
        direction: Direction,
    ) -> Option<TextSegment> {
        let (provider, length) = self.indexed_provider()?;
        let span = boundaries::indexed_range(request(length), length, boundary, direction, |position| {
            let (text, start, end) = self.text_unit_at_offset(position, TextUnit::Paragraph)?;
            if start > 0 {
                let (previous, _, _) = read_indexed_segment(&*provider, start - 1..start)?;
                // A provider can return a visual row as a paragraph fallback.
                // Accept only hard boundaries, keeping CRLF in one paragraph.
                if !matches!(previous.as_str(), "\r" | "\n") || previous == "\r" && text.starts_with('\n') {
                    return None;
                }
            }
            Some((text, start, end))
        })?;
        read_indexed_segment(&*provider, span)
    }

    fn range(&self, start: i32, end: i32) -> Option<Rc<dyn ITextRangeProvider>> {
        let start = usize::try_from(start).ok()?;
        let end = usize::try_from(end).ok()?;
        if start > end {
            return None;
        }
        let provider = self.text_provider()?;
        let native_start = provider.utf16_offset_from_character(start).ok()?;
        let native_end = provider.utf16_offset_from_character(end).ok()?;
        let native_range = if let (Some(start), Some(end)) = (native_start, native_end) {
            start..end
        } else {
            let text = self.get_text();
            text.utf16_offset(start)?..text.utf16_offset(end)?
        };
        provider.range_from_offsets(native_range).ok().flatten()
    }

    fn character_offset(&self, offset: usize) -> Option<usize> {
        self.text_provider()?.character_offset_from_utf16(offset).ok()?
            .or_else(|| self.get_text().character_offset(offset))
    }

    fn selections(&self) -> impl Iterator<Item = (Rc<dyn ITextRangeProvider>, Range<usize>)> + '_ {
        self.text_provider()
            .and_then(|provider| provider.get_selection().ok())
            .into_iter()
            .flatten()
            .filter_map(|range| {
                let offsets = range.text_offsets().ok().flatten()?;
                if offsets.is_empty() {
                    return None;
                }
                let start = self.character_offset(offsets.start)?;
                let end = self.character_offset(offsets.end - 1)? + 1;
                Some((range, start..end))
            })
    }

    fn selection(&self, index: i32) -> Option<(Rc<dyn ITextRangeProvider>, Range<usize>)> {
        self.selections().nth(usize::try_from(index).ok()?)
    }

    fn selection_state(&self) -> (Option<i32>, Vec<Range<usize>>) {
        (
            self.current_caret_offset(),
            self.selections().map(|(_, offsets)| offsets).collect(),
        )
    }

    fn get_text(&self) -> AccessibleText {
        let Some(node) = self.node.upgrade() else {
            return AccessibleText::new("");
        };
        let text = if let Some(provider) = node.peer().get_provider::<dyn IValueProvider>() {
            provider.value()
        } else {
            self.text_provider()
                .and_then(|provider| provider.document_range().ok()?.get_text(None).ok())
        };
        AccessibleText::new(text.as_deref().unwrap_or_default())
    }
}

fn read_indexed_segment(provider: &dyn ITextProvider, span: Range<i32>) -> Option<TextSegment> {
    if span.is_empty() {
        return Some((String::new(), span.start, span.end));
    }
    let start = provider.utf16_offset_from_character(span.start as usize).ok().flatten()?;
    let end = provider.utf16_offset_from_character(span.end as usize).ok().flatten()?;
    let range = provider.range_from_offsets(start..end).ok().flatten()?;
    let mut text = range.get_text(None).ok()?;
    if text.contains('\0') {
        text = text.replace('\0', "\u{fffd}");
    }
    Some((text, span.start, span.end))
}

fn character_offset_at_point(range: &dyn ITextRangeProvider, point: Point) -> Option<usize> {
    let offsets = range.text_offsets().ok().flatten()?;
    if offsets.is_empty() {
        return None;
    }
    range.get_bounding_rectangles().ok()?.iter()
        .any(|bounds| bounds.contains_exclusive(point))
        .then_some(offsets.start)
}

fn to_offset(offset: usize) -> i32 {
    i32::try_from(offset).unwrap_or(i32::MAX)
}

impl DBusInterface for AtSpiTextHandler {
    fn description(&self) -> &'static InterfaceDescription {
        &TEXT
    }

    fn call(&self, member: &str, body: &zbus::message::Body) -> CallResult {
        node_of(&self.node)?;
        match member {
            "GetStringAtOffset" => {
                let (offset, granularity) = args::<(i32, u32)>(body)?;
                reply(self.get_string_at_offset_async(offset, granularity))
            }
            "GetText" => {
                let (start_offset, end_offset) = args::<(i32, i32)>(body)?;
                reply((self.get_text_async(start_offset, end_offset),))
            }
            "SetCaretOffset" => reply((self.set_caret_offset_async(args::<i32>(body)?),)),
            "GetTextBeforeOffset" => {
                let (offset, boundary_type) = args::<(i32, u32)>(body)?;
                reply(self.get_text_before_offset_async(offset, boundary_type))
            }
            "GetTextAtOffset" => {
                let (offset, boundary_type) = args::<(i32, u32)>(body)?;
                reply(self.get_text_at_offset_async(offset, boundary_type))
            }
            "GetTextAfterOffset" => {
                let (offset, boundary_type) = args::<(i32, u32)>(body)?;
                reply(self.get_text_after_offset_async(offset, boundary_type))
            }
            "GetCharacterAtOffset" => reply((self.get_character_at_offset_async(args::<i32>(body)?),)),
            "GetAttributeValue" => {
                let (offset, attribute_name) = args::<(i32, String)>(body)?;
                reply((self.get_attribute_value_async(offset, &attribute_name),))
            }
            "GetAttributes" => reply(self.get_attributes_async(args::<i32>(body)?)),
            "GetAttributeRun" => {
                let (offset, include_defaults) = args::<(i32, bool)>(body)?;
                reply(self.get_attribute_run_async(offset, include_defaults))
            }
            "GetDefaultAttributes" => reply((self.get_default_attributes_async(),)),
            "GetDefaultAttributeSet" => reply((self.get_default_attribute_set_async(),)),
            "GetCharacterExtents" => {
                let (offset, coord_type) = args::<(i32, u32)>(body)?;
                reply(self.get_character_extents_async(offset, coord_type))
            }
            "GetOffsetAtPoint" => {
                let (x, y, coord_type) = args::<(i32, i32, u32)>(body)?;
                reply((self.get_offset_at_point_async(x, y, coord_type),))
            }
            "GetNSelections" => reply((self.get_n_selections_async(),)),
            "GetSelection" => reply(self.get_selection_async(args::<i32>(body)?)),
            "AddSelection" => {
                let (start_offset, end_offset) = args::<(i32, i32)>(body)?;
                reply((self.add_selection_async(start_offset, end_offset),))
            }
            "RemoveSelection" => reply((self.remove_selection_async(args::<i32>(body)?),)),
            "SetSelection" => {
                let (selection_num, start_offset, end_offset) = args::<(i32, i32, i32)>(body)?;
                reply((self.set_selection_async(selection_num, start_offset, end_offset),))
            }
            "GetRangeExtents" => {
                let (start_offset, end_offset, coord_type) = args::<(i32, i32, u32)>(body)?;
                reply(self.get_range_extents_async(start_offset, end_offset, coord_type))
            }
            "GetBoundedRanges" => {
                let (x, y, width, height, coord_type, x_clip_type, y_clip_type) =
                    args::<(i32, i32, i32, i32, u32, u32, u32)>(body)?;
                reply((self.get_bounded_ranges_async(x, y, width, height, coord_type, x_clip_type, y_clip_type),))
            }
            "ScrollSubstringTo" => {
                let (start_offset, end_offset, scroll_type) = args::<(i32, i32, u32)>(body)?;
                reply((self.scroll_substring_to_async(start_offset, end_offset, scroll_type),))
            }
            "ScrollSubstringToPoint" => {
                let (start_offset, end_offset, coord_type, x, y) = args::<(i32, i32, u32, i32, i32)>(body)?;
                reply((self.scroll_substring_to_point_async(start_offset, end_offset, coord_type, x, y),))
            }
            _ => Err(DBusError::unknown_method()),
        }
    }

    fn get_property(&self, name: &str) -> Option<Variant<'static>> {
        self.node.upgrade()?;
        Some(match name {
            "version" => Variant::from(self.version()),
            "CharacterCount" => Variant::from(self.character_count()),
            "CaretOffset" => Variant::from(self.caret_offset()),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this project.
    use super::*;

    fn units(text: &str) -> AccessibleText {
        AccessibleText::new(text)
    }

    fn segment(text: &str, start: i32, end: i32) -> TextSegment {
        (text.to_string(), start, end)
    }

    #[test]
    fn a_range_of_the_text() {
        let text = units("Hello world");
        assert_eq!(get_text(&text, 0, -1), "Hello world");
        assert_eq!(get_text(&text, 6, 11), "world");
        assert_eq!(get_text(&text, 6, 99), "world");
        assert_eq!(get_text(&text, -5, 5), "Hello");
        assert_eq!(get_text(&text, 5, 5), "");
        assert_eq!(get_text(&text, 7, 3), "");
        assert_eq!(get_text(&units(""), 0, -1), "");
    }

    #[test]
    fn the_text_at_an_offset() {
        let text = units("Hello world");
        assert_eq!(get_string_at_offset(&text, 1, GRANULARITY_CHAR), segment("e", 1, 2));
        // An offset outside the text is the nearest character.
        assert_eq!(get_string_at_offset(&text, 99, GRANULARITY_CHAR), segment("d", 10, 11));
        assert_eq!(get_string_at_offset(&text, -3, GRANULARITY_CHAR), segment("H", 0, 1));
        assert_eq!(get_string_at_offset(&text, 7, GRANULARITY_WORD), segment("world", 6, 11));
        // Word-start granularity keeps the separators before the next word.
        assert_eq!(get_string_at_offset(&text, 0, GRANULARITY_WORD), segment("Hello ", 0, 6));
        assert_eq!(get_string_at_offset(&text, 3, 2), segment("Hello world", 0, 11));
        assert_eq!(get_string_at_offset(&text, 3, 4), segment("Hello world", 0, 11));
        assert_eq!(get_string_at_offset(&units(""), 0, GRANULARITY_CHAR), segment("", 0, 0));
    }

    #[test]
    fn multiline_and_sentence_queries_return_the_containing_segment() {
        let text = units("😀!\r\nsecond line\nthird");
        assert_eq!(get_string_at_offset(&text, 3, GRANULARITY_LINE), segment("😀!\r\n", 0, 4));
        assert_eq!(get_string_at_offset(&text, 4, GRANULARITY_LINE), segment("second line\n", 4, 16));
        assert_eq!(get_string_at_offset(&text, 19, GRANULARITY_PARAGRAPH), segment("third", 16, 21));
        let sentences = units("Hello. Next sentence! 😀");
        assert_eq!(get_string_at_offset(&sentences, 3, GRANULARITY_SENTENCE), segment("Hello. ", 0, 7));
        assert_eq!(get_string_at_offset(&sentences, 9, GRANULARITY_SENTENCE), segment("Next sentence! ", 7, 22));
    }

    #[test]
    fn the_text_before_an_offset() {
        let text = units("Hello world");
        assert_eq!(boundaries::segment(&text, 0, BOUNDARY_CHAR, Direction::Before), segment("", 0, 0));
        assert_eq!(boundaries::segment(&text, 1, BOUNDARY_CHAR, Direction::Before), segment("H", 0, 1));
        assert_eq!(boundaries::segment(&text, 99, BOUNDARY_CHAR, Direction::Before), segment("d", 10, 11));
        assert_eq!(boundaries::segment(&text, 8, BOUNDARY_WORD_START, Direction::Before), segment("Hello ", 0, 6));
        assert_eq!(boundaries::segment(&text, 6, BOUNDARY_WORD_END, Direction::Before), segment("Hello", 0, 5));
    }

    #[test]
    fn the_text_after_an_offset() {
        let text = units("Hello world");
        assert_eq!(boundaries::segment(&text, 0, BOUNDARY_CHAR, Direction::After), segment("e", 1, 2));
        assert_eq!(boundaries::segment(&text, 10, BOUNDARY_CHAR, Direction::After), segment("", 11, 11));
        assert_eq!(boundaries::segment(&text, 0, BOUNDARY_WORD_START, Direction::After), segment("world", 6, 11));
        assert_eq!(boundaries::segment(&text, 4, BOUNDARY_WORD_START, Direction::After), segment("world", 6, 11));
        assert_eq!(boundaries::segment(&units(""), 0, BOUNDARY_CHAR, Direction::After), segment("", 0, 0));
    }

    #[test]
    fn legacy_start_and_end_boundaries_assign_separators_to_different_ranges() {
        let words = units("  𐐀name, next");
        assert_eq!(boundaries::segment(&words, 3, BOUNDARY_WORD_START, Direction::At),
            segment("𐐀name, ", 2, 9));
        assert_eq!(boundaries::segment(&words, 8, BOUNDARY_WORD_END, Direction::At),
            segment(", next", 7, 13));
        let sentences = units("One!  Two? 😀");
        assert_eq!(boundaries::segment(&sentences, 7, BOUNDARY_SENTENCE_START, Direction::At),
            segment("Two? ", 6, 11));
        assert_eq!(boundaries::segment(&sentences, 7, BOUNDARY_SENTENCE_END, Direction::At),
            segment("  Two?", 4, 10));

        let lines = units("😀A\r\nB\nC");
        let read = |direction, by_end| {
            let range = boundaries::line_range(4, lines.len(), by_end, direction, |offset| line_segment(&lines, offset));
            lines.segment(range.start, range.end)
        };
        assert_eq!(read(Direction::At, false), segment("B\n", 4, 6));
        assert_eq!(read(Direction::At, true), segment("\r\nB", 2, 5));
        assert_eq!(read(Direction::Before, true), segment("😀A", 0, 2));
        assert_eq!(read(Direction::After, true), segment("\nC", 5, 7));
    }

    #[test]
    fn text_ranges_use_unicode_scalars() {
        let text = units("aé😀 b\0");
        assert_eq!(text.len(), 6);
        assert_eq!(get_character_at_offset(&text, 0), 0x61);
        assert_eq!(get_character_at_offset(&text, 1), 0xe9);
        assert_eq!(get_character_at_offset(&text, 2), 0x1f600);
        assert_eq!(get_character_at_offset(&text, 5), 0xfffd);
        assert_eq!(get_character_at_offset(&text, 6), -1);
        assert_eq!(get_character_at_offset(&text, -1), -1);
        assert_eq!(get_text(&text, 2, 4), "😀 ");
        assert_eq!(get_string_at_offset(&text, 2, GRANULARITY_CHAR), segment("😀", 2, 3));
        assert_eq!(boundaries::segment(&text, 3, BOUNDARY_CHAR, Direction::Before), segment("😀", 2, 3));
        assert_eq!(boundaries::segment(&text, 1, BOUNDARY_CHAR, Direction::After), segment("😀", 2, 3));
        let words = units("😀 next word");
        assert_eq!(get_string_at_offset(&words, 3, GRANULARITY_WORD), segment("next ", 2, 7));
    }
}
