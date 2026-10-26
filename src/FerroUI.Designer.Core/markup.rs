use serde::{Deserialize, Serialize};

/// A decoded XAML attribute value. The containing syntax attribute also retains
/// its original source span, so inspection never rewrites its lexical spelling.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkupValue {
    Literal(String),
    Extension(MarkupExtension),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkupExtension {
    /// Qualified extension name, such as `Binding` or `local:Resolve`.
    pub name: String,
    pub arguments: Vec<MarkupArgument>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkupArgument {
    /// `None` denotes a positional argument.
    pub name: Option<String>,
    pub value: MarkupValue,
}

impl MarkupValue {
    /// Parse the nested markup-extension grammar without resolving types or
    /// resources. `{}{...}` is the standard XAML literal escape. Unknown extension
    /// names are retained; malformed braces and quotes are reported to the editor.
    pub fn parse(value: &str) -> Result<Self, String> {
        if let Some(literal) = value.strip_prefix("{}") {
            return Ok(Self::Literal(literal.to_owned()));
        }
        if !value.starts_with('{') {
            return Ok(Self::Literal(value.to_owned()));
        }
        parse_extension(value).map(Self::Extension)
    }
}

fn parse_extension(value: &str) -> Result<MarkupExtension, String> {
    if !value.ends_with('}') {
        return Err("Unclosed markup extension".into());
    }
    let inner = &value[1..value.len() - 1];
    let name_end = inner
        .find(|c: char| c.is_whitespace() || c == ',')
        .unwrap_or(inner.len());
    let name = &inner[..name_end];
    if name.is_empty()
        || name
            .chars()
            .any(|c| matches!(c, '{' | '}' | '=' | '\'' | '"'))
    {
        return Err("A markup extension requires a qualified type name".into());
    }
    let remainder = inner[name_end..].trim_start();
    if remainder.starts_with(',') {
        return Err("Unexpected comma after markup extension name".into());
    }
    let mut arguments = Vec::new();
    if !remainder.is_empty() {
        for argument in split_top_level(remainder, ',')? {
            let argument = argument.trim();
            if argument.is_empty() {
                return Err("Empty markup-extension argument".into());
            }
            let parts = split_top_level(argument, '=')?;
            let (name, raw) = if parts.len() > 1 {
                if parts.len() != 2 || parts[0].trim().is_empty() {
                    return Err("Invalid named markup-extension argument".into());
                }
                (Some(parts[0].trim().to_owned()), parts[1].trim())
            } else {
                (None, argument)
            };
            let parsed = if raw.starts_with('"') || raw.starts_with('\'') {
                let quote = raw.as_bytes()[0];
                if raw.len() < 2 || raw.as_bytes()[raw.len() - 1] != quote {
                    return Err("Unclosed quoted markup-extension value".into());
                }
                MarkupValue::Literal(unescape(&raw[1..raw.len() - 1]))
            } else {
                MarkupValue::parse(raw)?
            };
            if let Some(name) = &name {
                if arguments
                    .iter()
                    .any(|a: &MarkupArgument| a.name.as_ref() == Some(name))
                {
                    return Err(format!("Duplicate markup-extension argument: {name}"));
                }
            }
            arguments.push(MarkupArgument {
                name,
                value: parsed,
            });
        }
    }
    Ok(MarkupExtension {
        name: name.to_owned(),
        arguments,
    })
}

fn unescape(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            output.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else {
            output.push(ch);
        }
    }
    if escaped {
        output.push('\\');
    }
    output
}

fn split_top_level(value: &str, delimiter: char) -> Result<Vec<&str>, String> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    let mut start = 0;
    for (index, ch) in value.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '{' => depth += 1,
            '}' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or("Unexpected closing brace in markup extension")?;
            }
            _ if ch == delimiter && depth == 0 => {
                parts.push(&value[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    if quote.is_some() || depth != 0 || escaped {
        return Err("Unclosed markup-extension value".into());
    }
    parts.push(&value[start..]);
    Ok(parts)
}
