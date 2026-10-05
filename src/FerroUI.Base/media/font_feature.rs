use std::fmt;
use std::str::FromStr;

const DEFAULT_VALUE: i32 = 1;
const INFINITY_END: i32 = -1;

/// Font feature.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontFeature {
    /// Gets or sets the tag.
    pub tag: String,
    /// Gets or sets the value.
    pub value: i32,
    /// Gets or sets the start.
    pub start: i32,
    /// Gets or sets the end.
    pub end: i32,
}

impl Default for FontFeature {
    /// Creates an instance of `FontFeature`.
    fn default() -> Self {
        Self { tag: String::new(), value: DEFAULT_VALUE, start: 0, end: INFINITY_END }
    }
}

/// The pieces matched by the feature grammar.
struct FeatureMatch<'a> {
    tag: &'a str,
    value: &'a str,
    start: &'a str,
    end: &'a str,
    has_separator: bool,
}

/// A cursor over the characters of the input.
struct Cursor<'a> {
    s: &'a str,
    position: usize,
}

impl<'a> Cursor<'a> {
    fn rest(&self) -> &'a str {
        &self.s[self.position..]
    }

    fn skip_whitespace(&mut self) {
        let rest = self.rest();
        self.position += rest.len() - rest.trim_start().len();
    }

    fn eat(&mut self, c: char) -> bool {
        if self.rest().starts_with(c) {
            self.position += c.len_utf8();
            true
        } else {
            false
        }
    }

    fn digits(&mut self) -> &'a str {
        let rest = self.rest();
        let length = rest.find(|c: char| !c.is_numeric()).unwrap_or(rest.len());
        self.position += length;
        &rest[..length]
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Matches
/// `^\s*(?<Value>[+-])?\s*(?<Tag>\w{4})\s*(\[\s*(?<Start>\d+)?(\s*(?<Separator>:)\s*)?(?<End>\d+)?\s*\])?\s*(?(Value)()|(=\s*(?<Value>\d+|on|off)))?\s*$`.
fn match_feature(s: &str) -> Option<FeatureMatch<'_>> {
    let mut cursor = Cursor { s, position: 0 };

    cursor.skip_whitespace();

    let mut value = "";
    if cursor.rest().starts_with(['+', '-']) {
        value = &cursor.rest()[..1];
        cursor.position += 1;
    }

    cursor.skip_whitespace();

    let tag = {
        let rest = cursor.rest();
        let mut end = 0;
        let mut count = 0;
        for c in rest.chars() {
            if count == 4 || !is_word_char(c) {
                break;
            }
            end += c.len_utf8();
            count += 1;
        }
        if count != 4 {
            return None;
        }
        cursor.position += end;
        &rest[..end]
    };

    cursor.skip_whitespace();

    let mut start = "";
    let mut end = "";
    let mut has_separator = false;

    if cursor.eat('[') {
        cursor.skip_whitespace();
        start = cursor.digits();

        let before_separator = cursor.position;
        cursor.skip_whitespace();
        if cursor.eat(':') {
            has_separator = true;
            cursor.skip_whitespace();
        } else {
            cursor.position = before_separator;
        }

        end = cursor.digits();
        cursor.skip_whitespace();

        if !cursor.eat(']') {
            return None;
        }
    }

    cursor.skip_whitespace();

    if value.is_empty() && cursor.eat('=') {
        cursor.skip_whitespace();
        let digits = cursor.digits();
        if !digits.is_empty() {
            value = digits;
        } else if cursor.rest().starts_with("on") {
            value = "on";
            cursor.position += 2;
        } else if cursor.rest().starts_with("off") {
            value = "off";
            cursor.position += 3;
        } else {
            return None;
        }
    }

    cursor.skip_whitespace();

    if !cursor.rest().is_empty() {
        return None;
    }

    Some(FeatureMatch { tag, value, start, end, has_separator })
}

/// `int.TryParse(s, NumberStyles.None, InvariantCulture)`: ASCII digits only.
fn try_parse_digits(s: &str) -> Option<i32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

impl FontFeature {
    /// Parses a string to return a `FontFeature`.
    ///
    /// Syntax is the following:
    ///
    /// * Syntax — Value — Start — End
    /// * `kern` — 1 — 0 — ∞ — Turn feature on
    /// * `+kern` — 1 — 0 — ∞ — Turn feature on
    /// * `-kern` — 0 — 0 — ∞ — Turn feature off
    /// * `kern=0` — 0 — 0 — ∞ — Turn feature off
    /// * `kern=1` — 1 — 0 — ∞ — Turn feature on
    /// * `aalt=2` — 2 — 0 — ∞ — Choose 2nd alternate
    /// * `kern[]` — 1 — 0 — ∞ — Turn feature on
    /// * `kern[:]` — 1 — 0 — ∞ — Turn feature on
    /// * `kern[5:]` — 1 — 5 — ∞ — Turn feature on, partial
    /// * `kern[:5]` — 1 — 0 — 5 — Turn feature on, partial
    /// * `kern[3:5]` — 1 — 3 — 5 — Turn feature on, range
    /// * `kern[3]` — 1 — 3 — 3+1 — Turn feature on, single char
    /// * `aalt[3:5]=2` — 2 — 3 — 5 — Turn 2nd alternate on for range
    ///
    /// A string that does not match gives the default (empty tag) feature.
    pub fn parse(s: &str) -> FontFeature {
        let Some(matched) = match_feature(s) else {
            return FontFeature::default();
        };

        let start = try_parse_digits(matched.start);
        let end = try_parse_digits(matched.end);

        let mut string_value = matched.value;

        if string_value == "-" || string_value.eq_ignore_ascii_case("OFF") {
            string_value = "0";
        }

        if string_value == "+" || string_value.eq_ignore_ascii_case("ON") {
            string_value = "1";
        }

        FontFeature {
            tag: matched.tag.to_owned(),
            start: start.unwrap_or(0),
            end: match (end, start) {
                (Some(end), _) => end,
                (None, Some(start)) if !matched.has_separator => start + 1,
                _ => INFINITY_END,
            },
            value: try_parse_digits(string_value).unwrap_or(DEFAULT_VALUE),
        }
    }
}

impl FromStr for FontFeature {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(FontFeature::parse(s))
    }
}

/// Gets a string representation of the `FontFeature`.
impl fmt::Display for FontFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.value == 0 {
            f.write_str("-")?;
        }
        f.write_str(&self.tag)?;

        if self.start != 0 || self.end != INFINITY_END {
            f.write_str("[")?;

            if self.start > 0 {
                write!(f, "{}", self.start)?;
            }

            if self.end != self.start + 1 {
                f.write_str(":")?;
                if self.end != INFINITY_END {
                    write!(f, "{}", self.end)?;
                }
            }

            f.write_str("]")?;
        }

        if self.value == DEFAULT_VALUE || self.value == 0 {
            return Ok(());
        }

        write!(f, "={}", self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feature(tag: &str, value: i32, start: i32, end: i32) -> FontFeature {
        FontFeature { tag: tag.to_owned(), value, start, end }
    }

    #[test]
    fn parses_documented_syntax() {
        assert_eq!(FontFeature::parse("kern"), feature("kern", 1, 0, -1));
        assert_eq!(FontFeature::parse("+kern"), feature("kern", 1, 0, -1));
        assert_eq!(FontFeature::parse("-kern"), feature("kern", 0, 0, -1));
        assert_eq!(FontFeature::parse("kern=0"), feature("kern", 0, 0, -1));
        assert_eq!(FontFeature::parse("kern=1"), feature("kern", 1, 0, -1));
        assert_eq!(FontFeature::parse("aalt=2"), feature("aalt", 2, 0, -1));
        assert_eq!(FontFeature::parse("kern[]"), feature("kern", 1, 0, -1));
        assert_eq!(FontFeature::parse("kern[:]"), feature("kern", 1, 0, -1));
        assert_eq!(FontFeature::parse("kern[5:]"), feature("kern", 1, 5, -1));
        assert_eq!(FontFeature::parse("kern[:5]"), feature("kern", 1, 0, 5));
        assert_eq!(FontFeature::parse("kern[3:5]"), feature("kern", 1, 3, 5));
        assert_eq!(FontFeature::parse("kern[3]"), feature("kern", 1, 3, 4));
        assert_eq!(FontFeature::parse("aalt[3:5]=2"), feature("aalt", 2, 3, 5));
        assert_eq!(FontFeature::parse(" liga = off "), feature("liga", 0, 0, -1));
        assert_eq!(FontFeature::parse("liga=on"), feature("liga", 1, 0, -1));
    }

    #[test]
    fn invalid_input_gives_default() {
        assert_eq!(FontFeature::parse("kerning"), FontFeature::default());
        assert_eq!(FontFeature::parse("ker"), FontFeature::default());
        assert_eq!(FontFeature::parse("+kern=1"), FontFeature::default());
        assert_eq!(FontFeature::parse("kern[1 2]"), FontFeature::default());
    }

    #[test]
    fn display_round_trips() {
        for s in ["kern", "-kern", "aalt=2", "kern[5:]", "kern[:5]", "kern[3:5]", "kern[3]", "aalt[3:5]=2"] {
            assert_eq!(FontFeature::parse(s).to_string(), s);
        }
    }
}
