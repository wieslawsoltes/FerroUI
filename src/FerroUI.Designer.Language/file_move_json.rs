//! Byte ranges of existing JSON tokens, using serde for syntax and escaping.
use crate::{LanguageError, Result};
use ferroui_designer_core::{SourceSpan, TextPatch};
use serde_json::value::RawValue;

pub(super) fn string(token: &RawValue) -> serde_json::Result<String> {
    serde_json::from_str(token.get())
}

pub(super) fn replace(source: &str, token: &RawValue, value: &str) -> Result<TextPatch> {
    let raw = token.get();
    let start = (raw.as_ptr() as usize)
        .checked_sub(source.as_ptr() as usize)
        .filter(|start| *start <= source.len() && raw.len() <= source.len() - start)
        .ok_or_else(|| LanguageError("The saved reference is outside its source buffer".into()))?;
    Ok(TextPatch {
        span: SourceSpan::new(start, start + raw.len()),
        replacement: serde_json::to_string(value).map_err(error)?,
    })
}

/// Keep the original token for each dynamic map key, including escaped keys.
pub(super) fn entries(value: &RawValue) -> Result<Vec<(&RawValue, &RawValue)>> {
    let mut source = strip(value.get(), '{')?;
    let mut fields = Vec::new();
    if source.trim_start().starts_with('}') {
        return Ok(fields);
    }
    loop {
        let key = take(&mut source)?;
        source = strip(source, ':')?;
        let value = take(&mut source)?;
        fields.push((key, value));
        if source.trim_start().starts_with('}') {
            return Ok(fields);
        }
        source = strip(source, ',')?;
    }
}

fn strip(source: &str, token: char) -> Result<&str> {
    source
        .trim_start()
        .strip_prefix(token)
        .ok_or_else(|| LanguageError("Expected an object of scenario source remaps".into()))
}

fn take<'a>(source: &mut &'a str) -> Result<&'a RawValue> {
    let mut tokens = serde_json::Deserializer::from_str(source).into_iter::<&RawValue>();
    let value = tokens
        .next()
        .ok_or_else(|| LanguageError("Missing scenario source remap".into()))?
        .map_err(error)?;
    *source = &source[tokens.byte_offset()..];
    Ok(value)
}

pub(super) fn error(error: serde_json::Error) -> LanguageError {
    LanguageError(error.to_string())
}
