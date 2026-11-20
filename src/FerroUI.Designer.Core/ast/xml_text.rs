use super::SourceSpan;
use std::{borrow::Cow, ops::Range};

/// Decoded XML text and its byte boundaries in the authored source.
pub struct XmlText<'a> {
    text: Cow<'a, str>,
    start: usize,
    shifts: Vec<(usize, usize)>,
}

impl<'a> XmlText<'a> {
    pub fn attribute(source: &'a str, span: SourceSpan) -> Option<Self> {
        Self::new(source, span, true)
    }

    pub fn text_node(source: &'a str, span: SourceSpan) -> Option<Self> {
        Self::new(source, span, false)
    }

    fn new(source: &'a str, span: SourceSpan, attribute: bool) -> Option<Self> {
        let raw = source.get(span.range())?;
        let mut shifts = Vec::new();
        let text = if needs_decoding(raw, attribute) {
            let mut value = String::with_capacity(raw.len());
            let mut previous = 0;
            decode(raw, attribute, |character, end| {
                value.push(character);
                let shift = end - value.len();
                if shift != previous {
                    shifts.push((value.len(), shift));
                    previous = shift;
                }
            })?;
            Cow::Owned(value)
        } else {
            Cow::Borrowed(raw)
        };
        Some(Self {
            text,
            start: span.start,
            shifts,
        })
    }

    pub fn value(&self) -> &str {
        &self.text
    }

    /// Map UTF-8 byte boundaries back to the complete lexical entities and text.
    pub fn source_span(&self, range: Range<usize>) -> Option<SourceSpan> {
        self.text.get(range.clone())?;
        let offset = |index| {
            let boundary = self.shifts.partition_point(|(at, _)| *at <= index);
            let shift = boundary
                .checked_sub(1)
                .map_or(0, |index| self.shifts[index].1);
            self.start + index + shift
        };
        Some(SourceSpan::new(offset(range.start), offset(range.end)))
    }
}

pub(super) fn decode_xml(source: &str, attribute: bool) -> String {
    if !needs_decoding(source, attribute) {
        return source.to_owned();
    }
    let mut result = String::with_capacity(source.len());
    decode(source, attribute, |character, _| result.push(character)).expect("validated XML text");
    result
}

fn needs_decoding(source: &str, attribute: bool) -> bool {
    source.contains(['&', '\r']) || attribute && source.contains(['\n', '\t'])
}

fn decode(source: &str, attribute: bool, mut emit: impl FnMut(char, usize)) -> Option<()> {
    let mut pos = 0;
    while pos < source.len() {
        let rest = &source[pos..];
        let (character, len) = if let Some(reference) = rest.strip_prefix('&') {
            let end = reference.find(';')?;
            (entity(&reference[..end])?, end + 2)
        } else if rest.starts_with('\r') {
            (
                if attribute { ' ' } else { '\n' },
                if rest.starts_with("\r\n") { 2 } else { 1 },
            )
        } else {
            let raw = rest.chars().next()?;
            (
                if attribute && matches!(raw, '\n' | '\t') {
                    ' '
                } else {
                    raw
                },
                raw.len_utf8(),
            )
        };
        pos += len;
        emit(character, pos);
    }
    Some(())
}

fn entity(name: &str) -> Option<char> {
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        _ => {
            if let Some(hex) = name.strip_prefix("#x") {
                char::from_u32(u32::from_str_radix(hex, 16).ok()?)?
            } else {
                char::from_u32(name.strip_prefix('#')?.parse().ok()?)?
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_text_maps_unicode_entities_and_normalized_line_endings() {
        let source = "prefix &#x1F331;\r\n&gt;\tÅ&#x52;ow suffix";
        let start = "prefix ".len();
        let end = source.len() - " suffix".len();
        let text = XmlText::attribute(source, SourceSpan::new(start, end)).unwrap();
        assert_eq!(text.value(), "🌱 > ÅRow");
        assert_eq!(
            &source[text.source_span(0..4).unwrap().range()],
            "&#x1F331;"
        );
        assert_eq!(&source[text.source_span(4..5).unwrap().range()], "\r\n");
        assert_eq!(&source[text.source_span(5..6).unwrap().range()], "&gt;");
        assert_eq!(
            &source[text.source_span(7..12).unwrap().range()],
            "Å&#x52;ow"
        );
        assert!(text.source_span(1..4).is_none());
        let text = XmlText::text_node(source, SourceSpan::new(start, end)).unwrap();
        assert_eq!(text.value(), "🌱\n>\tÅRow");
    }
}
