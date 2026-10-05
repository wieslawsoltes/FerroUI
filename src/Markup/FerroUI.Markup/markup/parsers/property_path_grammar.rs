//! The grammar of property paths (`(ns:Type.Property).Child :> Type . Sub`).

use ferroui_base::data::core::ExpressionParseException;
use ferroui_base::utilities::CharacterReader;

/// A syntax element of a parsed property path.
#[derive(Clone, Debug, PartialEq)]
pub enum PropertyPathSyntax {
    /// A property of the current object.
    Property { name: String },
    /// A property qualified with its owner type (`(ns:Type.Property)`).
    TypeQualifiedProperty {
        name: String,
        type_name: String,
        type_namespace: Option<String>,
    },
    /// Traversal to the value of the preceding property (`.`).
    ChildTraversal,
    /// Type check of the current value (`:= Type`).
    EnsureType {
        type_name: String,
        type_namespace: Option<String>,
    },
    /// Type cast of the current value (`:> Type` or `as Type`).
    CastType {
        type_name: String,
        type_namespace: Option<String>,
    },
}

/// Parser of property paths.
pub struct PropertyPathGrammar;

impl PropertyPathGrammar {
    /// Parses `s` into a list of syntax elements.
    pub fn parse(s: &str) -> Result<Vec<PropertyPathSyntax>, ExpressionParseException> {
        let mut r = CharacterReader::new(s);
        parse(&mut r)
    }
}

type ParseResult<T> = Result<T, ExpressionParseException>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Start,
    Next,
    AfterProperty,
    End,
}

fn parse(r: &mut CharacterReader<'_>) -> ParseResult<Vec<PropertyPathSyntax>> {
    let mut state = State::Start;
    let mut parsed = Vec::new();
    while state != State::End {
        let syntax;
        (state, syntax) = match state {
            State::Start => {
                let (state, syntax) = parse_start(r)?;
                (state, Some(syntax))
            }
            State::Next => parse_next(r)?,
            State::AfterProperty => parse_after_property(r)?,
            State::End => (State::End, None),
        };

        if let Some(syntax) = syntax {
            parsed.push(syntax);
        }
    }

    Ok(parsed)
}

fn parse_next(r: &mut CharacterReader<'_>) -> ParseResult<(State, Option<PropertyPathSyntax>)> {
    r.skip_whitespace();
    if r.end() {
        return Ok((State::End, None));
    }

    let (state, syntax) = parse_start(r)?;
    Ok((state, Some(syntax)))
}

fn parse_start(r: &mut CharacterReader<'_>) -> ParseResult<(State, PropertyPathSyntax)> {
    if let Some(rv) = try_parse_casts(r)? {
        return Ok(rv);
    }
    r.skip_whitespace();

    if r.take_if('(') {
        return parse_type_qualified_property(r);
    }

    parse_property(r)
}

fn parse_type_qualified_property(
    r: &mut CharacterReader<'_>,
) -> ParseResult<(State, PropertyPathSyntax)> {
    r.skip_whitespace();
    const ERROR: &str = "Unable to parse qualified property name, expected `(ns:TypeName.PropertyName)` or `(TypeName.PropertyName)` after `(`";

    let (ns, name) = parse_xaml_identifier(r)?;

    if !r.take_if('.') {
        return Err(ExpressionParseException::new(r.position(), ERROR));
    }

    let property_name = r.parse_identifier();
    if property_name.is_empty() {
        return Err(ExpressionParseException::new(r.position(), ERROR));
    }

    r.skip_whitespace();
    if !r.take_if(')') {
        return Err(ExpressionParseException::new(
            r.position(),
            format!(
                "Expected ')' after qualified property name {}:{}.{}",
                ns.as_deref().unwrap_or(""),
                name,
                property_name
            ),
        ));
    }

    Ok((
        State::AfterProperty,
        PropertyPathSyntax::TypeQualifiedProperty {
            name: property_name.to_string(),
            type_name: name,
            type_namespace: ns,
        },
    ))
}

/// Parses `name` or `ns:name`, returning `(ns, name)`.
fn parse_xaml_identifier(r: &mut CharacterReader<'_>) -> ParseResult<(Option<String>, String)> {
    let ident = r.parse_identifier();
    if ident.is_empty() {
        return Err(ExpressionParseException::new(
            r.position(),
            "Expected identifier",
        ));
    }
    if r.take_if(':') {
        let part2 = r.parse_identifier();
        if part2.is_empty() {
            return Err(ExpressionParseException::new(
                r.position(),
                format!("Expected the rest of the identifier after {ident}:"),
            ));
        }
        return Ok((Some(ident.to_string()), part2.to_string()));
    }

    Ok((None, ident.to_string()))
}

fn parse_property(r: &mut CharacterReader<'_>) -> ParseResult<(State, PropertyPathSyntax)> {
    r.skip_whitespace();
    let prop = r.parse_identifier();
    if prop.is_empty() {
        return Err(ExpressionParseException::new(
            r.position(),
            "Unable to parse property name",
        ));
    }
    Ok((
        State::AfterProperty,
        PropertyPathSyntax::Property {
            name: prop.to_string(),
        },
    ))
}

fn try_parse_casts(
    r: &mut CharacterReader<'_>,
) -> ParseResult<Option<(State, PropertyPathSyntax)>> {
    if r.take_if_keyword(":=") {
        Ok(Some(parse_ensure_type(r)?))
    } else if r.take_if_keyword(":>") || r.take_if_keyword("as ") {
        Ok(Some(parse_cast_type(r)?))
    } else {
        Ok(None)
    }
}

fn parse_after_property(
    r: &mut CharacterReader<'_>,
) -> ParseResult<(State, Option<PropertyPathSyntax>)> {
    if let Some((state, syntax)) = try_parse_casts(r)? {
        return Ok((state, Some(syntax)));
    }

    r.skip_whitespace();
    let Some(c) = r.peek() else {
        return Ok((State::End, None));
    };
    if r.take_if('.') {
        return Ok((State::Next, Some(PropertyPathSyntax::ChildTraversal)));
    }

    Err(ExpressionParseException::new(
        r.position(),
        format!("Unexpected character {c} after property name"),
    ))
}

fn parse_ensure_type(r: &mut CharacterReader<'_>) -> ParseResult<(State, PropertyPathSyntax)> {
    r.skip_whitespace();
    let (ns, name) = parse_xaml_identifier(r)?;
    Ok((
        State::AfterProperty,
        PropertyPathSyntax::EnsureType {
            type_name: name,
            type_namespace: ns,
        },
    ))
}

fn parse_cast_type(r: &mut CharacterReader<'_>) -> ParseResult<(State, PropertyPathSyntax)> {
    r.skip_whitespace();
    let (ns, name) = parse_xaml_identifier(r)?;
    Ok((
        State::AfterProperty,
        PropertyPathSyntax::CastType {
            type_name: name,
            type_namespace: ns,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(s: &str, expected: &[PropertyPathSyntax]) {
        let parsed = PropertyPathGrammar::parse(s).unwrap();
        assert_eq!(parsed, expected);
    }

    fn error(s: &str) -> String {
        PropertyPathGrammar::parse(s).unwrap_err().message().to_string()
    }

    fn property(name: &str) -> PropertyPathSyntax {
        PropertyPathSyntax::Property {
            name: name.to_string(),
        }
    }

    fn qualified(name: &str, type_name: &str, ns: Option<&str>) -> PropertyPathSyntax {
        PropertyPathSyntax::TypeQualifiedProperty {
            name: name.to_string(),
            type_name: type_name.to_string(),
            type_namespace: ns.map(str::to_string),
        }
    }

    fn cast(type_name: &str, ns: Option<&str>) -> PropertyPathSyntax {
        PropertyPathSyntax::CastType {
            type_name: type_name.to_string(),
            type_namespace: ns.map(str::to_string),
        }
    }

    fn ensure(type_name: &str, ns: Option<&str>) -> PropertyPathSyntax {
        PropertyPathSyntax::EnsureType {
            type_name: type_name.to_string(),
            type_namespace: ns.map(str::to_string),
        }
    }

    #[test]
    fn property_path_should_support_simple_properties() {
        check("SomeProperty", &[property("SomeProperty")]);
    }

    #[test]
    fn property_path_should_ignore_trailing_whitespace() {
        check("  SomeProperty   ", &[property("SomeProperty")]);
    }

    #[test]
    fn property_path_should_support_qualified_properties() {
        check(
            " ( somens:SomeType.SomeProperty ) ",
            &[qualified("SomeProperty", "SomeType", Some("somens"))],
        );
    }

    #[test]
    fn property_path_should_support_property_paths() {
        check(
            " ( somens:SomeType.SomeProperty ).Child . SubChild ",
            &[
                qualified("SomeProperty", "SomeType", Some("somens")),
                PropertyPathSyntax::ChildTraversal,
                property("Child"),
                PropertyPathSyntax::ChildTraversal,
                property("SubChild"),
            ],
        );
    }

    #[test]
    fn property_path_should_support_casts() {
        check(
            " ( somens:SomeType.SomeProperty ) :> SomeType.Child as somens:SomeType . SubChild ",
            &[
                qualified("SomeProperty", "SomeType", Some("somens")),
                cast("SomeType", None),
                PropertyPathSyntax::ChildTraversal,
                property("Child"),
                cast("SomeType", Some("somens")),
                PropertyPathSyntax::ChildTraversal,
                property("SubChild"),
            ],
        );
    }

    #[test]
    fn property_path_should_support_ensure_type() {
        check(
            " ( somens:SomeType.SomeProperty ) := SomeType.Child := somens:SomeType . SubChild ",
            &[
                qualified("SomeProperty", "SomeType", Some("somens")),
                ensure("SomeType", None),
                PropertyPathSyntax::ChildTraversal,
                property("Child"),
                ensure("SomeType", Some("somens")),
                PropertyPathSyntax::ChildTraversal,
                property("SubChild"),
            ],
        );
    }

    // Tests specific to this port.

    #[test]
    fn unqualified_type_has_no_namespace() {
        check("(SomeType.SomeProperty)", &[qualified("SomeProperty", "SomeType", None)]);
    }

    #[test]
    fn trailing_traversal_is_accepted() {
        // As upstream: the path may end after a '.'.
        check("Foo.", &[property("Foo"), PropertyPathSyntax::ChildTraversal]);
    }

    #[test]
    fn empty_path_fails() {
        assert_eq!(error(""), "Unable to parse property name");
        assert_eq!(error("   "), "Unable to parse property name");
        assert_eq!(error("1Foo"), "Unable to parse property name");
    }

    #[test]
    fn unexpected_character_after_property_fails() {
        assert_eq!(error("Foo Bar"), "Unexpected character B after property name");
        assert_eq!(error("Foo)"), "Unexpected character ) after property name");
    }

    #[test]
    fn qualified_property_errors() {
        const ERROR: &str = "Unable to parse qualified property name, expected `(ns:TypeName.PropertyName)` or `(TypeName.PropertyName)` after `(`";
        assert_eq!(error("("), "Expected identifier");
        assert_eq!(error("(ns:"), "Expected the rest of the identifier after ns:");
        assert_eq!(error("(Type"), ERROR);
        assert_eq!(error("(Type)"), ERROR);
        assert_eq!(error("(Type.)"), ERROR);
        assert_eq!(
            error("(Type.Prop"),
            "Expected ')' after qualified property name :Type.Prop"
        );
        assert_eq!(
            error("(ns:Type.Prop.Other)"),
            "Expected ')' after qualified property name ns:Type.Prop"
        );
    }

    #[test]
    fn cast_errors() {
        assert_eq!(error("Foo :> "), "Expected identifier");
        assert_eq!(error("Foo := 1"), "Expected identifier");
        assert_eq!(error("Foo as ns:"), "Expected the rest of the identifier after ns:");
    }

    #[test]
    fn error_column_is_reported() {
        assert_eq!(PropertyPathGrammar::parse("Foo.1").unwrap_err().column(), 4);
    }
}
