use super::{FoldingError, FoldingManager, NewFolding};
use crate::document::TextDocument;
use quick_xml::errors::SyntaxError;
use quick_xml::events::attributes::{AttrError, Attributes};
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
use roxmltree::Error;

#[derive(Clone, Copy, Debug, Default)]
pub struct XmlFoldingStrategy {
    pub show_attributes_when_folded: bool,
}
impl XmlFoldingStrategy {
    /// Produces UTF-16 ranges for completed multiline elements and comments.
    /// An invalid document also returns the offset at which parsing stopped.
    pub fn create_new_foldings(&self, text: &str) -> (Vec<NewFolding>, Option<usize>) {
        // A document is already decoded text. A literal BOM is XML content.
        if text.starts_with('\u{feff}') {
            return (Vec::new(), Some(0));
        }
        // The tree parser validates the document, including namespace resolution
        // and references. Its error bounds a separate pass over the source
        // events, which can still yield complete regions from the valid prefix.
        let (limit, mut error, unresolved_namespace) = match roxmltree::Document::parse(text) {
            Ok(_) => (text.len(), None, false),
            Err(error) => {
                let unresolved_namespace = matches!(error, Error::UnknownNamespace(..));
                let (limit, offset) = error_location(text, error);
                (limit, Some(offset), unresolved_namespace)
            }
        };
        let mut reader = Reader::from_str(text);
        let mut folds = Vec::new();
        let mut open = Vec::new();
        let mut position = Position::default();
        while position.byte < limit {
            let start = position;
            let event = match reader.read_event() {
                Ok(event) => event,
                Err(parse_error) => {
                    let offset = match parse_error {
                        quick_xml::Error::Syntax(SyntaxError::UnclosedTag) => {
                            incomplete_start_tag_error(&text[start.byte..])
                                .map(|offset| start.byte + offset)
                        }
                        quick_xml::Error::Syntax(SyntaxError::UnclosedXmlDecl) if start.byte == 0 => {
                            invalid_declaration_value_offset(&text.as_bytes()[2..])
                                .map(|offset| offset + 2)
                        }
                        _ => None,
                    };
                    error = offset.filter(|&offset| offset <= limit).or(error);
                    error.get_or_insert(reader.error_position() as usize);
                    break;
                }
            };
            let end = reader.buffer_position() as usize;
            let attribute_error = match &event {
                Event::Start(tag) | Event::Empty(tag) if tag.name().as_ref().starts_with(b":") => {
                    Some(1)
                }
                Event::Start(tag) | Event::Empty(tag) => {
                    invalid_attribute_offset(tag.attributes(), tag.as_ref()).map(|offset| offset + 1)
                }
                Event::Decl(declaration) if start.byte == 0 => {
                    invalid_declaration_value_offset(declaration).map(|offset| offset + 2)
                }
                _ => None,
            };
            if let Some(offset) = attribute_error.map(|offset| start.byte + offset) {
                // Attribute values are read before the element's namespace is resolved.
                if offset <= limit || (unresolved_namespace && limit < end) {
                    error = Some(offset);
                    break;
                }
            }
            if let Event::PI(instruction) = &event {
                if let Some(offset) = invalid_instruction_target(instruction.target()) {
                    let offset = start.byte + 2 + offset;
                    if offset <= limit {
                        error = Some(offset);
                        break;
                    }
                }
            }
            if end > limit {
                break;
            }
            let Some(source) = text.get(start.byte..end) else {
                error.get_or_insert(start.byte);
                break;
            };
            position.advance(source);
            match event {
                Event::Start(tag) => open.push((start, tag)),
                Event::End(tag) => {
                    let Some((start_tag, opening)) = open.pop() else {
                        continue;
                    };
                    if start_tag.line == start.line {
                        continue;
                    }
                    let Some((name, suffix)) =
                        source.split_at_checked(2 + tag.name().as_ref().len())
                    else {
                        continue;
                    };
                    // The fold ends one column after the closing QName,
                    // clamped to the physical line containing the name.
                    let last = usize::from(!suffix.starts_with(['\r', '\n']) && !suffix.is_empty());
                    let fold_end = start.utf16 + name.encode_utf16().count() + last;
                    let mut fold = NewFolding::new(start_tag.utf16, fold_end);
                    fold.name = Some(element_title(&opening, self.show_attributes_when_folded));
                    folds.push(fold);
                }
                Event::Comment(body) if start.line != position.line => {
                    if let Ok(body) = std::str::from_utf8(body.as_ref()) {
                        let first = body.split(['\r', '\n']).next().unwrap_or("");
                        // XML normalizes CRLF to one character in the comment value.
                        let mut fold = NewFolding::new(
                            start.utf16,
                            position.utf16 - body.matches("\r\n").count(),
                        );
                        fold.name = Some(format!("<!--{first}-->"));
                        folds.push(fold);
                    }
                }
                Event::DocType(_) => {
                    error = Some(0);
                    break;
                }
                Event::Eof => break,
                Event::Text(_) => {
                    if let Some(index) = source.find("]]>") {
                        error = Some(start.byte + index);
                        break;
                    }
                }
                _ => {}
            }
        }
        folds.sort_by_key(|s| s.start_offset);
        (
            folds,
            error.map(|offset| {
                let mut end = offset.min(text.len());
                while !text.is_char_boundary(end) {
                    end -= 1;
                }
                text[..end].encode_utf16().count()
            }),
        )
    }
    pub fn update_foldings(
        &self,
        manager: &FoldingManager,
        document: &TextDocument,
    ) -> Result<(), FoldingError> {
        let snapshot = document.create_snapshot();
        let mut text = snapshot.text();
        if text.contains('\u{fffd}') {
            // Preserve lone-surrogate errors from the UTF-16 document. Each
            // invalid code unit maps to one invalid XML character.
            text = char::decode_utf16(snapshot.to_utf16())
                .map(|scalar| scalar.unwrap_or('\0'))
                .collect();
        }
        let (folds, error) = self.create_new_foldings(&text);
        manager.update_foldings(&folds, error)
    }
}

#[derive(Clone, Copy, Default)]
struct Position {
    byte: usize,
    utf16: usize,
    line: usize,
}

impl Position {
    fn advance(&mut self, source: &str) {
        self.byte += source.len();
        for ch in source.chars() {
            self.utf16 += ch.len_utf16();
            // Count line delimiters so positions on the same line can be compared.
            self.line += usize::from(matches!(ch, '\r' | '\n'));
        }
    }
}

fn element_title(tag: &BytesStart<'_>, show_attributes: bool) -> String {
    let mut title = String::from("<");
    title.push_str(std::str::from_utf8(tag.name().as_ref()).unwrap_or_default());
    if show_attributes {
        for attribute in tag.attributes().flatten() {
            let Ok(name) = std::str::from_utf8(attribute.key.as_ref()) else {
                continue;
            };
            let Ok(value) = attribute.normalized_value(XmlVersion::Explicit1_0) else {
                continue;
            };
            title.push(' ');
            title.push_str(name);
            title.push_str("=\"");
            for ch in value.chars() {
                match ch {
                    '&' => title.push_str("&amp;"),
                    '<' => title.push_str("&lt;"),
                    '>' => title.push_str("&gt;"),
                    '"' => title.push_str("&quot;"),
                    _ => title.push(ch),
                }
            }
            title.push('"');
        }
    }
    title.push('>');
    title
}

fn invalid_attribute_offset(attributes: Attributes<'_>, source: &[u8]) -> Option<usize> {
    for attribute in attributes {
        let attribute = attribute.ok()?;
        let key = attribute.key.as_ref();
        let (invalid, rewind) = if key.starts_with(b":") {
            (key, 0)
        } else if key == b"xml:space" {
            let value = attribute.normalized_value(XmlVersion::Explicit1_0).ok()?;
            if matches!(value.trim_matches([' ', '\t', '\r', '\n']), "default" | "preserve") {
                continue;
            }
            (key, 0)
        } else if key.starts_with(b"xmlns:") && attribute.value.is_empty() {
            (attribute.value.as_ref(), 1)
        } else if key == b"xmlns:xmlns" {
            let value = attribute.normalized_value(XmlVersion::Explicit1_0).ok()?;
            // The tree parser already rejects reserved namespace URIs.
            // Other values still cannot bind the reserved xmlns prefix.
            if matches!(
                value.as_ref(),
                "http://www.w3.org/2000/xmlns/" | "http://www.w3.org/XML/1998/namespace"
            ) {
                continue;
            }
            (key, 0)
        } else {
            continue;
        };
        // quick-xml borrows both slices. Empty namespace values report the
        // opening quote, which precedes the raw value slice.
        return invalid
            .as_ptr()
            .addr()
            .checked_sub(source.as_ptr().addr())
            .and_then(|offset| offset.checked_sub(rewind))
            .filter(|&offset| offset <= source.len());
    }
    None
}

fn incomplete_start_tag_error(source: &str) -> Option<usize> {
    let body = source.strip_prefix('<')?;
    if body.starts_with(':') {
        return Some(1);
    }
    if !body.starts_with(xml_name_start) {
        return None;
    }
    let name_end = body.find(|c| !xml_name_char(c)).unwrap_or(body.len());
    invalid_attribute_offset(Attributes::new(body, name_end), body.as_bytes())
        .map(|offset| offset + 1)
}

fn invalid_instruction_target(target: &[u8]) -> Option<usize> {
    if target.eq_ignore_ascii_case(b"xml") {
        return Some(0);
    }
    let target = std::str::from_utf8(target).ok()?;
    if !target.starts_with(|c| c != ':' && xml_name_start(c)) {
        return Some(0);
    }
    target.find(|c| c == ':' || !xml_name_char(c))
}

fn invalid_declaration_value_offset(declaration: &[u8]) -> Option<usize> {
    let source = std::str::from_utf8(declaration).ok()?;
    for attribute in Attributes::new(source, 3) {
        let attribute = match attribute {
            Ok(attribute) => attribute,
            Err(AttrError::ExpectedQuote(_, quote)) => {
                return declaration
                    .iter()
                    .rposition(|&item| item == quote)
                    .map(|offset| offset + 1);
            }
            Err(_) => return None,
        };
        let value = std::str::from_utf8(attribute.value.as_ref()).ok()?;
        // The attribute cursor borrows each raw value from the declaration.
        let offset = value.as_ptr().addr() - source.as_ptr().addr();
        // A declaration value is read directly, without entity expansion or
        // attribute newline normalization. The document is already decoded.
        if value.chars().any(|ch| {
            ch < ' ' || ch >= '\u{fffe}' || matches!(ch, '<' | '>' | '&' | '\'' | '"')
        }) {
            return Some(offset);
        }
        match attribute.key.as_ref() {
            b"version" if !value.starts_with("1.0") => return Some(offset),
            b"standalone" if !matches!(value, "yes" | "no") => return Some(offset - 1),
            b"version" | b"encoding" | b"standalone" => {}
            _ => return None,
        }
    }
    None
}

fn error_location(text: &str, error: Error) -> (usize, usize) {
    match error {
        Error::NoRootNode | Error::DtdDetected => return (text.len(), 0),
        Error::UnclosedRootNode | Error::UnexpectedEndOfStream => {
            let offset = if text.encode_utf16().take(4).count() < 4 {
                0
            } else {
                text.len()
            };
            return (text.len(), offset);
        }
        _ => {}
    }
    let pos = error.pos();
    // The validator's positions count Unicode scalars and LF-delimited rows.
    let mut rows = text.split_inclusive('\n');
    let row_start: usize = (&mut rows)
        .take(pos.row.saturating_sub(1) as usize)
        .map(str::len)
        .sum();
    let line = rows.next().unwrap_or("");
    let column = pos.col.saturating_sub(1) as usize;
    let byte = row_start
        + line
            .char_indices()
            .nth(column)
            .map_or(line.len(), |(byte, _)| byte);
    let tail = &text[byte..];
    if matches!(error, Error::UnknownToken(..)) && tail.starts_with("<!DOCTYPE") {
        return (byte, 0);
    }
    // At document level the validator can read a CDATA opener as an invalid
    // element name. The editor reports the opening '<'.
    if matches!(error, Error::InvalidName(_))
        && tail.starts_with("![CDATA")
        && text[..byte].ends_with('<')
    {
        return (byte - 1, byte - 1);
    }
    let offset = match error {
        Error::UnexpectedCloseTag(..) | Error::UnexpectedDeclaration(_) => byte + 2,
        Error::UnknownEntityReference(..) if tail.starts_with('&') => byte + 1,
        Error::MalformedEntityReference(_) => byte + malformed_reference_position(tail),
        Error::InvalidName(_) => byte + invalid_qualified_name_offset(tail).unwrap_or(0),
        Error::UnexpectedXmlUri(_) | Error::UnexpectedXmlnsUri(_) => {
            // The validator points at the attribute name. The editor reports
            // the start of its quoted value for misuse of a reserved URI.
            byte + tail.find(['\'', '"']).map_or(0, |quote| quote + 1)
        }
        Error::UnknownToken(_) if tail.starts_with("<![CDATA") => byte,
        Error::UnknownToken(_) if tail.starts_with("</") => byte + 2,
        Error::UnknownToken(_) if tail.starts_with('<') => byte + 1,
        Error::InvalidComment(_) => tail
            .strip_prefix("<!--")
            .and_then(|body| body.find("--"))
            .map_or(byte, |dash| byte + 4 + dash),
        _ => byte,
    }
    .min(text.len());
    (byte.min(offset), offset)
}

fn invalid_qualified_name_offset(source: &str) -> Option<usize> {
    let local_name_start = |ch| ch != ':' && xml_name_start(ch);
    if !source.starts_with(local_name_start) {
        return None;
    }
    let name_delimiter = |ch| ch == ':' || !xml_name_char(ch);
    let colon = source.find(name_delimiter)?;
    let local_name = source[colon..].strip_prefix(':')?;
    let local_start = colon + 1;
    if !local_name.starts_with(local_name_start) {
        return Some(local_start);
    }
    let local_end = local_name.find(name_delimiter)?;
    local_name[local_end..]
        .starts_with(':')
        .then_some(local_start + local_end)
}

fn malformed_reference_position(source: &str) -> usize {
    let Some(reference) = source.strip_prefix('&') else {
        return 0;
    };
    if let Some(numeric) = reference.strip_prefix('#') {
        let (digits, radix, prefix) = match numeric.strip_prefix('x') {
            Some(digits) => (digits, 16, 3),
            None => (numeric, 10, 2),
        };
        let length = digits.chars().take_while(|c| c.is_digit(radix)).count();
        // A terminated reference can still represent an invalid character.
        // Its position is the beginning of the numeric value.
        return prefix
            + if digits.as_bytes().get(length) == Some(&b';') {
                0
            } else {
                length
            };
    }
    let mut chars = reference.char_indices();
    if !chars.next().is_some_and(|(_, c)| xml_name_start(c)) {
        return 1;
    }
    1 + chars
        .find(|&(_, c)| !xml_name_char(c))
        .map_or(reference.len(), |(index, _)| index)
}

fn xml_name_char(c: char) -> bool {
    xml_name_start(c)
        || matches!(c, '0'..='9' | '-' | '.' | '\u{b7}' | '\u{300}'..='\u{36f}' | '\u{203f}'..='\u{2040}')
}

fn xml_name_start(c: char) -> bool {
    // XML 1.0 NameStartChar. Used to locate a missing entity terminator.
    matches!(c,
        ':' | '_' | 'A'..='Z' | 'a'..='z'
        | '\u{c0}'..='\u{d6}' | '\u{d8}'..='\u{f6}' | '\u{f8}'..='\u{2ff}'
        | '\u{370}'..='\u{37d}' | '\u{37f}'..='\u{1fff}' | '\u{200c}'..='\u{200d}'
        | '\u{2070}'..='\u{218f}' | '\u{2c00}'..='\u{2fef}' | '\u{3001}'..='\u{d7ff}'
        | '\u{f900}'..='\u{fdcf}' | '\u{fdf0}'..='\u{fffd}' | '\u{10000}'..='\u{effff}')
}

#[cfg(test)]
#[path = "xml_folding_strategy_tests.rs"]
mod tests;
