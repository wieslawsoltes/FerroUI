use crate::i_text_run_construction_context::ITextRunConstructionContext;
use crate::visual_line_element_text_run_properties::VisualLineElementTextRunProperties;
use crate::visual_line_error::VisualLineError;
use crate::TextArea;
use ferroui_base::input::{
    Cursor, KeyModifiers, PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::media::text_formatting::{TextRun, TextRunProperties};
use ferroui_base::media::{BrushExtensions, IBrush};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_base::Ref;
use ferroui_controls::Control;
use ferroui_edit_core::document::{CaretPositioningMode, LogicalDirection};
use std::any::Any;
use std::ops::Range;
use std::rc::Rc;

/// The independently owned lengths, offsets, and style of one visual element.
#[derive(Clone)]
pub struct VisualLineElementData {
    visual_length: usize,
    document_length: usize,
    visual_column: usize,
    relative_text_offset: usize,
    properties: VisualLineElementTextRunProperties,
    background: Option<Rc<dyn IBrush>>,
}
impl VisualLineElementData {
    pub fn new(
        visual_length: usize,
        document_length: usize,
        properties: &dyn TextRunProperties,
    ) -> Result<Self, VisualLineError> {
        if visual_length == 0 {
            return Err(VisualLineError::InvalidVisualLength);
        }
        if visual_length > i32::MAX as usize {
            return Err(VisualLineError::OffsetOverflow);
        }
        Ok(Self {
            visual_length,
            document_length,
            visual_column: 0,
            relative_text_offset: 0,
            properties: VisualLineElementTextRunProperties::new(properties),
            background: None,
        })
    }
    pub fn visual_length(&self) -> usize {
        self.visual_length
    }
    pub fn document_length(&self) -> usize {
        self.document_length
    }
    pub fn visual_column(&self) -> usize {
        self.visual_column
    }
    pub fn relative_text_offset(&self) -> usize {
        self.relative_text_offset
    }
    pub fn end_visual_column(&self) -> usize {
        self.visual_column + self.visual_length
    }
    pub fn end_relative_text_offset(&self) -> usize {
        self.relative_text_offset + self.document_length
    }
    pub fn text_run_properties(&self) -> &VisualLineElementTextRunProperties {
        &self.properties
    }
    pub fn text_run_properties_mut(&mut self) -> &mut VisualLineElementTextRunProperties {
        &mut self.properties
    }
    pub fn set_text_run_properties(&mut self, value: &dyn TextRunProperties) {
        self.properties = VisualLineElementTextRunProperties::new(value);
    }
    pub fn background_brush(&self) -> Option<&Rc<dyn IBrush>> {
        self.background.as_ref()
    }
    pub fn set_background_brush(&mut self, value: Option<Rc<dyn IBrush>>) {
        self.background =
            value.map(|brush| BrushExtensions::to_immutable(&brush) as Rc<dyn IBrush>);
    }
    pub(crate) fn set_offsets(&mut self, visual_column: usize, relative_text_offset: usize) {
        self.visual_column = visual_column;
        self.relative_text_offset = relative_text_offset;
    }
    /// Splits lengths, offsets, and styling without sharing mutable collections.
    pub fn split_at(
        &self,
        visual_column: usize,
        relative_text_offset: usize,
    ) -> Result<(Self, Self), VisualLineError> {
        if visual_column <= self.visual_column
            || visual_column >= self.end_visual_column()
            || relative_text_offset < self.relative_text_offset
            || relative_text_offset > self.end_relative_text_offset()
        {
            return Err(VisualLineError::InvalidRange {
                start: visual_column,
                end: visual_column,
                length: self.end_visual_column(),
            });
        }
        let visual_first = visual_column - self.visual_column;
        let document_first = relative_text_offset - self.relative_text_offset;
        let mut first = self.clone();
        let mut second = self.clone();
        first.visual_length = visual_first;
        first.document_length = document_first;
        second.visual_length -= visual_first;
        second.document_length -= document_first;
        second.set_offsets(visual_column, relative_text_offset);
        Ok((first, second))
    }
    pub fn checked_visual_index(&self, visual_column: usize) -> Result<usize, VisualLineError> {
        if visual_column < self.visual_column || visual_column >= self.end_visual_column() {
            Err(VisualLineError::InvalidRange {
                start: visual_column,
                end: visual_column,
                length: self.end_visual_column(),
            })
        } else {
            Ok(visual_column - self.visual_column)
        }
    }
}

/// One document-backed or inserted item in a native formatted visual line.
/// Atomic elements have caret stops at their boundaries; text elements refine
/// these mappings for their UTF-16 content and may be split by transformers.
pub trait VisualLineElement: Any {
    fn data(&self) -> &VisualLineElementData;
    fn data_mut(&mut self) -> &mut VisualLineElementData;
    fn as_any(&self) -> &dyn Any;
    /// Exposes a native child for host preparation before formatting. The host
    /// owns parenting, measurement, arrangement and eviction; creating a text
    /// run must not attach or measure the control during rendering.
    fn inline_control(&self) -> Option<Ref<Control>> {
        None
    }
    fn display_column_length(&self) -> usize {
        self.data().visual_length()
    }
    fn create_text_run(
        &self,
        start_visual_column: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Rc<dyn TextRun>, VisualLineError>;
    fn get_preceding_text(
        &self,
        _visual_column_limit: usize,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<ReadOnlyMemory<u16>, VisualLineError> {
        Ok(ReadOnlyMemory::empty())
    }
    fn can_split(&self) -> bool {
        false
    }
    fn split(
        &self,
        visual_column: usize,
    ) -> Result<(Box<dyn VisualLineElement>, Box<dyn VisualLineElement>), VisualLineError> {
        Err(VisualLineError::CannotSplitElement { visual_column })
    }
    fn get_visual_column(&self, relative_text_offset: usize) -> usize {
        let data = self.data();
        if relative_text_offset >= data.end_relative_text_offset() {
            data.end_visual_column()
        } else {
            data.visual_column()
        }
    }
    fn get_relative_offset(&self, visual_column: usize) -> usize {
        let data = self.data();
        if visual_column >= data.end_visual_column() {
            data.end_relative_text_offset()
        } else {
            data.relative_text_offset()
        }
    }
    fn get_next_caret_position(
        &self,
        visual_column: isize,
        direction: LogicalDirection,
        mode: CaretPositioningMode,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<usize>, VisualLineError> {
        Ok(atomic_caret_stop(
            self.data(),
            visual_column,
            direction,
            mode,
        ))
    }
    fn is_whitespace(
        &self,
        _visual_column: usize,
        _context: &dyn ITextRunConstructionContext,
    ) -> bool {
        false
    }
    fn handles_line_borders(&self) -> bool {
        false
    }
    fn is_document_offset_visible(&self, relative_text_offset: usize) -> bool {
        relative_text_offset == self.data().relative_text_offset()
            || relative_text_offset == self.data().end_relative_text_offset()
    }
    fn composition_range(&self) -> Option<Range<usize>> {
        None
    }
    fn query_cursor(&self, _modifiers: KeyModifiers) -> Option<Rc<Cursor>> {
        None
    }
    fn on_pointer_pressed(&self, _view: &TextArea, _event: &PointerPressedEventArgs) {}
    fn on_pointer_released(&self, _view: &TextArea, _event: &PointerReleasedEventArgs) {}
}

pub(crate) fn atomic_caret_stop(
    data: &VisualLineElementData,
    position: isize,
    direction: LogicalDirection,
    mode: CaretPositioningMode,
) -> Option<usize> {
    let start = data.visual_column();
    let end = data.end_visual_column();
    let include_end = !matches!(
        mode,
        CaretPositioningMode::WordStart | CaretPositioningMode::WordStartOrSymbol
    );
    match direction {
        LogicalDirection::Forward => {
            if position < start as isize {
                Some(start)
            } else if position < end as isize && include_end {
                Some(end)
            } else {
                None
            }
        }
        LogicalDirection::Backward => {
            if position > end as isize && include_end {
                Some(end)
            } else if position > start as isize {
                Some(start)
            } else {
                None
            }
        }
    }
}
