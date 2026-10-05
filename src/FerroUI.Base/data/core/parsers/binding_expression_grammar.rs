//! The grammar of binding expression paths (`Foo.Bar[0]`, `#name.Text`,
//! `$parent[Border].Tag`, `!Foo`, `((ns:Type)Foo).Bar`, `Foo^`, ...).

use crate::data::core::ExpressionParseException;
use crate::utilities::character_reader::CharacterReader;
use crate::utilities::span_helpers::{try_parse_int, NumberStyles};

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
    pub fn parse(text: &str) -> Result<(Vec<Node>, SourceMode), ExpressionParseException> {
        let mut r = CharacterReader::new(text);
        Self::parse_reader(&mut r)
    }

    /// Parses the remaining input of `r` into a list of nodes and the source mode.
    pub fn parse_reader(
        r: &mut CharacterReader<'_>,
    ) -> Result<(Vec<Node>, SourceMode), ExpressionParseException> {
        let mut result = Vec::new();
        let mode = parse(r, &mut result)?;
        Ok((result, mode))
    }
}

type ParseResult<T> = Result<T, ExpressionParseException>;

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
        return Err(ExpressionParseException::new(
            r.position(),
            "Expected end of expression.",
        ));
    }

    if matches!(state, State::BeforeMember | State::BeforeMemberNullable) {
        return Err(ExpressionParseException::new(
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
        return Err(ExpressionParseException::new(
            r.position(),
            "Invalid attached property name.",
        ));
    }

    let name = r.parse_identifier();

    if name.is_empty() {
        return Err(ExpressionParseException::new(
            r.position(),
            "Attached Property name expected after '.'.",
        ));
    }

    if r.end() || !r.take_if(')') {
        return Err(ExpressionParseException::new(r.position(), "Expected ')'."));
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
        return Err(ExpressionParseException::new(
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
            return Err(ExpressionParseException::new(r.position(), "Expected ')'."));
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
        return Err(ExpressionParseException::new(r.position(), "Expected ')'."));
    }

    Ok(result)
}

fn parse_element_name(r: &mut CharacterReader<'_>, nodes: &mut Vec<Node>) -> ParseResult<State> {
    let name = r.parse_identifier();

    if name.is_empty() {
        return Err(ExpressionParseException::new(
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
                return Err(ExpressionParseException::new(
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
                        return Err(ExpressionParseException::new(
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
        return Err(ExpressionParseException::new(
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
mod tests {
    use super::*;

    fn parse(s: &str) -> Vec<Node> {
        let mut r = CharacterReader::new(s);
        BindingExpressionGrammar::parse_reader(&mut r).unwrap().0
    }

    fn parse_err(s: &str) -> ExpressionParseException {
        BindingExpressionGrammar::parse(s).unwrap_err()
    }

    fn assert_error(s: &str, column: i32, message: &str) {
        let e = parse_err(s);
        assert_eq!((e.column(), e.message()), (column, message), "input: {s}");
    }

    fn assert_is_property(node: &Node, name: &str) {
        assert_is_property_nullable(node, name, false);
    }

    fn assert_is_property_nullable(node: &Node, name: &str, accepts_null: bool) {
        assert_eq!(
            node,
            &Node::PropertyName {
                accepts_null,
                property_name: name.to_string()
            }
        );
    }

    fn assert_is_attached_property(node: &Node, type_name: &str, name: &str) {
        assert_is_attached_property_nullable(node, type_name, name, false);
    }

    fn assert_is_attached_property_nullable(
        node: &Node,
        expected_type_name: &str,
        name: &str,
        expected_accepts_null: bool,
    ) {
        match node {
            Node::AttachedPropertyName {
                accepts_null,
                type_name,
                property_name,
                ..
            } => {
                assert_eq!(type_name, expected_type_name);
                assert_eq!(property_name, name);
                assert_eq!(*accepts_null, expected_accepts_null);
            }
            other => panic!("expected an attached property, got {other:?}"),
        }
    }

    fn assert_is_indexer(node: &Node, args: &[&str]) {
        match node {
            Node::Indexer { arguments } => assert_eq!(arguments, args),
            other => panic!("expected an indexer, got {other:?}"),
        }
    }

    fn type_cast(namespace: &str, type_name: &str) -> Node {
        Node::TypeCast {
            namespace: namespace.to_string(),
            type_name: type_name.to_string(),
        }
    }

    // BindingExpressionGrammarTests

    #[test]
    fn should_parse_single_property() {
        let result = parse("Foo");
        assert_eq!(result.len(), 1);
        assert_is_property(&result[0], "Foo");
    }

    #[test]
    fn should_parse_underscored_property() {
        let result = parse("_Foo");
        assert_eq!(result.len(), 1);
        assert_is_property(&result[0], "_Foo");
    }

    #[test]
    fn should_parse_property_with_digits() {
        let result = parse("F0o");
        assert_eq!(result.len(), 1);
        assert_is_property(&result[0], "F0o");
    }

    #[test]
    fn should_parse_dot() {
        let result = parse(".");
        assert_eq!(result, [Node::EmptyExpression]);
    }

    #[test]
    fn should_parse_single_attached_property() {
        let result = parse("(Foo.Bar)");
        assert_eq!(result.len(), 1);
        assert_is_attached_property(&result[0], "Foo", "Bar");
    }

    #[test]
    fn should_parse_property_chain() {
        let result = parse("Foo.Bar.Baz");

        assert_eq!(result.len(), 3);
        assert_is_property(&result[0], "Foo");
        assert_is_property(&result[1], "Bar");
        assert_is_property(&result[2], "Baz");
    }

    #[test]
    fn should_parse_property_chain_with_attached_property_1() {
        let result = parse("(Foo.Bar).Baz");

        assert_eq!(result.len(), 2);
        assert_is_attached_property(&result[0], "Foo", "Bar");
        assert_is_property(&result[1], "Baz");
    }

    #[test]
    fn should_parse_property_chain_with_attached_property_2() {
        let result = parse("Foo.(Bar.Baz)");

        assert_eq!(result.len(), 2);
        assert_is_property(&result[0], "Foo");
        assert_is_attached_property(&result[1], "Bar", "Baz");
    }

    #[test]
    fn should_parse_property_chain_with_attached_property_3() {
        let result = parse("Foo.(Bar.Baz).Last");

        assert_eq!(result.len(), 3);
        assert_is_property(&result[0], "Foo");
        assert_is_attached_property(&result[1], "Bar", "Baz");
        assert_is_property(&result[2], "Last");
    }

    #[test]
    fn should_parse_null_conditional_in_property_chain_1() {
        let result = parse("Foo?.Bar.Baz");

        assert_eq!(result.len(), 3);
        assert_is_property(&result[0], "Foo");
        assert_is_property_nullable(&result[1], "Bar", true);
        assert_is_property(&result[2], "Baz");
    }

    #[test]
    fn should_parse_null_conditional_in_property_chain_2() {
        let result = parse("Foo.Bar?.Baz");

        assert_eq!(result.len(), 3);
        assert_is_property(&result[0], "Foo");
        assert_is_property(&result[1], "Bar");
        assert_is_property_nullable(&result[2], "Baz", true);
    }

    #[test]
    fn should_parse_null_conditional_in_property_chain_3() {
        let result = parse("Foo?.(Bar.Baz)");

        assert_eq!(result.len(), 2);
        assert_is_property(&result[0], "Foo");
        assert_is_attached_property_nullable(&result[1], "Bar", "Baz", true);
    }

    #[test]
    fn should_parse_negated_property_chain() {
        let result = parse("!Foo.Bar.Baz");

        assert_eq!(result.len(), 4);
        assert_eq!(result[0], Node::Not);
        assert_is_property(&result[1], "Foo");
        assert_is_property(&result[2], "Bar");
        assert_is_property(&result[3], "Baz");
    }

    #[test]
    fn should_parse_double_negated_property_chain() {
        let result = parse("!!Foo.Bar.Baz");

        assert_eq!(result.len(), 5);
        assert_eq!(result[0], Node::Not);
        assert_eq!(result[1], Node::Not);
        assert_is_property(&result[2], "Foo");
        assert_is_property(&result[3], "Bar");
        assert_is_property(&result[4], "Baz");
    }

    #[test]
    fn should_parse_indexed_property() {
        let result = parse("Foo[15]");

        assert_eq!(result.len(), 2);
        assert_is_property(&result[0], "Foo");
        assert_is_indexer(&result[1], &["15"]);
    }

    #[test]
    fn should_parse_indexed_property_string_index() {
        let result = parse("Foo[Key]");

        assert_eq!(result.len(), 2);
        assert_is_property(&result[0], "Foo");
        assert_is_indexer(&result[1], &["Key"]);
    }

    #[test]
    fn should_parse_multiple_indexed_property() {
        let result = parse("Foo[15,6]");

        assert_eq!(result.len(), 2);
        assert_is_property(&result[0], "Foo");
        assert_is_indexer(&result[1], &["15", "6"]);
    }

    #[test]
    fn should_parse_multiple_indexed_property_with_space() {
        let result = parse("Foo[5, 16]");

        assert_eq!(result.len(), 2);
        assert_is_property(&result[0], "Foo");
        assert_is_indexer(&result[1], &["5", "16"]);
    }

    #[test]
    fn should_parse_consecutive_indexers() {
        let result = parse("Foo[15][16]");

        assert_eq!(result.len(), 3);
        assert_is_property(&result[0], "Foo");
        assert_is_indexer(&result[1], &["15"]);
        assert_is_indexer(&result[2], &["16"]);
    }

    #[test]
    fn should_parse_indexed_property_in_chain() {
        let result = parse("Foo.Bar[5, 6].Baz");

        assert_eq!(result.len(), 4);
        assert_is_property(&result[0], "Foo");
        assert_is_property(&result[1], "Bar");
        assert_is_indexer(&result[2], &["5", "6"]);
        assert_is_property(&result[3], "Baz");
    }

    #[test]
    fn should_parse_stream_node() {
        let result = parse("Foo^");

        assert_eq!(result.len(), 2);
        assert_eq!(result[1], Node::Stream);
    }

    #[test]
    fn should_parse_stream_node_on_empty_expression() {
        let result = parse("^");

        assert_eq!(result, [Node::EmptyExpression, Node::Stream]);
    }

    #[test]
    fn should_parse_stream_node_after_dot() {
        let result = parse(".^");

        assert_eq!(result, [Node::EmptyExpression, Node::Stream]);
    }

    #[test]
    fn should_parse_cast_to_nested_type() {
        let result = parse("((ns:Outer+Inner)Foo).Bar");

        assert_eq!(result[1], type_cast("ns", "Outer+Inner"));
    }

    #[test]
    fn should_parse_cast_to_non_nested_type() {
        let result = parse("((ns:Outer)Foo).Bar");

        assert_eq!(result[1], type_cast("ns", "Outer"));
    }

    // BindingExpressionGrammarTests_Errors

    #[test]
    fn identifier_cannot_start_with_digit() {
        assert_error("1Foo", 0, "Expected end of expression.");
    }

    #[test]
    fn identifier_cannot_start_with_symbol() {
        assert_error("Foo.%Bar", 4, "Expected end of expression.");
    }

    #[test]
    fn identifier_cannot_start_with_question_mark() {
        assert_error("?Foo", 0, "Expected end of expression.");
    }

    #[test]
    fn identifier_cannot_start_with_null_conditional() {
        assert_error("?.Foo", 0, "Expected end of expression.");
    }

    #[test]
    fn expression_cannot_end_with_period() {
        assert_error("Foo.Bar.", 8, "Unexpected end of expression.");
    }

    #[test]
    fn expression_cannot_end_with_question_mark() {
        assert_error("Foo.Bar?", 7, "Expected end of expression.");
    }

    #[test]
    fn expression_cannot_end_with_null_conditional() {
        assert_error("Foo.Bar?.", 9, "Unexpected end of expression.");
    }

    #[test]
    fn expression_cannot_start_with_period_then_token() {
        assert_error(".Bar", 1, "Expected end of expression.");
    }

    #[test]
    fn expression_cannot_have_empty_indexer() {
        assert_error("Foo.Bar[]", 8, "Expected indexer argument.");
    }

    #[test]
    fn expression_cannot_have_extra_comma_at_start_of_indexer() {
        assert_error("Foo.Bar[,3,4]", 8, "Expected indexer argument.");
    }

    #[test]
    fn expression_cannot_have_extra_comma_in_indexer() {
        assert_error("Foo.Bar[3,,4]", 10, "Expected indexer argument.");
    }

    #[test]
    fn expression_cannot_have_extra_comma_at_end_of_indexer() {
        assert_error("Foo.Bar[3,4,]", 12, "Expected indexer argument.");
    }

    #[test]
    fn expression_cannot_have_digit_after_indexer() {
        assert_error("Foo.Bar[3,4]5", 12, "Expected end of expression.");
    }

    #[test]
    fn expression_cannot_have_letter_after_indexer() {
        assert_error("Foo.Bar[3,4]A", 12, "Expected end of expression.");
    }

    // Tests specific to this port: node kinds and error paths the upstream
    // suite does not cover.

    fn parse_with_mode(s: &str) -> (Vec<Node>, SourceMode) {
        BindingExpressionGrammar::parse(s).unwrap()
    }

    fn property(name: &str) -> Node {
        Node::PropertyName {
            accepts_null: false,
            property_name: name.to_string(),
        }
    }

    #[test]
    fn empty_string_parses_to_no_nodes() {
        assert_eq!(parse_with_mode(""), (vec![], SourceMode::Data));
    }

    #[test]
    fn data_source_mode_for_property_paths() {
        assert_eq!(parse_with_mode("Foo.Bar").1, SourceMode::Data);
    }

    #[test]
    fn should_parse_element_name() {
        assert_eq!(
            parse_with_mode("#foo.Bar"),
            (
                vec![
                    Node::Name {
                        name: "foo".to_string()
                    },
                    property("Bar")
                ],
                SourceMode::Control
            )
        );
    }

    #[test]
    fn should_parse_negated_element_name() {
        let (nodes, mode) = parse_with_mode("!#foo");
        assert_eq!(
            nodes,
            [
                Node::Not,
                Node::Name {
                    name: "foo".to_string()
                }
            ]
        );
        assert_eq!(mode, SourceMode::Control);
        assert!(nodes[0].is_transform_node());
        assert!(!nodes[1].is_transform_node());
    }

    #[test]
    fn should_parse_self() {
        assert_eq!(
            parse_with_mode("$self.Foo"),
            (vec![Node::SelfNode, property("Foo")], SourceMode::Control)
        );
    }

    #[test]
    fn should_parse_parent() {
        assert_eq!(
            parse_with_mode("$parent.Foo"),
            (
                vec![
                    Node::Ancestor {
                        namespace: None,
                        type_name: None,
                        level: 0
                    },
                    property("Foo")
                ],
                SourceMode::Control
            )
        );
    }

    #[test]
    fn should_parse_parent_with_level() {
        assert_eq!(
            parse("$parent[2]"),
            [Node::Ancestor {
                namespace: None,
                type_name: None,
                level: 2
            }]
        );
    }

    #[test]
    fn should_parse_parent_with_type() {
        assert_eq!(
            parse("$parent[Border].Tag"),
            [
                Node::Ancestor {
                    namespace: Some(String::new()),
                    type_name: Some("Border".to_string()),
                    level: 0
                },
                property("Tag")
            ]
        );
    }

    #[test]
    fn should_parse_parent_with_namespaced_type_and_level() {
        assert_eq!(
            parse("$parent[local:Border; 1]"),
            [Node::Ancestor {
                namespace: Some("local".to_string()),
                type_name: Some("Border".to_string()),
                level: 1
            }]
        );
    }

    #[test]
    fn should_parse_namespaced_attached_property() {
        assert_eq!(
            parse("(local:Foo.Bar)"),
            [Node::AttachedPropertyName {
                accepts_null: false,
                namespace: "local".to_string(),
                type_name: "Foo".to_string(),
                property_name: "Bar".to_string()
            }]
        );
    }

    #[test]
    fn should_parse_type_cast_of_source() {
        assert_eq!(parse("(ns:Foo)"), [type_cast("ns", "Foo")]);
        assert_eq!(
            parse("(Foo).Bar"),
            [type_cast("", "Foo"), property("Bar")]
        );
    }

    #[test]
    fn should_parse_type_cast_after_member() {
        assert_eq!(
            parse("Foo(ns:Bar).Baz"),
            [property("Foo"), type_cast("ns", "Bar"), property("Baz")]
        );
        assert_eq!(
            parse("Foo.Bar((Baz)Qux)"),
            [
                property("Foo"),
                property("Bar"),
                property("Qux"),
                type_cast("", "Baz")
            ]
        );
    }

    #[test]
    fn should_parse_type_cast_of_member_in_chain() {
        assert_eq!(
            parse("Foo.((ns:Type)Bar).Baz"),
            [
                property("Foo"),
                property("Bar"),
                type_cast("ns", "Type"),
                property("Baz")
            ]
        );
    }

    #[test]
    fn should_parse_type_cast_of_attached_property_and_indexer() {
        assert_eq!(
            parse("((Type)(Owner.Prop))"),
            [
                Node::AttachedPropertyName {
                    accepts_null: false,
                    namespace: String::new(),
                    type_name: "Owner".to_string(),
                    property_name: "Prop".to_string()
                },
                type_cast("", "Type")
            ]
        );
        assert_eq!(
            parse("((Type)Foo[1])"),
            [
                property("Foo"),
                Node::Indexer {
                    arguments: vec!["1".to_string()]
                },
                type_cast("", "Type")
            ]
        );
    }

    #[test]
    fn should_parse_leading_indexer() {
        let result = parse("[0].Foo");
        assert_is_indexer(&result[0], &["0"]);
        assert_is_property(&result[1], "Foo");
    }

    #[test]
    fn should_parse_stream_in_chain() {
        assert_eq!(
            parse("Foo^.Bar^"),
            [property("Foo"), Node::Stream, property("Bar"), Node::Stream]
        );
    }

    #[test]
    fn should_parse_unicode_identifiers() {
        assert_eq!(parse("Żółw.Imię"), [property("Żółw"), property("Imię")]);
    }

    #[test]
    fn unterminated_prefixes_parse_to_no_nodes() {
        // As upstream: the loop stops at the end of the input before the
        // pending state is processed, and only a pending member is an error.
        assert_eq!(parse_with_mode("#"), (vec![], SourceMode::Data));
        assert_eq!(parse_with_mode("$"), (vec![], SourceMode::Data));
        assert_eq!(parse_with_mode("("), (vec![], SourceMode::Data));
        assert_eq!(parse_with_mode("!"), (vec![Node::Not], SourceMode::Data));
    }

    #[test]
    fn element_name_errors() {
        assert_error("#1", 1, "Element name expected after '#'.");
    }

    #[test]
    fn relative_source_errors() {
        assert_error("$foo", 4, "Unknown RelativeSource mode.");
        assert_error("$1", 1, "Unknown RelativeSource mode.");
        assert_error(
            "$parent[A;1;2]",
            14,
            "Too many arguments in RelativeSource syntax sugar",
        );
        assert_error(
            "$parent[A;B]",
            12,
            "The input string 'B' was not in a correct format.",
        );
        assert_error("$parent[A 1]", 11, "Expected ';'.");
        assert_error("$parent[]", 8, "Expected indexer argument.");
    }

    #[test]
    fn attached_property_errors() {
        assert_error("(Foo", 4, "Invalid attached property name.");
        assert_error("(Foo:", 5, "Invalid attached property name.");
        assert_error("(Foo-Bar)", 4, "Invalid attached property name.");
        assert_error("(Foo.)", 5, "Attached Property name expected after '.'.");
        assert_error("(Foo.", 5, "Attached Property name expected after '.'.");
        assert_error("(Foo.Bar", 8, "Expected ')'.");
        assert_error("(Foo.Bar.Baz)", 8, "Expected ')'.");
        assert_error("Foo?.(Bar", 9, "Invalid attached property name.");
    }

    #[test]
    fn type_cast_errors() {
        assert_error("((Foo", 5, "Expected ')'.");
        assert_error("((Foo)", 6, "Expected ')'.");
        assert_error("((Foo)Bar", 9, "Expected ')'.");
        assert_error("((Foo)Bar]", 9, "Expected ')'.");
        assert_error("Foo(Bar", 7, "Expected ')'.");
        assert_error("((Foo)Bar[])", 10, "Expected indexer argument.");
    }

    #[test]
    fn indexer_errors() {
        assert_error("Foo[1", 5, "Expected ','.");
        assert_error("Foo[1,", 6, "Expected ']'.");
        assert_error("Foo[1 2]", 7, "Expected ','.");
    }

    #[test]
    fn trailing_garbage_is_an_error() {
        assert_error("Foo Bar", 3, "Expected end of expression.");
        assert_error("Foo)", 3, "Expected end of expression.");
        assert_error("Foo!", 3, "Expected end of expression.");
    }
}
