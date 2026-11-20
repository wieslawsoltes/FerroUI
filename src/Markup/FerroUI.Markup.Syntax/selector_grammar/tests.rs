use super::*;

#[test]
fn retains_operand_spans_through_nested_selectors() {
    let source =
        "ns|Étiquette[Caption=🧭]:not(Button[(ui|Card.Tone)=Caption]), :is(ns|Étiquette)#Caption";
    let (syntax, operands) = SelectorGrammar::operand_ranges(source).unwrap();
    assert_eq!(syntax, SelectorGrammar::parse(source).unwrap());
    let text: Vec<_> = operands
        .iter()
        .map(|operand| match operand {
            SelectorOperand::Type(span) => ("type", &source[span.clone()], None),
            SelectorOperand::Property(span) => ("property", &source[span.clone()], None),
            SelectorOperand::AttachedProperty { owner, property } => (
                "attached",
                &source[owner.clone()],
                Some(&source[property.clone()]),
            ),
        })
        .collect();
    assert_eq!(
        text,
        [
            ("type", "Étiquette", None),
            ("property", "Caption", None),
            ("type", "Button", None),
            ("attached", "Card", Some("Tone")),
            ("type", "Étiquette", None),
        ]
    );
}

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
        [
            of_type("Button"),
            SelectorSyntax::Template,
            of_type("Border")
        ]
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
    assert_eq!(
        error("Button[Foo=bar"),
        "Expected ']', got end of selector."
    );
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
    assert_eq!(
        error("Button[(1.Column)=1]"),
        "Expected an identifier, got '1"
    );
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
