use crate::StudioError;
use ferroui_protranslate_core::{format_composite, placeholder_indexes, Culture, FormatArgument};
use ferroui_protranslate_formats::TranslationFileFormat;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormatSyntax {
    Composite,
    I18Next,
    Named,
    Printf,
}
impl FormatSyntax {
    pub fn for_format(format: TranslationFileFormat) -> Self {
        use TranslationFileFormat::*;
        match format {
            I18NextJson => Self::I18Next,
            FlutterArb | AppleStringsdict | AppleXcstrings => Self::Named,
            GettextPo | AndroidResources | AppleStrings => Self::Printf,
            _ => Self::Composite,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Placeholder {
    pub raw: String,
    pub normalized: String,
    pub name: String,
    pub start: usize,
    pub end: usize,
}

/// Recognize source placeholders without evaluating them. Byte ranges support
/// insertion/highlighting; escaped composite braces and printf %% are literals.
pub fn placeholders(
    text: &str,
    format: TranslationFileFormat,
) -> Result<Vec<Placeholder>, StudioError> {
    match FormatSyntax::for_format(format) {
        FormatSyntax::Composite => {
            placeholder_indexes(text)
                .map_err(|error| StudioError::InvalidInput(error.to_string()))?;
            let bytes = text.as_bytes();
            let mut position = 0;
            let mut result = Vec::new();
            while position < bytes.len() {
                if matches!(bytes[position], b'{' | b'}')
                    && bytes.get(position + 1) == Some(&bytes[position])
                {
                    position += 2;
                    continue;
                }
                if bytes[position] == b'{' {
                    let end =
                        position + text[position..].find('}').expect("core validated grammar") + 1;
                    let inner = &text[position + 1..end - 1];
                    let name = inner.split([',', ':']).next().unwrap_or("").trim();
                    let index = name
                        .parse::<usize>()
                        .map_err(|_| StudioError::InvalidInput("Invalid composite index".into()))?;
                    result.push(Placeholder {
                        raw: text[position..end].into(),
                        normalized: format!("{{{index}}}"),
                        name: index.to_string(),
                        start: position,
                        end,
                    });
                    position = end;
                } else {
                    position += 1;
                }
            }
            Ok(result)
        }
        FormatSyntax::I18Next => {
            let matcher = Regex::new(r"\{\{\s*(-?\s*[a-zA-Z0-9_.]+)(?:,\s*[^{}]+)?\s*\}\}")
                .expect("literal regex");
            let mut result = Vec::new();
            let mut position = 0;
            for capture in matcher.captures_iter(text) {
                let whole = capture.get(0).unwrap();
                if text[position..whole.start()].contains(['{', '}']) {
                    return Err(StudioError::InvalidInput(
                        "Malformed i18next interpolation".into(),
                    ));
                }
                let name = capture[1].trim().trim_start_matches('-').trim().to_owned();
                result.push(Placeholder {
                    raw: whole.as_str().into(),
                    normalized: format!("{{{{{name}}}}}"),
                    name,
                    start: whole.start(),
                    end: whole.end(),
                });
                position = whole.end();
            }
            if text[position..].contains(['{', '}']) {
                return Err(StudioError::InvalidInput(
                    "Malformed i18next interpolation".into(),
                ));
            }
            Ok(result)
        }
        FormatSyntax::Named => {
            let mut result = Vec::new();
            scan_named(text, 0, 0, &mut result)?;
            if matches!(
                format,
                TranslationFileFormat::AppleStringsdict | TranslationFileFormat::AppleXcstrings
            ) {
                result.extend(scan_printf(text));
                result.sort_by_key(|placeholder| placeholder.start);
            }
            Ok(result)
        }
        FormatSyntax::Printf => Ok(scan_printf(text)),
    }
}

fn scan_printf(text: &str) -> Vec<Placeholder> {
    let matcher =
        Regex::new(r"%(?:([0-9]+)\$)?[-+ #0]*[0-9]*(?:\.[0-9]+)?[lhjztL]*[diouxXeEfFgGaAcsp@%]")
            .expect("literal regex");
    matcher
        .captures_iter(text)
        .filter_map(|capture| {
            let whole = capture.get(0).unwrap();
            if whole.as_str() == "%%" {
                return None;
            }
            let name = capture
                .get(1)
                .map_or_else(|| whole.as_str().into(), |value| value.as_str().to_owned());
            Some(Placeholder {
                raw: whole.as_str().into(),
                normalized: capture.get(1).map_or_else(
                    || whole.as_str().into(),
                    |value| format!("%{}$", value.as_str()),
                ),
                name,
                start: whole.start(),
                end: whole.end(),
            })
        })
        .collect()
}

fn invalid_named() -> StudioError {
    StudioError::InvalidInput("Malformed named/ICU placeholder".into())
}
fn matching_brace(text: &str, start: usize) -> Result<usize, StudioError> {
    let mut depth = 0usize;
    for (offset, character) in text[start..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1).ok_or_else(invalid_named)?;
                if depth == 0 {
                    return Ok(start + offset + 1);
                }
            }
            _ => {}
        }
    }
    Err(invalid_named())
}
fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn plural_offset(text: &str) -> Result<(i64, usize), StudioError> {
    let leading = text.len() - text.trim_start().len();
    let Some(rest) = text[leading..].strip_prefix("offset:") else {
        return Ok((0, 0));
    };
    let whitespace = rest.len() - rest.trim_start().len();
    let number = &rest[whitespace..];
    let digits = number.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return Err(invalid_named());
    }
    let offset = number[..digits]
        .parse::<i64>()
        .map_err(|_| invalid_named())?;
    Ok((offset, leading + "offset:".len() + whitespace + digits))
}

fn branches(text: &str) -> Result<Vec<(&str, &str, usize)>, StudioError> {
    let mut result = Vec::new();
    let (_, mut position) = plural_offset(text)?;
    while position < text.len() {
        position += text[position..].len() - text[position..].trim_start().len();
        if position == text.len() {
            break;
        }
        let key_start = position;
        while position < text.len()
            && !text.as_bytes()[position].is_ascii_whitespace()
            && text.as_bytes()[position] != b'{'
        {
            position += 1;
        }
        let selector = &text[key_start..position];
        if selector.is_empty()
            || !selector
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'=' | b'-'))
        {
            return Err(invalid_named());
        }
        position += text[position..].len() - text[position..].trim_start().len();
        if text.as_bytes().get(position) != Some(&b'{') {
            return Err(invalid_named());
        }
        let end = matching_brace(text, position)?;
        if result.iter().any(|(key, _, _)| *key == selector) {
            return Err(StudioError::InvalidInput(
                "Duplicate plural/select branch".into(),
            ));
        }
        result.push((selector, &text[position + 1..end - 1], position + 1));
        position = end;
    }
    if !result.iter().any(|(key, _, _)| *key == "other") {
        return Err(StudioError::InvalidInput(
            "Plural/select requires an other branch".into(),
        ));
    }
    Ok(result)
}

fn scan_named(
    text: &str,
    offset: usize,
    depth: usize,
    result: &mut Vec<Placeholder>,
) -> Result<(), StudioError> {
    if depth > 32 {
        return Err(StudioError::InvalidInput(
            "Placeholder nesting exceeds 32 levels".into(),
        ));
    }
    let mut position = 0;
    while position < text.len() {
        match text.as_bytes()[position] {
            b'{' => {
                let end = matching_brace(text, position)?;
                let inner = &text[position + 1..end - 1];
                let mut parts = inner.splitn(3, ',');
                let name = parts.next().unwrap_or("").trim();
                if !valid_name(name) {
                    return Err(invalid_named());
                }
                result.push(Placeholder {
                    raw: text[position..end].into(),
                    normalized: format!("{{{name}}}"),
                    name: name.into(),
                    start: offset + position,
                    end: offset + end,
                });
                if let Some(kind) = parts.next() {
                    let style = parts.next().unwrap_or("").trim();
                    if matches!(kind.trim(), "plural" | "select" | "selectordinal") {
                        // Locate the third field directly; whitespace remains significant to byte ranges.
                        let third_start = inner
                            .find(',')
                            .and_then(|first| {
                                inner[first + 1..]
                                    .find(',')
                                    .map(|second| first + second + 2)
                            })
                            .ok_or_else(invalid_named)?;
                        let raw_style = &inner[third_start..];
                        let leading = raw_style.len() - raw_style.trim_start().len();
                        for (_, body, body_offset) in branches(style)? {
                            scan_named(
                                body,
                                offset + position + 1 + third_start + leading + body_offset,
                                depth + 1,
                                result,
                            )?;
                        }
                    } else if !matches!(
                        kind.trim(),
                        "number" | "date" | "time" | "spellout" | "ordinal" | "duration"
                    ) {
                        return Err(invalid_named());
                    }
                }
                position = end;
            }
            b'}' => return Err(invalid_named()),
            _ => position += 1,
        }
    }
    Ok(())
}

pub fn normalized_placeholders(
    text: &str,
    format: TranslationFileFormat,
) -> Result<Vec<String>, StudioError> {
    unique(
        placeholders(text, format)?
            .into_iter()
            .map(|value| value.normalized),
    )
}
pub fn source_placeholders(
    text: &str,
    format: TranslationFileFormat,
) -> Result<Vec<String>, StudioError> {
    unique(
        placeholders(text, format)?
            .into_iter()
            .map(|value| value.raw),
    )
}
fn unique(values: impl Iterator<Item = String>) -> Result<Vec<String>, StudioError> {
    let mut seen = BTreeSet::new();
    Ok(values.filter(|value| seen.insert(value.clone())).collect())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlaceholderDiagnostic {
    pub code: &'static str,
    pub message: String,
}

pub fn validate_placeholders(
    source: &str,
    target: &str,
    format: TranslationFileFormat,
) -> Vec<PlaceholderDiagnostic> {
    let mut diagnostics = Vec::new();
    if target.trim().is_empty() {
        diagnostics.push(PlaceholderDiagnostic {
            code: "PTS001",
            message: "Missing target translation value".into(),
        });
        return diagnostics;
    }
    let source = normalized_placeholders(source, format);
    let target = normalized_placeholders(target, format);
    let (source, target) = match (source, target) {
        (Ok(source), Ok(target)) => (source, target),
        (source, target) => {
            if let Err(error) = source {
                diagnostics.push(PlaceholderDiagnostic {
                    code: "PTS002",
                    message: format!("Source syntax: {error}"),
                });
            }
            if let Err(error) = target {
                diagnostics.push(PlaceholderDiagnostic {
                    code: "PTS003",
                    message: format!("Target syntax: {error}"),
                });
            }
            return diagnostics;
        }
    };
    for placeholder in &source {
        if !target.contains(placeholder) {
            diagnostics.push(PlaceholderDiagnostic {
                code: "PTS004",
                message: format!("Placeholder '{placeholder}' is missing in target translation"),
            });
        }
    }
    for placeholder in &target {
        if !source.contains(placeholder) {
            diagnostics.push(PlaceholderDiagnostic {
                code: "PTS005",
                message: format!("Unexpected placeholder '{placeholder}' in target translation"),
            });
        }
    }
    diagnostics
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuilderFormatType {
    Text,
    Number,
    Date,
    Plural,
    Select,
}

#[derive(Clone, Debug)]
pub struct FormatHelper {
    pub placeholder: String,
    pub kind: BuilderFormatType,
    pub decimal_places: u8,
    pub percentage: bool,
    pub date_pattern: String,
    /// Plural category or select key to text. An absent other branch uses "Value".
    pub branches: BTreeMap<String, String>,
}
impl Default for FormatHelper {
    fn default() -> Self {
        Self {
            placeholder: "0".into(),
            kind: BuilderFormatType::Text,
            decimal_places: 2,
            percentage: false,
            date_pattern: "yyyy-MM-dd".into(),
            branches: BTreeMap::new(),
        }
    }
}
impl FormatHelper {
    pub fn build(&self, format: TranslationFileFormat) -> Result<String, StudioError> {
        let syntax = FormatSyntax::for_format(format);
        let raw = self.placeholder.trim();
        let name = raw
            .trim_matches(['{', '}', '%'])
            .split([':', ','])
            .next()
            .unwrap_or("")
            .trim();
        let name = if name.is_empty() {
            if syntax == FormatSyntax::Composite {
                "0"
            } else {
                "value"
            }
        } else {
            name
        };
        if self.decimal_places > 99 {
            return Err(StudioError::InvalidInput(
                "Decimal places must be at most 99".into(),
            ));
        }
        if syntax == FormatSyntax::Composite && name.parse::<usize>().is_err() {
            return Err(StudioError::InvalidInput(
                "Composite placeholders require a numeric index".into(),
            ));
        }
        if (syntax == FormatSyntax::Named && !valid_name(name))
            || (syntax == FormatSyntax::I18Next && !name.split('.').all(valid_name))
        {
            return Err(StudioError::InvalidInput(
                "Named placeholders require an identifier".into(),
            ));
        }
        let index = Regex::new(r"^%?([0-9]+)\$")
            .expect("literal regex")
            .captures(raw)
            .map(|capture| format!("{}$", &capture[1]))
            .unwrap_or_default();
        let specifier = if raw.ends_with('@') {
            '@'
        } else if raw.ends_with(['d', 'i']) {
            'd'
        } else {
            's'
        };
        let output = match self.kind {
            BuilderFormatType::Number => match syntax {
                FormatSyntax::Composite => format!(
                    "{{{name}:{}{}}}",
                    if self.percentage { 'P' } else { 'C' },
                    self.decimal_places
                ),
                FormatSyntax::I18Next => format!(
                    "{{{{{name}, {}}}}}",
                    if self.percentage { "percent" } else { "number" }
                ),
                FormatSyntax::Printf => format!("%{index}.{}f", self.decimal_places),
                FormatSyntax::Named => format!(
                    "{{{name}, number, {}}}",
                    if self.percentage {
                        "percent"
                    } else {
                        "currency"
                    }
                ),
            },
            BuilderFormatType::Date => {
                if self.date_pattern.is_empty() || self.date_pattern.contains(['{', '}']) {
                    return Err(StudioError::InvalidInput(
                        "Invalid date format pattern".into(),
                    ));
                }
                match syntax {
                    FormatSyntax::Composite => format!("{{{name}:{}}}", self.date_pattern),
                    FormatSyntax::I18Next => format!("{{{{{name}, date}}}}"),
                    FormatSyntax::Printf => format!("%{index}{specifier}"),
                    FormatSyntax::Named => format!("{{{name}, date, {}}}", self.date_pattern),
                }
            }
            BuilderFormatType::Plural | BuilderFormatType::Select => {
                if syntax != FormatSyntax::Named {
                    return Err(StudioError::InvalidInput(
                        "Plural/select snippets require a named/ICU message format".into(),
                    ));
                }
                let kind = if self.kind == BuilderFormatType::Plural {
                    "plural"
                } else {
                    "select"
                };
                let mut output = format!("{{{name}, {kind},");
                for (key, value) in &self.branches {
                    if key == "other" || value.is_empty() {
                        continue;
                    }
                    if !valid_name(key)
                        && !(key.starts_with('=') && key[1..].parse::<i64>().is_ok())
                    {
                        return Err(StudioError::InvalidInput(
                            "Invalid plural/select branch".into(),
                        ));
                    }
                    output.push_str(&format!(" {key} {{{value}}}"));
                }
                output.push_str(&format!(
                    " other {{{}}}}}",
                    self.branches
                        .get("other")
                        .filter(|value| !value.is_empty())
                        .map(String::as_str)
                        .unwrap_or("Value")
                ));
                output
            }
            BuilderFormatType::Text => match syntax {
                FormatSyntax::I18Next => format!("{{{{{name}}}}}"),
                FormatSyntax::Printf => format!("%{index}{specifier}"),
                _ => format!("{{{name}}}"),
            },
        };
        placeholders(&output, format)?;
        Ok(output)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormatSuggestion {
    pub insertion: String,
    pub label: String,
}
pub fn autocomplete(
    source: &str,
    format: TranslationFileFormat,
    trigger: &str,
) -> Result<Vec<FormatSuggestion>, StudioError> {
    let syntax = FormatSyntax::for_format(format);
    let trigger = match (syntax, trigger) {
        (FormatSyntax::Composite | FormatSyntax::Named, "{") => "{",
        (FormatSyntax::I18Next, "{" | "{{") => "{{",
        (FormatSyntax::Printf, "%") => "%",
        (FormatSyntax::Named, "%")
            if matches!(
                format,
                TranslationFileFormat::AppleStringsdict | TranslationFileFormat::AppleXcstrings
            ) =>
        {
            "%"
        }
        _ => return Ok(Vec::new()),
    };
    let mut result = Vec::new();
    let mut seen = BTreeSet::new();
    let mut push = |insertion: String, label: &str| {
        if seen.insert(insertion.clone()) {
            result.push(FormatSuggestion {
                insertion,
                label: label.into(),
            });
        }
    };
    for placeholder in placeholders(source, format)? {
        if (trigger == "%") != placeholder.raw.starts_with('%') {
            continue;
        }
        let name = &placeholder.name;
        match trigger {
            "{" => {
                push(placeholder.raw.clone(), "Source placeholder");
                if FormatSyntax::for_format(format) == FormatSyntax::Composite {
                    for (specifier, label) in [
                        ("C", "Currency"),
                        ("D", "Long date"),
                        ("d", "Short date"),
                        ("N2", "Decimal"),
                    ] {
                        push(format!("{{{name}:{specifier}}}"), label);
                    }
                } else {
                    push(format!("{{{name}, number, currency}}"), "Currency");
                    push(format!("{{{name}, date, short}}"), "Date");
                    push(
                        format!("{{{name}, plural, one {{1 item}} other {{# items}}}}"),
                        "Plural",
                    );
                }
            }
            "{{" => {
                push(placeholder.raw.clone(), "Source placeholder");
                push(format!("{{{{{name}, number}}}}"), "Number");
                push(format!("{{{{{name}, date}}}}"), "Date");
            }
            "%" => {
                let index = if placeholder.raw.contains('$') {
                    format!("{name}$")
                } else {
                    String::new()
                };
                push(placeholder.raw.clone(), "Source placeholder");
                for (specifier, label) in [("s", "String"), ("d", "Integer"), (".2f", "Decimal")] {
                    push(format!("%{index}{specifier}"), label);
                }
                if matches!(
                    format,
                    TranslationFileFormat::AppleStrings
                        | TranslationFileFormat::AppleStringsdict
                        | TranslationFileFormat::AppleXcstrings
                ) {
                    push(format!("%{index}@"), "Object/string");
                }
            }
            _ => {}
        }
    }
    Ok(result)
}

#[derive(Clone, Debug)]
pub struct FormatPreviewValues {
    pub arguments: Vec<FormatArgument>,
    pub number: FormatArgument,
    pub date: FormatArgument,
    pub text: String,
    pub count: i64,
    pub selection: String,
}
impl FormatPreviewValues {
    /// The host supplies a date so previews are deterministic and timezone-explicit.
    pub fn new(date: impl Into<FormatArgument>) -> Self {
        let date = date.into();
        Self {
            arguments: vec![
                FormatArgument::from(1234.56),
                date.clone(),
                FormatArgument::from("Demo"),
                FormatArgument::from(42),
            ],
            number: FormatArgument::from(1234.56),
            date,
            text: "Demo".into(),
            count: 5,
            selection: "other".into(),
        }
    }
}

/// Composite messages execute the real formatter. Other syntaxes are deliberate
/// editor simulations using supplied sample values, not target runtime engines.
pub fn simulate_format(
    text: &str,
    format: TranslationFileFormat,
    culture: &Culture,
    values: &FormatPreviewValues,
) -> Result<String, StudioError> {
    placeholders(text, format)?;
    if FormatSyntax::for_format(format) == FormatSyntax::Composite {
        return format_composite(text, &values.arguments, culture)
            .map_err(|error| StudioError::InvalidInput(error.to_string()));
    }
    simulate_text(text, format, culture, values, 0)
}

fn simulate_text(
    text: &str,
    format: TranslationFileFormat,
    culture: &Culture,
    values: &FormatPreviewValues,
    depth: usize,
) -> Result<String, StudioError> {
    if depth > 32 {
        return Err(StudioError::InvalidInput(
            "Preview nesting exceeds 32 levels".into(),
        ));
    }
    let tokens = placeholders(text, format)?;
    let mut result = String::new();
    let mut position = 0;
    for token in tokens {
        if token.start < position {
            continue;
        }
        result.push_str(&text[position..token.start]);
        let formatted = if token.raw.starts_with('%') {
            let kind = token.raw.chars().last().unwrap_or('s');
            match kind {
                'd' | 'i' | 'u' | 'x' | 'X' | 'o' => "123".into(),
                'f' | 'F' | 'g' | 'G' | 'e' | 'E' => values
                    .number
                    .format("N2", culture)
                    .map_err(|error| StudioError::InvalidInput(error.to_string()))?,
                's' | '@' | 'c' => values.text.clone(),
                _ => "Value".into(),
            }
        } else {
            let inner = token.raw.trim_matches(['{', '}']);
            let mut parts = inner.splitn(3, ',');
            let _ = parts.next();
            let kind = parts.next().map(str::trim).unwrap_or("");
            let style = parts.next().map(str::trim).unwrap_or("");
            match kind {
                "plural" | "select" | "selectordinal" => {
                    // Strip exactly one pair; nested branch braces are meaningful.
                    let original_inner = &token.raw[1..token.raw.len() - 1];
                    let style = original_inner.splitn(3, ',').nth(2).unwrap_or("").trim();
                    let variants = branches(style)?;
                    let count = values
                        .count
                        .checked_sub(plural_offset(style)?.0)
                        .ok_or_else(|| {
                            StudioError::InvalidInput("Plural preview offset overflows".into())
                        })?;
                    let exact = format!("={}", values.count);
                    let selected = if kind == "select" {
                        values.selection.as_str()
                    } else if variants.iter().any(|(key, _, _)| *key == exact) {
                        exact.as_str()
                    } else if count == 1 {
                        "one"
                    } else {
                        "other"
                    };
                    let body = variants
                        .iter()
                        .find(|(key, _, _)| *key == selected)
                        .or_else(|| variants.iter().find(|(key, _, _)| *key == "other"))
                        .ok_or_else(invalid_named)?
                        .1;
                    simulate_text(
                        &body.replace('#', &count.to_string()),
                        format,
                        culture,
                        values,
                        depth + 1,
                    )?
                }
                "number" | "currency" | "percent" => {
                    let specifier = if kind == "currency" || style == "currency" {
                        "C2"
                    } else if kind == "percent" || style == "percent" {
                        "P2"
                    } else {
                        "N2"
                    };
                    values
                        .number
                        .format(specifier, culture)
                        .map_err(|error| StudioError::InvalidInput(error.to_string()))?
                }
                "date" | "time" => {
                    let pattern = match style {
                        "" | "short" => "d",
                        "long" => "D",
                        pattern => pattern,
                    };
                    values
                        .date
                        .format(pattern, culture)
                        .map_err(|error| StudioError::InvalidInput(error.to_string()))?
                }
                _ => values.text.clone(),
            }
        };
        result.push_str(&formatted);
        position = token.end;
    }
    result.push_str(&text[position..]);
    if FormatSyntax::for_format(format) == FormatSyntax::Printf {
        result = result.replace("%%", "%");
    }
    Ok(result)
}
