use std::rc::Rc;

use crate::media::text_formatting::unicode::GraphemeEnumerator;
use crate::media::text_formatting::{ITextSource, TextCharacters, TextRun, TextRunProperties};
use crate::utilities::{ReadOnlyMemory, ValueSpan};

/// The text source of a formatted text: one string with default properties
/// and optional property overrides for ranges of it.
#[derive(Clone)]
pub struct FormattedTextSource {
    text: ReadOnlyMemory<u16>,
    default_properties: Rc<dyn TextRunProperties>,
    text_modifier: Option<Rc<[ValueSpan<Rc<dyn TextRunProperties>>]>>,
}

impl FormattedTextSource {
    pub fn new(
        text: ReadOnlyMemory<u16>,
        default_properties: Rc<dyn TextRunProperties>,
        text_modifier: Option<Rc<[ValueSpan<Rc<dyn TextRunProperties>>]>>,
    ) -> Self {
        Self { text, default_properties, text_modifier }
    }

    /// Creates a span of text run properties that has modifier applied.
    ///
    /// * `text` — the text to create the properties for.
    /// * `first_text_source_index` — the first text source index.
    /// * `default_properties` — the default text properties.
    /// * `text_modifier` — the text properties modifier.
    ///
    /// Returns the created text style run.
    pub fn create_text_style_run(
        text: &[u16],
        first_text_source_index: i32,
        default_properties: &Rc<dyn TextRunProperties>,
        text_modifier: Option<&[ValueSpan<Rc<dyn TextRunProperties>>]>,
    ) -> ValueSpan<Rc<dyn TextRunProperties>> {
        let text_length = text.len() as i32;

        let text_modifier = match text_modifier {
            Some(text_modifier) if !text_modifier.is_empty() => text_modifier,
            _ => return ValueSpan::new(first_text_source_index, text_length, default_properties.clone()),
        };

        let mut current_properties = default_properties;

        let mut i = 0usize;

        let mut length = 0i32;

        while i < text_modifier.len() {
            let properties_override = &text_modifier[i];

            let text_range = TextRange::new(properties_override.start(), properties_override.length());

            if text_range.start + text_range.length <= first_text_source_index {
                i += 1;
                continue;
            }

            if text_range.start > first_text_source_index + text_length {
                length = text_length;
                break;
            }

            if text_range.start > first_text_source_index && **properties_override.value() != **current_properties {
                length = (text_range.start - first_text_source_index).abs().min(text_length);

                break;
            }

            length = 0.max(text_range.start + text_range.length - first_text_source_index);
            current_properties = properties_override.value();
            break;
        }

        if length < text_length && i == text_modifier.len() && **current_properties == **default_properties {
            length = text_length;
        }

        if length == 0 && **current_properties != **default_properties {
            current_properties = default_properties;
            length = text_length;
        }

        length = Self::coerce_length(text, length);

        ValueSpan::new(first_text_source_index, length, current_properties.clone())
    }

    fn coerce_length(text: &[u16], length: i32) -> i32 {
        let mut final_length = 0i32;

        let mut grapheme_enumerator = GraphemeEnumerator::new(text);

        while let Some(grapheme) = grapheme_enumerator.move_next() {
            final_length += grapheme.length() as i32;

            if final_length >= length {
                return final_length;
            }
        }

        length.min(text.len() as i32)
    }
}

impl ITextSource for FormattedTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        if text_source_index < 0 || text_source_index as usize > self.text.len() {
            return None;
        }

        let run_text = &self.text.span()[text_source_index as usize..];

        if run_text.is_empty() {
            return None;
        }

        let text_style_run = Self::create_text_style_run(
            run_text,
            text_source_index,
            &self.default_properties,
            self.text_modifier.as_deref(),
        );

        Some(Rc::new(TextCharacters::new(
            self.text.slice(text_source_index as usize, text_style_run.length() as usize),
            text_style_run.value().clone(),
        )))
    }
}

/// References a portion of a text buffer.
///
/// Upstream's `End`, `Take` and `Skip` members of this private type are
/// unused and not ported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TextRange {
    start: i32,
    length: i32,
}

impl TextRange {
    const fn new(start: i32, length: i32) -> Self {
        Self { start, length }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::text_formatting::testing::{utf16, TextTestScope};
    use crate::media::text_formatting::GenericTextRunProperties;
    use crate::media::{BaselineAlignment, Brushes, IBrush, Typeface};

    fn highlighted(typeface: &Typeface) -> Rc<dyn TextRunProperties> {
        let background: Rc<dyn IBrush> = Brushes::aqua();

        Rc::new(GenericTextRunProperties::with_all(
            typeface.clone(),
            12.0,
            None,
            None,
            Some(background),
            BaselineAlignment::Baseline,
            None,
            None,
        ))
    }

    #[test]
    fn get_text_run_with_two_text_style_overrides_should_generate_correct_first_run() {
        let _scope = TextTestScope::new();

        // Prepare a sample text: The two "He" at the beginning of each line should be displayed
        // with other text run properties
        let text = "Hello World\r\nHello";

        let typeface = Typeface::default_typeface();

        let default_text_run_properties: Rc<dyn TextRunProperties> =
            Rc::new(GenericTextRunProperties::new(typeface.clone()));

        let text_style_overrides: Rc<[ValueSpan<Rc<dyn TextRunProperties>>]> =
            Rc::from(vec![ValueSpan::new(0, 2, highlighted(&typeface)), ValueSpan::new(13, 2, highlighted(&typeface))]);

        let text_source = FormattedTextSource::new(utf16(text), default_text_run_properties, Some(text_style_overrides));

        let text_run = text_source.get_text_run(0).unwrap();

        assert_eq!(text_run.length(), 2);
        assert_eq!(text_run.text().to_string_lossy(), "He");

        // Additions: the runs after the first override.
        let text_run = text_source.get_text_run(2).unwrap();

        assert_eq!(text_run.text().to_string_lossy(), "llo World\r\n");

        let text_run = text_source.get_text_run(13).unwrap();

        assert_eq!(text_run.text().to_string_lossy(), "He");

        let text_run = text_source.get_text_run(15).unwrap();

        assert_eq!(text_run.text().to_string_lossy(), "llo");

        assert!(text_source.get_text_run(18).is_none());
        assert!(text_source.get_text_run(19).is_none());
        assert!(text_source.get_text_run(-1).is_none());
    }
}
