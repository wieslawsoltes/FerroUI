use serde::{Deserialize, Serialize};

/// A decoded XAML attribute value. The containing syntax attribute also retains
/// its original source span, so inspection never rewrites its lexical spelling.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkupValue {
    Literal(String),
    Extension(MarkupExtension),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkupExtension {
    /// Qualified extension name, such as `Binding` or `local:Resolve`.
    pub name: String,
    pub arguments: Vec<MarkupArgument>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkupArgument {
    /// `None` denotes a positional argument.
    pub name: Option<String>,
    pub value: MarkupValue,
}

impl MarkupValue {
    /// Parse nested markup extensions without resolving types or resources.
    /// `{}{...}` escapes a literal, and both quoted and unquoted argument values
    /// support XAML backslash escapes. Unknown extension names are retained.
    pub fn parse(value: &str) -> Result<Self, String> {
        if let Some(literal) = value.strip_prefix("{}") {
            return Ok(Self::Literal(literal.to_owned()));
        }
        if !value.starts_with('{') {
            return Ok(Self::Literal(value.to_owned()));
        }
        Parser::parse_complete(value, 0).map(Self::Extension)
    }
}

// Match the runtime XamlX parser's limit. Arbitrary editor or MCP input must
// produce a diagnostic instead of overflowing the parser or value drop stack.
const MAX_EXTENSION_DEPTH: usize = 128;

/// A cursor consumes each ordinary nested extension once. The previous parser
/// split every argument list twice at each nesting level, repeatedly scanning
/// complete subtrees and allocating temporary argument vectors.
struct Parser<'a> {
    source: &'a str,
    offset: usize,
}

impl<'a> Parser<'a> {
    fn parse_complete(source: &'a str, depth: usize) -> Result<MarkupExtension, String> {
        let mut parser = Self { source, offset: 0 };
        let extension = parser.extension(depth)?;
        parser.skip_whitespace();
        if parser.offset != source.len() {
            return Err("Unexpected content after markup extension".into());
        }
        Ok(extension)
    }

    fn extension(&mut self, depth: usize) -> Result<MarkupExtension, String> {
        if depth >= MAX_EXTENSION_DEPTH {
            return Err("Markup extensions are nested too deeply".into());
        }
        debug_assert_eq!(self.peek(), Some('{'));
        self.advance();
        self.skip_whitespace();
        let start = self.offset;
        while self.peek().is_some_and(|ch| {
            !ch.is_whitespace() && !matches!(ch, '{' | '}' | ',' | '=' | '\'' | '"' | '\\')
        }) {
            self.advance();
        }
        let name = &self.source[start..self.offset];
        if name.is_empty() {
            return Err("A markup extension requires a qualified type name".into());
        }
        if self
            .peek()
            .is_some_and(|ch| !ch.is_whitespace() && !matches!(ch, '}' | ','))
        {
            return Err(
                "The markup-extension name must be followed by whitespace or a closing brace"
                    .into(),
            );
        }
        let mut arguments = Vec::new();
        let mut names = std::collections::HashSet::new();
        self.skip_whitespace();
        if self.peek() == Some('}') {
            self.advance();
            return Ok(MarkupExtension {
                name: name.to_owned(),
                arguments,
            });
        }
        if self.peek() == Some(',') {
            return Err("Unexpected comma after markup extension name".into());
        }
        loop {
            self.skip_whitespace();
            if matches!(self.peek(), None | Some(',' | '}')) {
                return Err("Empty or unclosed markup-extension argument".into());
            }
            let (first, can_be_name) = self.value(depth)?;
            self.skip_whitespace();
            let (argument_name, value) = if self.peek() == Some('=') {
                let MarkupValue::Literal(argument_name) = first else {
                    return Err("Invalid named markup-extension argument".into());
                };
                if !can_be_name || argument_name.is_empty() {
                    return Err("Invalid named markup-extension argument".into());
                }
                if !names.insert(argument_name.clone()) {
                    return Err(format!(
                        "Duplicate markup-extension argument: {argument_name}"
                    ));
                }
                self.advance();
                let (value, _) = self.value(depth)?;
                (Some(argument_name), value)
            } else {
                if !names.is_empty() {
                    return Err(
                        "Positional argument follows a named markup-extension argument".into(),
                    );
                }
                (None, first)
            };
            arguments.push(MarkupArgument {
                name: argument_name,
                value,
            });
            self.skip_whitespace();
            match self.peek() {
                Some('}') => {
                    self.advance();
                    return Ok(MarkupExtension {
                        name: name.to_owned(),
                        arguments,
                    });
                }
                Some(',') => {
                    self.advance();
                }
                None => return Err("Unclosed markup extension".into()),
                _ => return Err("Unexpected content after markup-extension argument".into()),
            }
        }
    }

    /// The boolean indicates whether an unquoted token can be a property name.
    fn value(&mut self, depth: usize) -> Result<(MarkupValue, bool), String> {
        self.skip_whitespace();
        match self.peek() {
            Some('{') if self.source[self.offset..].starts_with("{}") => {
                self.offset += 2;
                Ok((MarkupValue::Literal(self.literal()?), false))
            }
            Some('{') => Ok((MarkupValue::Extension(self.extension(depth + 1)?), false)),
            Some('\'' | '"') => Ok((self.quoted_value(depth)?, false)),
            None => Err("Unclosed markup extension".into()),
            _ => Ok((MarkupValue::Literal(self.literal()?), true)),
        }
    }

    fn quoted_value(&mut self, depth: usize) -> Result<MarkupValue, String> {
        let quote = self.advance().unwrap();
        let start = self.offset;
        loop {
            match self.advance() {
                Some(ch) if ch == quote => {
                    let raw = &self.source[start..self.offset - quote.len_utf8()];
                    if let Some(literal) = raw.strip_prefix("{}") {
                        return Ok(MarkupValue::Literal(unescape(literal)));
                    }
                    // Quoting protects commas and equals signs; it does not
                    // turn a leading nested extension into a string. A leading
                    // escaped brace does remain literal after decoding.
                    if raw.starts_with('{') {
                        return Parser::parse_complete(&unescape(raw), depth + 1)
                            .map(MarkupValue::Extension);
                    }
                    return Ok(MarkupValue::Literal(unescape(raw)));
                }
                Some('\\') => {
                    self.advance()
                        .ok_or("Unclosed quoted markup-extension value")?;
                }
                Some(_) => {}
                None => return Err("Unclosed quoted markup-extension value".into()),
            }
        }
    }

    fn literal(&mut self) -> Result<String, String> {
        let start = self.offset;
        let mut braces = 0usize;
        while let Some(ch) = self.peek() {
            match ch {
                '\\' => {
                    self.advance();
                    self.advance().ok_or("Unclosed markup-extension escape")?;
                    continue;
                }
                '{' => braces += 1,
                '}' if braces > 0 => braces -= 1,
                ',' | '=' | '}' if braces == 0 => break,
                '\'' | '"' => {
                    return Err("Quote characters must enclose the complete argument value".into())
                }
                _ => {}
            }
            self.advance();
        }
        if braces != 0 {
            return Err("Unclosed brace in markup-extension literal".into());
        }
        Ok(unescape(self.source[start..self.offset].trim()))
    }

    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.offset += ch.len_utf8();
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.advance();
        }
    }
}

fn unescape(value: &str) -> String {
    if !value.contains('\\') {
        return value.to_owned();
    }
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            // Callers have already validated that every escape has a character.
            output.push(chars.next().unwrap_or('\\'));
        } else {
            output.push(ch);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{MarkupValue, MAX_EXTENSION_DEPTH};

    fn values(source: &str) -> Vec<MarkupValue> {
        let MarkupValue::Extension(extension) = MarkupValue::parse(source).unwrap() else {
            panic!("expected extension");
        };
        extension
            .arguments
            .into_iter()
            .map(|argument| argument.value)
            .collect()
    }

    #[test]
    fn unquoted_escapes_and_balanced_literal_braces_match_runtime_grammar() {
        assert_eq!(
            values(r"{Ext a\,b, \{c\}, a\=b, café\ 🦀, a{b}c, {}{literal}}"),
            ["a,b", "{c}", "a=b", "café 🦀", "a{b}c", "{literal}"]
                .map(|value| MarkupValue::Literal(value.into()))
        );
    }

    #[test]
    fn quoted_nested_extensions_and_escaped_literals_remain_distinct() {
        let parsed = values(r#"{Outer '{Inner 2}', '{}{literal}', '\{escaped}', "  quoted  "}"#);
        assert!(
            matches!(&parsed[0], MarkupValue::Extension(extension) if extension.name == "Inner")
        );
        assert_eq!(parsed[1], MarkupValue::Literal("{literal}".into()));
        assert_eq!(parsed[2], MarkupValue::Literal("{escaped}".into()));
        assert_eq!(parsed[3], MarkupValue::Literal("  quoted  ".into()));
    }

    #[test]
    fn incomplete_or_ambiguous_argument_boundaries_are_diagnostics() {
        for source in [
            "{Binding Path='a'b}",
            "{Binding Path=a'b'}",
            "{Binding Path={Resource key}tail}",
            "{Binding Path=A, B}",
            "{Binding Path=A, Path=B}",
            "{Binding Path=A,}",
            "{Binding Path=foo=bar}",
            "{Binding Path='unterminated}",
            "{Binding}trailing",
            "{Binding Path=a{b}",
            "{Binding Path=foo\\}",
        ] {
            assert!(MarkupValue::parse(source).is_err(), "{source}");
        }
    }

    #[test]
    fn attached_names_empty_values_and_whitespace_are_supported() {
        let MarkupValue::Extension(extension) =
            MarkupValue::parse("{ Ext Other.Prop=, Text=''}  ").unwrap()
        else {
            panic!("expected extension");
        };
        assert_eq!(extension.name, "Ext");
        assert_eq!(extension.arguments[0].name.as_deref(), Some("Other.Prop"));
        assert_eq!(
            extension.arguments[0].value,
            MarkupValue::Literal(String::new())
        );
        assert_eq!(
            extension.arguments[1].value,
            MarkupValue::Literal(String::new())
        );
    }

    #[test]
    fn deeply_nested_untrusted_input_is_bounded_without_stack_overflow() {
        let at_limit = format!(
            "{}value{}",
            "{Ext ".repeat(MAX_EXTENSION_DEPTH),
            "}".repeat(MAX_EXTENSION_DEPTH)
        );
        assert!(MarkupValue::parse(&at_limit).is_ok());
        let over_limit = format!(
            "{}value{}",
            "{Ext ".repeat(MAX_EXTENSION_DEPTH + 1),
            "}".repeat(MAX_EXTENSION_DEPTH + 1)
        );
        assert!(MarkupValue::parse(&over_limit)
            .unwrap_err()
            .contains("too deeply"));
        let much_deeper = format!("{}value{}", "{Ext ".repeat(20_000), "}".repeat(20_000));
        assert!(MarkupValue::parse(&much_deeper)
            .unwrap_err()
            .contains("too deeply"));
    }

    #[test]
    fn large_argument_lists_preserve_order_and_reject_duplicate_names() {
        let mut source = String::from("{Ext ");
        for index in 0..10_000 {
            if index != 0 {
                source.push(',');
            }
            source.push_str(&format!("P{index}=V{index}"));
        }
        source.push('}');
        let MarkupValue::Extension(extension) = MarkupValue::parse(&source).unwrap() else {
            panic!("expected extension");
        };
        assert_eq!(extension.arguments.len(), 10_000);
        assert_eq!(extension.arguments[9_999].name.as_deref(), Some("P9999"));
        assert_eq!(
            extension.arguments[9_999].value,
            MarkupValue::Literal("V9999".into())
        );
        source.insert_str(source.len() - 1, ",P0=duplicate");
        assert!(MarkupValue::parse(&source)
            .unwrap_err()
            .contains("Duplicate"));
    }
}
