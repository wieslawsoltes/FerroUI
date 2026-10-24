use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

/// A search match keeps its immutable input alive, so capture expansion remains
/// stable even when the editor document subsequently changes.
#[derive(Clone, Debug)]
pub struct SearchResult {
    pub offset: usize,
    pub length: usize,
    pub(crate) input: Rc<str>,
    pub(crate) captures: Vec<Option<Range<usize>>>,
    pub(crate) names: Rc<HashMap<String, usize>>,
}
impl SearchResult {
    pub fn end_offset(&self) -> usize {
        self.offset + self.length
    }
    pub fn matched_text(&self) -> &str {
        self.capture(0).unwrap_or("")
    }
    pub fn capture(&self, index: usize) -> Option<&str> {
        self.captures
            .get(index)?
            .as_ref()
            .map(|range| &self.input[range.clone()])
    }
    pub fn named_capture(&self, name: &str) -> Option<&str> {
        self.capture(*self.names.get(name)?)
    }

    /// Expands .NET replacement forms: `$1`, `${name}`, `$$`, `$&`, `$+`,
    /// `$\``, `$'` and `$_`. Unknown groups retain their literal spelling.
    pub fn replace_with(&self, replacement: &str) -> String {
        let mut output = String::new();
        let mut chars = replacement.char_indices().peekable();
        while let Some((start, ch)) = chars.next() {
            if ch != '$' {
                output.push(ch);
                continue;
            }
            let Some(&(_, next)) = chars.peek() else {
                output.push('$');
                break;
            };
            let range = self.captures[0]
                .as_ref()
                .expect("capture zero is always present");
            match next {
                '$' => {
                    chars.next();
                    output.push('$');
                }
                '&' => {
                    chars.next();
                    output.push_str(self.matched_text());
                }
                '`' => {
                    chars.next();
                    output.push_str(&self.input[..range.start]);
                }
                '\'' => {
                    chars.next();
                    output.push_str(&self.input[range.end..]);
                }
                '_' => {
                    chars.next();
                    output.push_str(&self.input);
                }
                '+' => {
                    chars.next();
                    output.push_str(
                        self.captures
                            .iter()
                            .enumerate()
                            .rev()
                            .find_map(|(i, r)| {
                                (i > 0 && r.is_some()).then(|| self.capture(i)).flatten()
                            })
                            .unwrap_or(""),
                    );
                }
                '{' => {
                    chars.next();
                    let mut name = String::new();
                    let mut closed = false;
                    for (_, c) in chars.by_ref() {
                        if c == '}' {
                            closed = true;
                            break;
                        }
                        name.push(c);
                    }
                    if closed {
                        let index = name
                            .parse::<usize>()
                            .ok()
                            .or_else(|| self.names.get(&name).copied());
                        if let Some(index) = index.filter(|i| *i < self.captures.len()) {
                            output.push_str(self.capture(index).unwrap_or(""));
                        } else {
                            output.push_str("${");
                            output.push_str(&name);
                            output.push('}');
                        }
                    } else {
                        output.push_str(&replacement[start..]);
                    }
                }
                c if c.is_ascii_digit() => {
                    let mut digits = String::new();
                    while let Some(&(_, c)) = chars.peek() {
                        if !c.is_ascii_digit() {
                            break;
                        }
                        digits.push(c);
                        chars.next();
                    }
                    if let Some(index) = digits
                        .parse::<usize>()
                        .ok()
                        .filter(|i| *i < self.captures.len())
                    {
                        output.push_str(self.capture(index).unwrap_or(""));
                    } else {
                        output.push('$');
                        output.push_str(&digits);
                    }
                }
                _ => output.push('$'),
            }
        }
        output
    }
}
