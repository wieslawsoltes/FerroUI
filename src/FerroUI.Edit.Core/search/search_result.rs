use super::SearchError;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

#[derive(Debug)]
pub(crate) struct CaptureLayout {
    pub(crate) slots: Vec<(usize, usize)>,
    pub(crate) names: HashMap<String, usize>,
}

/// A search match keeps its immutable input alive, so capture expansion remains
/// stable even when the editor document subsequently changes.
#[derive(Clone, Debug)]
pub struct SearchResult {
    pub offset: usize,
    pub length: usize,
    pub(crate) input: Rc<str>,
    pub(crate) captures: Vec<Option<Range<usize>>>,
    pub(crate) layout: Rc<CaptureLayout>,
}
impl SearchResult {
    pub fn end_offset(&self) -> usize {
        self.offset + self.length
    }
    pub fn matched_text(&self) -> &str {
        self.capture(0).unwrap_or("")
    }
    pub fn capture(&self, index: usize) -> Option<&str> {
        let slot = self.capture_slot(index)?;
        self.captures[slot]
            .as_ref()
            .map(|range| &self.input[range.clone()])
    }
    pub fn named_capture(&self, name: &str) -> Option<&str> {
        self.capture(*self.layout.names.get(name)?)
    }

    /// Expands capture substitutions. An out-of-range capture number leaves
    /// the template literal. Use `try_replace_with` to receive its diagnostic.
    pub fn replace_with(&self, replacement: &str) -> String {
        self.try_replace_with(replacement)
            .unwrap_or_else(|_| replacement.to_owned())
    }

    /// Expands the reference replacement syntax using source group numbers.
    /// Unknown references retain their literal spelling.
    pub fn try_replace_with(&self, replacement: &str) -> Result<String, SearchError> {
        let mut output = String::with_capacity(replacement.len());
        let mut remaining = replacement;
        while let Some(offset) = remaining.find('$') {
            output.push_str(&remaining[..offset]);
            let suffix = &remaining[offset + 1..];
            if let Some((value, length)) = self.substitution(suffix)? {
                output.push_str(value);
                remaining = &suffix[length..];
            } else {
                output.push('$');
                remaining = suffix;
            }
        }
        output.push_str(remaining);
        Ok(output)
    }

    fn substitution(&self, suffix: &str) -> Result<Option<(&str, usize)>, SearchError> {
        let range = self.captures[0]
            .as_ref()
            .expect("the match is capture zero");
        let value = match suffix.as_bytes().first() {
            Some(b'$') => "$",
            Some(b'&') => self.matched_text(),
            Some(b'`') => &self.input[..range.start],
            Some(b'\'') => &self.input[range.end..],
            Some(b'_') => &self.input,
            Some(b'+') => self
                .captures
                .last()
                .and_then(Option::as_ref)
                .map_or("", |range| &self.input[range.clone()]),
            Some(b'{') => return self.braced_substitution(&suffix[1..]),
            Some(b'0'..=b'9') => {
                let (number, length) = capture_number(suffix)?;
                return Ok(self.group_substitution(number, length));
            }
            _ => return Ok(None),
        };
        Ok(Some((value, 1)))
    }

    fn braced_substitution(&self, contents: &str) -> Result<Option<(&str, usize)>, SearchError> {
        if contents.as_bytes().first().is_some_and(u8::is_ascii_digit) {
            let (number, length) = capture_number(contents)?;
            return Ok(if contents[length..].starts_with('}') {
                self.group_substitution(number, length + 2)
            } else {
                None
            });
        }
        let Some(end) = contents
            .find(['}', '$'])
            .filter(|&end| contents.as_bytes()[end] == b'}')
        else {
            return Ok(None);
        };
        Ok(self
            .layout
            .names
            .get(&contents[..end])
            .and_then(|&number| self.group_substitution(number, end + 2)))
    }

    fn group_substitution(&self, number: usize, length: usize) -> Option<(&str, usize)> {
        let slot = self.capture_slot(number)?;
        let value = self.captures[slot]
            .as_ref()
            .map_or("", |range| &self.input[range.clone()]);
        Some((value, length))
    }

    fn capture_slot(&self, number: usize) -> Option<usize> {
        self.layout
            .slots
            .binary_search_by_key(&number, |&(number, _)| number)
            .ok()
    }
}

fn capture_number(text: &str) -> Result<(usize, usize), SearchError> {
    let end = text.bytes().take_while(u8::is_ascii_digit).count();
    let number = text[..end].parse::<i32>().map_err(|_| {
        SearchError::Replacement("Capture group number exceeds the supported range".into())
    })?;
    Ok((number as usize, end))
}
