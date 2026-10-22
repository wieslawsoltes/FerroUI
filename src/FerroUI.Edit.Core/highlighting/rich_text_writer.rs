use super::rich_text::checked_end;
use super::{HighlightingColor, RichText, RichTextError, RichTextModel};

/// A lossless UTF-16 writer with nested formatting and explicit span balance.
///
/// Formatting is scoped: an inner span overrides only its specified properties.
/// Hyperlinks are retained by HTML writers; plain and model writers retain their
/// text and formatting without storing link metadata.
pub trait RichTextWriter {
    fn write_utf16(&mut self, text: &[u16]) -> Result<(), RichTextError>;
    fn begin_span(&mut self, color: &HighlightingColor) -> Result<(), RichTextError>;
    fn end_span(&mut self) -> Result<(), RichTextError>;
    fn indent(&mut self) -> Result<(), RichTextError>;
    fn unindent(&mut self) -> Result<(), RichTextError>;

    fn write(&mut self, text: &str) -> Result<(), RichTextError> {
        self.write_utf16(&text.encode_utf16().collect::<Vec<_>>())
    }

    fn write_char(&mut self, value: char) -> Result<(), RichTextError> {
        self.write_utf16(value.encode_utf16(&mut [0; 2]))
    }

    /// Writes a line using LF. Explicit CR and CRLF writes remain unchanged.
    fn write_line(&mut self, text: &str) -> Result<(), RichTextError> {
        self.write(text)?;
        self.write_utf16(&[10])
    }

    fn begin_foreground_span(&mut self, foreground: &str) -> Result<(), RichTextError> {
        self.begin_span(&HighlightingColor {
            foreground: Some(foreground.to_owned()),
            ..Default::default()
        })
    }

    fn begin_bold_span(&mut self, bold: bool) -> Result<(), RichTextError> {
        self.begin_span(&HighlightingColor {
            bold: Some(bold),
            ..Default::default()
        })
    }

    fn begin_italic_span(&mut self, italic: bool) -> Result<(), RichTextError> {
        self.begin_span(&HighlightingColor {
            italic: Some(italic),
            ..Default::default()
        })
    }

    fn begin_hyperlink_span(&mut self, _uri: &str) -> Result<(), RichTextError> {
        self.begin_span(&HighlightingColor::default())
    }

    fn write_rich_text(&mut self, text: &RichText) -> Result<(), RichTextError> {
        self.write_rich_text_range(text, 0, text.len())
    }

    fn write_rich_text_range(
        &mut self,
        text: &RichText,
        offset: usize,
        length: usize,
    ) -> Result<(), RichTextError> {
        // Validate before writing so a rejected range never produces a prefix.
        for section in text.get_highlighted_sections(offset, length)? {
            self.begin_span(&section.color)?;
            let result = self
                .write_utf16(&text.text_utf16()[section.offset..section.offset + section.length]);
            let end_result = self.end_span();
            result.and(end_result)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct TextBuffer {
    text: Vec<u16>,
    indentation: Vec<u16>,
    indentation_level: usize,
    need_indentation: bool,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self {
            text: Vec::new(),
            indentation: vec![9],
            indentation_level: 0,
            need_indentation: true,
        }
    }
}

impl TextBuffer {
    fn write_utf16(&mut self, text: &[u16]) -> Result<(), RichTextError> {
        let indentation_length = self
            .indentation
            .len()
            .checked_mul(self.indentation_level)
            .ok_or(RichTextError::RangeOverflow {
                offset: self.indentation.len(),
                length: self.indentation_level,
            })?;
        let mut length = checked_end(self.text.len(), text.len())?;
        let mut need_indentation = self.need_indentation;
        for unit in text {
            if *unit == 10 || *unit == 13 {
                need_indentation = true;
            } else if need_indentation {
                length = checked_end(length, indentation_length)?;
                need_indentation = false;
            }
        }
        self.text.reserve(length - self.text.len());
        for unit in text {
            if *unit != 10 && *unit != 13 && self.need_indentation {
                for _ in 0..self.indentation_level {
                    self.text.extend_from_slice(&self.indentation);
                }
            }
            self.text.push(*unit);
            self.need_indentation = *unit == 10 || *unit == 13;
        }
        Ok(())
    }

    fn set_indentation_string(&mut self, value: &str) -> Result<(), RichTextError> {
        if value.contains(['\r', '\n']) {
            return Err(RichTextError::InvalidOption(
                "indentation cannot contain a line ending",
            ));
        }
        self.indentation = value.encode_utf16().collect();
        Ok(())
    }

    fn indent(&mut self) -> Result<(), RichTextError> {
        self.indentation_level = checked_end(self.indentation_level, 1)?;
        Ok(())
    }

    fn unindent(&mut self) -> Result<(), RichTextError> {
        self.indentation_level = self
            .indentation_level
            .checked_sub(1)
            .ok_or(RichTextError::IndentationUnderflow)?;
        Ok(())
    }
}

/// A buffered writer that discards styling while preserving every UTF-16 unit.
#[derive(Clone, Debug, Default)]
pub struct PlainRichTextWriter {
    buffer: TextBuffer,
    span_depth: usize,
}

impl PlainRichTextWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text_utf16(&self) -> &[u16] {
        &self.buffer.text
    }

    pub fn text(&self) -> String {
        String::from_utf16_lossy(self.text_utf16())
    }

    pub fn set_indentation_string(&mut self, value: &str) -> Result<(), RichTextError> {
        self.buffer.set_indentation_string(value)
    }

    pub fn finish(self) -> Result<Vec<u16>, RichTextError> {
        if self.span_depth != 0 {
            return Err(RichTextError::UnclosedSpans(self.span_depth));
        }
        Ok(self.buffer.text)
    }

    /// Returns a UTF-8 projection, replacing isolated UTF-16 surrogates.
    pub fn finish_string(self) -> Result<String, RichTextError> {
        Ok(String::from_utf16_lossy(&self.finish()?))
    }
}

impl RichTextWriter for PlainRichTextWriter {
    fn write_utf16(&mut self, text: &[u16]) -> Result<(), RichTextError> {
        self.buffer.write_utf16(text)
    }

    fn begin_span(&mut self, _color: &HighlightingColor) -> Result<(), RichTextError> {
        self.span_depth = checked_end(self.span_depth, 1)?;
        Ok(())
    }

    fn end_span(&mut self) -> Result<(), RichTextError> {
        self.span_depth = self
            .span_depth
            .checked_sub(1)
            .ok_or(RichTextError::SpanUnderflow)?;
        Ok(())
    }

    fn indent(&mut self) -> Result<(), RichTextError> {
        self.buffer.indent()
    }

    fn unindent(&mut self) -> Result<(), RichTextError> {
        self.buffer.unindent()
    }
}

/// Builds immutable rich text, flattening nested spans into effective formats.
/// It owns its text buffer; writing does not mutate a live editor document.
#[derive(Clone, Debug, Default)]
pub struct RichTextModelWriter {
    buffer: TextBuffer,
    model: RichTextModel,
    current_color: HighlightingColor,
    color_stack: Vec<HighlightingColor>,
}

impl RichTextModelWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.buffer.text.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.text.is_empty()
    }

    pub fn text_utf16(&self) -> &[u16] {
        &self.buffer.text
    }

    pub fn set_indentation_string(&mut self, value: &str) -> Result<(), RichTextError> {
        self.buffer.set_indentation_string(value)
    }

    /// Takes a snapshot, even while a span is open; later writes are independent.
    pub fn to_rich_text(&self) -> RichText {
        RichText::from_utf16_with_model(&self.buffer.text, &self.model)
    }

    pub fn finish(self) -> Result<RichText, RichTextError> {
        if !self.color_stack.is_empty() {
            return Err(RichTextError::UnclosedSpans(self.color_stack.len()));
        }
        Ok(self.to_rich_text())
    }
}

impl RichTextWriter for RichTextModelWriter {
    fn write_utf16(&mut self, text: &[u16]) -> Result<(), RichTextError> {
        let start = self.len();
        self.buffer.write_utf16(text)?;
        self.model
            .set_highlighting(start, self.len() - start, Some(&self.current_color))
    }

    fn begin_span(&mut self, color: &HighlightingColor) -> Result<(), RichTextError> {
        self.color_stack.push(self.current_color.clone());
        self.current_color = self.current_color.merge_with(color);
        Ok(())
    }

    fn end_span(&mut self) -> Result<(), RichTextError> {
        self.current_color = self.color_stack.pop().ok_or(RichTextError::SpanUnderflow)?;
        Ok(())
    }

    fn indent(&mut self) -> Result<(), RichTextError> {
        self.buffer.indent()
    }

    fn unindent(&mut self) -> Result<(), RichTextError> {
        self.buffer.unindent()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_writer_retains_lone_surrogates_and_all_line_endings() {
        let text = [
            b'a' as u16,
            13,
            10,
            0xd83d,
            0xde00,
            13,
            0xd800,
            10,
            b'z' as u16,
        ];
        let mut writer = PlainRichTextWriter::new();
        writer.begin_bold_span(true).unwrap();
        writer.write_utf16(&text[..4]).unwrap();
        writer.write_utf16(&text[4..]).unwrap();
        writer.end_span().unwrap();
        assert_eq!(writer.finish().unwrap(), text);
    }

    #[test]
    fn model_writer_merges_restores_and_clears_nested_formatting() {
        let mut writer = RichTextModelWriter::new();
        writer
            .begin_span(&HighlightingColor {
                foreground: Some("red".into()),
                bold: Some(true),
                underline: Some(true),
                ..Default::default()
            })
            .unwrap();
        writer.write("a").unwrap();
        writer
            .begin_span(&HighlightingColor {
                bold: Some(false),
                italic: Some(true),
                underline: Some(false),
                ..Default::default()
            })
            .unwrap();
        writer.write("😀").unwrap();
        let snapshot = writer.to_rich_text();
        writer.end_span().unwrap();
        writer.write("b").unwrap();
        writer.end_span().unwrap();
        writer.write("c").unwrap();
        let text = writer.finish().unwrap();
        assert_eq!(snapshot.text(), "a😀");
        assert_eq!(text.text(), "a😀bc");
        let inner = text.get_highlighting_at(1).unwrap();
        assert_eq!(inner.foreground.as_deref(), Some("red"));
        assert_eq!(inner.bold, Some(false));
        assert_eq!(inner.italic, Some(true));
        assert_eq!(inner.underline, Some(false));
        assert_eq!(text.get_highlighting_at(3).unwrap().bold, Some(true));
        assert_eq!(
            text.get_highlighting_at(4).unwrap(),
            &HighlightingColor::default()
        );
    }

    #[test]
    fn indentation_respects_crlf_split_writes_and_blank_lines() {
        let mut writer = PlainRichTextWriter::new();
        writer.set_indentation_string("  ").unwrap();
        writer.indent().unwrap();
        writer.write("a\r").unwrap();
        writer.write("\n\nb\rc").unwrap();
        writer.unindent().unwrap();
        writer.write("\nd").unwrap();
        assert_eq!(writer.finish_string().unwrap(), "  a\r\n\n  b\r  c\nd");
    }

    #[test]
    fn rich_text_subrange_writes_only_requested_utf16_units_and_format() {
        let mut model = RichTextModel::new();
        model.set_foreground(1, 2, Some("red")).unwrap();
        let text = RichText::with_model("a😀z", &model);
        let mut writer = RichTextModelWriter::new();
        writer.write_rich_text_range(&text, 1, 2).unwrap();
        assert!(writer.write_rich_text_range(&text, 4, 1).is_err());
        let result = writer.finish().unwrap();
        assert_eq!(result.text(), "😀");
        assert_eq!(
            result.get_highlighting_at(0).unwrap().foreground.as_deref(),
            Some("red")
        );
    }

    #[test]
    fn mismatched_spans_and_indentation_are_explicit_errors() {
        let mut plain = PlainRichTextWriter::new();
        assert!(matches!(
            plain.end_span(),
            Err(RichTextError::SpanUnderflow)
        ));
        assert!(matches!(
            plain.unindent(),
            Err(RichTextError::IndentationUnderflow)
        ));
        assert!(plain.set_indentation_string(" \n").is_err());
        plain.begin_bold_span(true).unwrap();
        assert!(matches!(
            plain.finish(),
            Err(RichTextError::UnclosedSpans(1))
        ));

        let mut model = RichTextModelWriter::new();
        assert!(matches!(
            model.end_span(),
            Err(RichTextError::SpanUnderflow)
        ));
        model.begin_bold_span(true).unwrap();
        assert!(matches!(
            model.finish(),
            Err(RichTextError::UnclosedSpans(1))
        ));
    }
}
