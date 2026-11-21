//! The grammar of binding expression paths (`Foo.Bar[0]`, `#name.Text`,
//! `$parent[Border].Tag`, `!Foo`, `((ns:Type)Foo).Bar`, `Foo^`, ...).

use crate::ExpressionParseError;
use crate::character_reader::CharacterReader;
use crate::span_helpers::{try_parse_int, NumberStyles};

/// The kind of source a parsed binding expression starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SourceMode {
    Data,
    Control,
}

/// A node of a parsed binding expression.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// The empty expression (`.`): the source itself.
    EmptyExpression,
    /// A property access; `accepts_null` is set for a null-conditional access (`?.`).
    PropertyName {
        accepts_null: bool,
        property_name: String,
    },
    /// An attached property access (`(Owner.Property)` or `(ns:Owner.Property)`).
    AttachedPropertyName {
        accepts_null: bool,
        namespace: String,
        type_name: String,
        property_name: String,
    },
    /// An indexer access (`[a, b]`).
    Indexer { arguments: Vec<String> },
    /// Logical negation (`!`). The only transform node.
    Not,
    /// The stream operator (`^`).
    Stream,
    /// The `$self` source.
    SelfNode,
    /// A named element source (`#name`).
    Name { name: String },
    /// An ancestor source (`$parent`, `$parent[Type]`, `$parent[1]`, `$parent[Type;1]`).
    Ancestor {
        namespace: Option<String>,
        type_name: Option<String>,
        level: i32,
    },
    /// A type cast (`(Type)` or `((ns:Type)Member)`).
    TypeCast { namespace: String, type_name: String },
}

impl Node {
    /// Gets a value indicating whether the node transforms the value of the
    /// rest of the expression instead of selecting a member.
    pub fn is_transform_node(&self) -> bool {
        matches!(self, Node::Not)
    }
}

/// Parser of binding expression paths.
pub struct BindingExpressionGrammar;

impl BindingExpressionGrammar {
    /// Parses `text` into a list of nodes and the source mode.
    pub fn parse(text: &str) -> Result<(Vec<Node>, SourceMode), ExpressionParseError> {
        let mut r = CharacterReader::new(text);
        Self::parse_reader(&mut r)
    }

    /// Parses the remaining input of `r` into a list of nodes and the source mode.
    pub fn parse_reader(
        r: &mut CharacterReader<'_>,
    ) -> Result<(Vec<Node>, SourceMode), ExpressionParseError> {
        let mut result = Vec::new();
        let mode = parse(r, &mut result)?;
        Ok((result, mode))
    }
}

type ParseResult<T> = Result<T, ExpressionParseError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Start,
    RelativeSource,
    ElementName,
    AfterMember,
    BeforeMember,
    BeforeMemberNullable,
    AttachedProperty,
    AttachedPropertyNullable,
    Indexer,
    TypeCast,
    End,
}

struct TypeName<'a> {
    namespace: &'a str,
    type_name: &'a str,
}

fn parse(r: &mut CharacterReader<'_>, nodes: &mut Vec<Node>) -> ParseResult<SourceMode> {
    let mut state = State::Start;
    let mut mode = SourceMode::Data;

    while !r.end() && state != State::End {
        match state {
            State::Start => state = parse_start(r, nodes),
            State::AfterMember => state = parse_after_member(r, nodes),
            State::BeforeMember => state = parse_before_member(r, nodes, false),
            State::BeforeMemberNullable => state = parse_before_member(r, nodes, true),
            State::AttachedProperty => state = parse_attached_property(r, nodes, false)?,
            State::AttachedPropertyNullable => state = parse_attached_property(r, nodes, true)?,
            State::Indexer => state = parse_indexer(r, nodes)?,
            State::TypeCast => state = parse_type_cast(r, nodes)?,
            State::ElementName => {
                state = parse_element_name(r, nodes)?;
                mode = SourceMode::Control;
            }
            State::RelativeSource => {
                state = parse_relative_source(r, nodes)?;
                mode = SourceMode::Control;
            }
            State::End => {}
        }
    }

    if !r.end() {
        return Err(ExpressionParseError::new(
            r.position(),
            "Expected end of expression.",
        ));
    }

    if matches!(state, State::BeforeMember | State::BeforeMemberNullable) {
        return Err(ExpressionParseError::new(
            r.position(),
            "Unexpected end of expression.",
        ));
    }

    Ok(mode)
}

fn parse_start(r: &mut CharacterReader<'_>, nodes: &mut Vec<Node>) -> State {
    if parse_not(r) {
        nodes.push(Node::Not);
        return State::Start;
    } else if parse_sharp(r) {
        return State::ElementName;
    } else if parse_dollar_sign(r) {
        return State::RelativeSource;
    } else if parse_open_brace(r) {
        if peek_open_brace(r) {
            return State::TypeCast;
        }

        return State::AttachedProperty;
    } else if peek_open_bracket(r) {
        return State::Indexer;
    } else if parse_dot(r) {
        nodes.push(Node::EmptyExpression);
        return State::AfterMember;
    } else if parse_stream_operator(r) {
        nodes.push(Node::EmptyExpression);
        nodes.push(Node::Stream);
        return State::AfterMember;
    } else {
        let identifier = r.parse_identifier();

        if !identifier.is_empty() {
            nodes.push(Node::PropertyName {
                accepts_null: false,
                property_name: identifier.to_string(),
            });
            return State::AfterMember;
        }
    }

    State::End
}

fn parse_after_member(r: &mut CharacterReader<'_>, nodes: &mut Vec<Node>) -> State {
    if let Some(accepts_null) = parse_member_accessor(r) {
        return if accepts_null {
            State::BeforeMemberNullable
        } else {
            State::BeforeMember
        };
    } else if parse_stream_operator(r) {
        nodes.push(Node::Stream);
        return State::AfterMember;
    } else if peek_open_bracket(r) {
        return State::Indexer;
    } else if parse_open_brace(r) {
        return State::TypeCast;
    }

    State::End
}

fn parse_before_member(
    r: &mut CharacterReader<'_>,
    nodes: &mut Vec<Node>,
    accepts_null: bool,
) -> State {
    if parse_open_brace(r) {
        if peek_open_brace(r) {
            return State::TypeCast;
        }

        if accepts_null {
            State::AttachedPropertyNullable
        } else {
            State::AttachedProperty
        }
    } else {
        let identifier = r.parse_identifier();

        if !identifier.is_empty() {
            nodes.push(Node::PropertyName {
                accepts_null,
                property_name: identifier.to_string(),
            });
            return State::AfterMember;
        }

        State::End
    }
}

fn parse_attached_property(
    r: &mut CharacterReader<'_>,
    nodes: &mut Vec<Node>,
    accepts_null: bool,
) -> ParseResult<State> {
    let TypeName {
        namespace: ns,
        type_name: owner,
    } = parse_type_name(r);

    if !r.end() && r.take_if(')') {
        nodes.push(Node::TypeCast {
            namespace: ns.to_string(),
            type_name: owner.to_string(),
        });
        return Ok(State::AfterMember);
    }

    if r.end() || !r.take_if('.') {
        return Err(ExpressionParseError::new(
            r.position(),
            "Invalid attached property name.",
        ));
    }

    let name = r.parse_identifier();

    if name.is_empty() {
        return Err(ExpressionParseError::new(
            r.position(),
            "Attached Property name expected after '.'.",
        ));
    }

    if r.end() || !r.take_if(')') {
        return Err(ExpressionParseError::new(r.position(), "Expected ')'."));
    }

    nodes.push(Node::AttachedPropertyName {
        accepts_null,
        namespace: ns.to_string(),
        type_name: owner.to_string(),
        property_name: name.to_string(),
    });
    Ok(State::AfterMember)
}

fn parse_indexer(r: &mut CharacterReader<'_>, nodes: &mut Vec<Node>) -> ParseResult<State> {
    let args = r.parse_arguments('[', ']')?;

    if args.is_empty() {
        return Err(ExpressionParseError::new(
            r.position(),
            "Indexer may not be empty.",
        ));
    }

    nodes.push(Node::Indexer { arguments: args });
    Ok(State::AfterMember)
}

fn parse_type_cast(r: &mut CharacterReader<'_>, nodes: &mut Vec<Node>) -> ParseResult<State> {
    let parse_member_before_add_cast = parse_open_brace(r);

    let TypeName {
        namespace: ns,
        type_name,
    } = parse_type_name(r);

    let mut result = State::AfterMember;

    if parse_member_before_add_cast {
        if !parse_close_brace(r) {
            return Err(ExpressionParseError::new(r.position(), "Expected ')'."));
        }

        result = parse_before_member(r, nodes, false);
        if result == State::AttachedProperty {
            result = parse_attached_property(r, nodes, false)?;
        }

        if r.peek() == Some('[') {
            result = parse_indexer(r, nodes)?;
        }
    }

    nodes.push(Node::TypeCast {
        namespace: ns.to_string(),
        type_name: type_name.to_string(),
    });

    if r.end() || !r.take_if(')') {
        return Err(ExpressionParseError::new(r.position(), "Expected ')'."));
    }

    Ok(result)
}

fn parse_element_name(r: &mut CharacterReader<'_>, nodes: &mut Vec<Node>) -> ParseResult<State> {
    let name = r.parse_identifier();

    if name.is_empty() {
        return Err(ExpressionParseError::new(
            r.position(),
            "Element name expected after '#'.",
        ));
    }

    nodes.push(Node::Name {
        name: name.to_string(),
    });
    Ok(State::AfterMember)
}

fn parse_relative_source(
    r: &mut CharacterReader<'_>,
    nodes: &mut Vec<Node>,
) -> ParseResult<State> {
    let mode = r.parse_identifier();

    if mode == "self" {
        nodes.push(Node::SelfNode);
    } else if mode == "parent" {
        let mut ancestor_namespace: Option<String> = None;
        let mut ancestor_type: Option<String> = None;
        let mut ancestor_level = 0;
        if peek_open_bracket(r) {
            let args = r.parse_arguments_with_delimiter('[', ']', ';')?;
            if args.len() > 2 || args.is_empty() {
                return Err(ExpressionParseError::new(
                    r.position(),
                    "Too many arguments in RelativeSource syntax sugar",
                ));
            } else if args.len() == 1 {
                if let Some(level) = try_parse_int(&args[0], NumberStyles::INTEGER) {
                    ancestor_type = None;
                    ancestor_level = level;
                } else {
                    let mut reader = CharacterReader::new(&args[0]);
                    let name = parse_type_name(&mut reader);
                    ancestor_namespace = Some(name.namespace.to_string());
                    ancestor_type = Some(name.type_name.to_string());
                }
            } else {
                let mut reader = CharacterReader::new(&args[0]);
                let name = parse_type_name(&mut reader);
                ancestor_namespace = Some(name.namespace.to_string());
                ancestor_type = Some(name.type_name.to_string());
                ancestor_level = match try_parse_int(&args[1], NumberStyles::INTEGER) {
                    Some(level) => level,
                    None => {
                        return Err(ExpressionParseError::new(
                            r.position(),
                            format!(
                                "The input string '{}' was not in a correct format.",
                                args[1]
                            ),
                        ));
                    }
                };
            }
        }
        nodes.push(Node::Ancestor {
            namespace: ancestor_namespace,
            type_name: ancestor_type,
            level: ancestor_level,
        });
    } else {
        return Err(ExpressionParseError::new(
            r.position(),
            "Unknown RelativeSource mode.",
        ));
    }

    Ok(State::AfterMember)
}

fn parse_type_name<'a>(r: &mut CharacterReader<'a>) -> TypeName<'a> {
    let type_name_or_namespace = r.parse_type_identifier();

    if !r.end() && r.take_if(':') {
        TypeName {
            namespace: type_name_or_namespace,
            type_name: r.parse_type_identifier(),
        }
    } else {
        TypeName {
            namespace: "",
            type_name: type_name_or_namespace,
        }
    }
}

fn parse_not(r: &mut CharacterReader<'_>) -> bool {
    !r.end() && r.take_if('!')
}

/// Returns `Some(accepts_null)` when a member accessor (`.` or `?.`) was consumed.
fn parse_member_accessor(r: &mut CharacterReader<'_>) -> Option<bool> {
    if r.end() {
        return None;
    }

    if r.take_if('.') {
        return Some(false);
    }

    if r.take_if_str("?.") {
        return Some(true);
    }

    None
}

fn parse_open_brace(r: &mut CharacterReader<'_>) -> bool {
    !r.end() && r.take_if('(')
}

fn parse_close_brace(r: &mut CharacterReader<'_>) -> bool {
    !r.end() && r.take_if(')')
}

fn peek_open_bracket(r: &CharacterReader<'_>) -> bool {
    !r.end() && r.peek() == Some('[')
}

fn peek_open_brace(r: &CharacterReader<'_>) -> bool {
    !r.end() && r.peek() == Some('(')
}

fn parse_stream_operator(r: &mut CharacterReader<'_>) -> bool {
    !r.end() && r.take_if('^')
}

fn parse_dollar_sign(r: &mut CharacterReader<'_>) -> bool {
    !r.end() && r.take_if('$')
}

fn parse_sharp(r: &mut CharacterReader<'_>) -> bool {
    !r.end() && r.take_if('#')
}

fn parse_dot(r: &mut CharacterReader<'_>) -> bool {
    !r.end() && r.take_if('.')
}

#[cfg(test)]
mod tests;
