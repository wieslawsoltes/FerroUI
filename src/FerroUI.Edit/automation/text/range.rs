use super::units::UnitNavigator;
use super::{TextContext, TextRange, subtract_ranges};
use crate::{TextDocument, TextEditor};
use ferroui_base::{Ref, WeakRef};
use ferroui_base::media::{Color, IBrush};
use ferroui_controls::Control;
use ferroui_controls::automation::peers::{AutomationPeer, ControlAutomationPeer};
use ferroui_controls::automation::provider::{
    ITextRangeProvider, ProviderError, TextAttribute, TextAttributeValue, TextPatternRangeEndpoint,
    TextProviderError, TextScrollPlacement, TextUnit, ViewGeometry,
};
use ferroui_edit_core::document::{AnchorSegment, ITextSource, SimpleSegment, TextComparison};
use ferroui_edit_core::editing::{Selection, SelectionSegment};
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

pub(super) mod attributes;

pub(super) struct EditorRange {
    context: TextContext,
    bounds: RefCell<RangeBounds>,
}

enum RangeBounds {
    Document,
    Anchored {
        anchors: AnchorSegment,
        // An inserted control and an ordinary caret can share a document offset.
        inline_object: Option<WeakRef<Control>>,
    },
}

impl EditorRange {
    pub(super) fn new(
        context: TextContext,
        document: &TextDocument,
        span: Range<usize>,
        inline_object: Option<WeakRef<Control>>,
    ) -> Result<Self, ProviderError> {
        Ok(Self {
            context,
            bounds: RefCell::new(RangeBounds::Anchored {
                anchors: Self::anchors(document, span)?,
                inline_object,
            }),
        })
    }

    pub(super) fn document(context: TextContext) -> Self {
        Self { context, bounds: RefCell::new(RangeBounds::Document) }
    }

    fn covers_document(&self) -> bool {
        matches!(*self.bounds.borrow(), RangeBounds::Document)
    }

    fn inline_object(&self) -> Option<WeakRef<Control>> {
        match &*self.bounds.borrow() {
            RangeBounds::Document => None,
            RangeBounds::Anchored { inline_object, .. } => inline_object.clone(),
        }
    }

    fn anchors(
        document: &TextDocument,
        span: Range<usize>,
    ) -> Result<AnchorSegment, ProviderError> {
        let anchors = AnchorSegment::new(document, span.start, span.len())?;
        if span.is_empty() {
            // Use one insertion affinity for a caret, including after replacement.
            return Ok(AnchorSegment {
                end: anchors.start.clone(),
                start: anchors.start,
            });
        }
        Ok(anchors)
    }

    fn state(&self) -> Result<(Ref<TextEditor>, Range<usize>), ProviderError> {
        let owner = self.context.editor()?;
        let span = match &*self.bounds.borrow() {
            RangeBounds::Document => 0..owner.document().text_length(),
            RangeBounds::Anchored { anchors, .. } => anchors.offset()?..anchors.end_offset()?,
        };
        Ok((owner, span))
    }

    fn set_span(&self, document: &TextDocument, span: Range<usize>) -> Result<(), ProviderError> {
        self.context.editor()?;
        self.bounds.replace(RangeBounds::Anchored {
            anchors: Self::anchors(document, span)?,
            inline_object: None,
        });
        Ok(())
    }

    fn other_span(&self, other: &dyn ITextRangeProvider) -> Result<Range<usize>, ProviderError> {
        let other = other
            .as_any()
            .downcast_ref::<Self>()
            .ok_or(TextProviderError::IncompatibleRange)?;
        if self.context.peer.upgrade() != other.context.peer.upgrade() {
            return Err(TextProviderError::IncompatibleRange.into());
        }
        Ok(other.state()?.1)
    }

    fn selection(span: Range<usize>) -> SelectionSegment {
        SelectionSegment::new(span.start, span.end)
    }

    fn selection_with_bounds(previous: SelectionSegment, span: Range<usize>) -> SelectionSegment {
        let (anchor, caret) = if previous.is_reversed() {
            (span.end, span.start)
        } else {
            (span.start, span.end)
        };
        let anchor_virtual_space = if anchor == previous.anchor {
            previous.anchor_virtual_space
        } else {
            0
        };
        let caret_virtual_space = if caret == previous.caret {
            previous.caret_virtual_space
        } else {
            0
        };
        SelectionSegment::with_virtual_space(anchor, anchor_virtual_space, caret, caret_virtual_space)
    }

    fn set_endpoint(
        &self,
        document: &TextDocument,
        span: Range<usize>,
        endpoint: TextPatternRangeEndpoint,
        position: usize,
    ) -> Result<(), ProviderError> {
        if position == endpoint_offset(span.clone(), endpoint) {
            return Ok(());
        }
        self.set_span(
            document,
            match endpoint {
                TextPatternRangeEndpoint::Start => position..position.max(span.end),
                TextPatternRangeEndpoint::End => span.start.min(position)..position,
            },
        )
    }
}

impl ITextRangeProvider for EditorRange {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn peer(&self) -> Result<Ref<AutomationPeer>, ProviderError> {
        self.context.editor()?;
        Ok(self
            .context
            .peer
            .upgrade()
            .ok_or(TextProviderError::ElementNotAvailable)?
            .upcast())
    }

    fn clone_range(&self) -> Result<TextRange, ProviderError> {
        if self.covers_document() {
            self.context.editor()?;
            return Ok(Rc::new(Self::document(self.context.clone())));
        }
        let (editor, span) = self.state()?;
        Ok(Rc::new(Self::new(
            self.context.clone(),
            &editor.document(),
            span,
            self.inline_object(),
        )?))
    }

    fn compare(&self, other: &dyn ITextRangeProvider) -> Result<bool, ProviderError> {
        Ok(self.state()?.1 == self.other_span(other)?)
    }

    fn compare_endpoints(
        &self,
        endpoint: TextPatternRangeEndpoint,
        other: &dyn ITextRangeProvider,
        target_endpoint: TextPatternRangeEndpoint,
    ) -> Result<i32, ProviderError> {
        let left = endpoint_offset(self.state()?.1, endpoint);
        let right = endpoint_offset(self.other_span(other)?, target_endpoint);
        Ok((left as i128 - right as i128).clamp(i32::MIN as i128, i32::MAX as i128) as i32)
    }

    fn expand_to_enclosing_unit(&self, unit: TextUnit) -> Result<(), ProviderError> {
        // A continuously scrolling editor uses Document as its Page fallback.
        if matches!(unit, TextUnit::Document | TextUnit::Page) {
            self.context.editor()?;
            self.bounds.replace(RangeBounds::Document);
            return Ok(());
        }
        let (editor, span) = self.state()?;
        let document = editor.document();
        let version = document.version();
        let range =
            UnitNavigator::new(&self.context, &editor, &document, unit).enclosing(span.start)?;
        check_version(&document, version)?;
        self.set_span(&document, range)
    }

    fn find_attribute(
        &self,
        attribute: TextAttribute,
        value: &TextAttributeValue,
        backwards: bool,
    ) -> Result<Option<TextRange>, ProviderError> {
        attributes::find(self, attribute, value, backwards)
    }

    fn find_text(
        &self,
        text: &str,
        backwards: bool,
        ignore_case: bool,
    ) -> Result<Option<TextRange>, ProviderError> {
        if text.is_empty() {
            return Err(TextProviderError::InvalidArgument.into());
        }
        let (editor, span) = self.state()?;
        let document = editor.document();
        let comparison = if ignore_case {
            TextComparison::OrdinalIgnoreCase
        } else {
            TextComparison::Ordinal
        };
        let start = if backwards {
            document.last_index_of_with_comparison(text, span.start, span.len(), comparison)?
        } else {
            document.index_of_with_comparison(text, span.start, span.len(), comparison)?
        };
        start
            .map(|start| {
                self.context
                    .make_range(&document, start..start + text.encode_utf16().count())
            })
            .transpose()
    }

    fn get_attribute_value(
        &self,
        attribute: TextAttribute,
    ) -> Result<TextAttributeValue, ProviderError> {
        attributes::get(self, attribute)
    }

    fn text_offsets(&self) -> Result<Option<Range<usize>>, ProviderError> {
        Ok(Some(self.state()?.1))
    }

    fn get_bounding_rectangles(&self) -> Result<Vec<ferroui_base::Rect>, ProviderError> {
        let (editor, span) = self.state()?;
        let document = editor.document();
        let version = document.version();
        let area = editor.text_area();
        let Some(geometry) = area
            .visual_root()
            .and_then(|root| ViewGeometry::in_root(&area, &root))
        else {
            return Ok(Vec::new());
        };
        if span.is_empty() {
            return Ok(Vec::new());
        }
        let mut rectangles = Vec::new();
        let visibility = area.automation_visibility(&document)?;
        for span in subtract_ranges(vec![span], visibility.hidden) {
            rectangles.extend(
                area.segment_rectangles(&SimpleSegment::new(span.start, span.len()), false)?,
            );
        }
        check_version(&document, version)?;
        self.context.editor()?;
        let viewport = area.content_viewport();
        Ok(rectangles
            .into_iter()
            .map(|rect| geometry.visible(rect.intersect(viewport)))
            .filter(|rect| !rect.is_empty())
            .collect())
    }

    fn get_enclosing_element(&self) -> Result<Ref<AutomationPeer>, ProviderError> {
        let (editor, span) = self.state()?;
        if !self.covers_document() {
            let document = editor.document();
            let version = document.version();
            let object = self.inline_object().and_then(|object| object.upgrade());
            let inline_objects = editor.text_area().automation_inline_objects()?;
            check_version(&document, version)?;
            self.context.editor()?;
            let containing = inline_objects.into_iter().find(|(control, range)| {
                if span.is_empty() {
                    (range == &span && object.as_ref() == Some(control))
                        || (range.start < span.start && span.start < range.end)
                } else {
                    range.start <= span.start && span.end <= range.end
                }
            });
            if let Some((control, _)) = containing {
                return Ok(ControlAutomationPeer::create_peer_for_element(&control));
            }
            if let Some(link) = self.context.enclosing_link(&editor, &span)? {
                return Ok(link);
            }
        }
        self.peer()
    }

    fn get_text(&self, limit: Option<usize>) -> Result<String, ProviderError> {
        let (editor, span) = self.state()?;
        let document = editor.document();
        let length = limit.map_or(span.len(), |limit| span.len().min(limit));
        let mut text = document.get_text_utf16(span.start, length)?;
        if length < span.len()
            && text
                .last()
                .is_some_and(|unit| (0xd800..=0xdbff).contains(unit))
            && (0xdc00..=0xdfff).contains(&document.get_char_at(span.start + length)?)
        {
            text.pop();
        }
        Ok(String::from_utf16_lossy(&text))
    }

    fn move_by(&self, unit: TextUnit, count: i32) -> Result<i32, ProviderError> {
        let (editor, span) = self.state()?;
        let document = editor.document();
        let version = document.version();
        let (next, moved) =
            UnitNavigator::new(&self.context, &editor, &document, unit).move_range(span, count)?;
        if moved != 0 {
            check_version(&document, version)?;
            self.set_span(&document, next)?;
        }
        Ok(moved)
    }

    fn move_endpoint_by_unit(
        &self,
        endpoint: TextPatternRangeEndpoint,
        unit: TextUnit,
        count: i32,
    ) -> Result<i32, ProviderError> {
        let (editor, span) = self.state()?;
        let document = editor.document();
        let version = document.version();
        let position = endpoint_offset(span.clone(), endpoint);
        let (next, moved) = UnitNavigator::new(&self.context, &editor, &document, unit)
            .move_endpoint(position, count)?;
        if moved != 0 {
            check_version(&document, version)?;
            self.set_endpoint(&document, span, endpoint, next)?;
        }
        Ok(moved)
    }

    fn move_endpoint_by_range(
        &self,
        endpoint: TextPatternRangeEndpoint,
        other: &dyn ITextRangeProvider,
        target_endpoint: TextPatternRangeEndpoint,
    ) -> Result<(), ProviderError> {
        let (editor, span) = self.state()?;
        let target = endpoint_offset(self.other_span(other)?, target_endpoint);
        self.set_endpoint(&editor.document(), span, endpoint, target)
    }

    fn select(&self) -> Result<(), ProviderError> {
        let (editor, span) = self.state()?;
        Ok(editor.set_selection(Selection::simple(span.start, span.end))?)
    }

    fn add_to_selection(&self) -> Result<(), ProviderError> {
        let (editor, span) = self.state()?;
        let current = editor.selection();
        let selected = current.primary();
        if !span.is_empty() && (selected.start_offset()..selected.end_offset()) == span {
            return Ok(());
        }
        let mut segments = current.segments().to_vec();
        if current.is_empty() && segments.len() == 1 {
            segments.clear();
        }
        let primary = segments.len();
        segments.push(Self::selection(span));
        Ok(editor.set_selection(Selection::from_segments(segments, primary))?)
    }

    fn remove_from_selection(&self) -> Result<(), ProviderError> {
        let (editor, span) = self.state()?;
        let current = editor.selection();
        let mut remaining = Vec::new();
        let mut primary = 0;
        for (index, segment) in current.segments().iter().enumerate() {
            let selected = segment.start_offset()..segment.end_offset();
            if index == current.primary_index() {
                primary = remaining.len();
            }
            if selected.is_empty() {
                if selected.start != span.start && !span.contains(&selected.start)
                    || span.is_empty() && !segment.is_empty()
                {
                    remaining.push(*segment);
                }
            } else if span.is_empty() || span.end <= selected.start || selected.end <= span.start {
                remaining.push(*segment);
            } else {
                let left = selected.start..span.start;
                let right = span.end..selected.end;
                let keep_caret_in_right = index == current.primary_index()
                    && !segment.is_reversed()
                    && !right.is_empty()
                    && !left.is_empty();
                for part in [left, right].into_iter().filter(|part| !part.is_empty()) {
                    remaining.push(Self::selection_with_bounds(*segment, part));
                }
                if keep_caret_in_right {
                    primary += 1;
                }
            }
        }
        if remaining.as_slice() == current.segments() {
            return Ok(());
        }
        let selection = if remaining.is_empty() {
            Selection::empty(editor.caret_offset())
        } else {
            Selection::from_segments(remaining, primary)
        };
        Ok(editor.set_selection(selection)?)
    }

    fn replace_in_selection(&self, replacement: &dyn ITextRangeProvider) -> Result<bool, ProviderError> {
        let (editor, span) = self.state()?;
        let replacement = self.other_span(replacement)?;
        let current = editor.selection();
        let mut segments = current.segments().to_vec();
        let Some(segment) = segments.iter_mut().find(|segment| {
            segment.start_offset() == span.start && segment.end_offset() == span.end
        }) else {
            return Ok(false);
        };
        if replacement == span {
            return Ok(true);
        }
        *segment = if replacement.is_empty() {
            Self::selection(replacement)
        } else {
            Self::selection_with_bounds(*segment, replacement)
        };
        editor.set_selection(Selection::from_segments(segments, current.primary_index()))?;
        Ok(true)
    }

    fn scroll_into_view(&self, align_to_top: bool) -> Result<(), ProviderError> {
        let (editor, span) = self.state()?;
        Ok(editor
            .text_area()
            .automation_scroll(&editor.document(), span, align_to_top)?)
    }

    fn scroll_to(&self, placement: TextScrollPlacement) -> Result<bool, ProviderError> {
        let (editor, span) = self.state()?;
        editor.text_area().automation_scroll_to(&editor.document(), span, placement)
    }

    fn get_children(&self) -> Result<Vec<Ref<AutomationPeer>>, ProviderError> {
        let (editor, span) = self.state()?;
        let whole_document = self.covers_document();
        if span.is_empty() && !whole_document {
            return Ok(Vec::new());
        }
        let document = editor.document();
        let version = document.version();
        let objects = editor.text_area().automation_inline_objects()?;
        let mut children = self.context.links_in_range(&editor, &span, whole_document)?;
        check_version(&document, version)?;
        self.context.editor()?;
        children.extend(objects
            .into_iter()
            .filter(|(_, object)| {
                whole_document || (span.start <= object.start && object.start < span.end
                    && object.end <= span.end && object != &span)
            })
            .map(|(control, range)| (ControlAutomationPeer::create_peer_for_element(&control), range)));
        children.sort_by_key(|(_, range)| range.start);
        Ok(children.into_iter().map(|(peer, _)| peer).collect())
    }
}

fn endpoint_offset(span: Range<usize>, endpoint: TextPatternRangeEndpoint) -> usize {
    match endpoint {
        TextPatternRangeEndpoint::Start => span.start,
        TextPatternRangeEndpoint::End => span.end,
    }
}

pub(super) fn check_version(
    document: &TextDocument,
    version: ferroui_edit_core::document::DocumentVersion,
) -> Result<(), TextProviderError> {
    if document.version() == version {
        Ok(())
    } else {
        Err(TextProviderError::ElementNotAvailable)
    }
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
