//! The grammar of style selectors (`Button.foo > :is(Control)#name /template/ ^[Foo=bar]`).

use crate::character_reader::{is_digit, is_white_space, CharacterReader};
use crate::span_helpers::{try_parse_int, NumberStyles};
use crate::ExpressionParseError;
use std::ops::Range;

mod spans;
pub use spans::SelectorOperand;
use spans::SourceRanges;

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

/// The name's byte range in a `#name` test, excluding the hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorName {
    pub range: Range<usize>,
    /// This name belongs to a template subtree after `/template/`.
    pub in_template: bool,
    /// The named control is an ancestor of the selector's subject.
    pub ancestor: bool,
}

impl SelectorGrammar {
    /// Parses `s` into a list of syntax elements.
    pub fn parse(s: &str) -> Result<Vec<SelectorSyntax>, ExpressionParseError> {
        let mut r = CharacterReader::new(s);
        parse(&mut r, None, false, &mut SourceRanges::new(s))
    }

    /// Collect name tests using the same grammar as runtime style application.
    pub fn name_ranges(
        s: &str,
    ) -> Result<(Vec<SelectorSyntax>, Vec<SelectorName>), ExpressionParseError> {
        let mut ranges = SourceRanges::new(s);
        ranges.names = Some(Vec::new());
        let syntax = parse(&mut CharacterReader::new(s), None, false, &mut ranges)?;
        Ok((syntax, ranges.names.unwrap()))
    }

    /// Collect selector type and property operands in lexical order, including inside `:not`.
    /// Consume one operand for each corresponding syntax node during a depth-first walk.
    pub fn operand_ranges(
        s: &str,
    ) -> Result<(Vec<SelectorSyntax>, Vec<SelectorOperand>), ExpressionParseError> {
        let mut ranges = SourceRanges::new(s);
        ranges.operands = Some(Vec::new());
        let syntax = parse(&mut CharacterReader::new(s), None, false, &mut ranges)?;
        Ok((syntax, ranges.operands.unwrap()))
    }
}

type ParseResult<T> = Result<T, ExpressionParseError>;

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

fn parse(
    r: &mut CharacterReader<'_>,
    end: Option<char>,
    starts_in_template: bool,
    ranges: &mut SourceRanges<'_>,
) -> ParseResult<Vec<SelectorSyntax>> {
    let mut state = State::Start;
    let mut selector = Vec::new();
    let mut in_template = starts_in_template;
    let mut subject_start = ranges.names.as_ref().map_or(0, Vec::len);
    while !r.end() && state != State::End {
        let syntax;
        (state, syntax) = match state {
            State::Start => parse_start(r),
            State::Middle => parse_middle(r, end),
            State::CanHaveType => (parse_can_have_type(r), None),
            State::Colon => parse_colon(r, in_template, ranges)?,
            State::Class => parse_class(r)?,
            State::Traversal => parse_traversal(r),
            State::TypeName => parse_type_name(r, ranges)?,
            State::Property => parse_property(r, ranges)?,
            State::Template => parse_template(r)?,
            State::Name => parse_name(r, |name| ranges.record_name(name, in_template))?,
            State::AttachedProperty => parse_attached_property(r, ranges)?,
            State::End => (State::End, None),
        };
        if let Some(syntax) = syntax {
            let combinator = matches!(
                syntax,
                SelectorSyntax::Template | SelectorSyntax::Child | SelectorSyntax::Descendant
            );
            if let Some(names) = &mut ranges.names {
                if combinator {
                    for name in &mut names[subject_start..] {
                        name.ancestor = true;
                    }
                }
                if combinator || matches!(syntax, SelectorSyntax::Comma) {
                    subject_start = names.len();
                }
                if matches!(syntax, SelectorSyntax::Template | SelectorSyntax::Comma) {
                    in_template = starts_in_template || matches!(syntax, SelectorSyntax::Template);
                }
            }
            selector.push(syntax);
        }
    }

    if state != State::Start
        && state != State::Middle
        && state != State::End
        && state != State::CanHaveType
    {
        return Err(ExpressionParseError::new(
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

fn parse_colon(
    r: &mut CharacterReader<'_>,
    in_template: bool,
    ranges: &mut SourceRanges<'_>,
) -> ParseResult<(State, Option<SelectorSyntax>)> {
    let identifier = r.parse_style_class();

    if identifier.is_empty() {
        return Err(ExpressionParseError::new(
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
        ranges.record_type(type_name);

        return Ok((
            State::CanHaveType,
            Some(SelectorSyntax::Is {
                type_name: type_name.into(),
                xmlns: xmlns.into(),
            }),
        ));
    }
    if identifier == NOT_KEYWORD && r.take_if('(') {
        let argument = parse(r, Some(')'), in_template, ranges)?;
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
        return Err(ExpressionParseError::new(
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
        return Err(ExpressionParseError::new(
            r.position(),
            format!("Expected 'template', got '{template}'"),
        ));
    } else if !r.take_if('/') {
        return Err(ExpressionParseError::new(r.position(), "Expected '/'"));
    }
    Ok((State::Start, Some(SelectorSyntax::Template)))
}

fn parse_name<'a>(
    r: &mut CharacterReader<'a>,
    name_range: impl FnOnce(&'a str),
) -> ParseResult<(State, Option<SelectorSyntax>)> {
    let name = r.parse_identifier();
    if name.is_empty() {
        return Err(ExpressionParseError::new(
            r.position(),
            "Expected a name after '#'.",
        ));
    }
    name_range(name);
    Ok((
        State::CanHaveType,
        Some(SelectorSyntax::Name {
            name: name.to_string(),
        }),
    ))
}

fn parse_type_name(
    r: &mut CharacterReader<'_>,
    ranges: &mut SourceRanges<'_>,
) -> ParseResult<(State, Option<SelectorSyntax>)> {
    let (xmlns, type_name) = parse_type(r)?;
    ranges.record_type(type_name);
    Ok((
        State::CanHaveType,
        Some(SelectorSyntax::OfType {
            type_name: type_name.into(),
            xmlns: xmlns.into(),
        }),
    ))
}

fn parse_property(
    r: &mut CharacterReader<'_>,
    ranges: &mut SourceRanges<'_>,
) -> ParseResult<(State, Option<SelectorSyntax>)> {
    let property = identifier_at(r);

    if r.take_if('(') {
        return Ok((State::AttachedProperty, None));
    } else if !r.take_if('=') {
        return Err(ExpressionParseError::new(
            r.position(),
            format!("Expected '=', got '{}'", peek_text(r)),
        ));
    }

    let value = r.take_until(']');

    take_close_bracket(r)?;
    ranges.record_property(property);

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
    ranges: &mut SourceRanges<'_>,
) -> ParseResult<(State, Option<SelectorSyntax>)> {
    let (xmlns, type_name) = parse_type(r)?;
    if !r.take_if('.') {
        return Err(ExpressionParseError::new(
            r.position(),
            format!("Expected '.', got '{}'", peek_text(r)),
        ));
    }
    let property = r.parse_identifier();
    if property.is_empty() {
        return Err(ExpressionParseError::new(
            r.position(),
            format!("Expected Attached Property Name, got '{}'", peek_text(r)),
        ));
    }

    if !r.take_if(')') {
        return Err(ExpressionParseError::new(
            r.position(),
            format!("Expected ')', got '{}'", peek_text(r)),
        ));
    }

    if !r.take_if('=') {
        return Err(ExpressionParseError::new(
            r.position(),
            format!("Expected '=', got '{}'", peek_text(r)),
        ));
    }

    let value = r.take_until(']');

    take_close_bracket(r)?;
    ranges.record_attached(type_name, property);

    let state = if r.end() { State::End } else { State::Middle };
    Ok((
        state,
        Some(SelectorSyntax::AttachedProperty {
            xmlns: xmlns.into(),
            type_name: type_name.into(),
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
        None => Err(ExpressionParseError::new(
            r.position(),
            "Expected ']', got end of selector.",
        )),
    }
}

/// Parses `Type` or `ns|Type`, returning `(xmlns, type_name)`.
fn parse_type<'a>(r: &mut CharacterReader<'a>) -> ParseResult<(&'a str, &'a str)> {
    let mut ns = "";
    let namespace_or_type_name = r.parse_identifier();

    if namespace_or_type_name.is_empty() {
        return Err(ExpressionParseError::new(
            r.position(),
            format!("Expected an identifier, got '{}", peek_text(r)),
        ));
    }

    let type_name = if !r.end() && r.take_if('|') {
        ns = namespace_or_type_name;
        if r.end() {
            return Err(ExpressionParseError::new(
                r.position(),
                "Unexpected end of selector.",
            ));
        }
        identifier_at(r)
    } else {
        namespace_or_type_name
    };

    Ok((ns, type_name))
}

fn identifier_at<'a>(r: &mut CharacterReader<'a>) -> &'a str {
    let word = r.parse_identifier();
    // The existing grammar can yield an empty identifier before runtime validation.
    if word.is_empty() {
        r.try_peek(0)
    } else {
        word
    }
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
            return Err(ExpressionParseError::new(
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
            return Err(ExpressionParseError::new(
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
            return Err(ExpressionParseError::new(
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
                return Err(ExpressionParseError::new(
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
                        return Err(ExpressionParseError::new(
                            r.position(),
                            "Couldn't parse nth-child sign. '+' or '-' was expected.",
                        ));
                    }
                };

                r.skip_whitespace();

                offset = match try_parse_int(r.take_until(')'), NumberStyles::INTEGER) {
                    Some(value) => value,
                    None => {
                        return Err(ExpressionParseError::new(
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
        Err(ExpressionParseError::new(
            r.position(),
            format!("Expected '{c}', got end of selector."),
        ))
    } else if !r.take_if(')') {
        Err(ExpressionParseError::new(
            r.position(),
            format!("Expected '{c}', got '{}'.", peek_text(r)),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
