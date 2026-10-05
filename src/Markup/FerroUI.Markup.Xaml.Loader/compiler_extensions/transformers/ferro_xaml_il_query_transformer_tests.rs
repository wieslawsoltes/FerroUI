//! Tests of `FerroXamlIlQueryTransformer` and the query nodes.

use std::rc::Rc;

use xamlx::ast::{IXamlAstNode, IXamlAstValueNode, XamlAstExtensions, XamlAstNodeExtensions, XamlLineInfo};
use xamlx::transform::IXamlAstTransformer;

use super::*;
use crate::compiler_extensions::FerroXamlDiagnosticCodes;
use crate::testing::styles::{
    create_styles_test_framework, describe_query, find_nodes, run_transformers, xmlns,
};
use crate::testing::TestFramework;

fn run(fw: &TestFramework, xaml: &str) -> Rc<dyn IXamlAstNode> {
    let pipeline: Vec<Box<dyn IXamlAstTransformer>> = vec![
        Box::new(FerroXamlIlQueryTransformer),
        Box::new(FerroXamlIlQueryTransformer),
    ];
    run_transformers(fw, xaml, pipeline).expect("errors are not fatal in the fixture")
}

fn query(text: &str) -> (TestFramework, Rc<dyn IXamlAstNode>) {
    let fw = create_styles_test_framework();
    let root = run(&fw, &format!("<ContainerQuery {} Query=\"{text}\"/>", xmlns()));
    (fw, root)
}

fn check(text: &str, expected: &str) {
    let (fw, root) = query(text);
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    let queries = find_nodes::<dyn XamlIlQueryNode>(&root);
    assert_eq!(queries.len(), 1, "{text}");
    assert_eq!(describe_query(&queries[0]), expected, "{text}");

    // The container query is wrapped (once) in a container scope without a target type
    let scopes = find_nodes::<FerroXamlIlTargetTypeMetadataNode>(&root);
    assert_eq!(scopes.len(), 1);
    assert!(root.same_node(&scopes[0]));
    assert_eq!(scopes[0].scope_type, ScopeTypes::Container);
    assert!(get_nullable_clr_type(&scopes[0].target_type()).expect("clr").is_none());
}

fn check_error(xaml: &str, message: &str) {
    let fw = create_styles_test_framework();
    let _ = run(&fw, xaml);
    let diagnostics = fw.reported_diagnostics();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, FerroXamlDiagnosticCodes::PARSE_ERROR);
    assert!(diagnostics[0].title.starts_with(message), "{:?}", diagnostics[0].title);
}

#[test]
fn width_and_height_features() {
    check("width:100", "Width(1,100)");
    check("min-width:400", "Width(5,400)");
    check("max-width:400", "Width(4,400)");
    // As upstream, a plain `height` is parsed as "less than or equals".
    check("height:100", "Height(4,100)");
    check("min-height:1", "Height(5,1)");
    check("max-height:2", "Height(4,2)");
}

#[test]
fn alternatives_become_an_or_node() {
    check("width:100, height:200", "Or[Width(1,100), Height(4,200)]");
    check(
        "width:1, width:2, width:3",
        "Or[Width(1,1), Width(1,2), Width(1,3)]",
    );
}

/// The upstream behaviour of `and`, ported as written: the and-node only survives when a
/// comma follows it.
#[test]
fn and_combinations_as_upstream() {
    check(
        "min-width:400 and max-width:800, height:1",
        "Or[And[Width(5,400), Width(4,800)], Height(4,1)]",
    );
    // Without a following comma the last feature alone is the result
    check("min-width:400 and max-width:800", "Width(4,800)");
    check("height:1, min-width:400 and max-width:800", "Or[Height(4,1), Width(4,800)]");
    // A feature between two `and`s is collected twice
    check(
        "width:1 and width:2 and width:3, height:4",
        "Or[And[Width(1,1), Width(1,2), Width(1,2), Width(1,3)], Height(4,4)]",
    );
}

#[test]
fn query_errors() {
    check_error(
        &format!("<ContainerQuery {} Query='foo'/>", xmlns()),
        "Unable to parse query: FerroUI.Data.Core.ExpressionParseException: ",
    );
    check_error(
        &format!(
            "<ContainerQuery {}><ContainerQuery.Query><Button/></ContainerQuery.Query></ContainerQuery>",
            xmlns()
        ),
        "Query property should be a text node",
    );
    check_error(
        &format!(
            "<ContainerQuery {}><ContainerQuery.Query><Button/><Button/></ContainerQuery.Query></ContainerQuery>",
            xmlns()
        ),
        "Query property should should have exactly one value",
    );
}

#[test]
fn container_query_without_query_is_left_alone() {
    let fw = create_styles_test_framework();
    let root = run(&fw, &format!("<ContainerQuery {} Name='a'/>", xmlns()));
    assert_eq!(fw.reported_diagnostics().len(), 0);
    assert!(find_nodes::<FerroXamlIlTargetTypeMetadataNode>(&root).is_empty());

    // Other types are not touched
    let root = run(&fw, &format!("<Style {}/>", xmlns()));
    assert!(find_nodes::<FerroXamlIlTargetTypeMetadataNode>(&root).is_empty());
}

#[test]
fn query_nodes_find_their_builder_methods() {
    let (fw, root) = query("min-width:400 and max-height:800, height:1");
    let well_known = fw.configuration.well_known_types();
    let queries = find_nodes::<dyn XamlIlQueryNode>(&root);
    let or = queries[0].as_any().downcast_ref::<XamlIlOrQueryNode>().expect("or");
    assert_eq!(
        queries[0].type_().get_clr_type().expect("type").full_name(),
        "FerroUI.Styling.StyleQuery"
    );
    assert!(queries[0].previous().is_none());
    assert!(queries[0].target_type().is_none());

    let or_method = or.builder_method(&fw.types).expect("Or");
    assert_eq!(or_method.name(), "Or");
    assert!(or_method.parameters()[0].name().starts_with("IReadOnlyList"));
    let members = or.list_members(&well_known).expect("list").expect("members");
    assert_eq!(members.add.name(), "Add");
    assert_eq!(members.list_type.generic_arguments()[0].name(), "StyleQuery");
    assert!(members.constructor.parameters().is_empty());

    let members = or.queries();
    let and = members[0].as_any().downcast_ref::<XamlIlAndQueryNode>().expect("and");
    let and_method = and.builder_method(&fw.types).expect("And");
    assert_eq!(and_method.name(), "And");
    assert!(and_method.parameters()[0].name().starts_with("IReadOnlyList"));

    let and_members = and.queries();
    let width = and_members[0].as_any().downcast_ref::<XamlIlWidthQuery>().expect("width");
    assert_eq!((width.operator_value(), width.value), (5, 400.0));
    assert_eq!(width.builder_method(&fw.types).expect("Width").parameters().len(), 3);
    assert!(and_members[0].previous().expect("initial").as_any().is::<XamlIlQueryInitialNode>());
    let height = and_members[1].as_any().downcast_ref::<XamlIlHeightQuery>().expect("height");
    assert_eq!((height.operator_value(), height.value), (4, 800.0));
    assert_eq!(height.builder_method(&fw.types).expect("Height").name(), "Height");

    // The node types the transformer never creates: data and failing method lookups
    let line = XamlLineInfo::new(1, 1);
    let initial: Rc<dyn XamlIlQueryNode> =
        XamlIlQueryInitialNode::new(&line, fw.t("FerroUI.Styling.StyleQuery"));
    let type_query = XamlIlTypeQuery::new(initial.clone(), fw.t("FerroUI.Controls.Button"), true);
    assert_eq!(type_query.target_type().map(|t| t.name()), Some("Button".to_string()));
    let error = type_query.builder_method(&fw.types, &well_known).err().expect("no OfType");
    assert!(error.is_type_system_exception());
    assert!(
        error.message().starts_with("Unable to find ") && error.message().ends_with(" in FerroUI.Styling.StyleQueries"),
        "{}",
        error.message()
    );
    let string_query = XamlIlStringQuery::new(type_query.clone(), XamlIlStringQueryType::Class, "foo");
    assert_eq!(string_query.string(), "foo");
    assert_eq!(string_query.target_type().map(|t| t.name()), Some("Button".to_string()));
    assert!(string_query.builder_method(&fw.types, &well_known).is_err());
    let combinator = XamlIlCombinatorQuery::new(string_query, CombinatorQueryType::Template);
    assert!(combinator.target_type().is_none());
    let error = combinator.builder_method(&fw.types).err().expect("no Template");
    assert_eq!(error.message(), "Unable to find  in FerroUI.Styling.StyleQueries");

    // Or/And target type: the common base type, none when a member has none
    let group = XamlIlAndQueryNode::new(&line, fw.t("FerroUI.Styling.StyleQuery"));
    assert!(group.target_type().is_none());
    group.add(type_query);
    group.add(XamlIlTypeQuery::new(initial.clone(), fw.t("FerroUI.Controls.TextBlock"), false));
    assert_eq!(group.target_type().map(|t| t.name()), Some("Control".to_string()));
    group.add(initial);
    assert!(group.target_type().is_none());
    let value: Rc<dyn IXamlAstValueNode> = group;
    assert!(value.is::<dyn XamlIlQueryNode>());
}

#[test]
fn or_after_an_open_and_is_an_error() {
    use ferroui_base::styling::StyleQueryComparisonOperator;
    use ferroui_markup::markup::parsers::ContainerQuerySyntax;

    let (fw, root) = query("width:1");
    let query_type = fw.t("FerroUI.Styling.StyleQuery");
    let initial: Rc<dyn XamlIlQueryNode> = XamlIlQueryInitialNode::new(&*root, query_type.clone());
    let width = ContainerQuerySyntax::Width {
        value: 1.0,
        operator: StyleQueryComparisonOperator::Equals,
    };
    let syntax = [width.clone(), ContainerQuerySyntax::And, ContainerQuerySyntax::Or];
    let error = super::ferro_xaml_il_query_transformer::create(&root, &query_type, &initial, &syntax)
        .err()
        .expect("error");
    assert!(matches!(error, xamlx::exceptions::XamlError::Parse(_)));
    assert!(error.message().starts_with("Previously opened And node is not closed."));

    // No syntax at all: the initial node
    let empty = super::ferro_xaml_il_query_transformer::create(&root, &query_type, &initial, &[])
        .expect("initial");
    assert!(empty.same_node(&initial));
}
