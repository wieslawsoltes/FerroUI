use crate::i_text_run_construction_context::ITextRunConstructionContext;
use crate::visual_line_element::{VisualLineElement, VisualLineElementData};
use crate::visual_line_error::VisualLineError;
use ferroui_base::media::text_formatting::{TextCharacters, TextRun, TextRunProperties};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_edit_core::document::{CaretPositioningMode, LogicalDirection, TextUtilities};
use std::any::Any;
use std::rc::Rc;

/// A splittable one-to-one mapping between document UTF-16 and visual columns.
#[derive(Clone)]
pub struct VisualLineText {
    pub(crate) data: VisualLineElementData,
}
impl VisualLineText {
    pub fn new(length: usize, properties: &dyn TextRunProperties) -> Result<Self, VisualLineError> {
        Ok(Self {
            data: VisualLineElementData::new(length, length, properties)?,
        })
    }
    pub(crate) fn with_data(data: VisualLineElementData) -> Self {
        Self { data }
    }
}
impl VisualLineElement for VisualLineText {
    fn data(&self) -> &VisualLineElementData {
        &self.data
    }
    fn data_mut(&mut self) -> &mut VisualLineElementData {
        &mut self.data
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn create_text_run(
        &self,
        start_visual_column: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Rc<dyn TextRun>, VisualLineError> {
        let relative = self.data.checked_visual_index(start_visual_column)?;
        let offset = context
            .first_document_line()
            .offset
            .checked_add(self.data.relative_text_offset())
            .and_then(|value| value.checked_add(relative))
            .ok_or(VisualLineError::OffsetOverflow)?;
        let text = context.get_text(offset, self.data.document_length() - relative)?;
        Ok(Rc::new(TextCharacters::new(
            text,
            self.data.text_run_properties().snapshot(),
        )))
    }
    fn get_preceding_text(
        &self,
        limit: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<ReadOnlyMemory<u16>, VisualLineError> {
        if limit < self.data.visual_column() || limit > self.data.end_visual_column() {
            return Err(VisualLineError::InvalidRange {
                start: limit,
                end: limit,
                length: self.data.end_visual_column(),
            });
        }
        context.get_text(
            context.first_document_line().offset + self.data.relative_text_offset(),
            limit - self.data.visual_column(),
        )
    }
    fn can_split(&self) -> bool {
        true
    }
    fn split(
        &self,
        visual_column: usize,
    ) -> Result<(Box<dyn VisualLineElement>, Box<dyn VisualLineElement>), VisualLineError> {
        let offset = self
            .data
            .relative_text_offset()
            .checked_add(visual_column.saturating_sub(self.data.visual_column()))
            .ok_or(VisualLineError::OffsetOverflow)?;
        let (first, second) = self.data.split_at(visual_column, offset)?;
        Ok((
            Box::new(Self::with_data(first)),
            Box::new(Self::with_data(second)),
        ))
    }
    fn get_visual_column(&self, offset: usize) -> usize {
        self.data.visual_column()
            + offset
                .saturating_sub(self.data.relative_text_offset())
                .min(self.data.document_length())
    }
    fn get_relative_offset(&self, column: usize) -> usize {
        self.data.relative_text_offset()
            + column
                .saturating_sub(self.data.visual_column())
                .min(self.data.visual_length())
    }
    fn get_next_caret_position(
        &self,
        visual_column: isize,
        direction: LogicalDirection,
        mode: CaretPositioningMode,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<usize>, VisualLineError> {
        let start = context
            .first_document_line()
            .offset
            .checked_add(self.data.relative_text_offset())
            .ok_or(VisualLineError::OffsetOverflow)?;
        let position = isize::try_from(start)
            .ok()
            .and_then(|start| start.checked_add(visual_column))
            .and_then(|offset| offset.checked_sub(self.data.visual_column() as isize))
            .ok_or(VisualLineError::OffsetOverflow)?;
        let next =
            TextUtilities::get_next_caret_position(context.document(), position, direction, mode)?;
        Ok(next
            .filter(|next| *next >= start && *next <= start + self.data.document_length())
            .map(|next| self.data.visual_column() + next - start))
    }
    fn is_whitespace(
        &self,
        visual_column: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> bool {
        let Ok(relative) = self.data.checked_visual_index(visual_column) else {
            return false;
        };
        context
            .document()
            .get_char_at(
                context.first_document_line().offset + self.data.relative_text_offset() + relative,
            )
            .ok()
            .and_then(|unit| char::from_u32(u32::from(unit)))
            .is_some_and(char::is_whitespace)
    }
    fn is_document_offset_visible(&self, offset: usize) -> bool {
        (self.data.relative_text_offset()..=self.data.end_relative_text_offset()).contains(&offset)
    }
}
