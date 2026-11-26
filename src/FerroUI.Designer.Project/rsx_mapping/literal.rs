//! Preserve the lexical spelling around a decoded fragment of a Rust string.
use crate::{require, ProjectError, Result};
use ferroui_build_scan::authored_symbols::AuthoredText;
use ferroui_designer_core::{SourceSpan, TextPatch};
use proc_macro2::Span;
use std::ops::Range;
use syn::LitStr;

pub(super) fn replace(
    source: &str,
    operand: &AuthoredText,
    decoded: Range<usize>,
    replacement: &str,
) -> Result<TextPatch> {
    let written = operand.imported_literal.as_deref()
        .or_else(|| source.get(operand.literal.clone()))
        .ok_or_else(unmapped)?;
    let raw = written.starts_with('r');
    let span = source_range(written, &operand.value, decoded.clone(), raw).ok_or_else(unmapped)?;
    let encoded: String;
    let text = if raw {
        replacement
    } else {
        encoded = LitStr::new(replacement, Span::call_site())
            .token()
            .to_string();
        encoded
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .ok_or_else(unmapped)?
    };
    let mut rewritten = written.to_owned();
    rewritten.replace_range(span.clone(), text);
    let mut expected = operand.value.clone();
    expected.replace_range(decoded, replacement);
    require(
        syn::parse_str::<LitStr>(&rewritten).is_ok_and(|literal| literal.value() == expected),
        "The changed RSX metadata needs a valid Rust string literal",
    )?;
    Ok(if operand.imported_literal.is_none() && operand.range == operand.literal {
        TextPatch {
            span: SourceSpan::new(
                operand.range.start + span.start,
                operand.range.start + span.end,
            ),
            replacement: text.into(),
        }
    } else {
        TextPatch {
            span: SourceSpan::new(operand.range.start, operand.range.end),
            replacement: rewritten,
        }
    })
}

fn unmapped() -> ProjectError {
    ProjectError("Cannot map the RSX metadata to its current Rust string".into())
}

fn source_range(
    written: &str,
    value: &str,
    range: Range<usize>,
    raw: bool,
) -> Option<Range<usize>> {
    if range.is_empty()
        || !value.is_char_boundary(range.start)
        || !value.is_char_boundary(range.end)
    {
        return None;
    }
    let mut source = written.find('"')? + 1;
    let end = written.rfind('"')?;
    let mut decoded = 0;
    let mut start = None;
    while source < end {
        let remaining = &written[source..end];
        let (character, length) = match remaining {
            _ if !raw && remaining.starts_with('\\') => escape(remaining)?,
            _ if !raw && remaining.starts_with("\r\n") => (Some('\n'), 2),
            _ => {
                let first = remaining.chars().next()?;
                (Some(first), first.len_utf8())
            }
        };
        if let Some(character) = character {
            if value.get(decoded..)?.chars().next()? != character {
                return None;
            }
            if decoded == range.start {
                start = Some(source);
            }
            decoded += character.len_utf8();
            if decoded == range.end {
                return Some(start?..source + length);
            }
        }
        source += length;
    }
    None
}

fn escape(remaining: &str) -> Option<(Option<char>, usize)> {
    let rest = remaining.strip_prefix('\\')?;
    let first = rest.as_bytes().first().copied()?;
    let character = match first {
        b'\\' => '\\',
        b'"' => '"',
        b'\'' => '\'',
        b't' => '\t',
        b'n' => '\n',
        b'r' => '\r',
        b'0' => '\0',
        b'x' => {
            return Some((
                Some(u8::from_str_radix(rest.get(1..3)?, 16).ok()?.into()),
                4,
            ))
        }
        b'u' => {
            let digits = rest.strip_prefix("u{")?;
            let close = digits.find('}')?;
            let value = u32::from_str_radix(&digits[..close].replace('_', ""), 16).ok()?;
            return Some((Some(char::from_u32(value)?), close + 4));
        }
        b'\r' if !rest.starts_with("\r\n") => return None,
        b'\n' | b'\r' => {
            return Some((
                None,
                1 + rest.len() - rest.trim_start_matches(['\r', '\n', '\t', ' ']).len(),
            ))
        }
        _ => return None,
    };
    Some((Some(character), 2))
}
