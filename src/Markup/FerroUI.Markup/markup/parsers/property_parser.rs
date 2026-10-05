//! Port of `Markup.Xaml/Parsers/PropertyParser.cs`.
//!
//! Upstream the file belongs to the markup runtime assembly. It lives here because both the
//! XAML compiler extensions and the markup runtime use it and the compiler must not depend on
//! the runtime library.

use ferroui_base::data::core::ExpressionParseException;
use ferroui_base::utilities::CharacterReader;

/// Parses a property reference: `Name`, `Owner.Name`, `ns:Owner.Name`,
/// optionally in parentheses (`(Owner.Name)`).
pub struct PropertyParser;

/// The parts of a property reference: namespace prefix, owner type and
/// property name.
pub type PropertyReference = (Option<String>, Option<String>, String);

impl PropertyParser {
    pub fn parse(text: &str) -> Result<PropertyReference, ExpressionParseException> {
        let r = CharacterReader::new(text);
        Self::parse_reader(r)
    }

    pub fn parse_reader(mut r: CharacterReader<'_>) -> Result<PropertyReference, ExpressionParseException> {
        if r.end() {
            return Err(ExpressionParseException::new(0, "Expected property name."));
        }

        let open_parens = r.take_if('(');
        let mut close_parens = false;
        let mut ns: Option<String> = None;
        let mut owner: Option<String> = None;
        let mut name: Option<String> = None;

        loop {
            let token = r.parse_identifier();

            if token.is_empty() {
                if r.end() {
                    break;
                }

                if open_parens && !r.end() {
                    close_parens = r.take_if(')');
                    if close_parens {
                        break;
                    }
                }

                if open_parens {
                    return Err(ExpressionParseException::new(r.position(), "Expected ')'."));
                }

                let peek = r.peek().map(String::from).unwrap_or_default();
                return Err(ExpressionParseException::new(r.position(), format!("Unexpected '{peek}'.")));
            } else if !r.end() && r.take_if(':') {
                if ns.is_some() {
                    return Err(ExpressionParseException::new(r.position(), "Unexpected ':'."));
                }
                ns = Some(token.to_string());
            } else if !r.end() && r.take_if('.') {
                if owner.is_some() {
                    return Err(ExpressionParseException::new(r.position(), "Unexpected '.'."));
                }
                owner = Some(token.to_string());
            } else {
                name = Some(token.to_string());
            }

            if r.end() {
                break;
            }
        }

        let Some(name) = name else {
            return Err(ExpressionParseException::new(0, "Expected property name."));
        };

        if open_parens && owner.is_none() {
            Err(ExpressionParseException::new(1, "Expected property owner."))
        } else if open_parens && !close_parens {
            Err(ExpressionParseException::new(r.position(), "Expected ')'."))
        } else if !r.end() {
            Err(ExpressionParseException::new(r.position(), "Expected end of expression."))
        } else {
            Ok((ns, owner, name))
        }
    }
}

#[cfg(test)]
mod tests {
    //! Port of `Parsers/PropertyParserTests.cs`.

    use super::*;

    fn parse(text: &str) -> PropertyReference {
        PropertyParser::parse_reader(CharacterReader::new(text)).unwrap()
    }

    fn fails(text: &str, column: i32, message: &str) {
        let ex = PropertyParser::parse_reader(CharacterReader::new(text)).unwrap_err();
        assert_eq!(ex.column(), column, "{text}");
        assert_eq!(ex.message(), message, "{text}");
    }

    #[test]
    fn parses_name() {
        assert_eq!(parse("Foo"), (None, None, "Foo".to_string()));
    }

    #[test]
    fn parses_owner_and_name() {
        assert_eq!(parse("Foo.Bar"), (None, Some("Foo".to_string()), "Bar".to_string()));
    }

    #[test]
    fn parses_namespace_owner_and_name() {
        assert_eq!(parse("foo:Bar.Baz"), (Some("foo".to_string()), Some("Bar".to_string()), "Baz".to_string()));
    }

    #[test]
    fn parses_owner_and_name_with_parentheses() {
        assert_eq!(parse("(Foo.Bar)"), (None, Some("Foo".to_string()), "Bar".to_string()));
    }

    #[test]
    fn parses_namespace_owner_and_name_with_parentheses() {
        assert_eq!(parse("(foo:Bar.Baz)"), (Some("foo".to_string()), Some("Bar".to_string()), "Baz".to_string()));
    }

    #[test]
    fn parse_of_text_is_parse_of_a_reader() {
        assert_eq!(PropertyParser::parse("foo:Bar.Baz").unwrap(), parse("foo:Bar.Baz"));
        assert_eq!(PropertyParser::parse("").unwrap_err().message(), "Expected property name.");
    }

    #[test]
    fn fails_with_empty_string() {
        fails("", 0, "Expected property name.");
    }

    #[test]
    fn fails_with_only_whitespace() {
        fails("  ", 0, "Unexpected ' '.");
    }

    #[test]
    fn fails_with_leading_whitespace() {
        fails(" Foo", 0, "Unexpected ' '.");
    }

    #[test]
    fn fails_with_trailing_whitespace() {
        fails("Foo ", 3, "Unexpected ' '.");
    }

    #[test]
    fn fails_with_invalid_property_name() {
        fails("123", 0, "Unexpected '1'.");
    }

    #[test]
    fn fails_with_trailing_junk() {
        fails("Foo%", 3, "Unexpected '%'.");
    }

    #[test]
    fn fails_with_invalid_property_name_after_owner() {
        fails("Foo.123", 4, "Unexpected '1'.");
    }

    #[test]
    fn fails_with_whitespace_between_owner_and_name() {
        fails("Foo. Bar", 4, "Unexpected ' '.");
    }

    #[test]
    fn fails_with_too_many_segments() {
        fails("Foo.Bar.Baz", 8, "Unexpected '.'.");
    }

    #[test]
    fn fails_with_too_many_namespaces() {
        fails("foo:bar:Baz", 8, "Unexpected ':'.");
    }

    #[test]
    fn fails_with_parens_but_no_owner() {
        fails("(Foo)", 1, "Expected property owner.");
    }

    #[test]
    fn fails_with_parens_and_namespace_but_no_owner() {
        fails("(foo:Bar)", 1, "Expected property owner.");
    }

    #[test]
    fn fails_with_missing_close_parens() {
        fails("(Foo.Bar", 8, "Expected ')'.");
    }

    #[test]
    fn fails_with_unexpected_close_parens() {
        fails("Foo.Bar)", 7, "Unexpected ')'.");
    }
}
