//! Parse one effect call and retain the authored range of each argument value.
use super::normalize_identifier;
use std::ops::Range;

pub(super) struct Argument<'a> {
    pub name: &'a str,
    pub value: String,
    span: Range<usize>,
}

pub(super) struct Invocation<'a> {
    pub name: &'a str,
    pub arguments: Vec<Argument<'a>>,
    insert_at: usize,
}

impl<'a> Invocation<'a> {
    /// A piece without a top-level equals sign continues the previous value,
    /// so both `center=10,20` and `center=(10,20)` are one argument.
    pub fn parse(source: &'a str) -> Option<Self> {
        let open = source.find('(')?;
        let close = source.trim_end().strip_suffix(')')?.len();
        let name = source[..open].trim();
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return None;
        }
        let inner = source.get(open + 1..close)?;
        let content = inner.trim();
        let start = open + 1 + inner.len() - inner.trim_start().len();
        let mut invocation = Self { name, arguments: Vec::new(), insert_at: start + content.len() };
        if content.is_empty() {
            invocation.insert_at = open + 1;
            return Some(invocation);
        }

        for range in split_top_level(content)? {
            let raw = &content[range.clone()];
            let piece = raw.trim();
            if piece.is_empty() {
                return None;
            }
            let piece_start = start + range.start + raw.len() - raw.trim_start().len();
            let piece_end = piece_start + piece.len();
            match find_top_level(piece, '=') {
                Some(equals) => {
                    let name = piece[..equals].trim();
                    let value = piece[equals + 1..].trim();
                    if name.is_empty() || value.is_empty() {
                        return None;
                    }
                    invocation.arguments.push(Argument {
                        name,
                        value: value.to_owned(),
                        span: piece_end - value.len()..piece_end,
                    });
                }
                None => {
                    let argument = invocation.arguments.last_mut()?;
                    argument.value.push(',');
                    argument.value.push_str(piece);
                    argument.span.end = piece_end;
                }
            }
        }
        Some(invocation)
    }

    pub fn replace_argument(&self, source: &str, name: &str, value: &str) -> String {
        let key = normalize_identifier(name);
        let mut updated = source.to_owned();
        // The registry applies repeated arguments in source order.
        if let Some(argument) = self.arguments.iter().rfind(|argument| normalize_identifier(argument.name) == key) {
            updated.replace_range(argument.span.clone(), value);
        } else {
            let separator = if self.arguments.is_empty() { "" } else { ", " };
            updated.insert_str(self.insert_at, &format!("{separator}{name}={value}"));
        }
        updated
    }
}

/// The pieces of `s` between the separators (`,` and `;`) that are outside
/// parentheses, brackets, braces and quotes; `None` when those do not
/// balance.
fn split_top_level(s: &str) -> Option<Vec<Range<usize>>> {
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    let mut quote = None;
    for (index, c) in s.char_indices() {
        if let Some(open) = quote {
            if c == open {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.checked_sub(1)?,
            ',' | ';' if depth == 0 => {
                pieces.push(start..index);
                start = index + c.len_utf8();
            }
            _ => {}
        }
    }
    if depth != 0 || quote.is_some() {
        return None;
    }
    pieces.push(start..s.len());
    Some(pieces)
}

/// The offset of the first `target` outside parentheses, brackets, braces
/// and quotes.
fn find_top_level(s: &str, target: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None;
    for (index, c) in s.char_indices() {
        if let Some(open) = quote {
            if c == open {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ if c == target && depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_with_commas_stay_one_argument() {
        let source = " glow ( color=rgba(1, 2, 3, 0.5), offset=10, 20; label='a, b' ) ";
        let parsed = Invocation::parse(source).unwrap();
        assert_eq!(parsed.name, "glow");
        assert_eq!(
            parsed.arguments.iter().map(|arg| (arg.name, arg.value.as_str())).collect::<Vec<_>>(),
            [("color", "rgba(1, 2, 3, 0.5)"), ("offset", "10,20"), ("label", "'a, b'")]
        );
        assert_eq!(
            parsed.replace_argument(source, "offset", "10,32"),
            " glow ( color=rgba(1, 2, 3, 0.5), offset=10,32; label='a, b' ) "
        );
    }

    #[test]
    fn editing_preserves_the_invocation_and_other_arguments() {
        let source = " Glow ( some_value=2; note='🪴, b'; Some-Value = 3 ) ";
        let parsed = Invocation::parse(source).unwrap();
        assert_eq!(
            parsed.replace_argument(source, "SomeValue", "4"),
            " Glow ( some_value=2; note='🪴, b'; Some-Value = 4 ) "
        );
        assert_eq!(
            parsed.replace_argument(source, "missing", "5"),
            " Glow ( some_value=2; note='🪴, b'; Some-Value = 3, missing=5 ) "
        );
        let empty = "invert(  )";
        assert!(Invocation::parse(empty).unwrap().arguments.is_empty());
        assert_eq!(Invocation::parse(empty).unwrap().replace_argument(empty, "value", "5"), "invert(value=5  )");
    }

    #[test]
    fn malformed_invocations_are_rejected() {
        for source in ["", "tint", "tint(", "(a=1)", "tint(a=1", "tint(a=)", "tint(=1)", "tint(1)", "tint(a=(1)", "ti nt(a=1)", "tint(a=1,)"] {
            assert!(Invocation::parse(source).is_none(), "{source}");
        }
    }
}
