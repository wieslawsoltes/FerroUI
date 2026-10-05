//! The grammar of style selectors (`Button.foo > :is(Control)#name /template/ ^[Foo=bar]`).

use ferroui_base::data::core::ExpressionParseException;
use ferroui_base::utilities::character_reader::{is_digit, is_white_space};
use ferroui_base::utilities::span_helpers::{try_parse_int, NumberStyles};
use ferroui_base::utilities::CharacterReader;

/// A syntax element of a parsed selector.
#[derive(Clone, Debug, PartialEq)]
pub enum SelectorSyntax {
    /// Matches a type exactly (`Button`, `ns|Button`).
    OfType { type_name: String, xmlns: String },
    /// Matches an attached property value (`[(ns|Owner.Property)=value]`).
    AttachedProperty {
        xmlns: String,
        type_name: String,
        property: String,
        value: String,
    },
    /// Matches a type or a type derived from it (`:is(Button)`).
    Is { type_name: String, xmlns: String },
    /// Matches a style class (`.foo`) or a pseudo-class (`:foo`, stored with the colon).
    Class { class: String },
    /// Matches a control name (`#foo`).
    Name { name: String },
    /// Matches a property value (`[Property=value]`).
    Property { property: String, value: String },
    /// The child combinator (`>`).
    Child,
    /// The descendant combinator (white space).
    Descendant,
    /// The template combinator (`/template/`).
    Template,
    /// Negation (`:not(...)`).
    Not { argument: Vec<SelectorSyntax> },
    /// `:nth-child(step n + offset)`.
    NthChild { offset: i32, step: i32 },
    /// `:nth-last-child(step n + offset)`.
    NthLastChild { offset: i32, step: i32 },
    /// Separator of alternatives (`,`).
    Comma,
    /// The nesting selector (`^`).
    Nesting,
}

/// Parser of style selectors.
pub struct SelectorGrammar;

impl SelectorGrammar {
    /// Parses `s` into a list of syntax elements.
    pub fn parse(s: &str) -> Result<Vec<SelectorSyntax>, ExpressionParseException> {
        let mut r = CharacterReader::new(s);
        parse(&mut r, None)
    }
}

type ParseResult<T> = Result<T, ExpressionParseException>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Start,
    Middle,
    Colon,
    Class,
    Name,
    CanHaveType,
    Traversal,
    TypeName,
    Property,
    AttachedProperty,
    Template,
    End,
}

/// The next character formatted for an error message; empty at the end of the input.
fn peek_text(r: &CharacterReader<'_>) -> String {
    r.peek().map(String::from).unwrap_or_default()
}

fn parse(r: &mut CharacterReader<'_>, end: Option<char>) -> ParseResult<Vec<SelectorSyntax>> {
    let mut state = State::Start;
    let mut selector = Vec::new();
    while !r.end() && state != State::End {
        let syntax;
        (state, syntax) = match state {
            State::Start => parse_start(r),
            State::Middle => parse_middle(r, end),
            State::CanHaveType => (parse_can_have_type(r), None),
            State::Colon => parse_colon(r)?,
            State::Class => parse_class(r)?,
            State::Traversal => parse_traversal(r),
            State::TypeName => parse_type_name(r)?,
            State::Property => parse_property(r)?,
            State::Template => parse_template(r)?,
            State::Name => parse_name(r)?,
            State::AttachedProperty => parse_attached_property(r)?,
            State::End => (State::End, None),
        };
        if let Some(syntax) = syntax {
            selector.push(syntax);
        }
    }

    if state != State::Start
        && state != State::Middle
        && state != State::End
        && state != State::CanHaveType
    {
        return Err(ExpressionParseException::new(
            r.position(),
            "Unexpected end of selector",
        ));
    }

    Ok(selector)
}

fn parse_start(r: &mut CharacterReader<'_>) -> (State, Option<SelectorSyntax>) {
    r.skip_whitespace();
    if r.end() {
        return (State::End, None);
    }

    if r.take_if(':') {
        return (State::Colon, None);
    } else if r.take_if('.') {
        return (State::Class, None);
    } else if r.take_if('#') {
        return (State::Name, None);
    } else if r.take_if('^') {
        return (State::CanHaveType, Some(SelectorSyntax::Nesting));
    }
    (State::TypeName, None)
}

fn parse_middle(r: &mut CharacterReader<'_>, end: Option<char>) -> (State, Option<SelectorSyntax>) {
    if r.take_if(':') {
        return (State::Colon, None);
    } else if r.take_if('.') {
        return (State::Class, None);
    } else if r.take_if_fn(is_white_space) || r.peek() == Some('>') {
        return (State::Traversal, None);
    } else if r.take_if('/') {
        return (State::Template, None);
    } else if r.take_if('#') {
        return (State::Name, None);
    } else if r.take_if(',') {
        return (State::Start, Some(SelectorSyntax::Comma));
    } else if r.take_if('^') {
        return (State::CanHaveType, Some(SelectorSyntax::Nesting));
    } else if end.is_some() && !r.end() && r.peek() == end {
        return (State::End, None);
    }
    (State::TypeName, None)
}

fn parse_can_have_type(r: &mut CharacterReader<'_>) -> State {
    if r.take_if('[') {
        return State::Property;
    }
    State::Middle
}

fn parse_colon(r: &mut CharacterReader<'_>) -> ParseResult<(State, Option<SelectorSyntax>)> {
    let identifier = r.parse_style_class();

    if identifier.is_empty() {
        return Err(ExpressionParseException::new(
            r.position(),
            "Expected class name, is, nth-child or nth-last-child selector after ':'.",
        ));
    }

    const IS_KEYWORD: &str = "is";
    const NOT_KEYWORD: &str = "not";
    const NTH_CHILD_KEYWORD: &str = "nth-child";
    const NTH_LAST_CHILD_KEYWORD: &str = "nth-last-child";

    if identifier == IS_KEYWORD && r.take_if('(') {
        let (xmlns, type_name) = parse_type(r)?;
        expect(r, ')')?;

        return Ok((
            State::CanHaveType,
            Some(SelectorSyntax::Is { type_name, xmlns }),
        ));
    }
    if identifier == NOT_KEYWORD && r.take_if('(') {
        let argument = parse(r, Some(')'))?;
        expect(r, ')')?;

        return Ok((State::Middle, Some(SelectorSyntax::Not { argument })));
    }
    if identifier == NTH_CHILD_KEYWORD && r.take_if('(') {
        let (step, offset) = parse_nth_child_arguments(r)?;

        return Ok((
            State::Middle,
            Some(SelectorSyntax::NthChild { step, offset }),
        ));
    }
    if identifier == NTH_LAST_CHILD_KEYWORD && r.take_if('(') {
        let (step, offset) = parse_nth_child_arguments(r)?;

        Ok((
            State::Middle,
            Some(SelectorSyntax::NthLastChild { step, offset }),
        ))
    } else {
        Ok((
            State::CanHaveType,
            Some(SelectorSyntax::Class {
                class: format!(":{identifier}"),
            }),
        ))
    }
}

fn parse_traversal(r: &mut CharacterReader<'_>) -> (State, Option<SelectorSyntax>) {
    r.skip_whitespace();
    if r.take_if('>') {
        r.skip_whitespace();
        (State::Middle, Some(SelectorSyntax::Child))
    } else if r.take_if('/') {
        (State::Template, None)
    } else if !r.end() {
        (State::Middle, Some(SelectorSyntax::Descendant))
    } else {
        (State::End, None)
    }
}

fn parse_class(r: &mut CharacterReader<'_>) -> ParseResult<(State, Option<SelectorSyntax>)> {
    let class = r.parse_style_class();
    if class.is_empty() {
        return Err(ExpressionParseException::new(
            r.position(),
            "Expected a class name after '.'.",
        ));
    }

    Ok((
        State::CanHaveType,
        Some(SelectorSyntax::Class {
            class: class.to_string(),
        }),
    ))
}

fn parse_template(r: &mut CharacterReader<'_>) -> ParseResult<(State, Option<SelectorSyntax>)> {
    let template = r.parse_identifier();
    const TEMPLATE_KEYWORD: &str = "template";
    if template != TEMPLATE_KEYWORD {
        return Err(ExpressionParseException::new(
            r.position(),
            format!("Expected 'template', got '{template}'"),
        ));
    } else if !r.take_if('/') {
        return Err(ExpressionParseException::new(r.position(), "Expected '/'"));
    }
    Ok((State::Start, Some(SelectorSyntax::Template)))
}

fn parse_name(r: &mut CharacterReader<'_>) -> ParseResult<(State, Option<SelectorSyntax>)> {
    let name = r.parse_identifier();
    if name.is_empty() {
        return Err(ExpressionParseException::new(
            r.position(),
            "Expected a name after '#'.",
        ));
    }
    Ok((
        State::CanHaveType,
        Some(SelectorSyntax::Name {
            name: name.to_string(),
        }),
    ))
}

fn parse_type_name(r: &mut CharacterReader<'_>) -> ParseResult<(State, Option<SelectorSyntax>)> {
    let (xmlns, type_name) = parse_type(r)?;
    Ok((
        State::CanHaveType,
        Some(SelectorSyntax::OfType { type_name, xmlns }),
    ))
}

fn parse_property(r: &mut CharacterReader<'_>) -> ParseResult<(State, Option<SelectorSyntax>)> {
    let property = r.parse_identifier();

    if r.take_if('(') {
        return Ok((State::AttachedProperty, None));
    } else if !r.take_if('=') {
        return Err(ExpressionParseException::new(
            r.position(),
            format!("Expected '=', got '{}'", peek_text(r)),
        ));
    }

    let value = r.take_until(']');

    take_close_bracket(r)?;

    Ok((
        State::CanHaveType,
        Some(SelectorSyntax::Property {
            property: property.to_string(),
            value: value.to_string(),
        }),
    ))
}

fn parse_attached_property(
    r: &mut CharacterReader<'_>,
) -> ParseResult<(State, Option<SelectorSyntax>)> {
    let (xmlns, type_name) = parse_type(r)?;
    if !r.take_if('.') {
        return Err(ExpressionParseException::new(
            r.position(),
            format!("Expected '.', got '{}'", peek_text(r)),
        ));
    }
    let property = r.parse_identifier();
    if property.is_empty() {
        return Err(ExpressionParseException::new(
            r.position(),
            format!("Expected Attached Property Name, got '{}'", peek_text(r)),
        ));
    }

    if !r.take_if(')') {
        return Err(ExpressionParseException::new(
            r.position(),
            format!("Expected ')', got '{}'", peek_text(r)),
        ));
    }

    if !r.take_if('=') {
        return Err(ExpressionParseException::new(
            r.position(),
            format!("Expected '=', got '{}'", peek_text(r)),
        ));
    }

    let value = r.take_until(']');

    take_close_bracket(r)?;

    let state = if r.end() { State::End } else { State::Middle };
    Ok((
        state,
        Some(SelectorSyntax::AttachedProperty {
            xmlns,
            type_name,
            property: property.to_string(),
            value: value.to_string(),
        }),
    ))
}

/// Consumes the `]` that ends a property value. Upstream takes the character
/// unconditionally and fails with an index-out-of-range error when the
/// selector ends before it.
fn take_close_bracket(r: &mut CharacterReader<'_>) -> ParseResult<()> {
    match r.take() {
        Some(_) => Ok(()),
        None => Err(ExpressionParseException::new(
            r.position(),
            "Expected ']', got end of selector.",
        )),
    }
}

/// Parses `Type` or `ns|Type`, returning `(xmlns, type_name)`.
fn parse_type(r: &mut CharacterReader<'_>) -> ParseResult<(String, String)> {
    let mut ns = "";
    let type_name;
    let namespace_or_type_name = r.parse_identifier();

    if namespace_or_type_name.is_empty() {
        return Err(ExpressionParseException::new(
            r.position(),
            format!("Expected an identifier, got '{}", peek_text(r)),
        ));
    }

    if !r.end() && r.take_if('|') {
        ns = namespace_or_type_name;
        if r.end() {
            return Err(ExpressionParseException::new(
                r.position(),
                "Unexpected end of selector.",
            ));
        }
        type_name = r.parse_identifier();
    } else {
        type_name = namespace_or_type_name;
    }

    Ok((ns.to_string(), type_name.to_string()))
}

/// Parses the arguments of `:nth-child(`/`:nth-last-child(` including the
/// closing parenthesis, returning `(step, offset)`.
fn parse_nth_child_arguments(r: &mut CharacterReader<'_>) -> ParseResult<(i32, i32)> {
    let step;
    let mut offset = 0;

    if r.peek() == Some('o') {
        let const_arg = r.take_until(')').trim_matches(is_white_space);
        if const_arg == "odd" {
            step = 2;
            offset = 1;
        } else {
            return Err(ExpressionParseException::new(
                r.position(),
                format!("Expected nth-child(odd). Actual '{const_arg}'."),
            ));
        }
    } else if r.peek() == Some('e') {
        let const_arg = r.take_until(')').trim_matches(is_white_space);
        if const_arg == "even" {
            step = 2;
            offset = 0;
        } else {
            return Err(ExpressionParseException::new(
                r.position(),
                format!("Expected nth-child(even). Actual '{const_arg}'."),
            ));
        }
    } else {
        r.skip_whitespace();

        let step_or_offset;
        let step_or_offset_str = r.take_while(|c| is_digit(c) || c == '-' || c == '+');
        if step_or_offset_str.is_empty() || step_or_offset_str == "+" {
            step_or_offset = 1;
        } else if step_or_offset_str == "-" {
            step_or_offset = -1;
        } else if let Some(value) = try_parse_int(step_or_offset_str, NumberStyles::INTEGER) {
            step_or_offset = value;
        } else {
            return Err(ExpressionParseException::new(
                r.position(),
                "Couldn't parse nth-child step or offset value. Integer was expected.",
            ));
        }

        r.skip_whitespace();

        if r.peek() == Some(')') {
            step = 0;
            offset = step_or_offset;
        } else {
            step = step_or_offset;

            if r.peek() != Some('n') {
                return Err(ExpressionParseException::new(
                    r.position(),
                    "Couldn't parse nth-child step value, \"xn+y\" pattern was expected.",
                ));
            }

            r.skip(1); // skip 'n'

            r.skip_whitespace();

            if r.peek() != Some(')') {
                let sign = match r.take() {
                    Some('+') => 1,
                    Some('-') => -1,
                    _ => {
                        return Err(ExpressionParseException::new(
                            r.position(),
                            "Couldn't parse nth-child sign. '+' or '-' was expected.",
                        ));
                    }
                };

                r.skip_whitespace();

                offset = match try_parse_int(r.take_until(')'), NumberStyles::INTEGER) {
                    Some(value) => value,
                    None => {
                        return Err(ExpressionParseException::new(
                            r.position(),
                            "Couldn't parse nth-child offset value. Integer was expected.",
                        ));
                    }
                };

                offset = offset.wrapping_mul(sign);
            }
        }
    }

    expect(r, ')')?;

    Ok((step, offset))
}

fn expect(r: &mut CharacterReader<'_>, c: char) -> ParseResult<()> {
    if r.end() {
        Err(ExpressionParseException::new(
            r.position(),
            format!("Expected '{c}', got end of selector."),
        ))
    } else if !r.take_if(')') {
        Err(ExpressionParseException::new(
            r.position(),
            format!("Expected '{c}', got '{}'.", peek_text(r)),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Vec<SelectorSyntax> {
        SelectorGrammar::parse(s).unwrap()
    }

    fn assert_fails(s: &str) {
        assert!(SelectorGrammar::parse(s).is_err(), "input: {s}");
    }

    fn error(s: &str) -> String {
        SelectorGrammar::parse(s).unwrap_err().message().to_string()
    }

    fn of_type(type_name: &str) -> SelectorSyntax {
        SelectorSyntax::OfType {
            type_name: type_name.to_string(),
            xmlns: String::new(),
        }
    }

    fn is(type_name: &str) -> SelectorSyntax {
        SelectorSyntax::Is {
            type_name: type_name.to_string(),
            xmlns: String::new(),
        }
    }

    fn class(class: &str) -> SelectorSyntax {
        SelectorSyntax::Class {
            class: class.to_string(),
        }
    }

    fn name(name: &str) -> SelectorSyntax {
        SelectorSyntax::Name {
            name: name.to_string(),
        }
    }

    fn property(property: &str, value: &str) -> SelectorSyntax {
        SelectorSyntax::Property {
            property: property.to_string(),
            value: value.to_string(),
        }
    }

    #[test]
    fn of_type_() {
        assert_eq!(parse("Button"), [of_type("Button")]);
    }

    #[test]
    fn namespaced_of_type() {
        assert_eq!(
            parse("x|Button"),
            [SelectorSyntax::OfType {
                type_name: "Button".to_string(),
                xmlns: "x".to_string()
            }]
        );
    }

    #[test]
    fn name_() {
        assert_eq!(parse("#foo"), [name("foo")]);
    }

    #[test]
    fn of_type_name() {
        assert_eq!(parse("Button#foo"), [of_type("Button"), name("foo")]);
    }

    #[test]
    fn is_() {
        assert_eq!(parse(":is(Button)"), [is("Button")]);
    }

    #[test]
    fn is_name() {
        assert_eq!(parse(":is(Button)#foo"), [is("Button"), name("foo")]);
    }

    #[test]
    fn namespaced_is_name() {
        assert_eq!(
            parse(":is(x|Button)#foo"),
            [
                SelectorSyntax::Is {
                    type_name: "Button".to_string(),
                    xmlns: "x".to_string()
                },
                name("foo")
            ]
        );
    }

    #[test]
    fn class_() {
        assert_eq!(parse(".foo"), [class("foo")]);
    }

    #[test]
    fn pseudoclass() {
        assert_eq!(parse(":foo"), [class(":foo")]);
    }

    #[test]
    fn of_type_class() {
        assert_eq!(parse("Button.foo"), [of_type("Button"), class("foo")]);
    }

    #[test]
    fn of_type_child_class() {
        assert_eq!(
            parse("Button > .foo"),
            [of_type("Button"), SelectorSyntax::Child, class("foo")]
        );
    }

    #[test]
    fn of_type_child_class_no_spaces() {
        assert_eq!(
            parse("Button>.foo"),
            [of_type("Button"), SelectorSyntax::Child, class("foo")]
        );
    }

    #[test]
    fn of_type_descendant_class() {
        assert_eq!(
            parse("Button .foo"),
            [of_type("Button"), SelectorSyntax::Descendant, class("foo")]
        );
    }

    #[test]
    fn of_type_template_class() {
        assert_eq!(
            parse("Button /template/ .foo"),
            [of_type("Button"), SelectorSyntax::Template, class("foo")]
        );
    }

    #[test]
    fn of_type_property() {
        assert_eq!(
            parse("Button[Foo=bar]"),
            [of_type("Button"), property("Foo", "bar")]
        );
    }

    #[test]
    fn of_type_attached_property() {
        assert_eq!(
            parse("Button[(Grid.Column)=1]"),
            [
                of_type("Button"),
                SelectorSyntax::AttachedProperty {
                    xmlns: String::new(),
                    type_name: "Grid".to_string(),
                    property: "Column".to_string(),
                    value: "1".to_string()
                }
            ]
        );
    }

    #[test]
    fn of_type_attached_property_with_namespace() {
        assert_eq!(
            parse("Button[(x|Grid.Column)=1]"),
            [
                of_type("Button"),
                SelectorSyntax::AttachedProperty {
                    xmlns: "x".to_string(),
                    type_name: "Grid".to_string(),
                    property: "Column".to_string(),
                    value: "1".to_string()
                }
            ]
        );
    }

    #[test]
    fn not_of_type() {
        assert_eq!(
            parse(":not(Button)"),
            [SelectorSyntax::Not {
                argument: vec![of_type("Button")]
            }]
        );
    }

    #[test]
    fn of_type_not_class() {
        assert_eq!(
            parse("Button:not(.foo)"),
            [
                of_type("Button"),
                SelectorSyntax::Not {
                    argument: vec![class("foo")]
                }
            ]
        );
    }

    #[test]
    fn nth_child_invalid_inputs() {
        for input in [
            ":nth-child(xn+2)",
            ":nth-child(2n+b)",
            ":nth-child(2n+)",
            ":nth-child(2na)",
            ":nth-child(2x+1)",
        ] {
            assert_fails(input);
        }
    }

    const NTH_VARIATIONS: [(&str, i32, i32); 12] = [
        ("(+1)", 0, 1),
        ("(1)", 0, 1),
        ("(-1)", 0, -1),
        ("(2n+1)", 2, 1),
        ("(n)", 1, 0),
        ("(+n)", 1, 0),
        ("(-n)", -1, 0),
        ("(-2n)", -2, 0),
        ("(n+5)", 1, 5),
        ("(n-5)", 1, -5),
        ("( 2n + 1 )", 2, 1),
        ("( 2n - 1 )", 2, -1),
    ];

    #[test]
    fn nth_child_variations() {
        for (arguments, step, offset) in NTH_VARIATIONS {
            let input = format!(":nth-child{arguments}");
            assert_eq!(
                parse(&input),
                [SelectorSyntax::NthChild { step, offset }],
                "input: {input}"
            );
        }
    }

    #[test]
    fn nth_last_child_variations() {
        for (arguments, step, offset) in NTH_VARIATIONS {
            let input = format!(":nth-last-child{arguments}");
            assert_eq!(
                parse(&input),
                [SelectorSyntax::NthLastChild { step, offset }],
                "input: {input}"
            );
        }
    }

    #[test]
    fn of_type_nth_child() {
        assert_eq!(
            parse("Button:nth-child(2n+1)"),
            [
                of_type("Button"),
                SelectorSyntax::NthChild { step: 2, offset: 1 }
            ]
        );
    }

    #[test]
    fn of_type_nth_child_without_offset() {
        assert_eq!(
            parse("Button:nth-child(2147483647n)"),
            [
                of_type("Button"),
                SelectorSyntax::NthChild {
                    step: i32::MAX,
                    offset: 0
                }
            ]
        );
    }

    #[test]
    fn of_type_nth_last_child() {
        assert_eq!(
            parse("Button:nth-last-child(2n+1)"),
            [
                of_type("Button"),
                SelectorSyntax::NthLastChild { step: 2, offset: 1 }
            ]
        );
    }

    #[test]
    fn of_type_nth_child_odd() {
        assert_eq!(
            parse("Button:nth-child(odd)"),
            [
                of_type("Button"),
                SelectorSyntax::NthChild { step: 2, offset: 1 }
            ]
        );
    }

    #[test]
    fn of_type_nth_child_even() {
        assert_eq!(
            parse("Button:nth-child(even)"),
            [
                of_type("Button"),
                SelectorSyntax::NthChild { step: 2, offset: 0 }
            ]
        );
    }

    #[test]
    fn is_descendent_not_of_type_class() {
        assert_eq!(
            parse(":is(Control) :not(Button.foo)"),
            [
                is("Control"),
                SelectorSyntax::Descendant,
                SelectorSyntax::Not {
                    argument: vec![of_type("Button"), class("foo")]
                }
            ]
        );
    }

    #[test]
    fn of_type_comma_is_class() {
        assert_eq!(
            parse("TextBlock, :is(Button).foo"),
            [
                of_type("TextBlock"),
                SelectorSyntax::Comma,
                is("Button"),
                class("foo")
            ]
        );
    }

    #[test]
    fn nesting_class() {
        assert_eq!(parse("^.foo"), [SelectorSyntax::Nesting, class("foo")]);
    }

    #[test]
    fn nesting_child_class() {
        assert_eq!(
            parse("^ > .foo"),
            [SelectorSyntax::Nesting, SelectorSyntax::Child, class("foo")]
        );
    }

    #[test]
    fn nesting_descendant_class() {
        assert_eq!(
            parse("^ .foo"),
            [
                SelectorSyntax::Nesting,
                SelectorSyntax::Descendant,
                class("foo")
            ]
        );
    }

    #[test]
    fn nesting_template_class() {
        assert_eq!(
            parse("^ /template/ .foo"),
            [
                SelectorSyntax::Nesting,
                SelectorSyntax::Template,
                class("foo")
            ]
        );
    }

    #[test]
    fn of_type_template_nesting() {
        assert_eq!(
            parse("Button /template/ ^"),
            [
                of_type("Button"),
                SelectorSyntax::Template,
                SelectorSyntax::Nesting
            ]
        );
    }

    #[test]
    fn nesting_property() {
        assert_eq!(
            parse("^[Foo=bar]"),
            [SelectorSyntax::Nesting, property("Foo", "bar")]
        );
    }

    #[test]
    fn not_nesting() {
        assert_eq!(
            parse(":not(^)"),
            [SelectorSyntax::Not {
                argument: vec![SelectorSyntax::Nesting]
            }]
        );
    }

    #[test]
    fn nesting_nth_child() {
        assert_eq!(
            parse("^:nth-child(2n+1)"),
            [
                SelectorSyntax::Nesting,
                SelectorSyntax::NthChild { step: 2, offset: 1 }
            ]
        );
    }

    #[test]
    fn nesting_comma_nesting_class() {
        assert_eq!(
            parse("^, ^.foo"),
            [
                SelectorSyntax::Nesting,
                SelectorSyntax::Comma,
                SelectorSyntax::Nesting,
                class("foo")
            ]
        );
    }

    #[test]
    fn namespace_alone_fails() {
        assert_fails("ns|");
    }

    #[test]
    fn dot_alone_fails() {
        assert_fails(". dot");
    }

    #[test]
    fn invalid_identifier_fails() {
        assert_fails("%foo");
    }

    #[test]
    fn invalid_class_fails() {
        assert_fails(".%foo");
    }

    #[test]
    fn not_without_argument_fails() {
        assert_fails(":not()");
    }

    #[test]
    fn not_without_closing_parenthesis_fails() {
        assert_fails(":not(Button");
    }

    // Tests specific to this port: error paths and end-of-input handling.

    #[test]
    fn empty_selector_parses_to_nothing() {
        assert_eq!(parse(""), []);
        assert_eq!(parse("   "), []);
    }

    #[test]
    fn template_without_spaces() {
        assert_eq!(
            parse("Button/template/Border"),
            [of_type("Button"), SelectorSyntax::Template, of_type("Border")]
        );
    }

    #[test]
    fn class_with_dash_and_pseudoclass_chain() {
        assert_eq!(
            parse("Button.foo-bar:pointerover"),
            [of_type("Button"), class("foo-bar"), class(":pointerover")]
        );
    }

    #[test]
    fn property_value_may_contain_anything_but_bracket() {
        assert_eq!(
            parse("Button[Tag=a b.c#d]"),
            [of_type("Button"), property("Tag", "a b.c#d")]
        );
    }

    #[test]
    fn attached_property_followed_by_class() {
        assert_eq!(
            parse("Button[(Grid.Column)=1].foo"),
            [
                of_type("Button"),
                SelectorSyntax::AttachedProperty {
                    xmlns: String::new(),
                    type_name: "Grid".to_string(),
                    property: "Column".to_string(),
                    value: "1".to_string()
                },
                class("foo")
            ]
        );
    }

    #[test]
    fn unexpected_end_of_selector() {
        assert_eq!(error("Button."), "Unexpected end of selector");
        assert_eq!(error("Button:"), "Unexpected end of selector");
        assert_eq!(error("#"), "Unexpected end of selector");
        assert_eq!(error("Button["), "Unexpected end of selector");
        assert_eq!(error("Button /"), "Unexpected end of selector");
        assert_eq!(error("Button[("), "Unexpected end of selector");
        // As upstream: a single trailing white space leaves a pending traversal.
        assert_eq!(error("Button "), "Unexpected end of selector");
    }

    #[test]
    fn colon_errors() {
        assert_eq!(
            error(":1"),
            "Expected class name, is, nth-child or nth-last-child selector after ':'."
        );
        assert_eq!(error(":is("), "Expected an identifier, got '");
        assert_eq!(error(":is(1)"), "Expected an identifier, got '1");
        assert_eq!(error(":is(Button"), "Expected ')', got end of selector.");
        assert_eq!(error(":is(Button.foo)"), "Expected ')', got '.'.");
        assert_eq!(error(":not(Button]"), "Expected an identifier, got ']");
    }

    #[test]
    fn keyword_without_parenthesis_is_a_pseudoclass() {
        assert_eq!(parse(":is"), [class(":is")]);
        assert_eq!(parse(":not.foo"), [class(":not"), class("foo")]);
        assert_eq!(parse(":nth-child"), [class(":nth-child")]);
    }

    #[test]
    fn name_and_template_errors() {
        assert_eq!(error("#1"), "Expected a name after '#'.");
        assert_eq!(error("Button /foo/"), "Expected 'template', got 'foo'");
        assert_eq!(error("Button /template"), "Expected '/'");
        assert_eq!(error("Button /template Border"), "Expected '/'");
    }

    #[test]
    fn property_errors() {
        assert_eq!(error("Button[Foo]"), "Expected '=', got ']'");
        assert_eq!(error("Button[Foo"), "Expected '=', got ''");
        assert_eq!(error("Button[Foo=bar"), "Expected ']', got end of selector.");
    }

    #[test]
    fn attached_property_errors() {
        assert_eq!(error("Button[(Grid)=1]"), "Expected '.', got ')'");
        assert_eq!(
            error("Button[(Grid.)=1]"),
            "Expected Attached Property Name, got ')'"
        );
        assert_eq!(error("Button[(Grid.Column=1]"), "Expected ')', got '='");
        assert_eq!(error("Button[(Grid.Column)1]"), "Expected '=', got '1'");
        assert_eq!(error("Button[(Grid.Column"), "Expected ')', got ''");
        assert_eq!(
            error("Button[(Grid.Column)=1"),
            "Expected ']', got end of selector."
        );
        assert_eq!(error("Button[(1.Column)=1]"), "Expected an identifier, got '1");
    }

    #[test]
    fn nth_child_errors() {
        assert_eq!(
            error(":nth-child(odd"),
            "Expected ')', got end of selector."
        );
        assert_eq!(
            error(":nth-child(other)"),
            "Expected nth-child(odd). Actual 'other'."
        );
        assert_eq!(
            error(":nth-child(eve)"),
            "Expected nth-child(even). Actual 'eve'."
        );
        assert_eq!(
            error(":nth-child(--1)"),
            "Couldn't parse nth-child step or offset value. Integer was expected."
        );
        assert_eq!(
            error(":nth-child("),
            "Couldn't parse nth-child step value, \"xn+y\" pattern was expected."
        );
        assert_eq!(
            error(":nth-child(2n"),
            "Couldn't parse nth-child sign. '+' or '-' was expected."
        );
        assert_eq!(
            error(":nth-child(2n*1)"),
            "Couldn't parse nth-child sign. '+' or '-' was expected."
        );
        assert_eq!(
            error(":nth-child(2n+1"),
            "Expected ')', got end of selector."
        );
        assert_eq!(
            error(":nth-child(99999999999)"),
            "Couldn't parse nth-child step or offset value. Integer was expected."
        );
    }

    #[test]
    fn error_column_is_reported() {
        assert_eq!(SelectorGrammar::parse("Button.%").unwrap_err().column(), 7);
    }
}
