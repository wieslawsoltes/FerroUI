use super::SnippetError;
use corvus_regex_ecma262::{Captures, Flags, Regex};
use std::iter::Peekable;
use std::rc::Rc;
use std::str::CharIndices;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub(super) struct TextMateTransform {
    pattern: String,
    regex: Rc<Regex>,
    flags: Flags,
    format: Vec<Format>,
}

impl PartialEq for TextMateTransform {
    fn eq(&self, other: &Self) -> bool {
        self.pattern == other.pattern && self.flags == other.flags && self.format == other.format
    }
}

impl Eq for TextMateTransform {}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Format {
    Text(String),
    Capture(usize, CaptureFormat),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CaptureFormat {
    Value,
    IfPresent(String),
    Default(String),
    Conditional(String, String),
    Case(Case),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    Upper,
    Lower,
    Capitalize,
    Camel,
    Pascal,
    Snake,
    Kebab,
}

impl TextMateTransform {
    pub(super) fn parse(
        input: &mut Peekable<CharIndices<'_>>,
        offset: usize,
    ) -> Result<Self, SnippetError> {
        let pattern = read_until(input, '/', &['/']).ok_or(SnippetError::InvalidSyntax(offset))?;
        let format = parse_format(input, offset)?;
        let options = read_until(input, '}', &[]).ok_or(SnippetError::InvalidSyntax(offset))?;
        let invalid_regex = |error: corvus_regex_ecma262::Error| {
            SnippetError::Extension(format!(
                "Invalid snippet transform at UTF-8 byte {offset}: {error}"
            ))
        };
        let flags = Flags::parse(&options).map_err(invalid_regex)?;
        // The native and linked browser hosts have monotonic clocks. The bare
        // wasm32 standard library has no clock service.
        let timeout = (!cfg!(all(target_arch = "wasm32", target_os = "unknown")))
            .then_some(Duration::from_millis(50));
        let regex = Regex::with_timeout(&pattern, flags, timeout).map_err(invalid_regex)?;
        Ok(Self {
            pattern,
            regex: Rc::new(regex),
            flags,
            format,
        })
    }

    pub(super) fn apply(&self, value: &str) -> Result<String, SnippetError> {
        let clock = self
            .regex
            .match_timeout()
            .map(|limit| (Instant::now(), limit));
        let input: Vec<_> = value.encode_utf16().collect();
        let mut output = Vec::new();
        let mut end = 0;
        let mut matched = false;
        for captures in self.regex.captures_iter_utf16(&input) {
            let captures = captures.map_err(|error| SnippetError::Extension(error.to_string()))?;
            if clock.is_some_and(|(start, limit)| start.elapsed() >= limit) {
                return Err(SnippetError::InvalidExpansion(
                    "snippet transform exceeded its time budget",
                ));
            }
            let range = captures.get(0).expect("capture 0 is the match").range();
            output.extend_from_slice(&input[end..range.start]);
            self.render(Some(&captures), &mut output);
            matched = true;
            end = range.end;
            if !self.flags.global {
                break;
            }
        }
        if !matched && self.format.iter().any(Format::has_default) {
            self.render(None, &mut output);
        } else {
            output.extend_from_slice(&input[end..]);
        }
        Ok(String::from_utf16_lossy(&output))
    }

    fn render(&self, captures: Option<&Captures<'_, [u16]>>, output: &mut Vec<u16>) {
        for part in &self.format {
            match part {
                Format::Text(text) => output.extend(text.encode_utf16()),
                Format::Capture(index, format) => {
                    let captured = captures
                        .and_then(|groups| groups.get(*index))
                        .map_or(&[][..], |group| group.as_slice());
                    format.render(captured, output);
                }
            }
        }
    }
}

impl Format {
    fn has_default(&self) -> bool {
        matches!(self, Self::Capture(_, CaptureFormat::Default(text) | CaptureFormat::Conditional(_, text)) if !text.is_empty())
    }
}

impl CaptureFormat {
    fn render(&self, value: &[u16], output: &mut Vec<u16>) {
        let present = !value.is_empty();
        match self {
            Self::Value => output.extend_from_slice(value),
            Self::IfPresent(text) if present => output.extend(text.encode_utf16()),
            Self::IfPresent(_) => {}
            Self::Default(_) if present => output.extend_from_slice(value),
            Self::Default(text) => output.extend(text.encode_utf16()),
            Self::Conditional(yes, no) => {
                output.extend(if present { yes } else { no }.encode_utf16())
            }
            Self::Case(case) => {
                output.extend(case.apply(&String::from_utf16_lossy(value)).encode_utf16())
            }
        }
    }
}

fn read_until(
    input: &mut Peekable<CharIndices<'_>>,
    delimiter: char,
    escaped: &[char],
) -> Option<String> {
    let mut value = String::new();
    while let Some((_, ch)) = input.next() {
        if ch == delimiter {
            return Some(value);
        }
        if ch == '\\'
            && input
                .peek()
                .is_some_and(|&(_, next)| escaped.contains(&next))
        {
            value.push(input.next().unwrap().1);
        } else {
            value.push(ch);
        }
    }
    None
}

fn parse_format(
    input: &mut Peekable<CharIndices<'_>>,
    offset: usize,
) -> Result<Vec<Format>, SnippetError> {
    let mut parts = Vec::new();
    let mut literal = String::new();
    while let Some((at, ch)) = input.next() {
        match ch {
            '/' => {
                if !literal.is_empty() {
                    parts.push(Format::Text(literal));
                }
                return Ok(parts);
            }
            '\\' if input
                .peek()
                .is_some_and(|&(_, next)| matches!(next, '\\' | '/')) =>
            {
                literal.push(input.next().unwrap().1);
            }
            '$' => {
                if let Some(capture) = parse_capture(input, at)? {
                    if !literal.is_empty() {
                        parts.push(Format::Text(std::mem::take(&mut literal)));
                    }
                    parts.push(capture);
                } else {
                    literal.push(ch);
                }
            }
            _ => literal.push(ch),
        }
    }
    Err(SnippetError::InvalidSyntax(offset))
}

fn parse_capture(
    input: &mut Peekable<CharIndices<'_>>,
    offset: usize,
) -> Result<Option<Format>, SnippetError> {
    let mut cursor = input.clone();
    let braced = cursor.next_if(|&(_, ch)| ch == '{').is_some();
    let mut digits = String::new();
    while let Some((_, ch)) = cursor.next_if(|&(_, ch)| ch.is_ascii_digit()) {
        digits.push(ch);
    }
    if digits.is_empty() {
        return Ok(None);
    }
    let index = digits
        .parse()
        .map_err(|_| SnippetError::InvalidSyntax(offset))?;
    let format = if braced {
        match cursor.next() {
            Some((_, '}')) => CaptureFormat::Value,
            Some((_, ':')) => {
                parse_capture_format(&mut cursor).ok_or(SnippetError::InvalidSyntax(offset))?
            }
            _ => return Err(SnippetError::InvalidSyntax(offset)),
        }
    } else {
        CaptureFormat::Value
    };
    *input = cursor;
    Ok(Some(Format::Capture(index, format)))
}

fn parse_capture_format(input: &mut Peekable<CharIndices<'_>>) -> Option<CaptureFormat> {
    let end = |input: &mut Peekable<CharIndices<'_>>| read_until(input, '}', &['}', '\\']);
    match input.peek().map(|&(_, ch)| ch) {
        Some('/') => {
            input.next();
            let operation = read_until(input, '}', &[])?;
            let case = match operation.as_str() {
                "upcase" => Case::Upper,
                "downcase" => Case::Lower,
                "capitalize" => Case::Capitalize,
                "camelcase" => Case::Camel,
                "pascalcase" => Case::Pascal,
                "snakecase" => Case::Snake,
                "kebabcase" => Case::Kebab,
                _ => return Some(CaptureFormat::Value),
            };
            Some(CaptureFormat::Case(case))
        }
        Some('+') => {
            input.next();
            end(input).map(CaptureFormat::IfPresent)
        }
        Some('-') => {
            input.next();
            end(input).map(CaptureFormat::Default)
        }
        Some('?') => {
            input.next();
            let yes = read_until(input, ':', &[':', '\\', '}'])?;
            Some(CaptureFormat::Conditional(yes, end(input)?))
        }
        _ => end(input).map(CaptureFormat::Default),
    }
}

impl Case {
    fn apply(self, value: &str) -> String {
        static WORDS: OnceLock<regex::Regex> = OnceLock::new();
        static SNAKE_HUMPS: OnceLock<regex::Regex> = OnceLock::new();
        static SNAKE_SEPARATORS: OnceLock<regex::Regex> = OnceLock::new();
        let words = || {
            WORDS
                .get_or_init(|| regex::Regex::new(r"[\p{L}0-9]+").unwrap())
                .find_iter(value)
        };
        match self {
            Self::Upper => value.to_uppercase(),
            Self::Lower => value.to_lowercase(),
            Self::Capitalize => change_initial(value, true),
            Self::Camel | Self::Pascal => {
                let mut result = String::new();
                for (i, word) in words().enumerate() {
                    result.push_str(&change_initial(
                        word.as_str(),
                        matches!(self, Self::Pascal) || i != 0,
                    ));
                }
                if result.is_empty() {
                    value.to_owned()
                } else {
                    result
                }
            }
            Self::Snake => {
                let humps =
                    SNAKE_HUMPS.get_or_init(|| regex::Regex::new(r"(\p{Ll})(\p{Lu})").unwrap());
                let separators =
                    SNAKE_SEPARATORS.get_or_init(|| regex::Regex::new(r"[\s-]+").unwrap());
                separators
                    .replace_all(&humps.replace_all(value, "${1}_${2}"), "_")
                    .to_lowercase()
            }
            Self::Kebab => {
                if words().next().is_none() {
                    return value.to_owned();
                }
                kebab_case(value)
            }
        }
    }
}

fn change_initial(value: &str, upper: bool) -> String {
    let Some(initial) = value.chars().next().filter(|ch| ch.len_utf16() == 1) else {
        return value.to_owned();
    };
    let mut output = String::with_capacity(value.len());
    if upper {
        output.extend(initial.to_uppercase());
    } else {
        output.extend(initial.to_lowercase());
    }
    output.push_str(&value[initial.len_utf8()..]);
    output
}

fn kebab_case(value: &str) -> String {
    // Keep acronyms together and start the following mixed-case word at its
    // capital. The word grammar follows the snippet format, including digits.
    static WORDS: OnceLock<fancy_regex::Regex> = OnceLock::new();
    let words = WORDS.get_or_init(|| fancy_regex::Regex::new(
        r"\p{Lu}{2,}(?=\p{Lu}\p{Ll}|[\s_-]|$)|\p{Lu}?\p{Ll}+[0-9]*|\p{Lu}(?=\p{Lu}\p{Ll}|[\s_-]|$)|[0-9]+"
    ).unwrap());
    let trimmed = value.trim().trim_matches('_');
    let tokens: Vec<_> = words
        .find_iter(trimmed)
        .filter_map(Result::ok)
        .map(|word| word.as_str().to_lowercase())
        .collect();
    if tokens.is_empty() {
        trimmed
            .split(|ch: char| ch.is_whitespace() || matches!(ch, '-' | '_'))
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>()
            .join("-")
            .to_lowercase()
    } else {
        tokens.join("-")
    }
}
