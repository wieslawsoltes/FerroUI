//! Lowers shared editor search and XSHD syntax to
//! the Rust regex backend. Matching and capture storage belong to the backend.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;
use std::{fmt::Write as _, ops::Range};

mod unicode_blocks;

#[derive(Clone, Copy)]
struct Options {
    explicit_capture: bool,
    ignore_whitespace: bool,
    multiline: bool,
    crlf: bool,
}

struct Scope {
    options: Options,
    extra_closes: usize,
}

enum Target {
    Number(usize),
    Name(String),
}

struct Reference {
    range: Range<usize>,
    target: Target,
    kind: ReferenceKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReferenceKind {
    Backreference,
    NumericEscape,
    Condition,
}

enum Capture {
    Unnamed,
    Numbered(usize),
    Named(String),
}

pub(crate) struct NormalizedPattern {
    pub(crate) expression: String,
    /// Pairs of source group number and backend group index, ordered by source number.
    pub(crate) capture_slots: Vec<(usize, usize)>,
    /// Word-boundary assertions benefit from the backend's candidate scan.
    pub(crate) has_unicode_word_boundaries: bool,
    has_final_line_anchor: bool,
    pub(crate) has_contiguous_anchor: bool,
}

impl NormalizedPattern {
    pub(crate) fn seek_candidates(&self) -> bool {
        self.has_unicode_word_boundaries || self.has_final_line_anchor
    }
}

/// Preserve the original syntax tree pattern; only the executable expression
/// is normalized. Backreferences are remapped because .NET numbers enabled
/// unnamed captures before named captures, while the backend uses source order.
pub(crate) fn normalize(
    pattern: &str,
    explicit_capture: bool,
    ignore_whitespace: bool,
) -> Result<NormalizedPattern, String> {
    normalize_pattern(
        pattern,
        Options {
            explicit_capture,
            ignore_whitespace,
            multiline: false,
            crlf: false,
        },
        true,
    )
}

pub(crate) fn normalize_search(
    pattern: &str,
    lower_word_boundaries: bool,
) -> Result<NormalizedPattern, String> {
    normalize_pattern(
        pattern,
        Options {
            explicit_capture: false,
            ignore_whitespace: false,
            multiline: true,
            crlf: false,
        },
        lower_word_boundaries,
    )
}

fn normalize_pattern(
    pattern: &str,
    mut options: Options,
    lower_word_boundaries: bool,
) -> Result<NormalizedPattern, String> {
    let mut output = String::with_capacity(pattern.len());
    let mut scopes: Vec<Scope> = Vec::new();
    let mut captures = Vec::new();
    let mut capture_names = HashSet::new();
    let mut references: Vec<Reference> = Vec::new();
    let mut has_unicode_word_boundaries = false;
    let mut has_final_line_anchor = false;
    let mut has_contiguous_anchor = false;
    let mut index = 0;
    while index < pattern.len() {
        let rest = &pattern[index..];
        match rest.as_bytes()[0] {
            b'\\' => {
                match pattern.as_bytes().get(index + 1) {
                    Some(b'1'..=b'9') => {
                        let digits = pattern[index + 1..]
                            .bytes()
                            .take_while(u8::is_ascii_digit)
                            .count();
                        let end = index + 1 + digits;
                        reference(
                            &mut output,
                            &mut references,
                            &pattern[index + 1..end],
                            ReferenceKind::NumericEscape,
                        )?;
                        index = end;
                        continue;
                    }
                    Some(b'k') => {
                        if let Some((name, after)) = delimited_name(pattern, index + 2) {
                            reference(
                                &mut output,
                                &mut references,
                                name,
                                ReferenceKind::Backreference,
                            )?;
                            index = after;
                            continue;
                        }
                    }
                    Some(b'Z') if !options.crlf => {
                        output.push_str(FINAL_LINE_ANCHOR);
                        has_final_line_anchor = true;
                        index += 2;
                        continue;
                    }
                    Some(b'x' | b'u' | b'U') => {
                        if let Some((character, trailing)) =
                            surrogate_escape_pair(rest, options.ignore_whitespace)?
                        {
                            // The reference compares the two escaped units without case folding.
                            write!(output, "(?-i:{character})")
                                .expect("writing to a string cannot fail");
                            index = pattern.len() - trailing.len();
                            continue;
                        }
                        if let Some(length) =
                            braced_hex_escape(&rest[2..], options.ignore_whitespace, &mut output)
                        {
                            index += 2 + length;
                            continue;
                        }
                    }
                    Some(b'G') => has_contiguous_anchor = true,
                    Some(&kind @ (b'b' | b'B'))
                        if !has_boundary_specifier(&rest[2..], options.ignore_whitespace) =>
                    {
                        has_unicode_word_boundaries = true;
                        if lower_word_boundaries {
                            write_word_boundary(&mut output, kind == b'b');
                            index += 2;
                            continue;
                        }
                    }
                    _ => {}
                }
                index = character_escape(pattern, index, false, &mut output)?;
            }
            b'[' => {
                index = character_class(pattern, index, &mut output, 0)?;
            }
            b'{' if needs_literal_brace(&rest[1..], options.ignore_whitespace) => {
                output.push_str(r"\{");
                index += 1;
            }
            b'$' if !options.multiline && !options.crlf => {
                output.push_str(FINAL_LINE_ANCHOR);
                has_final_line_anchor = true;
                index += 1;
            }
            b'#' if options.ignore_whitespace => {
                let end = rest.find('\n').map_or(pattern.len(), |end| index + end);
                output.push_str(&pattern[index..end]);
                index = end;
            }
            b'\x0c' if options.ignore_whitespace => {
                // The target's extended mode skips spaces but keeps form feeds.
                output.push(' ');
                index += 1;
            }
            b'(' if rest.starts_with("(?#") => {
                let end = rest.find(')').ok_or("Unclosed regex comment")? + index + 1;
                // The source comment stops at the first ')', including after '\'.
                output.push_str("(?#)");
                index = end;
            }
            b'(' if rest.starts_with("(?(") => {
                // Capture-existence conditions have a delimiter parenthesis,
                // which must not turn into an unnamed capturing group.
                output.push_str("(?:(?(");
                scopes.push(Scope {
                    options,
                    extra_closes: 1,
                });
                let start = index + 3;
                let condition = &pattern[start..];
                let target = condition.split_once(')').map(|(target, _)| target);
                // An ASCII digit introduces a numeric condition, even if
                // the rest is malformed. Only names can fall back to text.
                if condition.as_bytes().first().is_some_and(u8::is_ascii_digit)
                    && target.is_none_or(|target| !target.bytes().all(|ch| ch.is_ascii_digit()))
                {
                    return Err("Malformed numeric capture condition".into());
                }
                if let Some(target) = target.filter(|target| is_capture_word(target)) {
                    reference(
                        &mut output,
                        &mut references,
                        target,
                        ReferenceKind::Condition,
                    )?;
                    output.push(')');
                    index = start + target.len() + 1;
                    continue;
                }
                // In .NET an assertion condition omits the extra parentheses
                // required by the backend around the conditional expression.
                if pattern[start..].starts_with("?=")
                    || pattern[start..].starts_with("?!")
                    || pattern[start..].starts_with("?<=")
                    || pattern[start..].starts_with("?<!")
                {
                    let prefix_len = if pattern[start..].starts_with("?<") {
                        3
                    } else {
                        2
                    };
                    output.push('(');
                    output.push_str(&pattern[start..start + prefix_len]);
                    output.push_str("(?:");
                    scopes.push(Scope {
                        options,
                        extra_closes: 2,
                    });
                    index = start + prefix_len;
                } else {
                    // An expression condition is a zero-width assertion in
                    // .NET, even when its syntax has no explicit `?=` prefix.
                    output.push_str("(?=(?:");
                    scopes.push(Scope {
                        options,
                        extra_closes: 2,
                    });
                    index = start;
                }
            }
            b'(' => {
                if let Some((end, scoped, new_options, flags)) =
                    inline_options(pattern, index, options)
                {
                    if scoped {
                        output.push_str("(?");
                        output.push_str(&flags);
                        output.push(':');
                        scopes.push(Scope {
                            options,
                            extra_closes: 0,
                        });
                    } else if flags.is_empty() {
                        // An option directive is an empty expression. Keeping
                        // it avoids attaching a following quantifier to the
                        // preceding token when the sole option was `n`.
                        output.push_str("(?:)");
                    } else {
                        output.push_str("(?");
                        output.push_str(&flags);
                        output.push(')');
                    }
                    options = new_options;
                    index = end;
                    continue;
                }
                let name_start = if rest.starts_with("(?<")
                    && !rest.starts_with("(?<=")
                    && !rest.starts_with("(?<!")
                {
                    Some(index + 2)
                } else if rest.starts_with("(?'") {
                    Some(index + 2)
                } else if rest.starts_with("(?P<") {
                    Some(index + 3)
                } else {
                    None
                };
                if let Some((name, end)) =
                    name_start.and_then(|start| delimited_name(pattern, start))
                {
                    if name.contains('-') {
                        return Err("Balancing groups are not supported".into());
                    }
                    if name.chars().all(|ch| ch.is_ascii_digit()) {
                        let number = name
                            .parse::<i32>()
                            .ok()
                            .filter(|&number| number > 0)
                            .ok_or("Invalid capture number")?;
                        captures.push(Capture::Numbered(number as usize));
                        output.push_str("(?:(");
                    } else if !capture_names.insert(name) {
                        return Err(format!("Duplicate capture name '{name}' is not supported"));
                    } else {
                        captures.push(Capture::Named(name.to_owned()));
                        output.push_str("(?:(?<");
                        output.push_str(name);
                        output.push('>');
                    }
                    // The backend restores inline flags for scoped option
                    // groups; the wrapper also gives named groups that scope.
                    scopes.push(Scope {
                        options,
                        extra_closes: 1,
                    });
                    index = end;
                } else if !rest.starts_with("(?") && !rest.starts_with("(*") {
                    if skip_regex_trivia(&rest[1..], options.ignore_whitespace).starts_with('?') {
                        return Err("Regex quantifier has no preceding expression".into());
                    }
                    if options.explicit_capture {
                        output.push_str("(?:");
                        scopes.push(Scope {
                            options,
                            extra_closes: 0,
                        });
                    } else {
                        captures.push(Capture::Unnamed);
                        output.push_str("(?:(");
                        scopes.push(Scope {
                            options,
                            extra_closes: 1,
                        });
                    }
                    index += 1;
                } else {
                    // Lookarounds and atomic groups keep their own syntax and
                    // gain a scope for any nested unscoped option directives.
                    output.push_str("(?:(");
                    scopes.push(Scope {
                        options,
                        extra_closes: 1,
                    });
                    index += 1;
                }
            }
            b')' => {
                output.push(')');
                if let Some(scope) = scopes.pop() {
                    options = scope.options;
                    for _ in 0..scope.extra_closes {
                        output.push(')');
                    }
                }
                index += 1;
            }
            _ => {
                let ch = rest.chars().next().unwrap();
                output.push(ch);
                index += ch.len_utf8();
            }
        }
    }
    if !scopes.is_empty() {
        return Err("Unclosed regex group".into());
    }
    let mut numbers = BTreeMap::from([(0, 0)]);
    let mut names = HashMap::new();
    let mut next_number = 1usize;
    for (index, capture) in captures.iter().enumerate() {
        let number = match capture {
            Capture::Unnamed => {
                let number = next_number;
                next_number += 1;
                number
            }
            Capture::Numbered(number) => *number,
            Capture::Named(_) => continue,
        };
        if numbers.insert(number, index + 1).is_some() {
            return Err(format!(
                "Duplicate capture number {number} is not supported"
            ));
        }
    }
    for (index, capture) in captures.iter().enumerate() {
        if let Capture::Named(name) = capture {
            while numbers.contains_key(&next_number) {
                next_number += 1;
            }
            numbers.insert(next_number, index + 1);
            names.insert(name.as_str(), index + 1);
            next_number += 1;
        }
    }
    let mut result = String::with_capacity(output.len());
    let mut start = 0;
    for reference in references {
        result.push_str(&output[start..reference.range.start]);
        start = reference.range.end;
        if let Target::Name(name) = &reference.target {
            if reference.kind == ReferenceKind::Condition && !names.contains_key(name.as_str()) {
                // .NET treats an unknown name in a condition as an expression
                // test, while an unknown numeric capture remains an error.
                result.push_str(&format!("(?={name})"));
                continue;
            }
        }
        let number = match reference.target {
            Target::Number(number) => match numbers.get(&number) {
                Some(&slot) => slot,
                None if reference.kind == ReferenceKind::NumericEscape && number >= 10 => {
                    let digits = &output[reference.range];
                    if let Some((octal, length)) = octal_escape(digits) {
                        write_escape(&mut result, octal);
                        result.push_str(&digits[length..]);
                        continue;
                    }
                    return Err(format!("Invalid octal escape '\\{digits}'"));
                }
                None => return Err(format!("Reference to undefined capture {number}")),
            },
            Target::Name(ref name) => names
                .get(name.as_str())
                .copied()
                .ok_or_else(|| format!("Reference to undefined capture '{name}'"))?,
        };
        if reference.kind == ReferenceKind::Condition {
            result.push_str(&number.to_string());
        } else {
            result.push_str(&format!("\\k<{number}>"));
        }
    }
    result.push_str(&output[start..]);
    Ok(NormalizedPattern {
        expression: result,
        capture_slots: numbers.into_iter().collect(),
        has_unicode_word_boundaries,
        has_final_line_anchor,
        has_contiguous_anchor,
    })
}

fn reference(
    output: &mut String,
    references: &mut Vec<Reference>,
    target: &str,
    kind: ReferenceKind,
) -> Result<(), String> {
    let start = output.len();
    output.push_str(target);
    references.push(Reference {
        range: start..output.len(),
        target: if target.chars().all(|ch| ch.is_ascii_digit()) {
            Target::Number(
                target
                    .parse::<i32>()
                    .map_err(|_| "Invalid capture number")? as usize,
            )
        } else {
            Target::Name(target.to_owned())
        },
        kind,
    });
    Ok(())
}

fn delimited_name(pattern: &str, start: usize) -> Option<(&str, usize)> {
    let delimiter = match pattern.as_bytes().get(start)? {
        b'<' => '>',
        b'\'' => '\'',
        _ => return None,
    };
    let name_start = start + 1;
    let end = pattern[name_start..].find(delimiter)? + name_start;
    Some((&pattern[name_start..end], end + 1))
}

fn octal_escape(digits: &str) -> Option<(u8, usize)> {
    let mut value = 0u16;
    let mut length = 0;
    for byte in digits
        .bytes()
        .take(3)
        .take_while(|byte| (b'0'..=b'7').contains(byte))
    {
        value = value * 8 + u16::from(byte - b'0');
        length += 1;
    }
    (length != 0).then_some((value as u8, length))
}

fn write_escape(output: &mut String, value: u8) {
    write!(output, "\\x{value:02x}").expect("writing to a string cannot fail");
}

const BMP: &str = r"[\x00-\x{FFFF}]";
const WORD_CATEGORIES: &str = r"\p{L}\p{Mn}\p{Nd}\p{Pc}";
// Let the native candidate scan seek to a line ending. The lookahead then
// checks for at most one LF before strict end of input.
const FINAL_LINE_ANCHOR: &str = r"(?:(?m:$)(?=\n?\z))";

fn boundary_word_class() -> String {
    // Join controls are boundary word characters and do not belong to \w.
    format!(r"[{WORD_CATEGORIES}\x{{200C}}\x{{200D}}&&{BMP}]")
}

pub(crate) fn word_boundary_bytes(text: &str) -> impl Iterator<Item = usize> + '_ {
    boundary_word_runs()
        .find_iter(text)
        .flat_map(|run| [run.start(), run.end()])
}

fn boundary_word_runs() -> &'static regex::Regex {
    static WORD_RUN: OnceLock<regex::Regex> = OnceLock::new();
    WORD_RUN.get_or_init(|| {
        regex::Regex::new(&format!("{}+", boundary_word_class()))
            .expect("boundary word class is a valid regex")
    })
}

fn is_capture_word(name: &str) -> bool {
    // Preserve backend identifiers, including supplementary letters.
    !name.is_empty()
        && (name.chars().all(|ch| ch.is_alphanumeric() || ch == '_')
            || !name.is_ascii()
                && boundary_word_runs()
                    .find(name)
                    .is_some_and(|word| word.range() == (0..name.len())))
}

pub(crate) fn native_word_boundaries_agree(text: &str) -> bool {
    if text.is_ascii() {
        return true;
    }
    // The native word set contains every reference boundary word character.
    // A scalar in the difference can change a boundary on either side of it.
    static DIFFERENCE: OnceLock<regex::Regex> = OnceLock::new();
    let difference = DIFFERENCE.get_or_init(|| {
        regex::Regex::new(&format!(r"[\w--{}]", boundary_word_class()))
            .expect("word category difference is a valid regex")
    });
    !difference.is_match(text)
}

fn has_boundary_specifier(rest: &str, ignore_whitespace: bool) -> bool {
    // The backend accepts boundary extensions such as \b{start}. Leave their
    // parsing and diagnostics to it. Numeric braces quantify the assertion.
    let rest = skip_regex_trivia(rest, ignore_whitespace);
    rest.strip_prefix('{').is_some_and(|after_brace| {
        let content = skip_regex_trivia(after_brace, ignore_whitespace);
        !matches!(content.as_bytes().first(), Some(b'0'..=b'9' | b','))
    })
}

fn skip_regex_trivia(mut rest: &str, ignore_whitespace: bool) -> &str {
    loop {
        if ignore_whitespace {
            rest = rest.trim_start_matches([' ', '\r', '\n', '\t', '\x0c']);
            if rest.starts_with('#') {
                rest = rest.split_once('\n').map_or("", |(_, tail)| tail);
                continue;
            }
        }
        match rest
            .strip_prefix("(?#")
            .and_then(|comment| comment.split_once(')'))
        {
            Some((_, tail)) => rest = tail,
            None => return rest,
        }
    }
}

fn needs_literal_brace(contents: &str, ignore_whitespace: bool) -> bool {
    let token = skip_regex_trivia(contents, ignore_whitespace);
    if !matches!(token.as_bytes().first(), Some(b'0'..=b'9' | b',')) {
        return false;
    }
    // The backend can skip trivia inside a repetition count. A source count
    // needs adjacent digits and separators, and its minimum is required.
    let after_minimum = contents.trim_start_matches(|ch: char| ch.is_ascii_digit());
    let compact_count = after_minimum.len() < contents.len()
        && (after_minimum.starts_with('}')
            || after_minimum.strip_prefix(',').is_some_and(|maximum| {
                maximum
                    .trim_start_matches(|ch: char| ch.is_ascii_digit())
                    .starts_with('}')
            }));
    !compact_count
}

fn surrogate_escape_pair(
    pattern: &str,
    ignore_whitespace: bool,
) -> Result<Option<(char, &str)>, String> {
    let Some((high, after_high)) =
        escaped_utf16_unit(pattern).filter(|(unit, _)| (0xD800..=0xDBFF).contains(unit))
    else {
        return Ok(None);
    };
    let after_trivia = skip_regex_trivia(after_high, ignore_whitespace);
    let Some((low, trailing)) =
        escaped_utf16_unit(after_trivia).filter(|(unit, _)| (0xDC00..=0xDFFF).contains(unit))
    else {
        return Ok(None);
    };
    let next = skip_regex_trivia(trailing, ignore_whitespace);
    let repeated = next.starts_with(['+', '*', '?'])
        || next.strip_prefix('{').is_some_and(|count| {
            count.as_bytes().first().is_some_and(u8::is_ascii_digit)
                && !needs_literal_brace(count, false)
        });
    if repeated {
        return Err("A quantifier after a UTF-16 escape pair targets its low surrogate. Group the pair to quantify the complete character.".into());
    }
    let character = char::decode_utf16([high, low])
        .next()
        .and_then(Result::ok)
        .expect("valid surrogate pair");
    Ok(Some((character, trailing)))
}

fn escaped_utf16_unit(pattern: &str) -> Option<(u16, &str)> {
    let source = pattern.strip_prefix(r"\u")?;
    let hex = source.get(..4)?;
    if !hex.bytes().all(|digit| digit.is_ascii_hexdigit()) {
        return None;
    }
    Some((u16::from_str_radix(hex, 16).ok()?, &source[4..]))
}

fn braced_hex_escape(contents: &str, ignore_whitespace: bool, output: &mut String) -> Option<usize> {
    // The backend also accepts braced hex escapes. Read the whole escape so
    // its braces aren't processed as a repetition count.
    let mut remaining = skip_regex_trivia(contents, ignore_whitespace).strip_prefix('{')?;
    let mut digits = String::new();
    while digits.len() < 8 {
        remaining = skip_regex_trivia(remaining, ignore_whitespace);
        let Some(digit) = remaining.chars().next().filter(char::is_ascii_hexdigit) else {
            break;
        };
        digits.push(digit);
        remaining = &remaining[1..];
    }
    let remaining = skip_regex_trivia(remaining, ignore_whitespace).strip_prefix('}')?;
    if digits.is_empty() {
        return None;
    }
    write!(output, r"\x{{{digits}}}").expect("writing to a string cannot fail");
    Some(contents.len() - remaining.len())
}

fn write_word_boundary(output: &mut String, boundary: bool) {
    // Each side of the assertion tests one scalar.
    let word = boundary_word_class();
    let (after_word, after_nonword) = if boundary { ('!', '=') } else { ('=', '!') };
    write!(
        output,
        r"(?:(?<={word})(?{after_word}{word})|(?<!{word})(?{after_nonword}{word}))"
    )
    .expect("writing to a string cannot fail");
}

fn character_escape(
    pattern: &str,
    start: usize,
    in_class: bool,
    output: &mut String,
) -> Result<usize, String> {
    let rest = &pattern[start + 1..];
    match rest.as_bytes().first() {
        Some(b'0'..=b'7') if in_class || rest.starts_with('0') => {
            let (value, length) = octal_escape(rest).expect("first digit is octal");
            write_escape(output, value);
            Ok(start + 1 + length)
        }
        Some(b'8' | b'9') if in_class => Err("Invalid octal escape in character class".into()),
        Some(b'b') if in_class => {
            write_escape(output, 8);
            Ok(start + 2)
        }
        Some(b'A' | b'B' | b'G' | b'Z' | b'z') if in_class => {
            Err("Regex anchor is invalid in a character class".into())
        }
        Some(&kind @ (b'p' | b'P')) => {
            let property = rest[1..]
                .strip_prefix('{')
                .and_then(|property| property.split_once('}'));
            if let Some((name, trailing)) = property {
                let end = pattern.len() - trailing.len();
                if let Some((first, last)) = unicode_blocks::range(name) {
                    if (0xD800..=0xDFFF).contains(&first) {
                        return Err(format!(
                            "Unicode block {name} requires matching UTF-16 surrogate units"
                        ));
                    }
                    let negation = if kind == b'P' { "^" } else { "" };
                    write!(output, r"[{negation}\x{{{first:X}}}-\x{{{last:X}}}]")
                        .expect("writing to a string cannot fail");
                } else {
                    output.push_str(&pattern[start..end]);
                }
                return Ok(end);
            }
            output.push_str(&pattern[start..start + 2]);
            Ok(start + 2)
        }
        Some(&kind @ (b'w' | b'W' | b'd' | b'D')) => {
            let categories = if kind.eq_ignore_ascii_case(&b'w') {
                WORD_CATEGORIES
            } else {
                r"\p{Nd}"
            };
            let negation = if kind.is_ascii_uppercase() { "^" } else { "" };
            write!(output, "[{negation}{categories}&&{BMP}]")
                .expect("writing to a string cannot fail");
            Ok(start + 2)
        }
        Some(b'c') => {
            let value = rest
                .as_bytes()
                .get(1)
                .ok_or("Missing ASCII control character")?;
            let value = value.to_ascii_uppercase();
            if !(b'@'..=b'_').contains(&value) {
                return Err("Invalid ASCII control character".into());
            }
            write_escape(output, value - b'@');
            Ok(start + 3)
        }
        _ => {
            let end = start + 1 + rest.chars().next().map_or(0, char::len_utf8);
            output.push_str(&pattern[start..end]);
            Ok(end)
        }
    }
}

fn character_class(
    pattern: &str,
    start: usize,
    output: &mut String,
    depth: usize,
) -> Result<usize, String> {
    if depth >= 128 {
        return Err("Regex character class nesting exceeds 128".into());
    }
    let bytes = pattern.as_bytes();
    let base = output.len();
    output.push('[');
    let mut index = start + 1;
    if bytes.get(index) == Some(&b'^') {
        output.push('^');
        index += 1;
    }
    let members = index;
    let mut expect_range_end = false;
    while let Some(&byte) = bytes.get(index) {
        let (end, can_start_range) = match byte {
            b'\\' => {
                let escape = bytes.get(index + 1);
                let is_set = matches!(
                    escape,
                    Some(b'd' | b'D' | b's' | b'S' | b'w' | b'W' | b'p' | b'P')
                );
                if expect_range_end && is_set {
                    return Err("A character class escape cannot end a range".into());
                }
                let end = character_escape(pattern, index, true, output)?;
                (end, !is_set && escape != Some(&b'-'))
            }
            b'-' if !expect_range_end && index > members && bytes.get(index + 1) == Some(&b'[') => {
                // Group the base class to apply its negation before subtraction.
                output.insert(base, '[');
                output.push_str("]--");
                let end = character_class(pattern, index + 1, output, depth + 1)?;
                if bytes.get(end) != Some(&b']') {
                    return Err("Subtraction must end the regex character class".into());
                }
                output.push(']');
                return Ok(end + 1);
            }
            b']' if index > members => {
                output.push(']');
                return Ok(index + 1);
            }
            b'&' | b'-' | b'~'
                if !expect_range_end && index > members
                    && bytes.get(index + 1) == Some(&byte)
                    && bytes.get(index + 2) == Some(&b'[') =>
            {
                // Recognize the backend's bracketed set operands as well.
                output.push_str(&pattern[index..index + 2]);
                (character_class(pattern, index + 2, output, depth + 1)?, false)
            }
            b'[' if bytes.get(index + 1) == Some(&b':') => {
                let name = index + 2 + usize::from(bytes.get(index + 2) == Some(&b'^'));
                let end = name
                    + bytes[name..]
                        .iter()
                        .take_while(|ch| ch.is_ascii_alphabetic())
                        .count();
                if end > name && bytes.get(end..end + 2) == Some(b":]") {
                    output.push_str(&pattern[index..end + 2]);
                    (end + 2, false)
                } else {
                    output.push_str(r"\[");
                    (index + 1, true)
                }
            }
            _ => {
                let character = pattern[index..].chars().next().unwrap();
                if matches!(character, '[' | ']' | '-' | '&' | '~') {
                    output.push('\\');
                }
                output.push(character);
                (index + character.len_utf8(), true)
            }
        };
        // Only a range operator remains unescaped. The backend otherwise
        // interprets a range ending or starting in '-' as set subtraction.
        let rest = &bytes[end..];
        expect_range_end = !expect_range_end
            && can_start_range
            && matches!(rest, [b'-', target, ..] if !matches!(target, b'[' | b']'))
            && !rest.starts_with(b"--[");
        index = end;
        if expect_range_end {
            output.push('-');
            index += 1;
        }
    }
    Err("Unclosed regex character class".into())
}

fn inline_options(
    pattern: &str,
    start: usize,
    mut options: Options,
) -> Option<(usize, bool, Options, String)> {
    if !pattern[start..].starts_with("(?") {
        return None;
    }
    let mut positive = String::new();
    let mut negative = String::new();
    let mut enabled = true;
    let mut seen_flag = false;
    let mut index = start + 2;
    loop {
        let byte = *pattern.as_bytes().get(index)?;
        match byte {
            b'n' => {
                options.explicit_capture = enabled;
                seen_flag = true;
            }
            b'i' | b'm' | b's' | b'x' | b'R' | b'U' | b'u' => {
                match byte {
                    b'm' => options.multiline = enabled,
                    b'x' => options.ignore_whitespace = enabled,
                    b'R' => options.crlf = enabled,
                    _ => {}
                }
                if enabled {
                    positive.push(byte as char);
                } else {
                    negative.push(byte as char);
                }
                seen_flag = true;
            }
            b'-' if enabled => enabled = false,
            b':' | b')' if seen_flag || (byte == b':' && enabled) => {
                if !negative.is_empty() {
                    positive.push('-');
                    positive.push_str(&negative);
                }
                return Some((index + 1, byte == b':', options, positive));
            }
            _ => return None,
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fancy_regex::{Regex, RegexBuilder};

    fn explicit_capture(pattern: &str, whitespace: bool) -> Result<String, String> {
        Ok(normalize(pattern, true, whitespace)?.expression)
    }

    fn regex(pattern: &str, whitespace: bool) -> Regex {
        let normalized = explicit_capture(pattern, whitespace).unwrap();
        RegexBuilder::new(&normalized)
            .ignore_whitespace(whitespace)
            .build()
            .unwrap()
    }

    #[test]
    fn plain_groups_are_not_captures_and_named_captures_remain() {
        let expression = regex(r"(a)(?<word>b)\1\k<word>", false);
        let found = expression.captures("abbb").unwrap().unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found.name("word").unwrap().as_str(), "b");
        assert!(explicit_capture(r"(a)\1", false).is_err());
    }

    #[test]
    fn explicit_capture_options_scope_and_unnamed_numbering_precede_names() {
        let expression = regex(r"(?<named>a)(?-n:(b)(?n)(c))(d)\1\2", false);
        assert!(expression.is_match("abcdba").unwrap());
        assert!(!expression.is_match("abcdab").unwrap());
        assert_eq!(expression.captures_len(), 3);
        let expression = regex(r"(?-n)(a)(?n)(b)(?-n:(c))(d)\1\2", false);
        assert!(expression.is_match("abcdac").unwrap());
        assert_eq!(expression.captures_len(), 3);
    }

    #[test]
    fn named_and_assertion_conditions_keep_their_delimiter_structure() {
        let expression = regex(r"^(?<letter>a)?(?(letter)b|c)$", false);
        assert!(expression.is_match("ab").unwrap());
        assert!(expression.is_match("c").unwrap());
        assert!(!expression.is_match("b").unwrap());
        let expression = regex(r"^(?(?=a)(a)|(b))$", false);
        assert!(expression.is_match("a").unwrap());
        assert!(expression.is_match("b").unwrap());
        assert_eq!(expression.captures_len(), 1);
        for pattern in [r"^(?(ab)ab|c)$", r"^(?(a[b])ab|c)$"] {
            let expression = regex(pattern, false);
            assert!(expression.is_match("ab").unwrap());
            assert!(expression.is_match("c").unwrap());
            assert!(!expression.is_match("b").unwrap());
        }
    }

    #[test]
    fn conditions_recognize_unicode_capture_names_and_check_numeric_references() {
        use crate::search::{SearchMode, SearchStrategyFactory};

        for name in [
            "e\u{0301}",
            "\u{0301}start",
            "left\u{203f}right",
            "join\u{200d}ed",
            "split\u{200c}name",
            "١٢",
        ] {
            let pattern = format!(r"\A(?<{name}>a)?(?({name})b|c)\z");
            let search = SearchStrategyFactory::create(&pattern, false, false, SearchMode::RegEx)
                .unwrap();
            let hit = search.find_next("ab", 0, 2).unwrap().unwrap();
            assert_eq!(hit.named_capture(name), Some("a"), "{pattern}");
            assert_eq!(hit.try_replace_with(&format!("${{{name}}}")).unwrap(), "a");
            assert!(search.find_next("c", 0, 1).unwrap().is_some(), "{pattern}");
            assert!(search.find_next("ac", 0, 2).unwrap().is_none(), "{pattern}");
            assert!(search.find_next("b", 0, 1).unwrap().is_none(), "{pattern}");
        }
        let expression = regex("\\A(?(e\u{0301})e\u{0301}|c)\\z", false);
        assert!(expression.is_match("e\u{0301}").unwrap());
        assert!(expression.is_match("c").unwrap());
        let backend_extension = regex(r"\A(?<𐐀>a)?(?(𐐀)b|c)\z", false);
        assert!(backend_extension.is_match("ab").unwrap());
        for pattern in ["(?(1name)a|b)", "(?(2[0])a|b)", "(?(123)a|b)"] {
            assert!(normalize_search(pattern, true).is_err(), "{pattern}");
        }
    }

    #[test]
    fn escapes_classes_and_comments_do_not_create_groups_or_change_flags() {
        let expression = regex(
            "(?x) \\( (a) [()] [\\[()] (?# (?n ) # (?-n) (\n (?<word>b) \\k<word>",
            false,
        );
        assert_eq!(expression.captures_len(), 2);
        assert!(expression.is_match("(a([bb").unwrap());
        let expression = regex(
            r"(?x:(a)# ignored (?-n) (
        )(?-x:(b)#)(c)",
            false,
        );
        assert_eq!(expression.captures_len(), 1);
        assert!(expression.is_match("ab#c").unwrap());
    }

    #[test]
    fn extended_whitespace_obeys_source_token_boundaries() {
        for (pattern, whitespace, matching, different) in [
            ("\\Aa\u{c}b\\z", true, "ab", "a\u{c}b"),
            ("(?x)\\Aa\u{b}b\\z", false, "a\u{b}b", "ab"),
            ("(?x)\\Aa[ #\u{c}]b\\z", false, "a\u{c}b", "ab"),
            ("(?x)\\Aa\\\u{c}b\\z", false, "a\u{c}b", "ab"),
            (r"\Aa(?# hidden\)b\z", false, "ab", "a"),
            (r"(?x)\Aa{1, 3}\z", false, "a{1,3}", "aaa"),
            ("(?x)\\Aa{1,\u{c}3}\\z", false, "a{1,3}", "aaa"),
            ("(?x)\\Aa{1,# comment\n3}\\z", false, "a{1,3}", "aaa"),
            (r"\Aa{1,(?# comment)3}\z", false, "a{1,3}", "aaa"),
            (r"(?x)\Aa{1,3}\z", false, "aaa", "a{1,3}"),
            (r"(?x)\Aa{,3}\z", false, "a{,3}", "aaa"),
            ("(?x)\\Aa(?-x:\u{c})b\\z", false, "a\u{c}b", "ab"),
            (r"(?x)\A\x{4 1}\u{1f642}\z", false, "A🙂", "a🙂"),
        ] {
            let expression = regex(pattern, whitespace);
            assert!(expression.is_match(matching).unwrap(), "{pattern:?}");
            assert!(!expression.is_match(different).unwrap(), "{pattern:?}");
        }
        let lazy = regex("(?x)a+\u{c}?", false);
        assert_eq!(lazy.find("aaa").unwrap().unwrap().as_str(), "a");
        assert!(explicit_capture(r"(?x)\A( ? :a)\z", false).is_err());
    }

    #[test]
    fn named_and_assertion_groups_restore_nested_inline_option_scopes() {
        let expression = regex(r"(?<word>(?i)a)b", false);
        assert!(expression.is_match("Ab").unwrap());
        assert!(!expression.is_match("AB").unwrap());
        let expression = regex(r"(?=(?i)a)ab", false);
        assert!(expression.is_match("ab").unwrap());
        assert!(!expression.is_match("Ab").unwrap());
    }

    #[test]
    fn numeric_escapes_prefer_a_declared_capture_and_consume_up_to_three_octal_digits() {
        for (pattern, matching, different) in [
            (r"(a)\10", "a\u{0008}", "aa0"),
            (r"(?<10>a)\10", "aa", "a\u{0008}"),
            (r"(?<10>a)\010", "a\u{0008}", "aa"),
            (r"\118", "\t8", "\t"),
            (r"\1234", "S4", "S"),
            (r"\777\400", "ÿ\0", "777400"),
            (r"\78+", "\u{0007}88", "\u{0007}"),
            (r"\c@\ca\c[", "\0\u{0001}\u{001b}", "[a]"),
        ] {
            let source = format!(r"\A(?:{pattern})\z");
            let normalized = normalize(&source, false, false).unwrap();
            let expression = Regex::new(&normalized.expression).unwrap();
            assert!(expression.is_match(matching).unwrap(), "{pattern}");
            assert!(!expression.is_match(different).unwrap(), "{pattern}");
        }
        for invalid in [
            r"\8",
            r"\80",
            r"\k<10>",
            r"(?(10)a|b)",
            r"\2147483648",
            r"\c?",
        ] {
            assert!(normalize(invalid, false, false).is_err(), "{invalid}");
        }
    }

    #[test]
    fn class_subtraction_and_literal_members_follow_reference_regexes() {
        for (pattern, matching, different) in [
            (r"[\1-\3\40\b]", "\u{0001}\u{0002}\u{0003} \u{0008}", "80"),
            (r"[\c[\c]]", "\u{001b}\u{001d}", "[]"),
            (r"[[()]", "[()", "]#"),
            (r"[]]", "]", "[x"),
            (r"[-[]", "-[", "]x"),
            (r"[a&&b]", "a&b", "c~"),
            (r"[--a]", "-.@a", "!+b"),
            (r"[]-a]", "]^_a", "[b"),
            (r"[!--b]", "!+-b", ".a"),
            (r"[---]", "-", "+.a"),
            (r"[\--a]", "-a", ".b"),
            (r"[\d-a]", "-0a", ".b"),
            (r"[\p{Nd}-a]", "-0a", ".b"),
            (r"[!---[#]]", "!+-", "#.a"),
            (r"[a-z-[aeiou-[e]]]", "bez", "aiou"),
            (r"[^a-z-[0-9]]", "B#", "b1"),
            (r"[a-z-[^aeiou]]", "aeiou", "bz"),
            (r"[a-[a]]", "", "a-[]"),
            (r"[a-z&&[aeiou]]", "aeiou", "bz"),
            (r"[a-z--[aeiou]]", "bz", "aeiou"),
            (r"[a-c~~[b-e]]", "ade", "bcf"),
            (r"[[:digit:]]", "39", "x[]"),
        ] {
            let expression = regex(pattern, false);
            for ch in matching.chars() {
                assert!(
                    expression.is_match(&ch.to_string()).unwrap(),
                    "{pattern} {ch:?}"
                );
            }
            for ch in different.chars() {
                assert!(
                    !expression.is_match(&ch.to_string()).unwrap(),
                    "{pattern} {ch:?}"
                );
            }
        }
        for invalid in [r"[\8]", r"[a-z-[aeiou]X]", r"[a-[b]"] {
            assert!(normalize(invalid, false, false).is_err(), "{invalid}");
        }
        for invalid in [r"[a--b]", r"[a-\d]", r"[a-\p{Nd}]"] {
            let compiled = normalize(invalid, false, false)
                .ok()
                .and_then(|pattern| Regex::new(&pattern.expression).ok());
            assert!(compiled.is_none(), "{invalid}");
        }
    }

    #[test]
    fn unsupported_capture_histories_are_rejected_instead_of_rebound() {
        for pattern in [r"(?<x>a)(?<x>b)", r"(?<1>a)(?<1>b)", r"(?<x-y>a)"] {
            assert!(explicit_capture(pattern, false).is_err(), "{pattern}");
        }
    }
}
