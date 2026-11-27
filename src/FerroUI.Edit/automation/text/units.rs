use super::{TextContext, range::attributes};
use crate::{TextDocument, TextEditor, VisualLine, VisualLineElement, VisualLineLinkText};
use ferroui_base::Ref;
use ferroui_base::media::{IBrush, TextDecoration};
use ferroui_base::media::text_formatting::TextRunProperties;
use ferroui_base::media::text_formatting::unicode::{GraphemeEnumerator, WordBreakEnumerator};
use ferroui_controls::automation::provider::{ProviderError, TextProviderError, TextUnit};
use ferroui_edit_core::document::DocumentLine;
use std::ops::Range;
use std::rc::Rc;

pub(super) struct UnitNavigator<'a> {
    context: &'a TextContext,
    editor: &'a TextEditor,
    document: &'a TextDocument,
    unit: TextUnit,
    units: Vec<Range<usize>>,
}

impl<'a> UnitNavigator<'a> {
    pub(super) fn new(
        context: &'a TextContext,
        editor: &'a TextEditor,
        document: &'a TextDocument,
        unit: TextUnit,
    ) -> Self {
        Self {
            context,
            editor,
            document,
            unit,
            units: Vec::new(),
        }
    }

    pub(super) fn enclosing(&mut self, position: usize) -> Result<Range<usize>, ProviderError> {
        let len = self.document.text_length();
        if position > len {
            return Err(TextProviderError::InvalidArgument.into());
        }
        if self.units.first().is_none_or(|unit| position < unit.start)
            || self.units.last().is_none_or(|unit| position >= unit.end)
        {
            self.units = self.load_units(position)?;
        }
        let index = self
            .units
            .partition_point(|unit| unit.start <= position)
            .saturating_sub(1);
        Ok(self.units.get(index).cloned().unwrap_or(position..position))
    }

    pub(super) fn move_endpoint(
        &mut self,
        mut position: usize,
        count: i32,
    ) -> Result<(usize, i32), ProviderError> {
        let mut moved = 0;
        let len = self.document.text_length();
        while moved != count {
            if count > 0 && position < len {
                position = self.enclosing(position)?.end;
                moved += 1;
            } else if count < 0 && position > 0 {
                position = self.enclosing(position - 1)?.start;
                moved -= 1;
            } else {
                break;
            }
        }
        Ok((position, moved))
    }

    pub(super) fn move_range(
        &mut self,
        range: Range<usize>,
        count: i32,
    ) -> Result<(Range<usize>, i32), ProviderError> {
        if count == 0 {
            return Ok((range, 0));
        }
        if range.is_empty() {
            let (position, moved) = self.move_endpoint(range.start, count)?;
            return Ok((position..position, moved));
        }
        let mut unit = self.enclosing(range.start)?;
        let mut moved = 0;
        let len = self.document.text_length();
        while moved != count {
            if count > 0 && unit.end < len {
                unit = self.enclosing(unit.end)?;
                moved += 1;
            } else if count < 0 && unit.start > 0 {
                unit = self.enclosing(unit.start - 1)?;
                moved -= 1;
            } else {
                break;
            }
        }
        Ok((if moved == 0 { range } else { unit }, moved))
    }

    fn load_units(&self, position: usize) -> Result<Vec<Range<usize>>, ProviderError> {
        let document = self.document;
        match self.unit {
            // The editor scrolls continuously and has no pagination.
            TextUnit::Page | TextUnit::Document => Ok(vec![0..document.text_length()]),
            TextUnit::Line => Ok(self.editor.text_area().automation_rows(document, position)?),
            TextUnit::Format => Ok(vec![attributes::format_unit(
                self.context, self.editor, document, position,
            )?]),
            TextUnit::Word => self.word_units(position),
            TextUnit::Paragraph | TextUnit::Character => {
                let line = document.get_line_by_offset(position)?;
                let offset = line.offset();
                let end = offset + line.total_length();
                if self.unit == TextUnit::Paragraph || offset == end {
                    return Ok(vec![offset..end]);
                }
                let text = document.get_text_utf16(offset, line.total_length())?;
                let mut units = Vec::new();
                let mut graphemes = GraphemeEnumerator::new(&text);
                while let Some(grapheme) = graphemes.move_next() {
                    let start = offset + grapheme.offset();
                    units.push(start..start + grapheme.length());
                }
                Ok(units)
            }
        }
    }

    fn word_units(&self, position: usize) -> Result<Vec<Range<usize>>, ProviderError> {
        let document = self.document;
        let line = document.get_line_by_offset(position)?;
        let number = line.line_number();
        let mut starts = self.word_starts(&line)?;
        // End each word at the next lexical start, including separators
        // across hard breaks. Read neighboring lines only for missing bounds.
        if starts.first().is_none_or(|&start| position < start) {
            let mut previous = 0;
            for number in (1..number).rev() {
                let line = document.get_line_by_number(number)?;
                if let Some(&start) = self.word_starts(&line)?.last() {
                    previous = start;
                    break;
                }
            }
            starts.insert(0, previous);
        }
        let mut end = document.text_length();
        for number in number + 1..=document.line_count() {
            let line = document.get_line_by_number(number)?;
            if let Some(&start) = self.word_starts(&line)?.first() {
                end = start;
                break;
            }
        }
        starts.push(end);
        Ok(starts.windows(2).map(|pair| pair[0]..pair[1]).collect())
    }

    fn word_starts(&self, line: &DocumentLine) -> Result<Vec<usize>, ProviderError> {
        let offset = line.offset();
        let text = self.document.get_text_utf16(offset, line.length())?;
        let mut words = WordBreakEnumerator::new(&text);
        let mut starts = Vec::new();
        while let Some(word) = words.move_next() {
            if char::decode_utf16(word.text().iter().copied())
                .filter_map(Result::ok)
                .any(|character| character.is_alphanumeric() || character == '_')
            {
                starts.push(offset + word.offset());
            }
        }
        Ok(starts)
    }
}

pub(super) struct Style<'a> {
    pub(super) properties: &'a dyn TextRunProperties,
    pub(super) foreground: Option<&'a Rc<dyn IBrush>>,
    pub(super) background: Option<&'a Rc<dyn IBrush>>,
    link_underline: Option<Ref<TextDecoration>>,
}

impl<'a> Style<'a> {
    pub(super) fn from_properties(properties: &'a dyn TextRunProperties) -> Self {
        Self {
            properties,
            foreground: properties.foreground_brush(),
            background: properties.background_brush(),
            link_underline: None,
        }
    }

    pub(super) fn decorations(&self) -> impl Iterator<Item = Ref<TextDecoration>> + use<'a> {
        self.properties.text_decorations().into_iter()
            .flat_map(|collection| collection.iter())
            .chain(self.link_underline.clone())
    }
}

impl PartialEq for Style<'_> {
    fn eq(&self, other: &Self) -> bool {
        let left = self.properties;
        let right = other.properties;
        left.typeface() == right.typeface()
            && left.font_rendering_em_size() == right.font_rendering_em_size()
            && left.font_scale() == right.font_scale()
            && left.baseline_alignment() == right.baseline_alignment()
            && left.culture_info() == right.culture_info()
            && left.font_features() == right.font_features()
            && self.foreground == other.foreground
            && self.background == other.background
            && if self.link_underline.is_none() && other.link_underline.is_none() {
                left.text_decorations() == right.text_decorations()
            } else {
                self.decorations().eq(other.decorations())
            }
    }
}

pub(super) fn formatting_runs(line: &VisualLine) -> Vec<(Range<usize>, Style<'_>)> {
    let mut runs: Vec<(Range<usize>, Style<'_>)> = Vec::new();
    for (span, style) in style_runs(line) {
        if let Some((previous, _)) = runs.last_mut().filter(|(_, previous)| *previous == style) {
            previous.end = span.end;
        } else {
            runs.push((span, style));
        }
    }
    runs
}

pub(super) fn style_runs(line: &VisualLine) -> impl Iterator<Item = (Range<usize>, Style<'_>)> {
    let offset = line.first_document_line().offset();
    let last = line.last_document_line();
    let text = line.elements().iter().map(move |element| {
        let data = element.data();
        let span = offset + data.relative_text_offset()..offset + data.end_relative_text_offset();
        let mut style = Style::from_properties(data.text_run_properties());
        style.foreground = data.unselected_foreground();
        style.background = data.background_brush().or(style.background);
        if let Some(link) = element.as_any().downcast_ref::<VisualLineLinkText>() {
            style.foreground = link.style.foreground.as_ref();
            style.background = link.style.background.as_ref().or_else(|| data.background_brush());
            style.link_underline = link.underline_decorations()
                .and_then(|underline| underline.iter().next())
                .filter(|underline| !style.decorations().any(|current| current == *underline));
        }
        (span, style)
    });
    let delimiter = (
        last.end_offset()..last.end_offset() + last.delimiter_length(),
        Style::from_properties(&**line.context().global_text_run_properties()),
    );
    text.filter(|(span, _)| !span.is_empty())
        .chain(std::iter::once(delimiter))
}

pub(super) fn link(element: &dyn VisualLineElement) -> Option<&VisualLineLinkText> {
    element.as_any().downcast_ref::<VisualLineLinkText>()
}

pub(super) fn element_groups(line: &VisualLine) -> impl Iterator<Item = &[Box<dyn VisualLineElement>]> {
    line.elements().chunk_by(|left, right| {
        link(left.as_ref()).zip(link(right.as_ref()))
            .is_some_and(|(left, right)| left.is_same_split_link(right))
    })
}

pub(super) fn format_object_boundaries(line: &VisualLine) -> impl Iterator<Item = usize> + '_ {
    let offset = line.first_document_line().offset();
    element_groups(line).filter_map(move |group| {
        let first = group.first()?;
        let last = group.last()?;
        (first.inline_control().is_some()
            || link(first.as_ref()).is_some_and(|link| link.navigate_uri.is_some()))
            .then(|| [
                offset + first.data().relative_text_offset(),
                offset + last.data().end_relative_text_offset(),
            ])
    }).flatten()
}
