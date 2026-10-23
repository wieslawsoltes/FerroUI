//! Equivalence coverage for native inline text's scoped preview route. These
//! are core state/dependency tests, not native shaping or timing benchmarks.
use crate::*;
use std::collections::BTreeMap;

fn text(mode: TextAutoResize) -> Node {
    let mut node = Node::new(NodeKind::Text, "Rich text", 18.0, 12.0, 140.0, 64.0).with_text(
        "A🦀\nשלום\n",
        18.0,
        Color::BLACK,
    );
    node.text_auto_resize = mode;
    node.apply_text_style(
        0..5,
        &TextStylePatch {
            font_weight: TextStyleChange::Set(700),
            ..Default::default()
        },
    )
    .unwrap();
    node.apply_paragraph_style(
        0..6,
        &ParagraphStylePatch {
            indent: TextStyleChange::Set(7.0),
            ..Default::default()
        },
    )
    .unwrap();
    node.apply_paragraph_style(
        6..6,
        &ParagraphStylePatch {
            direction: TextStyleChange::Set(TextDirection::RightToLeft),
            space_after: TextStyleChange::Set(4.0),
            ..Default::default()
        },
    )
    .unwrap();
    node
}

fn edit_text(node: &mut Node, ordinal: usize) {
    let prefix = format!("{ordinal}Ω🦀\n");
    let value = format!("{prefix}{}", node.text);
    assert!(node
        .replace_text_range_preserving_runs(value, 0..0, prefix.len())
        .unwrap());
    node.apply_text_style(
        0..prefix.len(),
        &TextStylePatch {
            font_size: TextStyleChange::Set(20.0 + ordinal as f64),
            ..Default::default()
        },
    )
    .unwrap();
    node.apply_paragraph_style(
        0..prefix.len(),
        &ParagraphStylePatch {
            indent: TextStyleChange::Set(10.0 + ordinal as f64),
            space_before: TextStyleChange::Set(3.0),
            ..Default::default()
        },
    )
    .unwrap();
}

/// This is the shell's guarded route. Imported auto-layout geometry may not
/// have been arranged yet, so those documents retain the prior full pass.
fn preview(editor: &mut Editor, id: NodeId, ordinal: usize, reference: bool) {
    edit_text(editor.document.nodes.get_mut(&id).unwrap(), ordinal);
    if reference || editor.has_auto_layout() {
        assert!(editor.document.layout_all().errors.is_empty());
        editor.refresh();
    } else {
        editor.refresh_preview_nodes(&[id]);
    }
}

fn authored(document: &Document) -> Document {
    let mut result = document.clone();
    result.revision = 0;
    for node in result.nodes.values_mut() {
        node.revision = 0;
    }
    result
}

fn equivalent(actual: &Editor, reference: &Editor) {
    // Both paths publish once per preview; compare revisions as well as all
    // authored data, without formatting two 10,000-node documents on success.
    assert!(
        actual.document == reference.document,
        "scoped/full documents differ"
    );
    assert_eq!(actual.selection, reference.selection);
    assert_eq!(actual.active_page, reference.active_page);
    assert_eq!(actual.history_len(), reference.history_len());
    assert_eq!(actual.can_undo(), reference.can_undo());
    assert_eq!(actual.can_redo(), reference.can_redo());
    assert_eq!(actual.scene.transforms, reference.scene.transforms);
    assert_eq!(actual.scene.draw_order, reference.scene.draw_order);
    for rect in [
        Rect::new(-100.0, -100.0, 800.0, 600.0),
        Rect::new(900.0, 900.0, 200.0, 200.0),
    ] {
        assert_eq!(actual.visible_nodes(rect), reference.visible_nodes(rect));
    }
    for &id in actual.document.nodes.keys() {
        assert_eq!(
            actual.scene.spatial.bounds(id),
            reference.scene.spatial.bounds(id),
            "bounds of {id}"
        );
    }
}

fn scoped(editor: &Editor, token: &EditorRenderToken, id: NodeId, parents: &[NodeId]) {
    let mut ancestors = parents.to_vec();
    ancestors.sort_unstable();
    assert_eq!(
        editor.render_changes_since(token),
        EditorRenderChanges::Nodes {
            nodes: vec![id],
            ancestors
        }
    );
}

#[test]
fn text_previews_in_ten_thousand_nodes_match_full_reference_and_keep_scoped_history() {
    let mut document = Document::new("Ten thousand text preview dependencies");
    let page = document.pages[0].id;
    let mut outer = Node::new(NodeKind::Frame, "Affine parent", 50.0, 30.0, 500.0, 300.0);
    outer.clip_content = true;
    outer.affine_transform = Some(Transform {
        a: 1.0,
        b: 0.2,
        c: 0.3,
        d: 1.1,
        e: 0.0,
        f: 0.0,
    });
    let outer = document.insert_node(page, outer).unwrap();
    let mut inner = Node::new(NodeKind::Group, "Rotated group", 20.0, 10.0, 400.0, 240.0);
    inner.rotation = -13.0;
    let inner = document.insert_node(outer, inner).unwrap();
    let ids: Vec<_> = [
        TextAutoResize::Fixed,
        TextAutoResize::Height,
        TextAutoResize::WidthAndHeight,
    ]
    .into_iter()
    .map(|mode| document.insert_node(inner, text(mode)).unwrap())
    .collect();
    while document.nodes.len() < 10_000 {
        let index = document.nodes.len();
        let mut node = Node::new(
            NodeKind::Rectangle,
            format!("Unrelated {index}"),
            (index % 100) as f64 * 16.0,
            800.0 + (index / 100) as f64 * 16.0,
            8.0,
            8.0,
        );
        if index % 64 == 0 {
            node = Node::new(
                NodeKind::Text,
                format!("Unrelated text {index}"),
                node.x,
                node.y,
                100.0,
                20.0,
            )
            .with_text("Unchanged café Ω 🦀", 14.0, Color::BLACK);
        }
        document.insert_node(page, node).unwrap();
    }
    let unrelated = *document.nodes.keys().next_back().unwrap();
    let mut actual = Editor::new(document.clone());
    let mut reference = Editor::new(document);
    let initial = authored(&actual.document);
    let unrelated_revision = actual.document.nodes[&unrelated].revision;
    assert!(!actual.has_auto_layout());
    for (index, &id) in ids.iter().enumerate() {
        actual.selection = vec![id];
        reference.selection = vec![id];
        let bounds = actual.document.nodes[&id].bounds();
        actual.begin_node_transaction(&[id], "Edit text");
        reference.begin_transaction("Edit text");
        for ordinal in 1..=3 {
            let token = actual.render_change_token();
            preview(&mut actual, id, ordinal, false);
            preview(&mut reference, id, ordinal, true);
            equivalent(&actual, &reference);
            scoped(&actual, &token, id, &[outer, inner]);
            assert_eq!(
                actual.history_len(),
                index,
                "a preview cannot commit its typing gesture"
            );
            assert_eq!(
                actual.document.nodes[&id].bounds(),
                bounds,
                "native text auto sizing does not mutate core layout geometry"
            );
            assert_eq!(
                actual.document.nodes[&unrelated].revision,
                unrelated_revision
            );
        }
        assert!(actual.commit_transaction());
        assert!(reference.commit_transaction());
        equivalent(&actual, &reference);
        assert_eq!(actual.history_len(), index + 1);
        let committed = authored(&actual.document);
        actual.begin_node_transaction(&[id], "Canceled text");
        reference.begin_transaction("Canceled text");
        preview(&mut actual, id, 9, false);
        preview(&mut reference, id, 9, true);
        let token = actual.render_change_token();
        actual.cancel_transaction();
        reference.cancel_transaction();
        equivalent(&actual, &reference);
        scoped(&actual, &token, id, &[outer, inner]);
        assert!(authored(&actual.document) == committed);
    }
    for &id in ids.iter().rev() {
        let token = actual.render_change_token();
        assert!(actual.undo());
        assert!(reference.undo());
        equivalent(&actual, &reference);
        scoped(&actual, &token, id, &[outer, inner]);
    }
    assert!(authored(&actual.document) == initial);
    let token = actual.render_change_token();
    actual.begin_node_transaction(&[ids[0]], "No-op inline edit");
    reference.begin_transaction("No-op inline edit");
    assert!(!actual.commit_transaction());
    assert!(!reference.commit_transaction());
    equivalent(&actual, &reference);
    assert!(actual.can_redo());
    assert_eq!(
        actual.render_changes_since(&token),
        EditorRenderChanges::Nodes {
            nodes: vec![],
            ancestors: vec![]
        }
    );
    for &id in &ids {
        let token = actual.render_change_token();
        assert!(actual.redo());
        assert!(reference.redo());
        equivalent(&actual, &reference);
        scoped(&actual, &token, id, &[outer, inner]);
    }
    actual.document.validate().unwrap();
}

fn layout_document(settle: bool) -> (Document, NodeId, NodeId, NodeId) {
    let mut document = Document::new("Nested auto layout text");
    let page = document.pages[0].id;
    let mut outer = Node::new(NodeKind::Frame, "Row hug", 30.0, 20.0, 700.0, 500.0);
    outer.layout = Some(AutoLayout {
        axis: LayoutAxis::Row,
        width: SizingMode::Hug,
        height: SizingMode::Hug,
        gap: 12.0,
        padding: Padding::all(8.0),
        ..Default::default()
    });
    let outer = document.insert_node(page, outer).unwrap();
    let mut inner = Node::new(NodeKind::Frame, "Column hug", 91.0, 83.0, 400.0, 300.0);
    inner.layout = Some(AutoLayout {
        axis: LayoutAxis::Column,
        width: SizingMode::Hug,
        height: SizingMode::Hug,
        gap: 6.0,
        padding: Padding::all(4.0),
        ..Default::default()
    });
    let inner = document.insert_node(outer, inner).unwrap();
    let id = document
        .insert_node(inner, text(TextAutoResize::WidthAndHeight))
        .unwrap();
    document
        .insert_node(
            inner,
            Node::new(NodeKind::Rectangle, "Inner sibling", 70.0, 70.0, 30.0, 20.0),
        )
        .unwrap();
    document
        .insert_node(
            outer,
            Node::new(
                NodeKind::Rectangle,
                "Outer sibling",
                500.0,
                300.0,
                60.0,
                40.0,
            ),
        )
        .unwrap();
    if settle {
        assert!(document.layout_all().errors.is_empty());
        assert!(
            document.layout_all().changed.is_empty(),
            "fixture must be at a fixed point"
        );
    }
    (document, outer, inner, id)
}

#[test]
fn text_preview_nested_auto_layout_preserves_full_fallback_and_text_independent_geometry() {
    let (document, _, _, id) = layout_document(true);
    let mut actual = Editor::new(document.clone());
    let mut reference = Editor::new(document);
    let initial = authored(&actual.document);
    let geometry: BTreeMap<_, _> = actual
        .document
        .nodes
        .iter()
        .map(|(&id, node)| (id, node.bounds()))
        .collect();
    assert!(actual.has_auto_layout());
    actual.begin_node_transaction(&[id], "Edit laid out text");
    reference.begin_transaction("Edit laid out text");
    for ordinal in 1..=3 {
        let token = actual.render_change_token();
        preview(&mut actual, id, ordinal, false);
        preview(&mut reference, id, ordinal, true);
        equivalent(&actual, &reference);
        assert_eq!(
            actual.render_changes_since(&token),
            EditorRenderChanges::Full
        );
        assert_eq!(
            actual
                .document
                .nodes
                .iter()
                .map(|(&id, node)| (id, node.bounds()))
                .collect::<BTreeMap<_, _>>(),
            geometry
        );
    }
    assert!(actual.commit_transaction());
    assert!(reference.commit_transaction());
    let committed = authored(&actual.document);
    let token = actual.render_change_token();
    assert!(actual.undo());
    assert!(reference.undo());
    equivalent(&actual, &reference);
    assert!(authored(&actual.document) == initial);
    assert_eq!(
        actual.render_changes_since(&token),
        EditorRenderChanges::Full
    );
    assert!(actual.redo());
    assert!(reference.redo());
    equivalent(&actual, &reference);
    assert!(authored(&actual.document) == committed);
    actual.begin_node_transaction(&[id], "Canceled laid out text");
    reference.begin_transaction("Canceled laid out text");
    preview(&mut actual, id, 9, false);
    preview(&mut reference, id, 9, true);
    actual.cancel_transaction();
    reference.cancel_transaction();
    equivalent(&actual, &reference);
    assert!(authored(&actual.document) == committed);
}

#[test]
fn text_preview_of_unsettled_import_retains_first_edit_arrangement_and_exact_cancel_history() {
    let (document, outer, inner, id) = layout_document(false);
    let document = Document::from_json(&document.to_json().unwrap()).unwrap();
    let mut actual = Editor::new(document.clone());
    let mut reference = Editor::new(document);
    let initial = authored(&actual.document);
    assert_eq!(actual.document.nodes[&inner].x, 91.0);
    assert_eq!(actual.document.nodes[&id].x, 18.0);
    assert!(actual.has_auto_layout());
    actual.begin_node_transaction(&[id], "Canceled imported text");
    reference.begin_transaction("Canceled imported text");
    preview(&mut actual, id, 1, false);
    preview(&mut reference, id, 1, true);
    equivalent(&actual, &reference);
    assert_eq!(actual.document.nodes[&inner].x, 8.0);
    assert_eq!(actual.document.nodes[&id].x, 4.0);
    assert_ne!(
        actual.document.nodes[&outer].width,
        initial.nodes[&outer].width
    );
    actual.cancel_transaction();
    reference.cancel_transaction();
    equivalent(&actual, &reference);
    assert!(
        authored(&actual.document) == initial,
        "cancel restores imported positions as well as text"
    );
    actual.begin_node_transaction(&[id], "Imported text");
    reference.begin_transaction("Imported text");
    preview(&mut actual, id, 2, false);
    preview(&mut reference, id, 2, true);
    assert!(actual.commit_transaction());
    assert!(reference.commit_transaction());
    equivalent(&actual, &reference);
    assert_eq!(actual.history_len(), 1);
    let committed = actual.to_json().unwrap();
    assert_eq!(
        Document::from_json(&committed).unwrap().to_json().unwrap(),
        committed
    );
    assert!(actual.undo());
    assert!(reference.undo());
    equivalent(&actual, &reference);
    assert!(authored(&actual.document) == initial);
    assert!(actual.redo());
    assert!(reference.redo());
    equivalent(&actual, &reference);
    assert_eq!(actual.to_json().unwrap(), committed);
}

#[test]
fn text_preview_preserves_existing_component_and_variable_binding_policy() {
    let mut document = Document::new("Bound component text preview");
    let page = document.pages[0].id;
    let master = document
        .insert_node(
            page,
            Node::new(NodeKind::Component, "Master", 0.0, 0.0, 240.0, 160.0),
        )
        .unwrap();
    let source = document
        .insert_node(master, text(TextAutoResize::Height))
        .unwrap();
    let variable = document.allocate_id();
    document.variables.insert(
        variable,
        Variable {
            id: variable,
            name: "Label".into(),
            collection: "Text".into(),
            values: BTreeMap::from([(
                "Default".into(),
                VariableValue::String(document.nodes[&source].text.clone()),
            )]),
        },
    );
    document
        .nodes
        .get_mut(&source)
        .unwrap()
        .variable_bindings
        .insert("text".into(), variable);
    let mut builder = Editor::new(document);
    let instance = builder.create_instance(master, 300.0, 0.0).unwrap();
    let id = builder.document.nodes[&instance].children[0];
    builder
        .document
        .nodes
        .get_mut(&id)
        .unwrap()
        .overridden_properties
        .insert("text_paragraphs".into());
    let document = builder.document;
    document.validate().unwrap();
    let mut actual = Editor::new(document.clone());
    let mut reference = Editor::new(document);
    let source_before = actual.document.nodes[&source].clone();
    let target_before = actual.document.nodes[&id].clone();
    let initial = actual.to_json().unwrap();
    actual.begin_node_transaction(&[id], "Bound label text");
    reference.begin_transaction("Bound label text");
    let token = actual.render_change_token();
    preview(&mut actual, id, 1, false);
    preview(&mut reference, id, 1, true);
    equivalent(&actual, &reference);
    scoped(&actual, &token, id, &[instance]);
    let target = &actual.document.nodes[&id];
    assert_eq!(target.component_source, target_before.component_source);
    assert_eq!(target.variable_bindings, target_before.variable_bindings);
    assert_eq!(
        target.overridden_properties,
        target_before.overridden_properties
    );
    assert_eq!(actual.document.nodes[&source], source_before);
    assert!(actual.commit_transaction());
    assert!(reference.commit_transaction());
    equivalent(&actual, &reference);
    let committed = actual.to_json().unwrap();
    assert_eq!(
        Document::from_json(&committed).unwrap().to_json().unwrap(),
        committed
    );
    assert!(actual.undo());
    assert!(reference.undo());
    equivalent(&actual, &reference);
    assert_eq!(actual.to_json().unwrap(), initial);
    assert!(actual.redo());
    assert!(reference.redo());
    equivalent(&actual, &reference);
    assert_eq!(actual.to_json().unwrap(), committed);
}
