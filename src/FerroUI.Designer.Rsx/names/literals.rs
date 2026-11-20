//! Map decoded Rust string ranges back to their authored token bytes.
use ferroui_designer_core::{SourceSpan, TextPatch};
use std::ops::Range;

pub(super) fn rewrite_literal(
    source: &str,
    token: SourceSpan,
    value: &str,
    ranges: impl IntoIterator<Item = Range<usize>>,
    replacement: &str,
) -> Result<Vec<TextPatch>, String> {
    let error = || "The control name has no exact mapping in this Rust string".to_owned();
    let text = source.get(token.range()).ok_or_else(error)?;
    let first_quote = text.find('"').ok_or_else(error)?;
    let body = text
        .get(first_quote + 1..text.rfind('"').ok_or_else(error)?)
        .ok_or_else(error)?;
    let offset = token.start + first_quote + 1;
    let map = if body == value {
        None
    } else {
        Some(escaped_positions(body, value, text.starts_with('r')).ok_or_else(error)?)
    };
    ranges
        .into_iter()
        .map(|range| {
            let (start, end) = match &map {
                Some(positions) => (
                    positions
                        .get(range.start)
                        .and_then(|entry| *entry)
                        .ok_or_else(error)?
                        .start,
                    positions
                        .get(range.end.checked_sub(1).ok_or_else(error)?)
                        .and_then(|entry| *entry)
                        .ok_or_else(error)?
                        .end,
                ),
                None => (range.start, range.end),
            };
            Ok(TextPatch {
                span: SourceSpan::new(offset + start, offset + end),
                replacement: replacement.into(),
            })
        })
        .collect()
}

fn escaped_positions(body: &str, value: &str, raw: bool) -> Option<Vec<Option<SourceSpan>>> {
    let mut positions = vec![None; value.len()];
    let mut cursor = 0;
    for (start, character) in value.char_indices() {
        if !raw {
            while let Some(rest) = body[cursor..]
                .strip_prefix("\\\r\n")
                .or_else(|| body[cursor..].strip_prefix("\\\n"))
            {
                cursor = body.len() - rest.trim_start_matches([' ', '\t', '\r', '\n']).len();
            }
        }
        let text = body.get(cursor..)?;
        let width = if !raw && text.starts_with('\\') {
            match text.as_bytes().get(1)? {
                b'x' => 4,
                b'u' => text.find('}')? + 1,
                _ => 2,
            }
        } else if character == '\n' && text.starts_with("\r\n") {
            2
        } else if text.starts_with(character) {
            character.len_utf8()
        } else {
            return None;
        };
        if width > text.len() {
            return None;
        }
        positions[start..start + character.len_utf8()]
            .fill(Some(SourceSpan::new(cursor, cursor + width)));
        cursor += width;
    }
    Some(positions)
}
