use crate::{Document, NodeId, NodeKind};

fn template_part(document: &Document, template: &str, name: &str) -> NodeId {
    let template = document.ast().find_name(template).unwrap();
    document
        .ast()
        .subtree(template)
        .find(|node| node.xaml_name() == Some(name))
        .unwrap()
        .id
}

#[test]
fn source_queries_match_linear_reference_for_every_byte_and_boundary() {
    let source = "\u{feff}<?xml version='1.0'?>\r\n<!--before--><Grid><TextBlock Text='ż🦀'/><?inside value?><Border><![CDATA[é < &]]><!--note--></Border></Grid>\n<!--after-->";
    let document = Document::new(source).unwrap();
    let ast = document.ast();
    for offset in 0..=source.len() + 1 {
        let expected = ast
            .nodes()
            .filter(|node| node.span.contains(offset))
            .min_by_key(|node| node.span.len())
            .map(|node| node.id);
        let expected_element = ast
            .elements()
            .filter(|node| node.span.contains(offset))
            .min_by_key(|node| node.span.len())
            .map(|node| node.id);
        assert_eq!(ast.node_at(offset), expected, "byte {offset}");
        assert_eq!(ast.element_at(offset), expected_element, "byte {offset}");
    }
    assert_eq!(ast.node_at(usize::MAX), None);
}

#[test]
fn traversal_and_source_queries_follow_source_order_after_ids_move() {
    let mut document = Document::new(
        "<Grid><Border Name='First'><Button Name='A'/><!--keep--></Border><Border Name='Second'><Button Name='B'/></Border></Grid>",
    )
    .unwrap();
    let first = document.ast().find_name("First").unwrap();
    let second = document.ast().find_name("Second").unwrap();
    let b = document.ast().find_name("B").unwrap();
    let source = "<Grid><Border Name='Second'><Button Name='B'/></Border><Border Name='First'><Button Name='A'/><!--keep--></Border></Grid>";
    assert!(document.update_source("Reorder", source).valid);
    let ast = document.ast();
    assert_eq!(ast.element_children(ast.root()), vec![second, first]);
    let spans: Vec<_> = ast
        .nodes_in_source_order()
        .map(|node| node.span.start)
        .collect();
    assert!(spans.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(
        ast.subtree(second).map(|node| node.id).collect::<Vec<_>>(),
        vec![second, b]
    );
    assert_eq!(ast.subtree(u64::MAX).count(), 0);
    assert_eq!(ast.element_at(source.find("Name='B'").unwrap()), Some(b));
    assert_eq!(ast.subtree(first).last().unwrap().kind, NodeKind::Comment);
}

#[test]
fn identical_names_in_separate_templates_reconcile_independently() {
    let mut document = Document::new(
        "<Grid><DataTemplate Name='First'><StackPanel><Button Name='Part' Width='1'/></StackPanel></DataTemplate><DataTemplate Name='Second'><StackPanel><Button Name='Part' Width='2'/></StackPanel></DataTemplate></Grid>",
    )
    .unwrap();
    let first = template_part(&document, "First", "Part");
    let second = template_part(&document, "Second", "Part");
    let update = document.update_source(
        "Change wrappers",
        "<Grid><DataTemplate Name='First'><Border><Button Name='Part' Width='3'/></Border></DataTemplate><DataTemplate Name='Second'><Border><Button Name='Part' Width='4'/></Border></DataTemplate></Grid>",
    );
    assert!(update.valid);
    assert_eq!(template_part(&document, "First", "Part"), first);
    assert_eq!(template_part(&document, "Second", "Part"), second);
}

#[test]
fn a_name_reappearing_in_another_template_cannot_steal_old_identity() {
    let mut document = Document::new(
        "<Grid><DataTemplate Name='First'><Button Name='Part'/></DataTemplate><DataTemplate Name='Second'/></Grid>",
    )
    .unwrap();
    let previous = template_part(&document, "First", "Part");
    assert!(document.update_source(
        "Replace a scoped part",
        "<Grid><DataTemplate Name='First'/><DataTemplate Name='Second'><Button Name='Part'/></DataTemplate></Grid>",
    ).valid);
    assert_ne!(template_part(&document, "Second", "Part"), previous);
    assert!(document.ast().node(previous).is_none());
}

#[test]
fn nested_template_scopes_match_after_their_declarations_match() {
    let mut document = Document::new(
        "<Grid><DataTemplate Name='Outer'><StackPanel><ControlTemplate Name='Inner'><StackPanel><Button Name='Part'/></StackPanel></ControlTemplate><Button Name='Part'/></StackPanel></DataTemplate></Grid>",
    )
    .unwrap();
    let inner = template_part(&document, "Outer", "Inner");
    let nested_part = template_part(&document, "Inner", "Part");
    let outer_part = document
        .ast()
        .subtree(document.ast().find_name("Outer").unwrap())
        .find(|node| {
            node.xaml_name() == Some("Part") && document.ast().name_scope(node.id) != inner
        })
        .unwrap()
        .id;
    assert!(document.update_source(
        "Change nested wrappers",
        "<Grid><DataTemplate Name='Outer'><DockPanel><Button Name='Part'/><ControlTemplate Name='Inner'><Border><Button Name='Part'/></Border></ControlTemplate></DockPanel></DataTemplate></Grid>",
    ).valid);
    assert_eq!(template_part(&document, "Outer", "Inner"), inner);
    assert_eq!(template_part(&document, "Inner", "Part"), nested_part);
    assert_eq!(
        document.ast().node(outer_part).unwrap().xaml_name(),
        Some("Part")
    );
    assert_eq!(
        document.ast().name_scope(outer_part),
        document.ast().find_name("Outer").unwrap()
    );
}

#[test]
fn xml_line_endings_normalize_decoded_tokens_without_touching_source() {
    let source = "<Grid><!--a\r\nb\rc--><?sample a\r\nb\rc?><TextBlock><![CDATA[a\r\nb\rc]]>&#13;\r\n</TextBlock></Grid>";
    let document = Document::new(source).unwrap();
    assert_eq!(document.source(), source);
    for kind in [NodeKind::Comment, NodeKind::CData] {
        assert_eq!(
            document
                .ast()
                .nodes()
                .find(|node| node.kind == kind)
                .unwrap()
                .text
                .as_deref(),
            Some("a\nb\nc")
        );
    }
    assert_eq!(
        document
            .ast()
            .nodes()
            .find(|node| node.kind == NodeKind::ProcessingInstruction)
            .unwrap()
            .text
            .as_deref(),
        Some("sample a\nb\nc")
    );
    assert_eq!(
        document
            .ast()
            .nodes()
            .find(|node| node.kind == NodeKind::Text)
            .unwrap()
            .text
            .as_deref(),
        Some("\r\n")
    );
}

#[test]
fn scope_iterator_matches_ancestor_queries_including_trivia() {
    let source = "<Grid><!--outer--><DataTemplate Name='First'><Border><ControlTemplate Name='Inner'><!--inside--><Button/></ControlTemplate></Border></DataTemplate><DataTemplate Name='Second'><Button/></DataTemplate></Grid>";
    let document = Document::new(source).unwrap();
    let ast = document.ast();
    let nodes: Vec<_> = ast.nodes_with_name_scopes().collect();
    assert_eq!(nodes.len(), ast.nodes().count());
    for (node, scope) in nodes {
        assert_eq!(scope, ast.name_scope(node.id));
    }
}

#[test]
fn adjacent_cdata_has_an_independent_lossless_syntax_budget() {
    let source = "<Grid><![CDATA[a]]><![CDATA[b]]><![CDATA[c]]><![CDATA[d]]></Grid>";
    // The validating XML DOM merges these sections into one text node.
    assert!(roxmltree::Document::parse_with_options(
        source,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 4
        }
    )
    .is_ok());
    assert!(super::Ast::parse_with_limit(source, 5).is_ok());
    assert!(super::Ast::parse_with_limit(source, 4)
        .unwrap_err()
        .to_string()
        .contains("syntax nodes"));
}

#[test]
fn serialized_ast_roundtrips_without_changing_wire_shape_or_queries() {
    let document = Document::new("<?xml version='1.0'?>\n<Grid xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><!--comment--><Border x:Name='A' Text=''><![CDATA[a]]>text</Border><Button/></Grid>\n").unwrap();
    let wire = serde_json::to_value(document.ast()).unwrap();
    assert_eq!(wire.as_object().unwrap().len(), 3);
    let restored: super::Ast = serde_json::from_value(wire).unwrap();
    assert_eq!(&restored, document.ast());
    for offset in 0..document.source().len() {
        assert_eq!(restored.node_at(offset), document.ast().node_at(offset));
    }
}

#[test]
fn serialized_ast_rejects_cycles_missing_links_and_unordered_intervals() {
    let document =
        Document::new("<Grid><Border><Button/></Border><TextBlock Text='ok'/></Grid>").unwrap();
    let wire = serde_json::to_value(document.ast()).unwrap();
    let root = document.ast().root();
    let children = document.ast().element_children(root);
    let border = children[0];
    let button = document.ast().element_children(border)[0];
    let text = children[1];
    type Corruption = Box<dyn FnOnce(&mut serde_json::Value)>;
    let corruptions: Vec<Corruption> = vec![
        Box::new(|ast| ast["root"] = serde_json::json!(999)),
        Box::new(move |ast| ast["nodes"][root.to_string()]["parent"] = serde_json::json!(button)),
        Box::new(move |ast| {
            ast["nodes"][button.to_string()]["children"] = serde_json::json!([root])
        }),
        Box::new(move |ast| {
            ast["nodes"][border.to_string()]["children"] = serde_json::json!([999])
        }),
        Box::new(move |ast| {
            ast["nodes"][root.to_string()]["children"] = serde_json::json!([text, border])
        }),
        Box::new(move |ast| {
            ast["nodes"][root.to_string()]["children"] = serde_json::json!([border, border, text])
        }),
        Box::new(move |ast| ast["nodes"][button.to_string()]["parent"] = serde_json::json!(root)),
        Box::new(move |ast| ast["nodes"][border.to_string()]["id"] = serde_json::json!(999)),
        Box::new(move |ast| {
            ast["nodes"][button.to_string()]["span"]["end"] = serde_json::json!(usize::MAX)
        }),
        Box::new(move |ast| {
            ast["nodes"][text.to_string()]["attributes"][0]["value_span"]["end"] =
                serde_json::json!(usize::MAX)
        }),
        Box::new(move |ast| {
            ast["nodes"]
                .as_object_mut()
                .unwrap()
                .remove(&button.to_string());
        }),
    ];
    for corrupt in corruptions {
        let mut invalid = wire.clone();
        corrupt(&mut invalid);
        assert!(serde_json::from_value::<super::Ast>(invalid).is_err());
    }
}
