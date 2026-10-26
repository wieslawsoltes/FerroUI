use crate::{model::select_entries, *};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub(crate) fn import(
    content: &str,
    format: TranslationFileFormat,
    options: &TranslationFormatOptions,
    path: Option<&str>,
    result: &mut TranslationFormatResult,
) -> Result<(), String> {
    match format {
        TranslationFileFormat::AppleStrings => import_apple(content, options, path, result),
        TranslationFileFormat::GettextPo => import_po(content, options, path, result),
        TranslationFileFormat::Csv | TranslationFileFormat::Tsv => {
            import_delimited(content, delimiter(format, options)?, options, result)
        }
        _ => unreachable!(),
    }
}

fn delimiter(
    format: TranslationFileFormat,
    options: &TranslationFormatOptions,
) -> Result<char, String> {
    let delimiter = if format == TranslationFileFormat::Tsv {
        '\t'
    } else {
        options.delimiter
    };
    if matches!(delimiter, '"' | '\r' | '\n' | '\0') {
        Err("Delimiter must not be a quote, newline, or NUL.".into())
    } else {
        Ok(delimiter)
    }
}

pub(crate) fn decode_escapes(value: &str, strict: bool) -> Result<String, String> {
    let mut input = value.chars().peekable();
    let mut output = String::new();
    while let Some(c) = input.next() {
        if c != '\\' {
            output.push(c);
            continue;
        }
        let escaped = input.next().ok_or("Trailing backslash in string escape.")?;
        match escaped {
            'n' => output.push('\n'),
            'r' => output.push('\r'),
            't' => output.push('\t'),
            'b' => output.push('\u{8}'),
            'f' => output.push('\u{c}'),
            'v' => output.push('\u{b}'),
            'a' => output.push('\u{7}'),
            '\\' | '"' | '\'' | '?' | '@' => output.push(escaped),
            'u' | 'U' => {
                let digits: String = input.by_ref().take(4).collect();
                if digits.len() != 4 {
                    return Err("Unicode escapes require four hexadecimal digits.".into());
                }
                let mut code =
                    u32::from_str_radix(&digits, 16).map_err(|_| "Invalid Unicode escape.")?;
                if (0xd800..=0xdbff).contains(&code) {
                    if input.next() != Some('\\') || !matches!(input.next(), Some('u' | 'U')) {
                        return Err(
                            "High surrogate requires a matching low-surrogate escape.".into()
                        );
                    }
                    let low: String = input.by_ref().take(4).collect();
                    let low = u32::from_str_radix(&low, 16)
                        .map_err(|_| "Invalid low-surrogate escape.")?;
                    if !(0xdc00..=0xdfff).contains(&low) {
                        return Err("Invalid low-surrogate escape.".into());
                    }
                    code = 0x10000 + ((code - 0xd800) << 10) + low - 0xdc00;
                }
                output.push(char::from_u32(code).ok_or("Invalid Unicode scalar value.")?);
            }
            '0'..='7' => {
                let mut code = escaped.to_digit(8).unwrap();
                for _ in 0..2 {
                    if let Some(next) = input.peek().copied().filter(|c| matches!(c, '0'..='7')) {
                        input.next();
                        code = code * 8 + next.to_digit(8).unwrap();
                    } else {
                        break;
                    }
                }
                output.push(char::from_u32(code).ok_or("Invalid octal escape.")?);
            }
            _ if strict => return Err(format!("Unsupported string escape '\\{escaped}'.")),
            _ => {
                output.push('\\');
                output.push(escaped);
            }
        }
    }
    Ok(output)
}

fn escaped(value: &str) -> String {
    let mut output = String::new();
    for c in value.chars() {
        match c {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            c if c.is_ascii_control() => {
                write!(output, "\\{:03o}", c as u32).unwrap();
            }
            _ => output.push(c),
        }
    }
    output
}

struct AppleParser<'a> {
    text: &'a str,
    offset: usize,
    comment: Option<String>,
}
impl<'a> AppleParser<'a> {
    fn peek(&self) -> Option<char> {
        self.text[self.offset..].chars().next()
    }
    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.offset += c.len_utf8();
        Some(c)
    }
    fn whitespace(&mut self) -> Result<(), String> {
        loop {
            while self.peek().is_some_and(char::is_whitespace) {
                self.advance();
            }
            if self.text[self.offset..].starts_with("/*") {
                self.offset += 2;
                let end = self.text[self.offset..]
                    .find("*/")
                    .ok_or("Unterminated .strings block comment.")?;
                let next = self.text[self.offset..self.offset + end].trim().to_owned();
                append(&mut self.comment, &next);
                self.offset += end + 2;
            } else if self.text[self.offset..].starts_with("//") {
                self.offset += 2;
                let end = self.text[self.offset..]
                    .find('\n')
                    .unwrap_or(self.text.len() - self.offset);
                let next = self.text[self.offset..self.offset + end].trim().to_owned();
                append(&mut self.comment, &next);
                self.offset += end;
            } else {
                return Ok(());
            }
        }
    }
    fn quoted(&mut self) -> Result<String, String> {
        if self.advance() != Some('"') {
            return Err(format!(
                "Expected a quoted .strings token at byte {}.",
                self.offset
            ));
        }
        let start = self.offset;
        let mut backslash = false;
        while let Some(c) = self.advance() {
            if backslash {
                backslash = false;
            } else if c == '\\' {
                backslash = true;
            } else if c == '"' {
                return decode_escapes(&self.text[start..self.offset - 1], true);
            }
        }
        Err("Unterminated .strings quoted token.".into())
    }
}
fn append(value: &mut Option<String>, next: &str) {
    if let Some(value) = value {
        value.push('\n');
        value.push_str(next);
    } else {
        *value = Some(next.into());
    }
}

fn import_apple(
    content: &str,
    options: &TranslationFormatOptions,
    path: Option<&str>,
    result: &mut TranslationFormatResult,
) -> Result<(), String> {
    let culture = options
        .culture
        .clone()
        .or_else(|| infer_apple_culture(path));
    let mut parser = AppleParser {
        text: content,
        offset: 0,
        comment: None,
    };
    loop {
        parser.whitespace()?;
        if parser.peek().is_none() {
            return Ok(());
        }
        let key = parser.quoted()?;
        parser.whitespace()?;
        if parser.advance() != Some('=') {
            return Err(format!(".strings entry '{key}' has no '='."));
        }
        parser.whitespace()?;
        let value = parser.quoted()?;
        parser.whitespace()?;
        if parser.advance() != Some(';') {
            return Err(format!(
                ".strings entry '{key}' has no terminating semicolon."
            ));
        }
        let entry = result.catalog.add(key, culture.as_deref(), value);
        entry.comment = parser.comment.take();
    }
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct PoEntry {
    id: Option<String>,
    plural: Option<String>,
    target: Option<String>,
    context: Option<String>,
    translations: BTreeMap<usize, String>,
    comment: Option<String>,
    metadata: BTreeMap<String, String>,
    fuzzy: bool,
}
#[derive(Clone, Copy)]
enum PoField {
    Id,
    Plural,
    Target,
    Context,
    Index(usize),
}
impl PoEntry {
    fn active(&mut self, field: PoField) -> &mut String {
        match field {
            PoField::Id => self.id.get_or_insert_with(String::new),
            PoField::Plural => self.plural.get_or_insert_with(String::new),
            PoField::Target => self.target.get_or_insert_with(String::new),
            PoField::Context => self.context.get_or_insert_with(String::new),
            PoField::Index(i) => self.translations.entry(i).or_default(),
        }
    }
    fn complete(&self) -> bool {
        self.id.is_some() && (self.target.is_some() || !self.translations.is_empty())
    }
}
fn po_token(token: &str) -> Result<String, String> {
    let token = token.trim();
    let inner = token
        .strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .ok_or("PO values must be quoted strings.")?;
    let mut escaped = false;
    for c in inner.chars() {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return Err("Unescaped quotation mark in PO token.".into());
        }
    }
    // Keep literal boundaries until all continuation lines have been collected:
    // an escaped UTF-8 sequence may span adjacent literals, while a hex escape
    // must stop at the closing quote rather than absorb the next literal.
    Ok(token.into())
}

fn decode_po_literals(literals: &str) -> Result<String, String> {
    let mut output = Vec::new();
    for literal in literals.split('\n') {
        let inner = literal
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .ok_or("PO values must be quoted strings.")?;
        let mut bytes = inner.bytes().peekable();
        while let Some(byte) = bytes.next() {
            if byte != b'\\' {
                output.push(byte);
                continue;
            }
            let escaped = bytes.next().ok_or("Trailing backslash in PO string.")?;
            output.push(match escaped {
                b'a' => 7, b'b' => 8, b't' => 9, b'n' => 10, b'v' => 11, b'f' => 12, b'r' => 13,
                b'\\' | b'"' | b'\'' | b'?' => escaped,
                b'0'..=b'7' => {
                    let mut value = u32::from(escaped - b'0');
                    for _ in 0..2 {
                        if let Some(next) = bytes.peek().copied().filter(|value| matches!(value, b'0'..=b'7')) {
                            bytes.next(); value = value * 8 + u32::from(next - b'0');
                        } else { break; }
                    }
                    u8::try_from(value).map_err(|_| "PO octal escape exceeds one byte.")?
                }
                b'x' => {
                    let mut value = 0u32;
                    let mut count = 0;
                    while let Some(next) = bytes.peek().copied().filter(u8::is_ascii_hexdigit) {
                        bytes.next(); count += 1;
                        value = value.checked_mul(16).and_then(|value| value.checked_add((next as char).to_digit(16).unwrap())).ok_or("PO hex escape exceeds one byte.")?;
                    }
                    if count == 0 { return Err("PO hex escape requires a hexadecimal digit.".into()); }
                    u8::try_from(value).map_err(|_| "PO hex escape exceeds one byte.")?
                }
                b'u' | b'U' => return Err("PO syntax does not support universal Unicode escapes; use UTF-8 text or byte escapes.".into()),
                _ => return Err(format!("Unsupported PO escape '\\{}'.", escaped as char)),
            });
        }
    }
    String::from_utf8(output).map_err(|_| "PO string escapes do not form valid UTF-8; decode the source encoding before importing.".into())
}
fn parse_po(
    content: &str,
    diagnostics: &mut Vec<TranslationFormatDiagnostic>,
) -> Result<Vec<PoEntry>, String> {
    let mut entries = Vec::new();
    let mut current = PoEntry::default();
    let mut active = None;
    for (line_index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            if current.id.is_some() {
                entries.push(std::mem::take(&mut current));
            }
            active = None;
            continue;
        }
        if line.starts_with("#~") {
            diagnostics.push(TranslationFormatDiagnostic::new(
                Severity::Info,
                "PTF174",
                format!("Obsolete PO line {} was skipped.", line_index + 1),
            ));
            continue;
        }
        if line.starts_with('#') {
            if current.complete() {
                entries.push(std::mem::take(&mut current));
                active = None;
            }
            if let Some(comment) = line.strip_prefix("#.") {
                append(&mut current.comment, comment.trim());
            } else if let Some(flags) = line.strip_prefix("#,").or_else(|| line.strip_prefix("#="))
            {
                if line.starts_with("#=") {
                    current
                        .metadata
                        .insert("po.flags_prefix".into(), "=".into());
                }
                current.fuzzy |= flags.split(',').any(|f| f.trim() == "fuzzy");
                current
                    .metadata
                    .entry("po.flags".into())
                    .and_modify(|value| {
                        value.push_str(", ");
                        value.push_str(flags.trim());
                    })
                    .or_insert_with(|| flags.trim().into());
            } else if let Some(previous) = line.strip_prefix("#|") {
                current
                    .metadata
                    .entry("po.previous".into())
                    .and_modify(|value| {
                        value.push('\n');
                        value.push_str(previous.trim());
                    })
                    .or_insert_with(|| previous.trim().into());
            } else if let Some(reference) = line.strip_prefix("#:") {
                current
                    .metadata
                    .entry("po.references".into())
                    .and_modify(|v| {
                        v.push('\n');
                        v.push_str(reference.trim());
                    })
                    .or_insert_with(|| reference.trim().into());
            } else if let Some(comment) = line.strip_prefix('#') {
                current
                    .metadata
                    .entry("po.translator_comment".into())
                    .and_modify(|v| {
                        v.push('\n');
                        v.push_str(comment.trim());
                    })
                    .or_insert_with(|| comment.trim().into());
            }
            continue;
        }
        if line.starts_with('"') {
            let field = active.ok_or_else(|| {
                format!(
                    "PO continuation at line {} has no directive.",
                    line_index + 1
                )
            })?;
            let token = po_token(line)?;
            current.active(field).push('\n');
            current.active(field).push_str(&token);
            continue;
        }
        let directive = |name: &str| {
            line.strip_prefix(name)
                .filter(|rest| rest.starts_with(char::is_whitespace))
                .map(str::trim_start)
        };
        let (field, token) = if let Some(token) = directive("msgid_plural") {
            if current.plural.is_some()
                || current.target.is_some()
                || !current.translations.is_empty()
            {
                return Err("Duplicate or misplaced PO msgid_plural directive.".into());
            }
            (PoField::Plural, token)
        } else if let Some(token) = directive("msgid") {
            if current.id.is_some() {
                if !current.complete() {
                    return Err("PO entry is missing msgstr before the next msgid.".into());
                }
                entries.push(std::mem::take(&mut current));
            }
            (PoField::Id, token)
        } else if let Some(token) = directive("msgctxt") {
            if current.complete() {
                entries.push(std::mem::take(&mut current));
            }
            if current.context.is_some() || current.id.is_some() {
                return Err("Duplicate or misplaced PO msgctxt directive.".into());
            }
            (PoField::Context, token)
        } else if let Some(token) = line.strip_prefix("msgstr[") {
            let (index, token) = token
                .split_once(']')
                .ok_or("Malformed indexed PO msgstr.")?;
            if !token.starts_with(char::is_whitespace) {
                return Err("Indexed PO msgstr must be followed by whitespace.".into());
            }
            let index: usize = index
                .parse()
                .map_err(|_| "PO plural index must be a nonnegative integer.")?;
            if index > 65535 {
                return Err("PO plural index is too large.".into());
            }
            if current.translations.contains_key(&index) {
                return Err("Duplicate indexed PO msgstr.".into());
            }
            (PoField::Index(index), token.trim())
        } else if let Some(token) = directive("msgstr") {
            if current.target.is_some() {
                return Err("Duplicate PO msgstr directive.".into());
            }
            (PoField::Target, token)
        } else {
            return Err(format!(
                "Unrecognized PO directive at line {}.",
                line_index + 1
            ));
        };
        if !matches!(field, PoField::Id | PoField::Context) && current.id.is_none() {
            return Err(format!(
                "PO directive at line {} precedes msgid.",
                line_index + 1
            ));
        }
        *current.active(field) = po_token(token)?;
        active = Some(field);
    }
    if current.id.is_some() {
        entries.push(current);
    } else if current.context.is_some() {
        return Err("PO context has no msgid.".into());
    }
    for entry in &mut entries {
        if entry.target.is_none() && entry.translations.is_empty() {
            return Err("PO entry is missing msgstr.".into());
        }
        if !entry.translations.is_empty() && entry.plural.is_none() {
            return Err("Indexed PO translations require msgid_plural.".into());
        }
        if !entry.translations.is_empty() && entry.target.is_some() {
            return Err("PO entry mixes singular and plural msgstr directives.".into());
        }
        if entry.plural.is_some() && entry.translations.is_empty() {
            return Err("PO msgid_plural requires indexed msgstr directives.".into());
        }
        if entry
            .translations
            .keys()
            .copied()
            .enumerate()
            .any(|(expected, actual)| expected != actual)
        {
            return Err("PO plural indices must start at zero and be consecutive.".into());
        }
        for value in [
            &mut entry.id,
            &mut entry.plural,
            &mut entry.target,
            &mut entry.context,
        ]
        .into_iter()
        .flatten()
        .chain(entry.translations.values_mut())
        {
            *value = decode_po_literals(value)?;
        }
    }
    Ok(entries)
}

fn import_po(
    content: &str,
    options: &TranslationFormatOptions,
    path: Option<&str>,
    result: &mut TranslationFormatResult,
) -> Result<(), String> {
    let entries = parse_po(content, &mut result.diagnostics)?;
    let mut culture = options
        .culture
        .clone()
        .or_else(|| infer_culture(path, ".po"))
        .or_else(|| infer_culture(path, ".pot"));
    let mut header = BTreeMap::new();
    for entry in &entries {
        if entry.id.as_deref() == Some("") && entry.context.is_none() {
            if result.catalog.metadata.contains_key("po.header_entry") {
                return Err("PO catalog contains multiple header entries.".into());
            }
            if entry.plural.is_some() {
                return Err("PO header cannot contain plural forms.".into());
            }
            result.catalog.metadata.insert(
                "po.header_entry".into(),
                serde_json::to_string(entry).map_err(|e| e.to_string())?,
            );
            for line in entry.target.as_deref().unwrap_or("").lines() {
                if let Some((key, value)) = line.split_once(':') {
                    header.insert(key.trim().to_ascii_lowercase(), value.trim().to_owned());
                }
            }
        }
    }
    let obsolete = content
        .lines()
        .filter(|line| line.trim_start().starts_with("#~"))
        .collect::<Vec<_>>()
        .join("\n");
    if !obsolete.is_empty() {
        result
            .catalog
            .metadata
            .insert("po.obsolete".into(), obsolete);
    }
    if let Some(language) = header.get("language").filter(|s| !s.is_empty()) {
        culture = Some(TranslationCatalog::normalize_culture_name(Some(language)));
    }
    let plural_count = header
        .get("plural-forms")
        .map(|forms| po_plural_count(forms))
        .transpose()?;
    for entry in entries {
        let id = entry.id.as_deref().unwrap_or("");
        if id.is_empty() && entry.context.is_none() {
            continue;
        }
        let key = match &entry.context {
            Some(context) => format!("{context}\u{4}{id}"),
            None => id.into(),
        };
        let mut metadata = entry.metadata.clone();
        if let Some(forms) = header.get("plural-forms") {
            metadata.insert("po.plural_forms".into(), forms.clone());
        }
        if entry.translations.is_empty() {
            let target = entry.target.as_deref().unwrap_or("");
            let value = if target.is_empty() {
                metadata.insert("po.untranslated".into(), "true".into());
                id
            } else {
                target
            };
            let record = result.catalog.add(key, culture.as_deref(), value);
            record.source = Some(id.into());
            record.context = entry.context;
            record.comment = entry.comment;
            record.metadata = metadata;
            if entry.fuzzy {
                record.state = Some("fuzzy".into());
            }
        } else {
            if plural_count.is_some_and(|count| entry.translations.len() != count) {
                return Err(format!(
                    "PO plural '{key}' has {} translations but Plural-Forms requires {}.",
                    entry.translations.len(),
                    plural_count.unwrap()
                ));
            }
            metadata.insert(
                "po.msgid_plural".into(),
                entry.plural.clone().unwrap_or_default(),
            );
            if entry.translations.len() > 2 || header.contains_key("plural-forms") {
                result.diagnostics.push(TranslationFormatDiagnostic::new(Severity::Warning,"PTF171",format!("PO plural indices for '{key}' are retained; plural formulas are not evaluated into CLDR categories.")));
            }
            for (index, target) in entry.translations {
                let category = match index {
                    0 => "one".into(),
                    1 => "other".into(),
                    _ => format!("index{index}"),
                };
                let record =
                    result
                        .catalog
                        .add(format!("{key}.{category}"), culture.as_deref(), target);
                record.source = Some(if index == 0 {
                    id.into()
                } else {
                    entry.plural.clone().unwrap_or_default()
                });
                record.context = entry.context.clone();
                record.comment = entry.comment.clone();
                record.plural_category = Some(category);
                record.metadata = metadata.clone();
                record.metadata.insert("po.index".into(), index.to_string());
                if entry.fuzzy {
                    record.state = Some("fuzzy".into());
                }
            }
        }
    }
    if result.catalog.entries.is_empty() {
        result.diagnostics.push(TranslationFormatDiagnostic::new(
            Severity::Warning,
            "PTF170",
            "No gettext PO entries were imported.",
        ));
    }
    Ok(())
}

fn parse_rows(content: &str, delimiter: char) -> Result<Vec<Vec<String>>, String> {
    #[derive(Clone, Copy)]
    enum State {
        Start,
        Bare,
        Quoted,
        Closed,
    }
    let mut state = State::Start;
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut input = content.chars().peekable();
    while let Some(c) = input.next() {
        if matches!(state, State::Quoted) {
            if c == '"' {
                if input.peek() == Some(&'"') {
                    input.next();
                    field.push('"');
                } else {
                    state = State::Closed;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        if c == delimiter {
            row.push(std::mem::take(&mut field));
            state = State::Start;
        } else if c == '\n' || c == '\r' {
            if c == '\r' && input.peek() == Some(&'\n') {
                input.next();
            }
            row.push(std::mem::take(&mut field));
            rows.push(std::mem::take(&mut row));
            state = State::Start;
        } else if c == '"' && matches!(state, State::Start) {
            state = State::Quoted;
        } else if c == '"' {
            return Err("Unescaped quote in delimited field.".into());
        } else if matches!(state, State::Closed) {
            return Err("Unexpected characters after a quoted delimited field.".into());
        } else {
            field.push(c);
            state = State::Bare;
        }
    }
    if matches!(state, State::Quoted) {
        return Err("Unterminated quoted delimited field.".into());
    }
    if !row.is_empty() || !field.is_empty() || matches!(state, State::Closed | State::Bare) {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}
fn formula(value: &str) -> bool {
    value.starts_with(['=', '+', '-', '@', '\t', '\r'])
}

#[derive(Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct CatalogProperties {
    name: Option<String>,
    source_culture: Option<String>,
    metadata: BTreeMap<String, String>,
}

fn import_delimited(
    content: &str,
    delimiter: char,
    options: &TranslationFormatOptions,
    result: &mut TranslationFormatResult,
) -> Result<(), String> {
    let rows = parse_rows(content, delimiter)?;
    let Some(header) = rows.first() else {
        return Ok(());
    };
    let mut columns = BTreeMap::new();
    for (index, name) in header.iter().enumerate() {
        if columns
            .insert(name.trim().to_ascii_lowercase(), index)
            .is_some()
        {
            return Err("Duplicate delimited column name.".into());
        }
    }
    if !columns.contains_key("key") || !columns.contains_key("value") {
        result.diagnostics.push(TranslationFormatDiagnostic::new(
            Severity::Error,
            "PTF190",
            "Delimited catalogs require 'key' and 'value' columns.",
        ));
        return Ok(());
    }
    let mut properties = None;
    for (row_index, row) in rows.iter().enumerate().skip(1) {
        if row.iter().all(|field| field.is_empty()) {
            continue;
        }
        if row.len() != header.len() {
            return Err(format!(
                "Delimited row {} has {} columns; expected {}.",
                row_index + 1,
                row.len(),
                header.len()
            ));
        }
        let get = |key: &str| {
            columns
                .get(key)
                .and_then(|&index| row.get(index))
                .map(String::as_str)
        };
        let key = get("key").unwrap_or("");
        let retained = get("catalogmetadata").filter(|value| !value.is_empty());
        if let Some(value) = retained {
            let value = crate::json::parse_checked(value)?;
            let next: CatalogProperties = serde_json::from_value(value)
                .map_err(|e| format!("Invalid catalog metadata in row {}: {e}", row_index + 1))?;
            if properties
                .as_ref()
                .is_some_and(|properties| properties != &next)
            {
                return Err("Delimited rows have conflicting catalog metadata.".into());
            }
            if options.name.is_none() {
                result.catalog.name = next.name.clone().or(result.catalog.name.clone());
            }
            if options.source_culture.is_none() {
                result.catalog.source_culture = next
                    .source_culture
                    .clone()
                    .or(result.catalog.source_culture.clone());
            }
            result.catalog.metadata = next.metadata.clone();
            properties = Some(next);
        }
        if key.trim().is_empty() {
            if retained.is_some()
                && row.iter().enumerate().all(|(index, value)| {
                    Some(&index) == columns.get("catalogmetadata") || value.is_empty()
                })
            {
                continue;
            }
            result.diagnostics.push(TranslationFormatDiagnostic::new(
                Severity::Warning,
                "PTF191",
                format!("Ignored row {} with an empty key.", row_index + 1),
            ));
            continue;
        }
        let entry = result.catalog.add(
            key,
            get("culture")
                .filter(|s| !s.is_empty())
                .or(options.culture.as_deref()),
            get("value").unwrap_or(""),
        );
        entry.source = get("source").filter(|s| !s.is_empty()).map(str::to_owned);
        entry.comment = get("comment").filter(|s| !s.is_empty()).map(str::to_owned);
        entry.context = get("context").filter(|s| !s.is_empty()).map(str::to_owned);
        entry.state = get("state").filter(|s| !s.is_empty()).map(str::to_owned);
        entry.plural_category = get("plural").filter(|s| !s.is_empty()).map(str::to_owned);
        entry.format = get("format").filter(|s| !s.is_empty()).map(str::to_owned);
        if let Some(metadata) = get("metadata").filter(|s| !s.is_empty()) {
            entry.metadata = serde_json::from_value(crate::json::parse_checked(metadata)?)
                .map_err(|e| format!("Invalid metadata JSON in row {}: {e}", row_index + 1))?;
        }
        if row.iter().any(|value| formula(value)) {
            result.diagnostics.push(TranslationFormatDiagnostic::entry(Severity::Warning,"PTF192","Formula-looking spreadsheet cell retained as text; review spreadsheet import settings.",entry));
        }
    }
    if columns.keys().any(|name| {
        !matches!(
            name.as_str(),
            "key"
                | "culture"
                | "value"
                | "source"
                | "comment"
                | "context"
                | "state"
                | "plural"
                | "format"
                | "metadata"
                | "catalogmetadata"
        )
    }) {
        result.diagnostics.push(TranslationFormatDiagnostic::new(
            Severity::Warning,
            "PTF193",
            "Unrecognized delimited columns were ignored.",
        ));
    }
    Ok(())
}

pub(crate) fn export(
    catalog: &TranslationCatalog,
    format: TranslationFileFormat,
    options: &TranslationFormatOptions,
) -> Result<String, String> {
    match format {
        TranslationFileFormat::AppleStrings => {
            let mut output = String::new();
            let culture = options
                .culture
                .clone()
                .or_else(|| catalog.cultures().into_iter().next());
            for entry in select_entries(catalog, culture.as_deref()) {
                if options.include_comments {
                    if let Some(comment) = &entry.comment {
                        for line in comment.split('\n') {
                            writeln!(output, "// {line}").unwrap();
                        }
                    }
                }
                writeln!(
                    output,
                    "\"{}\" = \"{}\";",
                    escaped(&entry.key),
                    escaped(&entry.value)
                )
                .unwrap();
            }
            Ok(output)
        }
        TranslationFileFormat::GettextPo => export_po(catalog, options),
        TranslationFileFormat::Csv | TranslationFileFormat::Tsv => {
            export_delimited(catalog, options, delimiter(format, options)?)
        }
        _ => unreachable!(),
    }
}

fn write_po_notes(
    output: &mut String,
    entry: &TranslationCatalogEntry,
    options: &TranslationFormatOptions,
) {
    if options.include_comments {
        if let Some(comment) = &entry.comment {
            for line in comment.lines() {
                writeln!(output, "#. {line}").unwrap();
            }
        }
        if let Some(comment) = entry.metadata.get("po.translator_comment") {
            for line in comment.lines() {
                writeln!(output, "# {line}").unwrap();
            }
        }
    }
    if let Some(references) = entry.metadata.get("po.references") {
        for line in references.lines() {
            writeln!(output, "#: {line}").unwrap();
        }
    }
    let flags = entry
        .metadata
        .get("po.flags")
        .map(String::as_str)
        .unwrap_or("");
    let mut flags: Vec<_> = flags
        .split(',')
        .map(str::trim)
        .filter(|flag| !flag.is_empty() && *flag != "fuzzy")
        .collect();
    if entry.state.as_deref() == Some("fuzzy") {
        flags.insert(0, "fuzzy");
    }
    if !flags.is_empty() {
        let prefix = if entry
            .metadata
            .get("po.flags_prefix")
            .is_some_and(|value| value == "=")
        {
            "="
        } else {
            ","
        };
        writeln!(output, "#{prefix} {}", flags.join(", ")).unwrap();
    }
    if let Some(previous) = entry.metadata.get("po.previous") {
        for line in previous.lines() {
            writeln!(output, "#| {line}").unwrap();
        }
    }
    if let Some(context) = po_context(entry) {
        writeln!(output, "msgctxt \"{}\"", escaped(context)).unwrap();
    }
}
fn po_id(entry: &TranslationCatalogEntry) -> &str {
    entry.source.as_deref().unwrap_or_else(|| {
        entry
            .base_key()
            .split_once('\u{4}')
            .map(|(_, key)| key)
            .unwrap_or_else(|| entry.base_key())
    })
}
fn po_context(entry: &TranslationCatalogEntry) -> Option<&str> {
    entry
        .context
        .as_deref()
        .or_else(|| entry.key.split_once('\u{4}').map(|(context, _)| context))
}
fn po_plural_count(forms: &str) -> Result<usize, String> {
    let mut values = forms
        .split(';')
        .filter_map(|field| field.split_once('='))
        .filter(|(name, _)| name.trim() == "nplurals")
        .map(|(_, value)| value.trim());
    let count = values
        .next()
        .ok_or("PO Plural-Forms requires nplurals.")?
        .parse::<usize>()
        .map_err(|_| "PO nplurals must be an integer.")?;
    if count == 0 || count > 65536 || values.next().is_some() {
        return Err("PO nplurals must occur once and be between 1 and 65536.".into());
    }
    Ok(count)
}
fn export_po(
    catalog: &TranslationCatalog,
    options: &TranslationFormatOptions,
) -> Result<String, String> {
    let culture = options
        .culture
        .clone()
        .or_else(|| catalog.cultures().into_iter().next());
    let entries = select_entries(catalog, culture.as_deref());
    let retained_header: PoEntry = match catalog.metadata.get("po.header_entry") {
        Some(value) => serde_json::from_value(crate::json::parse_checked(value)?)
            .map_err(|e| format!("Invalid retained PO header: {e}"))?,
        None => PoEntry::default(),
    };
    let header = retained_header.target.as_deref().unwrap_or("");
    let forms: Vec<_> = entries
        .iter()
        .filter_map(|entry| entry.metadata.get("po.plural_forms"))
        .collect();
    if let Some(first) = forms.first() {
        if forms.iter().any(|forms| *forms != *first) {
            return Err("PO entries disagree on Plural-Forms header.".into());
        }
    }
    let forms = forms.first().map(|forms| forms.as_str()).or_else(|| {
        header
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.trim().eq_ignore_ascii_case("plural-forms"))
            .map(|(_, value)| value.trim())
    });
    let plural_count = forms.map(po_plural_count).transpose()?;
    let mut output = String::new();
    write_po_notes(
        &mut output,
        &TranslationCatalogEntry {
            comment: retained_header.comment,
            metadata: retained_header.metadata,
            state: retained_header.fuzzy.then(|| "fuzzy".into()),
            ..Default::default()
        },
        options,
    );
    output.push_str("msgid \"\"\nmsgstr \"\"\n");
    for line in header.lines().filter(|line| {
        !line.split_once(':').is_some_and(|(name, _)| {
            matches!(
                name.trim().to_ascii_lowercase().as_str(),
                "content-type" | "language" | "plural-forms"
            )
        })
    }) {
        writeln!(output, "\"{}\\n\"", escaped(line)).unwrap();
    }
    output.push_str("\"Content-Type: text/plain; charset=UTF-8\\n\"\n");
    if let Some(culture) = &culture {
        writeln!(output, "\"Language: {}\\n\"", escaped(culture)).unwrap();
    }
    if let Some(forms) = forms {
        writeln!(output, "\"Plural-Forms: {}\\n\"", escaped(forms)).unwrap();
    }
    output.push('\n');
    let mut identities = BTreeSet::new();
    let mut plurals = BTreeMap::<&str, Vec<&TranslationCatalogEntry>>::new();
    for entry in entries {
        if entry.plural_category.is_some() {
            plurals.entry(entry.base_key()).or_default().push(entry);
            continue;
        }
        if po_id(entry).is_empty() && po_context(entry).is_none() {
            return Err("The empty gettext msgid is reserved for the header.".into());
        }
        if !identities.insert((po_context(entry), po_id(entry))) {
            return Err(format!(
                "Multiple records export to the same gettext context/msgid '{}'.",
                po_id(entry)
            ));
        }
        write_po_notes(&mut output, entry, options);
        writeln!(output, "msgid \"{}\"", escaped(po_id(entry))).unwrap();
        let value = if entry
            .metadata
            .get("po.untranslated")
            .is_some_and(|v| v == "true")
            && entry.source.as_deref() == Some(&entry.value)
        {
            ""
        } else {
            &entry.value
        };
        writeln!(output, "msgstr \"{}\"\n", escaped(value)).unwrap();
    }
    for (key, entries) in plurals {
        let mut indexed = BTreeMap::new();
        for entry in &entries {
            let index = if let Some(index) = entry.metadata.get("po.index") {
                index
                    .parse::<usize>()
                    .map_err(|_| "Invalid retained PO plural index.")?
            } else {
                match entry.plural_category.as_deref() { Some("one")=>0,Some("other")=>1,Some(category)=>return Err(format!("Cannot infer gettext index for CLDR category '{category}'; retain po.index and po.plural_forms metadata.")),None=>unreachable!() }
            };
            if index > 65535 {
                return Err("PO plural index is too large.".into());
            }
            if indexed.insert(index, *entry).is_some() {
                return Err(format!("Duplicate gettext plural index for '{key}'."));
            }
        }
        let singular = indexed
            .get(&0)
            .ok_or_else(|| format!("PO plural '{key}' needs an index zero / one record."))?;
        if plural_count.is_some_and(|count| indexed.len() != count) {
            return Err(format!(
                "PO plural '{key}' does not match the nplurals header."
            ));
        }
        if po_id(singular).is_empty() && po_context(singular).is_none() {
            return Err("The empty gettext msgid is reserved for the header.".into());
        }
        if !identities.insert((po_context(singular), po_id(singular))) {
            return Err(format!(
                "Multiple records export to the same gettext context/msgid '{}'.",
                po_id(singular)
            ));
        }
        if entries.iter().any(|entry| {
            po_context(entry) != po_context(singular)
                || entry.comment != singular.comment
                || entry.state != singular.state
                || [
                    "po.flags",
                    "po.flags_prefix",
                    "po.references",
                    "po.translator_comment",
                    "po.previous",
                ]
                .iter()
                .any(|key| entry.metadata.get(*key) != singular.metadata.get(*key))
        }) {
            return Err(format!(
                "PO plural '{key}' has conflicting shared context, comments, or flags."
            ));
        }
        let plural = indexed
            .iter()
            .find(|(i, _)| **i > 0)
            .map(|(_, entry)| *entry);
        let plural_id = singular.metadata.get("po.msgid_plural").map(String::as_str)
            .or_else(|| plural.map(po_id))
            .ok_or_else(|| format!("PO plural '{key}' needs retained po.msgid_plural or an additional plural record."))?;
        if entries
            .iter()
            .filter_map(|entry| entry.metadata.get("po.msgid_plural"))
            .any(|value| value != plural_id)
        {
            return Err(format!(
                "PO plural '{key}' has conflicting source plural messages."
            ));
        }
        write_po_notes(&mut output, singular, options);
        writeln!(
            output,
            "msgid \"{}\"\nmsgid_plural \"{}\"",
            escaped(po_id(singular)),
            escaped(plural_id)
        )
        .unwrap();
        for (expected, (index, entry)) in indexed.into_iter().enumerate() {
            if expected != index {
                return Err(format!("PO plural '{key}' has an index gap at {expected}."));
            }
            writeln!(output, "msgstr[{index}] \"{}\"", escaped(&entry.value)).unwrap();
        }
        output.push('\n');
    }
    if let Some(obsolete) = catalog.metadata.get("po.obsolete") {
        if obsolete
            .lines()
            .any(|line| !line.trim_start().starts_with("#~"))
        {
            return Err("Retained obsolete PO content must contain only #~ lines.".into());
        }
        output.push_str(obsolete);
        output.push('\n');
    }
    Ok(output)
}

fn append_row(output: &mut String, fields: &[&str], delimiter: char) {
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            output.push(delimiter);
        }
        if field.contains([delimiter, '"', '\r', '\n']) {
            output.push('"');
            output.push_str(&field.replace('"', "\"\""));
            output.push('"');
        } else {
            output.push_str(field);
        }
    }
    output.push('\n');
}
fn export_delimited(
    catalog: &TranslationCatalog,
    options: &TranslationFormatOptions,
    delimiter: char,
) -> Result<String, String> {
    let has_metadata = catalog
        .entries
        .iter()
        .any(|entry| !entry.metadata.is_empty());
    let mut header = vec![
        "key", "culture", "value", "source", "comment", "context", "state", "plural", "format",
    ];
    if has_metadata {
        header.push("metadata");
    }
    let properties = CatalogProperties {
        name: catalog.name.clone(),
        source_culture: catalog.source_culture.clone(),
        metadata: catalog.metadata.clone(),
    };
    let properties = if properties != CatalogProperties::default() {
        header.push("catalogMetadata");
        Some(serde_json::to_string(&properties).map_err(|e| e.to_string())?)
    } else {
        None
    };
    let mut output = String::new();
    append_row(&mut output, &header, delimiter);
    let mut entries: Vec<_> = catalog
        .entries
        .iter()
        .filter(|entry| {
            options.culture.as_deref().is_none_or(|culture| {
                entry
                    .culture
                    .as_deref()
                    .is_none_or(|tag| tag.eq_ignore_ascii_case(culture))
            })
        })
        .collect();
    entries.sort_by(|a, b| {
        a.key.cmp(&b.key).then_with(|| {
            a.culture
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase()
                .cmp(&b.culture.as_deref().unwrap_or("").to_ascii_lowercase())
        })
    });
    if entries.is_empty() {
        if let Some(properties) = &properties {
            let mut fields = vec![""; header.len()];
            fields[header.len() - 1] = properties;
            append_row(&mut output, &fields, delimiter);
        }
    }
    for (index, entry) in entries.into_iter().enumerate() {
        let metadata = if entry.metadata.is_empty() {
            String::new()
        } else {
            serde_json::to_string(&entry.metadata).map_err(|e| e.to_string())?
        };
        let mut fields = vec![
            entry.key.as_str(),
            entry.culture.as_deref().unwrap_or(""),
            &entry.value,
            entry.source.as_deref().unwrap_or(""),
            if options.include_comments {
                entry.comment.as_deref().unwrap_or("")
            } else {
                ""
            },
            entry.context.as_deref().unwrap_or(""),
            entry.state.as_deref().unwrap_or(""),
            entry.plural_category.as_deref().unwrap_or(""),
            entry.format.as_deref().unwrap_or(""),
        ];
        if has_metadata {
            fields.push(&metadata);
        }
        if let Some(properties) = &properties {
            fields.push(if index == 0 { properties } else { "" });
        }
        append_row(&mut output, &fields, delimiter);
    }
    Ok(output)
}
