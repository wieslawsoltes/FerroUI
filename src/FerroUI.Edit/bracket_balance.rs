use crate::input_languages::InputToken;
use ferroui_textmate::{LanguageConfiguration, StandardTokenType};
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) fn editing_brackets(configuration: &LanguageConfiguration) -> Vec<(&str, &str)> {
    if configuration
        .raw
        .get("brackets")
        .is_none_or(|value| value.is_null())
    {
        vec![("(", ")"), ("{", "}"), ("[", "]")]
    } else {
        configuration.brackets()
    }
}

#[derive(Default)]
struct Delimiter {
    opening: Option<usize>,
    closing: Vec<usize>,
}

struct DelimiterPattern {
    text: Vec<u16>,
    delimiter: Delimiter,
}

/// Configured bracket spellings and the openers each closing spelling accepts.
pub(crate) struct BracketSyntax {
    opener_count: usize,
    languages: HashMap<u8, LanguageBrackets>,
}

struct LanguageBrackets {
    patterns: HashMap<u16, Vec<DelimiterPattern>>,
    block_comment: Option<String>,
}

impl BracketSyntax {
    pub(crate) fn new<'a>(
        configurations: impl IntoIterator<Item = (u8, &'a LanguageConfiguration)>,
    ) -> Option<Self> {
        let mut opener_count = 0;
        let languages = configurations
            .into_iter()
            .filter_map(|(id, configuration)| {
                Some((id, LanguageBrackets::new(configuration, &mut opener_count)?))
            })
            .collect();
        (opener_count != 0).then_some(Self {
            opener_count,
            languages,
        })
    }

    pub(crate) fn opener(&self, language: u8, spelling: &str) -> Option<usize> {
        self.languages.get(&language)?.opener(spelling)
    }

    pub(crate) fn scan_line(
        &self,
        text: &str,
        tokens: &[InputToken],
        stack: BracketStack,
    ) -> BracketLine {
        let mut line = BracketLine {
            stack,
            unmatched: Vec::new(),
        };
        let text = folded(text);
        let mut tokens = tokens.iter().peekable();
        while let Some(token) = tokens.next() {
            if token.kind != StandardTokenType::Other || !token.brackets {
                continue;
            }
            let Some(syntax) = self.languages.get(&token.id) else {
                continue;
            };
            let start = token.start.min(text.len());
            let mut end = token.end;
            while tokens.peek().is_some_and(|next| {
                next.start == end
                    && next.id == token.id
                    && next.kind == StandardTokenType::Other
                    && next.brackets
            }) {
                end = tokens.next().unwrap().end;
            }
            syntax.scan_code(
                &text[start..end.min(text.len())],
                start,
                self.opener_count,
                &mut line,
            );
        }
        line
    }

    pub(crate) fn remove_delimiters(&self, language: u8, text: &str) -> Option<String> {
        let syntax = self.languages.get(&language)?;
        let original: Vec<_> = text.encode_utf16().collect();
        let folded: Vec<_> = original.iter().copied().map(fold_unit).collect();
        let mut index = 0;
        let mut retained = Vec::new();
        let mut last = 0;
        while index < folded.len() {
            if let Some(pattern) = syntax.match_at(&folded, index) {
                retained.extend_from_slice(&original[last..index]);
                index += pattern.text.len();
                last = index;
            } else {
                index += 1;
            }
        }
        if last == 0 {
            return None;
        }
        retained.extend_from_slice(&original[last..]);
        Some(String::from_utf16_lossy(&retained))
    }
}

impl LanguageBrackets {
    fn new(configuration: &LanguageConfiguration, next_id: &mut usize) -> Option<Self> {
        let mut openings = HashMap::new();
        let mut delimiters = HashMap::<_, Delimiter>::new();
        for (open, close) in editing_brackets(configuration) {
            if open.is_empty() || close.is_empty() {
                continue;
            }
            let open = folded(open);
            let close = folded(close);
            if open == close {
                continue;
            }
            let id = *openings.entry(open.clone()).or_insert_with(|| {
                let id = *next_id;
                *next_id += 1;
                id
            });
            delimiters.entry(open).or_default().opening = Some(id);
            let closing = &mut delimiters.entry(close).or_default().closing;
            if !closing.contains(&id) {
                closing.push(id);
            }
        }
        if openings.is_empty() {
            return None;
        }
        let mut patterns: HashMap<u16, Vec<DelimiterPattern>> = HashMap::new();
        for (text, delimiter) in delimiters {
            patterns
                .entry(text[0])
                .or_default()
                .push(DelimiterPattern { text, delimiter });
        }
        for patterns in patterns.values_mut() {
            patterns.sort_unstable_by(|a, b| b.text.len().cmp(&a.text.len()));
        }
        Some(Self {
            patterns,
            block_comment: configuration
                .block_comment()
                .map(|(open, _)| open.to_owned()),
        })
    }

    fn opener(&self, spelling: &str) -> Option<usize> {
        if self
            .block_comment
            .as_ref()
            .is_some_and(|open| !open.is_empty() && spelling.contains(open))
        {
            return None;
        }
        let spelling = folded(spelling);
        let patterns = self.patterns.get(spelling.first()?)?;
        patterns
            .iter()
            .find(|pattern| pattern.text == spelling && pattern.delimiter.closing.is_empty())?
            .delimiter
            .opening
    }

    fn match_at(&self, text: &[u16], index: usize) -> Option<&DelimiterPattern> {
        self.patterns.get(&text[index])?.iter().find(|pattern| {
            let end = index + pattern.text.len();
            text[index..].starts_with(&pattern.text)
                && (!needs_boundary(pattern.text[0]) || word_boundary(text, index))
                && (!needs_boundary(pattern.text[pattern.text.len() - 1])
                    || word_boundary(text, end))
        })
    }

    fn scan_code(
        &self,
        text: &[u16],
        first_column: usize,
        opener_count: usize,
        line: &mut BracketLine,
    ) {
        let mut index = 0;
        while index < text.len() {
            let Some(pattern) = self.match_at(text, index) else {
                index += 1;
                continue;
            };
            let delimiter = &pattern.delimiter;
            if !delimiter.closing.is_empty() {
                if !line.stack.close(&delimiter.closing) {
                    let column = first_column + index;
                    line.unmatched
                        .extend(delimiter.closing.iter().map(|&id| (id, column)));
                }
            } else if let Some(id) = delimiter.opening {
                line.stack.push(id, opener_count);
            }
            index += pattern.text.len();
        }
    }
}

fn folded(text: &str) -> Vec<u16> {
    text.encode_utf16().map(fold_unit).collect()
}

fn fold_unit(unit: u16) -> u16 {
    if unit <= 127 {
        return u16::from((unit as u8).to_ascii_lowercase());
    }
    char::from_u32(u32::from(unit))
        .and_then(|ch| {
            let mut letters = ch.to_lowercase();
            let lower = letters.next()?;
            letters.next().is_none().then_some(lower)
        })
        .and_then(|ch| u16::try_from(ch as u32).ok())
        .unwrap_or(unit)
}

fn ascii_word(unit: u16) -> bool {
    u8::try_from(unit).is_ok_and(|ch| ch.is_ascii_alphanumeric() || ch == b'_')
}

fn needs_boundary(unit: u16) -> bool {
    ascii_word(unit) || unit == u16::from(b' ')
}

fn word_boundary(text: &[u16], index: usize) -> bool {
    let before = index
        .checked_sub(1)
        .is_some_and(|before| ascii_word(text[before]));
    before != text.get(index).copied().is_some_and(ascii_word)
}

#[derive(Clone, Default)]
pub(crate) struct BracketStack(Option<Rc<OpenBracket>>);

struct OpenBracket {
    id: usize,
    fingerprint: u64,
    parent: BracketStack,
    present: Rc<[u64]>,
}

impl BracketStack {
    fn push(&mut self, id: usize, kinds: usize) {
        let (word, bit) = (id / 64, 1_u64 << (id % 64));
        let existing = self.0.as_ref().map(|node| &node.present);
        let present = match existing {
            Some(set) if set[word] & bit != 0 => Rc::clone(set),
            Some(set) => {
                let mut updated = set.to_vec();
                updated[word] |= bit;
                Rc::from(updated)
            }
            None => {
                let mut updated = vec![0; kinds.div_ceil(64)];
                updated[word] = bit;
                Rc::from(updated)
            }
        };
        let fingerprint = self
            .0
            .as_ref()
            .map_or(0, |node| node.fingerprint)
            .wrapping_mul(1_099_511_628_211)
            .wrapping_add(id as u64 + 1);
        let parent = Self(self.0.take());
        self.0 = Some(Rc::new(OpenBracket {
            id,
            fingerprint,
            parent,
            present,
        }));
    }

    fn close(&mut self, candidates: &[usize]) -> bool {
        let Some(top) = self.0.as_ref() else {
            return false;
        };
        if !candidates
            .iter()
            .any(|&id| top.present[id / 64] & (1_u64 << (id % 64)) != 0)
        {
            return false;
        }
        let mut current = top;
        while !candidates.contains(&current.id) {
            current = current
                .parent
                .0
                .as_ref()
                .expect("present opener has a stack entry");
        }
        self.0 = current.parent.0.clone();
        true
    }
}

impl PartialEq for BracketStack {
    fn eq(&self, other: &Self) -> bool {
        let (mut left, mut right) = (self.0.as_ref(), other.0.as_ref());
        loop {
            match (left, right) {
                (None, None) => return true,
                (Some(a), Some(b)) if Rc::ptr_eq(a, b) => return true,
                (Some(a), Some(b)) if a.id == b.id && a.fingerprint == b.fingerprint => {
                    (left, right) = (a.parent.0.as_ref(), b.parent.0.as_ref());
                }
                _ => return false,
            }
        }
    }
}

impl Drop for BracketStack {
    fn drop(&mut self) {
        // A single retained line can own a deep chain of unclosed brackets.
        let mut next = self.0.take();
        while let Some(node) = next {
            match Rc::try_unwrap(node) {
                Ok(mut node) => next = node.parent.0.take(),
                Err(_) => break,
            }
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct BracketLine {
    pub(crate) stack: BracketStack,
    unmatched: Vec<(usize, usize)>,
}

/// The last globally unmatched closer for each opener, in line-relative units.
/// Rebuild only when replacing one of those lines can reveal an earlier closer.
pub(crate) struct UnmatchedClosers(Option<HashMap<usize, (usize, usize)>>);

impl Default for UnmatchedClosers {
    fn default() -> Self {
        Self(Some(HashMap::new()))
    }
}

impl UnmatchedClosers {
    pub(crate) fn splice(&mut self, first: usize, old_end: usize, new_end: usize) {
        let Some(latest) = self.0.as_mut() else {
            return;
        };
        if latest
            .values()
            .any(|&(line, _)| (first..old_end).contains(&line))
        {
            self.0 = None;
            return;
        }
        for (line, _) in latest.values_mut() {
            if *line >= old_end {
                *line = *line - old_end + new_end;
            }
        }
    }

    pub(crate) fn replace_line(
        &mut self,
        index: usize,
        old: Option<&BracketLine>,
        new: &BracketLine,
    ) {
        let Some(latest) = self.0.as_mut() else {
            return;
        };
        if let Some(old) = old {
            if old.unmatched == new.unmatched {
                return;
            }
            if old
                .unmatched
                .iter()
                .any(|(id, _)| latest.get(id).is_some_and(|&(line, _)| line == index))
            {
                self.0 = None;
                return;
            }
        }
        add_closers(latest, index, new);
    }

    pub(crate) fn latest<'a>(
        &mut self,
        id: usize,
        lines: impl Iterator<Item = (usize, &'a BracketLine)>,
    ) -> Option<(usize, usize)> {
        self.0
            .get_or_insert_with(|| {
                let mut rebuilt = HashMap::new();
                for (index, line) in lines {
                    add_closers(&mut rebuilt, index, line);
                }
                rebuilt
            })
            .get(&id)
            .copied()
    }
}

fn add_closers(latest: &mut HashMap<usize, (usize, usize)>, line_index: usize, line: &BracketLine) {
    for &(id, column) in &line.unmatched {
        let position = (line_index, column);
        let previous = latest.entry(id).or_insert(position);
        *previous = (*previous).max(position);
    }
}
