use crate::i_text_run_construction_context::ITextRunConstructionContext;
use crate::i_text_view_connect::ITextViewConnect;
use crate::visual_line_element::{VisualLineElement, VisualLineElementData};
use crate::visual_line_element_generator::VisualLineElementGenerator;
use crate::visual_line_error::VisualLineError;
use ferroui_base::media::text_formatting::{TextCharacters, TextRun};
use ferroui_base::media::TextDecorations;
use ferroui_base::utilities::ReadOnlyMemory;
use std::any::Any;
use std::ops::Range;
use std::rc::Rc;

/// Projects nonempty IME text over a selected document range without mutating the
/// document. Virtual-space padding is a separate unadorned prefix.
pub struct PreeditElementGenerator {
    start: usize,
    end: usize,
    units: ReadOnlyMemory<u16>,
    padding: usize,
    composition_length: usize,
}
impl PreeditElementGenerator {
    pub fn new(
        start: usize,
        end: usize,
        text: &str,
        virtual_padding: usize,
    ) -> Result<Self, VisualLineError> {
        if start > end {
            return Err(VisualLineError::InvalidRange {
                start,
                end,
                length: end,
            });
        }
        let composition_length = text.encode_utf16().count();
        if composition_length == 0 {
            return Err(VisualLineError::InvalidVisualLength);
        }
        let length = virtual_padding
            .checked_add(composition_length)
            .filter(|length| *length <= i32::MAX as usize)
            .ok_or(VisualLineError::OffsetOverflow)?;
        let mut units = Vec::new();
        units
            .try_reserve_exact(length)
            .map_err(|_| VisualLineError::AllocationFailed)?;
        units.resize(virtual_padding, 32);
        units.extend(text.encode_utf16());
        Ok(Self {
            start,
            end,
            units: ReadOnlyMemory::from_vec(units),
            padding: virtual_padding,
            composition_length,
        })
    }
    pub fn document_range(&self) -> Range<usize> {
        self.start..self.end
    }
}
impl ITextViewConnect for PreeditElementGenerator {}
impl VisualLineElementGenerator for PreeditElementGenerator {
    fn start_generation(
        &self,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<(), VisualLineError> {
        if self.end > context.document().text_length() {
            return Err(VisualLineError::InvalidRange {
                start: self.start,
                end: self.end,
                length: context.document().text_length(),
            });
        }
        Ok(())
    }
    fn runs_on_long_lines(&self) -> bool {
        true
    }
    fn get_first_interested_offset(
        &self,
        start: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<usize>, VisualLineError> {
        Ok(
            (start <= self.start && self.start <= context.last_document_line().end_offset())
                .then_some(self.start),
        )
    }
    fn construct_element(
        &self,
        offset: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<Box<dyn VisualLineElement>>, VisualLineError> {
        if offset != self.start {
            return Ok(None);
        }
        Ok(Some(Box::new(PreeditTextElement {
            data: VisualLineElementData::new(
                self.units.len(),
                self.end - self.start,
                &**context.global_text_run_properties(),
            )?,
            units: self.units.clone(),
            padding: self.padding,
            composition_length: self.composition_length,
        })))
    }
}

pub struct PreeditTextElement {
    data: VisualLineElementData,
    units: ReadOnlyMemory<u16>,
    padding: usize,
    composition_length: usize,
}
impl VisualLineElement for PreeditTextElement {
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
        column: usize,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<Rc<dyn TextRun>, VisualLineError> {
        let relative = self.data.checked_visual_index(column)?;
        let mut properties = self.data.text_run_properties().clone();
        let text = if relative < self.padding {
            self.units.slice(relative, self.padding - relative)
        } else {
            properties.set_text_decorations(Some(TextDecorations::underline()));
            self.units.slice_from(relative)
        };
        Ok(Rc::new(TextCharacters::new(text, properties.snapshot())))
    }
    fn composition_range(&self) -> Option<Range<usize>> {
        let start = self.data.visual_column() + self.padding;
        Some(start..start + self.composition_length)
    }
}
