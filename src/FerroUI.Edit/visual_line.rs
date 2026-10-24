use crate::i_text_run_construction_context::{
    ITextRunConstructionContext, TextRunConstructionContext, VisualLineBuildOptions,
};
use crate::visual_line_element::VisualLineElement;
use crate::visual_line_element_generator::VisualLineElementGeneratorRef;
use crate::visual_line_error::VisualLineError;
use crate::visual_line_text::VisualLineText;
use crate::visual_line_transformer::VisualLineTransformerRef;
use ferroui_base::media::text_formatting::{
    ITextSource, TextEndOfParagraph, TextLayout, TextParagraphProperties, TextRun, TextRunCache,
    TextRunProperties,
};
use ferroui_base::media::TextTrimming;
use ferroui_edit_core::document::{
    CaretPositioningMode, DocumentLine, LogicalDirection, TextSnapshot,
};
use std::cell::{OnceCell, RefCell};
use std::ops::Range;
use std::rc::Rc;

/// Default performance cutoff for optional generators.
pub const VISUAL_LINE_LENGTH_LIMIT: usize = 3000;

/// The construction phase owns mutable elements. Finishing consumes the builder,
/// so cached lines cannot be structurally modified after the formatter sees them.
pub struct VisualLineBuilder {
    context: TextRunConstructionContext,
    elements: Vec<Box<dyn VisualLineElement>>,
    visual_length: usize,
    document_length: usize,
}

struct GenerationGuard(Vec<VisualLineElementGeneratorRef>);
impl Drop for GenerationGuard {
    fn drop(&mut self) {
        for generator in &self.0 {
            generator.finish_generation();
        }
    }
}

impl VisualLineBuilder {
    pub fn generate(
        document: TextSnapshot,
        first_line_number: usize,
        properties: Rc<dyn TextRunProperties>,
        generators: &[VisualLineElementGeneratorRef],
        options: VisualLineBuildOptions,
    ) -> Result<Self, VisualLineError> {
        let first_line = document.get_line_by_number(first_line_number)?;
        let mut builder = Self {
            context: TextRunConstructionContext {
                document,
                first_line,
                last_line: first_line,
                properties,
                display_column: 0,
                options,
                cached_text: RefCell::new(None),
            },
            elements: Vec::new(),
            visual_length: 0,
            document_length: 0,
        };
        let mut guard = GenerationGuard(Vec::new());
        for generator in generators {
            guard.0.push(generator.clone());
            generator.start_generation(&builder.context)?;
        }
        let active: Vec<_> = generators
            .iter()
            .filter(|generator| {
                first_line.length <= VISUAL_LINE_LENGTH_LIMIT || generator.runs_on_long_lines()
            })
            .collect();
        let mut offset = first_line.offset;
        let mut skip_current = false;
        loop {
            let Some(requested) = offset.checked_add(usize::from(skip_current)) else {
                break;
            };
            if requested > builder.context.last_line.end_offset() {
                break;
            }
            let mut piece_end = builder.context.last_line.end_offset();
            let mut interests = Vec::with_capacity(active.len());
            for generator in &active {
                let interested =
                    generator.get_first_interested_offset(requested, &builder.context)?;
                if let Some(returned) = interested {
                    if returned < requested {
                        return Err(VisualLineError::GeneratorInterestBeforeStart {
                            requested,
                            returned,
                        });
                    }
                    piece_end = piece_end.min(returned);
                }
                interests.push(interested);
            }
            if piece_end > offset {
                builder.add_element(Box::new(VisualLineText::new(
                    piece_end - offset,
                    &*builder.context.properties,
                )?))?;
                offset = piece_end;
            }
            skip_current = true;
            for (generator, interest) in active.iter().zip(interests) {
                if interest != Some(offset) {
                    continue;
                }
                if let Some(element) = generator.construct_element(offset, &builder.context)? {
                    let length = element.data().document_length();
                    let next = offset
                        .checked_add(length)
                        .ok_or(VisualLineError::OffsetOverflow)?;
                    if next > builder.context.document.text_length() {
                        return Err(VisualLineError::Document(
                            ferroui_edit_core::document::DocumentError::InvalidRange {
                                offset,
                                length,
                                text_length: builder.context.document.text_length(),
                            },
                        ));
                    }
                    if next > builder.context.last_line.end_offset() {
                        let last = builder.context.document.get_line_by_offset(next)?;
                        if last.end_offset() < next {
                            return Err(VisualLineError::ElementEndsInDelimiter { offset: next });
                        }
                        builder.context.last_line = last;
                    }
                    builder.add_element(element)?;
                    if length > 0 {
                        offset = next;
                        skip_current = false;
                        break;
                    }
                }
            }
        }
        drop(guard);
        builder.recalculate_offsets()?;
        Ok(builder)
    }

    fn expected_document_length(&self) -> usize {
        self.context.last_line.end_offset() - self.context.first_line.offset
    }
    fn add_element(
        &mut self,
        mut element: Box<dyn VisualLineElement>,
    ) -> Result<(), VisualLineError> {
        let visual = self
            .visual_length
            .checked_add(element.data().visual_length())
            .filter(|length| *length <= i32::MAX as usize)
            .ok_or(VisualLineError::OffsetOverflow)?;
        if element.data().visual_length() == 0 {
            return Err(VisualLineError::InvalidVisualLength);
        }
        let document = self
            .document_length
            .checked_add(element.data().document_length())
            .ok_or(VisualLineError::OffsetOverflow)?;
        let display = self
            .context
            .display_column
            .checked_add(element.display_column_length())
            .ok_or(VisualLineError::OffsetOverflow)?;
        element
            .data_mut()
            .set_offsets(self.visual_length, self.document_length);
        self.elements.push(element);
        self.visual_length = visual;
        self.document_length = document;
        self.context.display_column = display;
        Ok(())
    }
    fn recalculate_offsets(&mut self) -> Result<(), VisualLineError> {
        let expected = self.expected_document_length();
        let mut visual = 0usize;
        let mut document = 0usize;
        let mut display = 0usize;
        for element in &mut self.elements {
            if element.data().visual_length() == 0 {
                return Err(VisualLineError::InvalidVisualLength);
            }
            let next_visual = visual
                .checked_add(element.data().visual_length())
                .filter(|length| *length <= i32::MAX as usize)
                .ok_or(VisualLineError::OffsetOverflow)?;
            let next_document = document
                .checked_add(element.data().document_length())
                .ok_or(VisualLineError::OffsetOverflow)?;
            display = display
                .checked_add(element.display_column_length())
                .ok_or(VisualLineError::OffsetOverflow)?;
            element.data_mut().set_offsets(visual, document);
            visual = next_visual;
            document = next_document;
        }
        if document != expected {
            return Err(VisualLineError::DocumentLengthChanged {
                expected,
                actual: document,
            });
        }
        if self.context.options.end_of_line.is_some() && visual == i32::MAX as usize {
            return Err(VisualLineError::OffsetOverflow);
        }
        self.visual_length = visual;
        self.document_length = document;
        self.context.display_column = display;
        Ok(())
    }
    pub fn context(&self) -> &dyn ITextRunConstructionContext {
        &self.context
    }
    pub fn first_document_line(&self) -> DocumentLine {
        self.context.first_line
    }
    pub fn last_document_line(&self) -> DocumentLine {
        self.context.last_line
    }
    pub fn elements(&self) -> &[Box<dyn VisualLineElement>] {
        &self.elements
    }
    pub fn visual_length(&self) -> usize {
        self.visual_length
    }
    pub fn document_length(&self) -> usize {
        self.document_length
    }
    pub fn get_visual_column(&self, offset: usize) -> usize {
        visual_column(&self.elements, self.visual_length, offset)
    }
    pub fn get_relative_offset(&self, column: usize) -> usize {
        relative_offset(&self.elements, self.document_length, column)
    }

    /// Replaces whole elements atomically after checking represented document
    /// length and all formatter indices.
    pub fn replace_elements(
        &mut self,
        range: Range<usize>,
        replacements: Vec<Box<dyn VisualLineElement>>,
    ) -> Result<(), VisualLineError> {
        if range.start > range.end || range.end > self.elements.len() {
            return Err(VisualLineError::InvalidRange {
                start: range.start,
                end: range.end,
                length: self.elements.len(),
            });
        }
        let old_document = self.elements[range.clone()]
            .iter()
            .try_fold(0usize, |sum, element| {
                sum.checked_add(element.data().document_length())
            })
            .ok_or(VisualLineError::OffsetOverflow)?;
        let new_document = replacements
            .iter()
            .try_fold(0usize, |sum, element| {
                sum.checked_add(element.data().document_length())
            })
            .ok_or(VisualLineError::OffsetOverflow)?;
        if old_document != new_document {
            return Err(VisualLineError::DocumentLengthChanged {
                expected: old_document,
                actual: new_document,
            });
        }
        let old_visual: usize = self.elements[range.clone()]
            .iter()
            .map(|element| element.data().visual_length())
            .sum();
        let new_visual = replacements
            .iter()
            .try_fold(0usize, |sum, element| {
                (element.data().visual_length() > 0)
                    .then(|| sum.checked_add(element.data().visual_length()))
                    .flatten()
            })
            .ok_or(VisualLineError::InvalidVisualLength)?;
        self.visual_length
            .checked_sub(old_visual)
            .and_then(|value| value.checked_add(new_visual))
            .filter(|length| *length <= i32::MAX as usize)
            .ok_or(VisualLineError::OffsetOverflow)?;
        self.elements.splice(range, replacements);
        self.recalculate_offsets()
    }
    /// Applies a transformation to complete elements inside a visual range,
    /// splitting text where necessary and leaving partial atomic elements intact.
    pub fn change_visual_elements(
        &mut self,
        start: usize,
        end: usize,
        mut change: impl FnMut(&mut dyn VisualLineElement),
    ) -> Result<(), VisualLineError> {
        if start > end || end > self.visual_length {
            return Err(VisualLineError::InvalidRange {
                start,
                end,
                length: self.visual_length,
            });
        }
        if start == end {
            return Ok(());
        }
        self.split_boundary(start)?;
        self.split_boundary(end)?;
        for element in &mut self.elements {
            if element.data().visual_column() >= start && element.data().end_visual_column() <= end
            {
                change(&mut **element);
            }
        }
        self.recalculate_offsets()
    }
    fn split_boundary(&mut self, column: usize) -> Result<(), VisualLineError> {
        let found = self.elements.iter().position(|element| {
            element.data().visual_column() < column && column < element.data().end_visual_column()
        });
        if let Some(index) = found.filter(|index| self.elements[*index].can_split()) {
            let old_start = self.elements[index].data().visual_column();
            let old_end = self.elements[index].data().end_visual_column();
            let (first, second) = self.elements[index].split(column)?;
            if first.data().visual_length() != column - old_start
                || second.data().visual_length() != old_end - column
            {
                return Err(VisualLineError::InvalidSplit {
                    visual_column: column,
                });
            }
            self.replace_elements(index..index + 1, vec![first, second])?;
        }
        Ok(())
    }
    /// Applies a change using absolute UTF-16 document offsets.
    pub fn change_document_part(
        &mut self,
        start: usize,
        end: usize,
        change: impl FnMut(&mut dyn VisualLineElement),
    ) -> Result<(), VisualLineError> {
        let origin = self.context.first_line.offset;
        let document_end = self.context.last_line.end_offset();
        if start < origin || start > end || end > document_end {
            return Err(VisualLineError::InvalidRange {
                start,
                end,
                length: document_end,
            });
        }
        self.change_visual_elements(
            self.get_visual_column(start - origin),
            self.get_visual_column(end - origin),
            change,
        )
    }
    pub fn apply_text_run_properties(
        &mut self,
        start: usize,
        end: usize,
        properties: &dyn TextRunProperties,
    ) -> Result<(), VisualLineError> {
        self.change_document_part(start, end, |element| {
            element.data_mut().set_text_run_properties(properties)
        })
    }
    pub fn run_transformers(
        &mut self,
        transformers: &[VisualLineTransformerRef],
    ) -> Result<(), VisualLineError> {
        for transformer in transformers {
            let context = self.context.clone();
            transformer.transform(&context, self)?;
            self.recalculate_offsets()?;
        }
        Ok(())
    }
    /// Logical document lines with visible element starts; fully hidden lines are skipped.
    pub fn source_lines(&self) -> Result<Vec<DocumentLine>, VisualLineError> {
        source_lines(&self.context, &self.elements)
    }
    pub fn finish(mut self) -> Result<Rc<VisualLine>, VisualLineError> {
        self.recalculate_offsets()?;
        Ok(Rc::new(VisualLine {
            context: self.context,
            elements: self.elements,
            visual_length: self.visual_length,
            document_length: self.document_length,
        }))
    }
}

/// A retained visual line, shared by layout, hit testing, pointer dispatch and
/// background geometry. The source snapshot remains stable across document edits.
pub struct VisualLine {
    context: TextRunConstructionContext,
    elements: Vec<Box<dyn VisualLineElement>>,
    visual_length: usize,
    document_length: usize,
}
impl VisualLine {
    pub fn context(&self) -> &dyn ITextRunConstructionContext {
        &self.context
    }
    pub fn first_document_line(&self) -> DocumentLine {
        self.context.first_line
    }
    pub fn last_document_line(&self) -> DocumentLine {
        self.context.last_line
    }
    pub fn elements(&self) -> &[Box<dyn VisualLineElement>] {
        &self.elements
    }
    pub fn visual_length(&self) -> usize {
        self.visual_length
    }
    pub fn visual_length_with_end_of_line_marker(&self) -> usize {
        self.visual_length
            + usize::from(
                self.context.options.end_of_line.is_some()
                    && self.context.last_line.delimiter_length > 0,
            )
    }
    pub fn document_length(&self) -> usize {
        self.document_length
    }
    pub fn get_visual_column(&self, offset: usize) -> usize {
        visual_column(&self.elements, self.visual_length, offset)
    }
    pub fn visual_offset(&self, offset: usize) -> usize {
        self.get_visual_column(offset)
    }
    pub fn get_relative_offset(&self, column: usize) -> usize {
        relative_offset(&self.elements, self.document_length, column)
    }
    /// Rounds a formatter hit inside an atomic multi-column element to its nearest
    /// document boundary. Exact mapping is available through get_relative_offset.
    pub fn document_offset(&self, column: usize) -> usize {
        let Some(element) = self.element_at(column) else {
            return self.document_length;
        };
        let data = element.data();
        let offset = element.get_relative_offset(column);
        if offset == data.relative_text_offset()
            && column.saturating_sub(data.visual_column())
                >= data.end_visual_column().saturating_sub(column)
        {
            data.end_relative_text_offset()
        } else {
            offset
        }
    }
    pub fn element_at(&self, column: usize) -> Option<&dyn VisualLineElement> {
        self.elements
            .iter()
            .find(|element| {
                element.data().visual_column() <= column
                    && column < element.data().end_visual_column()
            })
            .map(|element| &**element)
    }
    pub fn is_document_offset_visible(&self, offset: usize) -> bool {
        if self.elements.is_empty() {
            return offset == 0;
        }
        self.elements.iter().any(|element| {
            element.data().relative_text_offset() <= offset
                && offset <= element.data().end_relative_text_offset()
                && element.is_document_offset_visible(offset)
        })
    }
    pub fn composition_range(&self) -> Option<Range<usize>> {
        self.elements
            .iter()
            .find_map(|element| element.composition_range())
    }
    pub fn source_lines(&self) -> Result<Vec<DocumentLine>, VisualLineError> {
        source_lines(&self.context, &self.elements)
    }
    pub fn get_next_caret_position(
        &self,
        position: isize,
        direction: LogicalDirection,
        mode: CaretPositioningMode,
        allow_virtual_space: bool,
    ) -> Result<Option<usize>, VisualLineError> {
        let normal = matches!(
            mode,
            CaretPositioningMode::Normal | CaretPositioningMode::EveryCodepoint
        );
        let allow_virtual_space = allow_virtual_space && normal;
        if self.elements.is_empty() {
            return Ok(match direction {
                LogicalDirection::Forward if allow_virtual_space => {
                    position.checked_add(1).map(|next| next.max(0) as usize)
                }
                LogicalDirection::Backward if allow_virtual_space && position > 0 => {
                    Some(position as usize - 1)
                }
                LogicalDirection::Forward if position < 0 => Some(0),
                LogicalDirection::Backward if position > 0 => Some(0),
                _ => None,
            });
        }
        let first = &self.elements[0];
        let last = &self.elements[self.elements.len() - 1];
        match direction {
            LogicalDirection::Backward => {
                if position > self.visual_length as isize && !last.handles_line_borders() {
                    return Ok(Some(if allow_virtual_space {
                        position as usize - 1
                    } else {
                        self.visual_length
                    }));
                }
                for element in self
                    .elements
                    .iter()
                    .rev()
                    .filter(|element| (element.data().visual_column() as isize) < position)
                {
                    let requested = position
                        .min((element.data().end_visual_column() as isize).saturating_add(1));
                    if let Some(next) = element.get_next_caret_position(
                        requested,
                        direction,
                        mode,
                        &self.context,
                    )? {
                        let in_element = element.data().visual_column() <= next
                            && next <= element.data().end_visual_column();
                        let advances = match direction {
                            LogicalDirection::Forward => next as isize > requested,
                            LogicalDirection::Backward => (next as isize) < requested,
                        };
                        if !in_element || !advances {
                            return Err(VisualLineError::InvalidCaretPosition {
                                requested,
                                returned: next,
                            });
                        }
                        return Ok(Some(next));
                    }
                }
                Ok((position > 0 && normal && !first.handles_line_borders()).then_some(0))
            }
            LogicalDirection::Forward => {
                if position < 0 && normal && !first.handles_line_borders() {
                    return Ok(Some(0));
                }
                for element in self
                    .elements
                    .iter()
                    .filter(|element| (element.data().end_visual_column() as isize) > position)
                {
                    let requested = position.max(element.data().visual_column() as isize - 1);
                    if let Some(next) = element.get_next_caret_position(
                        requested,
                        direction,
                        mode,
                        &self.context,
                    )? {
                        let in_element = element.data().visual_column() <= next
                            && next <= element.data().end_visual_column();
                        let advances = match direction {
                            LogicalDirection::Forward => next as isize > requested,
                            LogicalDirection::Backward => (next as isize) < requested,
                        };
                        if !in_element || !advances {
                            return Err(VisualLineError::InvalidCaretPosition {
                                requested,
                                returned: next,
                            });
                        }
                        return Ok(Some(next));
                    }
                }
                if allow_virtual_space || !last.handles_line_borders() {
                    if position < self.visual_length as isize {
                        return Ok(Some(self.visual_length));
                    }
                    if allow_virtual_space {
                        return Ok(position.checked_add(1).map(|next| next as usize));
                    }
                }
                Ok(None)
            }
        }
    }
    pub fn text_source(self: &Rc<Self>) -> Rc<VisualLineTextSource> {
        VisualLineTextSource::new(self.clone())
    }
    /// Formats through FerroUI and reports invalid extension runs after the
    /// formatter returns, instead of silently truncating the rendered line.
    #[allow(clippy::too_many_arguments)]
    pub fn create_text_layout(
        self: &Rc<Self>,
        paragraph: Rc<dyn TextParagraphProperties>,
        max_width: f64,
        max_height: f64,
        max_lines: i32,
        trimming: Option<Rc<dyn TextTrimming>>,
        cache: Option<Rc<TextRunCache>>,
    ) -> Result<TextLayout, VisualLineError> {
        let source = self.text_source();
        let layout = TextLayout::from_text_source(
            source.clone(),
            paragraph,
            trimming,
            max_width,
            max_height,
            max_lines,
            cache,
        );
        if let Some(error) = source.error() {
            return Err(error);
        }
        Ok(layout)
    }
}
fn visual_column(elements: &[Box<dyn VisualLineElement>], length: usize, offset: usize) -> usize {
    elements
        .iter()
        .find(|element| {
            element.data().relative_text_offset() <= offset
                && offset <= element.data().end_relative_text_offset()
        })
        .map_or(length, |element| element.get_visual_column(offset))
}
fn relative_offset(elements: &[Box<dyn VisualLineElement>], length: usize, column: usize) -> usize {
    elements
        .iter()
        .find(|element| {
            element.data().visual_column() <= column && column < element.data().end_visual_column()
        })
        .map_or(length, |element| element.get_relative_offset(column))
}
fn source_lines(
    context: &TextRunConstructionContext,
    elements: &[Box<dyn VisualLineElement>],
) -> Result<Vec<DocumentLine>, VisualLineError> {
    let mut lines = vec![context.first_line];
    for element in elements {
        let offset = context.first_line.offset + element.data().relative_text_offset();
        if offset >= lines.last().unwrap().offset + lines.last().unwrap().total_length() {
            let line = context.document.get_line_by_offset(offset)?;
            if lines.last() != Some(&line) {
                lines.push(line);
            }
        }
    }
    Ok(lines)
}

/// Native text source whose run validation remains observable to the host.
pub struct VisualLineTextSource {
    line: Rc<VisualLine>,
    error: RefCell<Option<VisualLineError>>,
    end_of_line_run: OnceCell<Rc<dyn TextRun>>,
}
impl VisualLineTextSource {
    pub fn new(line: Rc<VisualLine>) -> Rc<Self> {
        Rc::new(Self {
            line,
            error: RefCell::new(None),
            end_of_line_run: OnceCell::new(),
        })
    }
    pub fn visual_line(&self) -> &Rc<VisualLine> {
        &self.line
    }
    pub fn error(&self) -> Option<VisualLineError> {
        self.error.borrow().clone()
    }
    pub fn get_preceding_text(
        &self,
        limit: usize,
    ) -> Result<ferroui_base::utilities::ReadOnlyMemory<u16>, VisualLineError> {
        if limit == 0 {
            return Ok(ferroui_base::utilities::ReadOnlyMemory::empty());
        }
        let Some(element) = self.line.element_at(limit - 1) else {
            return Ok(ferroui_base::utilities::ReadOnlyMemory::empty());
        };
        let text = element.get_preceding_text(limit, &self.line.context)?;
        let remaining = limit - element.data().visual_column();
        if text.len() > remaining {
            return Err(VisualLineError::InvalidPrecedingText {
                visual_column: limit,
                length: text.len(),
                remaining,
            });
        }
        Ok(text)
    }
}
impl VisualLineTextSource {
    fn end_of_line_run(&self) -> Result<Rc<dyn TextRun>, VisualLineError> {
        if let Some(run) = self.end_of_line_run.get() {
            return Ok(run.clone());
        }
        let context = &self.line.context;
        let options = context
            .options
            .end_of_line
            .as_ref()
            .expect("end-of-line run requested only when enabled");
        let line = context.last_line;
        let glyph = match line.delimiter_length {
            2 => options.crlf_glyph.as_str(),
            1 => match context.document.get_char_at(line.end_offset())? {
                13 => options.cr_glyph.as_str(),
                10 => options.lf_glyph.as_str(),
                _ => "?",
            },
            _ => "",
        };
        let mut properties =
            crate::visual_line_element_text_run_properties::VisualLineElementTextRunProperties::new(
                &*context.properties,
            );
        properties.set_foreground_brush(options.foreground.clone());
        properties.set_font_rendering_em_size(context.properties.font_rendering_em_size() - 2.0);
        let layout = crate::formatted_text_element::FormattedTextElement::prepare_text(
            glyph,
            properties.snapshot(),
        );
        let run: Rc<dyn TextRun> = Rc::new(crate::formatted_text_element::FormattedTextRun::new(
            crate::formatted_text_element::FormattedTextContent::Layout(layout),
            context.properties.clone(),
        ));
        let _ = self.end_of_line_run.set(run.clone());
        Ok(run)
    }
}
impl ITextSource for VisualLineTextSource {
    fn get_text_run(&self, index: i32) -> Option<Rc<dyn TextRun>> {
        let source_length =
            self.line.visual_length + usize::from(self.line.context.options.end_of_line.is_some());
        if index < 0 || index as usize > source_length {
            return None;
        }
        let column = index as usize;
        if self.error.borrow().is_some() {
            return Some(Rc::new(TextEndOfParagraph::new()));
        }
        if column >= self.line.visual_length {
            if column == self.line.visual_length && self.line.context.options.end_of_line.is_some()
            {
                return Some(match self.end_of_line_run() {
                    Ok(run) => run,
                    Err(error) => {
                        *self.error.borrow_mut() = Some(error);
                        Rc::new(TextEndOfParagraph::new())
                    }
                });
            }
            return Some(Rc::new(TextEndOfParagraph::new()));
        }
        let element = self.line.element_at(column)?;
        let result = element
            .create_text_run(column, &self.line.context)
            .and_then(|run| {
                let remaining = element.data().end_visual_column() - column;
                if run.length() <= 0 || run.length() as usize > remaining {
                    Err(VisualLineError::InvalidRunLength {
                        visual_column: column,
                        length: run.length(),
                        remaining,
                    })
                } else if let Some(drawable) = run.as_drawable() {
                    let size = drawable.size();
                    if !size.width.is_finite()
                        || !size.height.is_finite()
                        || size.width < 0.0
                        || size.height < 0.0
                        || !drawable.baseline().is_finite()
                    {
                        Err(VisualLineError::InvalidRunMetrics {
                            visual_column: column,
                        })
                    } else {
                        Ok(run)
                    }
                } else {
                    Ok(run)
                }
            });
        match result {
            Ok(run) => Some(run),
            Err(error) => {
                *self.error.borrow_mut() = Some(error);
                Some(Rc::new(TextEndOfParagraph::new()))
            }
        }
    }
}
