//! Ported from the upstream `ExpressionNodeFactoryTests`.

use super::test_support::*;
use ferroui_base::data::core::expression_nodes::{
    ExpressionNode, ReflectionIndexerNode, LogicalNotNode, PropertyAccessorNode, StreamNode,
};
use std::any::Any;
use std::rc::Rc;

fn is<T: ExpressionNode>(node: &Rc<dyn ExpressionNode>) -> bool {
    let any: &dyn Any = &**node;
    any.is::<T>()
}

#[track_caller]
fn assert_is_property(node: &Rc<dyn ExpressionNode>, name: &str) {
    assert!(is::<PropertyAccessorNode>(node));
    let p = node.as_property_accessor_node().expect("a property accessor node");
    assert_eq!(p.property_name(), name);
}

#[track_caller]
fn assert_is_indexer(node: &Rc<dyn ExpressionNode>, args: &[&str]) {
    let any: &dyn Any = &**node;
    let e = any.downcast_ref::<ReflectionIndexerNode>().expect("an indexer node");
    assert_eq!(e.arguments(), args);
}

fn parse(path: &str) -> Vec<Rc<dyn ExpressionNode>> {
    parse_nodes(path, None).expect("a valid path")
}

#[test]
fn should_build_single_property() {
    let result = parse("Foo");

    assert_is_property(&result[0], "Foo");
}

#[test]
fn should_build_underscored_property() {
    let result = parse("_Foo");

    assert_is_property(&result[0], "_Foo");
}

#[test]
fn should_build_property_with_digits() {
    let result = parse("F0o");

    assert_is_property(&result[0], "F0o");
}

#[test]
fn should_build_dot() {
    let result = parse(".");

    // No nodes: the equivalent of the null result upstream.
    assert!(result.is_empty());
}

#[test]
fn should_build_property_chain() {
    let result = parse("Foo.Bar.Baz");

    assert_eq!(result.len(), 3);
    assert_is_property(&result[0], "Foo");
    assert_is_property(&result[1], "Bar");
    assert_is_property(&result[2], "Baz");
}

#[test]
fn should_build_negated_property_chain() {
    let result = parse("!Foo.Bar.Baz");

    assert_eq!(result.len(), 4);
    assert_is_property(&result[0], "Foo");
    assert_is_property(&result[1], "Bar");
    assert_is_property(&result[2], "Baz");
    assert!(is::<LogicalNotNode>(&result[3]));
}

#[test]
fn should_build_double_negated_property_chain() {
    let result = parse("!!Foo.Bar.Baz");

    assert_eq!(result.len(), 5);
    assert_is_property(&result[0], "Foo");
    assert_is_property(&result[1], "Bar");
    assert_is_property(&result[2], "Baz");
    assert!(is::<LogicalNotNode>(&result[3]));
    assert!(is::<LogicalNotNode>(&result[4]));
}

#[test]
fn should_build_indexed_property() {
    let result = parse("Foo[15]");

    assert_eq!(result.len(), 2);
    assert_is_property(&result[0], "Foo");
    assert_is_indexer(&result[1], &["15"]);
    assert!(is::<ReflectionIndexerNode>(&result[1]));
}

#[test]
fn should_build_indexed_property_string_index() {
    let result = parse("Foo[Key]");

    assert_eq!(result.len(), 2);
    assert_is_property(&result[0], "Foo");
    assert_is_indexer(&result[1], &["Key"]);
    assert!(is::<ReflectionIndexerNode>(&result[1]));
}

#[test]
fn should_build_multiple_indexed_property() {
    let result = parse("Foo[15,6]");

    assert_eq!(result.len(), 2);
    assert_is_property(&result[0], "Foo");
    assert_is_indexer(&result[1], &["15", "6"]);
}

#[test]
fn should_build_multiple_indexed_property_with_space() {
    let result = parse("Foo[5, 16]");

    assert_eq!(result.len(), 2);
    assert_is_property(&result[0], "Foo");
    assert_is_indexer(&result[1], &["5", "16"]);
}

#[test]
fn should_build_consecutive_indexers() {
    let result = parse("Foo[15][16]");

    assert_eq!(result.len(), 3);
    assert_is_property(&result[0], "Foo");
    assert_is_indexer(&result[1], &["15"]);
    assert_is_indexer(&result[2], &["16"]);
}

#[test]
fn should_build_indexed_property_in_chain() {
    let result = parse("Foo.Bar[5, 6].Baz");

    assert_eq!(result.len(), 4);
    assert_is_property(&result[0], "Foo");
    assert_is_property(&result[1], "Bar");
    assert_is_indexer(&result[2], &["5", "6"]);
    assert_is_property(&result[3], "Baz");
}

#[test]
fn should_build_stream_node() {
    let result = parse("Foo^");

    assert_eq!(result.len(), 2);
    assert!(is::<StreamNode>(&result[1]));
}
