//! A check of the nesting of an XML text before it reaches a recursive parser.

/// Whether the elements of an XML text nest at most `limit` levels deep.
///
/// XML parsers that recurse per element overflow the stack on a deeply nested document, so a
/// reader of outside data calls this first. The scan is linear and allocates nothing. A document
/// type with an internal subset is refused, because the entities it declares can hold markup
/// that the scan does not see. Text that is not well formed is left to the parser to report.
pub fn xml_nesting_is_within(source: &str, limit: usize) -> bool {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut index = 0;
    while let Some(offset) = find(bytes, index, b"<") {
        index = offset;
        let rest = &bytes[index..];
        if rest.starts_with(b"<!--") {
            index = skip_past(bytes, index + 4, b"-->");
        } else if rest.starts_with(b"<![CDATA[") {
            index = skip_past(bytes, index + 9, b"]]>");
        } else if rest.starts_with(b"<?") {
            index = skip_past(bytes, index + 2, b"?>");
        } else if rest.starts_with(b"<!") {
            match declaration_end(bytes, index + 2) {
                Some(end) => index = end,
                None => return false,
            }
        } else if rest.starts_with(b"</") {
            depth = depth.saturating_sub(1);
            index = skip_past(bytes, index + 2, b">");
        } else {
            let end = tag_end(bytes, index + 1);
            if end > 0 && bytes.get(end - 1) != Some(&b'/') {
                depth += 1;
                if depth > limit {
                    return false;
                }
            }
            index = end + 1;
        }
    }
    true
}

fn find(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    bytes
        .get(from..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| from + offset)
}

fn skip_past(bytes: &[u8], from: usize, needle: &[u8]) -> usize {
    find(bytes, from, needle).map_or(bytes.len(), |end| end + needle.len())
}

/// The index of the `>` that closes a tag, outside its quoted attribute values.
fn tag_end(bytes: &[u8], from: usize) -> usize {
    let mut quote = None;
    for (offset, &byte) in bytes[from.min(bytes.len())..].iter().enumerate() {
        match quote {
            Some(open) if byte == open => quote = None,
            Some(_) => {}
            None if byte == b'"' || byte == b'\'' => quote = Some(byte),
            None if byte == b'>' => return from + offset,
            None => {}
        }
    }
    bytes.len()
}

/// The index after a `<!...>` declaration, or `None` when it has an internal subset.
fn declaration_end(bytes: &[u8], from: usize) -> Option<usize> {
    let mut quote = None;
    for (offset, &byte) in bytes[from.min(bytes.len())..].iter().enumerate() {
        match quote {
            Some(open) if byte == open => quote = None,
            Some(_) => {}
            None if byte == b'"' || byte == b'\'' => quote = Some(byte),
            None if byte == b'[' => return None,
            None if byte == b'>' => return Some(from + offset + 1),
            None => {}
        }
    }
    Some(bytes.len())
}

#[cfg(test)]
mod tests {
    use super::xml_nesting_is_within;

    #[test]
    fn counts_open_elements_only() {
        let text = r#"<?xml version="1.0"?><!DOCTYPE plist PUBLIC "a" "b">
            <!-- <x><x><x> --><a b="</a>>"><c/><d><![CDATA[<x><x>]]></d></a>"#;
        assert!(xml_nesting_is_within(text, 2));
        assert!(!xml_nesting_is_within(text, 1));
    }

    #[test]
    fn refuses_deep_nesting_and_an_internal_subset() {
        let deep = format!("{}{}", "<a>".repeat(100_000), "</a>".repeat(100_000));
        assert!(!xml_nesting_is_within(&deep, 128));
        assert!(!xml_nesting_is_within(r#"<!DOCTYPE a [<!ENTITY b "<c/>">]><a>&b;</a>"#, 128));
    }

    #[test]
    fn leaves_text_that_is_not_well_formed_to_the_parser() {
        assert!(xml_nesting_is_within("<a b=\"unterminated", 128));
        assert!(xml_nesting_is_within("</a></b><", 128));
        assert!(xml_nesting_is_within("", 0));
    }
}
