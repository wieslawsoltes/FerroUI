use super::*;
use crate::{
    Document, Editor, LayoutAxis, NodeId, NodeKind, Padding, SizingMode, VariableEdit,
    VariableEditResult, VariableKind,
};

fn fixture() -> (Editor, NodeId, [NodeId; 2]) {
    let mut document = Document::new("Responsive spacing");
    let mut node = Node::new(NodeKind::Frame, "Row", 20.0, 30.0, 320.0, 180.0);
    node.layout = Some(AutoLayout {
        padding: Padding::all(10.0),
        gap: 8.0,
        cross_gap: 12.0,
        width: SizingMode::Fixed,
        height: SizingMode::Fixed,
        ..AutoLayout::default()
    });
    let frame = document.insert_node(document.pages[0].id, node).unwrap();
    let a = document
        .insert_node(
            frame,
            Node::new(NodeKind::Rectangle, "A", 0.0, 0.0, 40.0, 25.0),
        )
        .unwrap();
    let b = document
        .insert_node(
            frame,
            Node::new(NodeKind::Rectangle, "B", 0.0, 0.0, 50.0, 30.0),
        )
        .unwrap();
    assert!(document.layout_all().errors.is_empty());
    let mut editor = Editor::new(document);
    editor.selection = vec![frame];
    (editor, frame, [a, b])
}

fn add(editor: &mut Editor, name: &str, value: VariableValue) -> VariableId {
    editor
        .edit_variables(VariableEdit::Upsert {
            id: None,
            name: name.into(),
            collection: "Spacing".into(),
            mode: "Default".into(),
            value,
            replace_all_modes: false,
        })
        .unwrap()
        .variable
        .unwrap()
}

fn save(
    editor: &mut Editor,
    id: VariableId,
    mode: &str,
    value: f64,
) -> Result<VariableEditResult, crate::VariableEditError> {
    let variable = &editor.document.variables[&id];
    editor.edit_variables(VariableEdit::Upsert {
        id: Some(id),
        name: variable.name.clone(),
        collection: variable.collection.clone(),
        mode: mode.into(),
        value: VariableValue::Number(value),
        replace_all_modes: false,
    })
}

fn bind(
    editor: &mut Editor,
    frame: NodeId,
    field: LayoutVariableField,
    variable: Option<VariableId>,
) {
    editor
        .edit_variables(VariableEdit::Bind {
            nodes: vec![frame],
            property: field.key().into(),
            variable,
        })
        .unwrap();
}

fn assert_authored_state(editor: &Editor, expected: &Document) {
    // Native revisions advance on Undo/Redo to invalidate render caches. The
    // portable serialization contains every authored field and intentionally
    // omits those monotonic implementation counters.
    assert_eq!(
        editor.document.to_json().unwrap(),
        expected.to_json().unwrap()
    );
}

#[test]
fn canonical_layout_spacing_keys_are_typed_and_unambiguous() {
    for field in LayoutVariableField::ALL {
        assert_eq!(parse_layout_variable_binding(field.key()), Some(field));
        assert_eq!(
            crate::variable_binding_kind(field.key()),
            Some(VariableKind::Number)
        );
    }
    for key in [
        "layout.padding",
        "layout.padding.Top",
        "layout.padding.top.extra",
        "layout_padding.top",
        "layout.padding.0",
        "layout.gap ",
        "layout.Gap",
        "layout.gap.0",
    ] {
        assert_eq!(parse_layout_variable_binding(key), None);
        assert_eq!(crate::variable_binding_kind(key), None);
    }
}

#[test]
fn layout_spacing_modes_and_aliases_reflow_and_round_trip_with_exact_history() {
    let (mut editor, frame, [a, b]) = fixture();
    let spacing = add(&mut editor, "Base", VariableValue::Number(16.25));
    let alias = add(&mut editor, "Alias", VariableValue::Alias(spacing));
    for field in LayoutVariableField::ALL {
        bind(&mut editor, frame, field, Some(alias));
    }
    let before = editor.document.to_json().unwrap();
    let depth = editor.history_len();
    editor
        .edit_variables(VariableEdit::AddMode {
            name: "Roomy".into(),
            copy_from: "Default".into(),
        })
        .unwrap();
    save(&mut editor, spacing, "Roomy", 32.125).unwrap();
    let layout = editor.document.nodes[&frame].layout.as_ref().unwrap();
    assert_eq!(layout.padding, Padding::all(32.125));
    assert_eq!(layout.gap, 32.125);
    assert_eq!(layout.cross_gap, 32.125);
    assert_eq!(
        (editor.document.nodes[&a].x, editor.document.nodes[&a].y),
        (32.125, 32.125)
    );
    assert_eq!(editor.document.nodes[&b].x, 104.25);
    assert_eq!(editor.selection, [frame]);
    let after = editor.document.to_json().unwrap();
    assert_eq!(
        Document::from_json(&after).unwrap().to_json().unwrap(),
        after
    );
    assert_eq!(editor.history_len(), depth + 2);
    assert!(editor.undo());
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(editor.redo());
    assert!(editor.redo());
    assert_eq!(editor.document.to_json().unwrap(), after);
    let depth = editor.history_len();
    editor
        .edit_variables(VariableEdit::ActivateMode {
            mode: "Default".into(),
        })
        .unwrap();
    assert_eq!(
        editor.document.nodes[&frame]
            .layout
            .as_ref()
            .unwrap()
            .padding,
        Padding::all(16.25)
    );
    assert_eq!(editor.document.nodes[&b].x, 72.5);
    assert_eq!(editor.history_len(), depth + 1);
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), after);
}

#[test]
fn invalid_spacing_modes_targets_locks_and_alias_types_preserve_document_and_real_redo() {
    let (mut editor, frame, [a, _]) = fixture();
    let spacing = add(&mut editor, "Spacing", VariableValue::Number(12.5));
    let color = add(
        &mut editor,
        "Color",
        VariableValue::Color(crate::Color::BLACK),
    );
    bind(
        &mut editor,
        frame,
        LayoutVariableField::PaddingLeft,
        Some(spacing),
    );
    editor
        .edit_variables(VariableEdit::AddMode {
            name: "Inactive".into(),
            copy_from: "Default".into(),
        })
        .unwrap();
    editor
        .edit_variables(VariableEdit::ActivateMode {
            mode: "Default".into(),
        })
        .unwrap();
    save(&mut editor, spacing, "Default", 20.0).unwrap();
    let after = editor.document.clone();
    assert!(editor.undo());
    let before = editor.document.clone();
    let revision = editor.revision;
    let token = editor.render_change_token();
    let depth = editor.history_len();
    for value in [-1.0, f64::NEG_INFINITY, f64::INFINITY, f64::NAN] {
        assert!(save(&mut editor, spacing, "Inactive", value).is_err());
        assert_eq!(editor.document, before);
        assert_eq!(editor.revision, revision);
        assert_eq!(editor.render_change_token(), token);
        assert_eq!(editor.history_len(), depth);
        assert!(editor.can_redo());
    }
    for (nodes, variable) in [
        (vec![frame, a], Some(spacing)),
        (vec![frame], Some(color)),
        (vec![u64::MAX], Some(spacing)),
    ] {
        assert!(editor
            .edit_variables(VariableEdit::Bind {
                nodes,
                property: LayoutVariableField::Gap.key().into(),
                variable
            })
            .is_err());
        assert_eq!(editor.document, before);
        assert_eq!(editor.history_len(), depth);
        assert!(editor.can_redo());
    }
    assert!(editor.redo());
    assert_authored_state(&editor, &after);
    editor.document.nodes.get_mut(&frame).unwrap().locked = true;
    let before = editor.document.clone();
    assert!(editor
        .edit_variables(VariableEdit::Bind {
            nodes: vec![frame],
            property: LayoutVariableField::PaddingLeft.key().into(),
            variable: None
        })
        .is_err());
    assert_eq!(editor.document, before);
}

#[test]
fn padding_numeric_edits_detach_only_changed_sides_and_equal_values_preserve_redo() {
    let (mut editor, frame, _) = fixture();
    let spacing = add(&mut editor, "Spacing", VariableValue::Number(10.0));
    for field in LayoutVariableField::ALL {
        bind(&mut editor, frame, field, Some(spacing));
    }
    let before = editor.document.clone();
    editor
        .set_layout_padding(
            frame,
            Padding {
                top: 25.75,
                ..Padding::all(10.0)
            },
        )
        .unwrap();
    let after = editor.document.clone();
    let node = &editor.document.nodes[&frame];
    assert!(!node
        .variable_bindings
        .contains_key(LayoutVariableField::PaddingTop.key()));
    for field in LayoutVariableField::ALL.into_iter().skip(1) {
        assert_eq!(node.variable_bindings[field.key()], spacing);
    }
    assert!(editor.undo());
    assert_authored_state(&editor, &before);
    let token = editor.render_change_token();
    let depth = editor.history_len();
    editor
        .set_layout_padding(frame, Padding::all(10.0))
        .unwrap();
    assert_authored_state(&editor, &before);
    assert_eq!(editor.render_change_token(), token);
    assert_eq!(editor.history_len(), depth);
    assert!(editor.redo());
    assert_authored_state(&editor, &after);
    save(&mut editor, spacing, "Default", 17.5).unwrap();
    let layout = editor.document.nodes[&frame].layout.as_ref().unwrap();
    assert_eq!(
        layout.padding,
        Padding {
            top: 25.75,
            right: 17.5,
            bottom: 17.5,
            left: 17.5
        }
    );
    assert_eq!(layout.gap, 17.5);
}

#[test]
fn generic_scoped_and_inspector_spacing_edits_detach_values_without_detaching_unrelated_bindings() {
    let (mut editor, frame, _) = fixture();
    let spacing = add(&mut editor, "Spacing", VariableValue::Number(10.0));
    for field in LayoutVariableField::ALL {
        bind(&mut editor, frame, field, Some(spacing));
    }
    editor
        .document
        .nodes
        .get_mut(&frame)
        .unwrap()
        .variable_bindings
        .insert("width".into(), spacing);
    editor.refresh();
    // A direct document update can make the cached render node stale. Capturing
    // actual spacing must not detach its binding during an unrelated rename.
    editor
        .document
        .nodes
        .get_mut(&frame)
        .unwrap()
        .layout
        .as_mut()
        .unwrap()
        .gap = 11.0;
    editor.update_node(frame, "Rename", |node| node.name = "Renamed".into());
    assert_eq!(
        editor.document.nodes[&frame].variable_bindings[LayoutVariableField::Gap.key()],
        spacing
    );
    editor.update_node(frame, "Gap", |node| {
        node.layout.as_mut().unwrap().gap = 18.0
    });
    assert!(!editor.document.nodes[&frame]
        .variable_bindings
        .contains_key(LayoutVariableField::Gap.key()));
    editor.update_nodes_with_layout(&[frame], "Uniform padding", |node| {
        node.layout.as_mut().unwrap().padding = Padding::all(25.0)
    });
    let node = &editor.document.nodes[&frame];
    for field in LayoutVariableField::ALL.into_iter().take(4) {
        assert!(!node.variable_bindings.contains_key(field.key()));
    }
    assert_eq!(
        node.variable_bindings[LayoutVariableField::CrossGap.key()],
        spacing
    );
    assert_eq!(node.variable_bindings["width"], spacing);
    editor.update_nodes(&[frame], "Remove layout", |node| node.layout = None);
    assert_eq!(editor.document.nodes[&frame].variable_bindings.len(), 1);
    editor.document.validate_variables().unwrap();
}

#[test]
fn unbinding_and_deleting_spacing_freeze_resolved_values_with_exact_undo() {
    let (mut editor, frame, _) = fixture();
    let spacing = add(&mut editor, "Spacing", VariableValue::Number(21.75));
    bind(
        &mut editor,
        frame,
        LayoutVariableField::PaddingLeft,
        Some(spacing),
    );
    bind(&mut editor, frame, LayoutVariableField::Gap, Some(spacing));
    let before = editor.document.clone();
    bind(&mut editor, frame, LayoutVariableField::PaddingLeft, None);
    assert_eq!(
        editor.document.nodes[&frame]
            .layout
            .as_ref()
            .unwrap()
            .padding
            .left,
        21.75
    );
    assert!(!editor.document.nodes[&frame]
        .variable_bindings
        .contains_key(LayoutVariableField::PaddingLeft.key()));
    save(&mut editor, spacing, "Default", 29.0).unwrap();
    assert_eq!(
        editor.document.nodes[&frame]
            .layout
            .as_ref()
            .unwrap()
            .padding
            .left,
        21.75
    );
    assert_eq!(
        editor.document.nodes[&frame].layout.as_ref().unwrap().gap,
        29.0
    );
    editor
        .edit_variables(VariableEdit::Delete { id: spacing })
        .unwrap();
    assert!(editor.document.nodes[&frame].variable_bindings.is_empty());
    assert_eq!(
        editor.document.nodes[&frame].layout.as_ref().unwrap().gap,
        29.0
    );
    assert!(editor.undo());
    assert!(editor.undo());
    assert!(editor.undo());
    assert_authored_state(&editor, &before);
}

#[test]
fn spacing_bindings_preserve_component_layout_ownership_during_master_sync() {
    let (mut editor, master, _) = fixture();
    let first = add(&mut editor, "Master", VariableValue::Number(10.0));
    let local = add(&mut editor, "Instance", VariableValue::Number(30.5));
    bind(
        &mut editor,
        master,
        LayoutVariableField::PaddingLeft,
        Some(first),
    );
    editor.selection = vec![master];
    assert_eq!(editor.create_component(), Some(master));
    let instance = editor.create_instance(master, 400.0, 30.0).unwrap();
    bind(
        &mut editor,
        instance,
        LayoutVariableField::PaddingLeft,
        Some(local),
    );
    let before = editor.document.clone();
    assert!(editor.document.nodes[&instance]
        .overridden_properties
        .contains("layout"));
    editor
        .set_layout_padding(
            master,
            Padding {
                left: 40.0,
                ..Padding::all(10.0)
            },
        )
        .unwrap();
    editor.sync_instances(master);
    assert_eq!(
        editor.document.nodes[&instance]
            .layout
            .as_ref()
            .unwrap()
            .padding
            .left,
        30.5
    );
    assert_eq!(
        editor.document.nodes[&instance].variable_bindings[LayoutVariableField::PaddingLeft.key()],
        local
    );
    save(&mut editor, local, "Default", 34.25).unwrap();
    assert_eq!(
        editor.document.nodes[&instance]
            .layout
            .as_ref()
            .unwrap()
            .padding
            .left,
        34.25
    );
    assert_eq!(
        editor.document.nodes[&master]
            .layout
            .as_ref()
            .unwrap()
            .padding
            .left,
        40.0
    );
    let mut independent = Editor::new(before);
    bind(
        &mut independent,
        instance,
        LayoutVariableField::PaddingLeft,
        None,
    );
    independent.sync_instances(master);
    assert!(!independent.document.nodes[&instance]
        .variable_bindings
        .contains_key(LayoutVariableField::PaddingLeft.key()));
    assert_eq!(
        independent.document.nodes[&instance]
            .layout
            .as_ref()
            .unwrap()
            .padding
            .left,
        30.5
    );
}

#[test]
fn variable_authoring_rejects_active_transactions_and_overflow_without_partial_reflow() {
    let (mut editor, frame, _) = fixture();
    let spacing = add(&mut editor, "Spacing", VariableValue::Number(10.0));
    bind(
        &mut editor,
        frame,
        LayoutVariableField::PaddingLeft,
        Some(spacing),
    );
    bind(
        &mut editor,
        frame,
        LayoutVariableField::PaddingRight,
        Some(spacing),
    );
    editor
        .document
        .nodes
        .get_mut(&frame)
        .unwrap()
        .layout
        .as_mut()
        .unwrap()
        .width = SizingMode::Hug;
    editor.document.layout_all();
    editor.refresh();
    let before = editor.document.clone();
    let depth = editor.history_len();
    assert!(save(&mut editor, spacing, "Default", f64::MAX).is_err());
    assert_eq!(editor.document, before);
    assert_eq!(editor.history_len(), depth);
    editor.begin_transaction("Owned gesture");
    assert!(save(&mut editor, spacing, "Default", 20.0)
        .unwrap_err()
        .to_string()
        .contains("Finish the current edit"));
    assert!(editor.has_active_transaction());
    assert_eq!(editor.document, before);
    assert_eq!(editor.history_len(), depth);
    editor.cancel_transaction();
}

#[test]
fn empty_grid_and_nested_hug_spacing_follow_existing_layout_rules() {
    let (mut editor, frame, [a, b]) = fixture();
    let spacing = add(&mut editor, "Spacing", VariableValue::Number(14.25));
    {
        let node = editor.document.nodes.get_mut(&frame).unwrap();
        let layout = node.layout.as_mut().unwrap();
        layout.axis = LayoutAxis::Grid;
        layout.columns = 1;
    }
    bind(
        &mut editor,
        frame,
        LayoutVariableField::CrossGap,
        Some(spacing),
    );
    assert_eq!(editor.document.nodes[&a].y, 10.0);
    assert_eq!(editor.document.nodes[&b].y, 49.25);
    editor
        .document
        .nodes
        .get_mut(&frame)
        .unwrap()
        .layout
        .as_mut()
        .unwrap()
        .height = SizingMode::Hug;
    bind(
        &mut editor,
        frame,
        LayoutVariableField::PaddingBottom,
        Some(spacing),
    );
    assert_eq!(editor.document.nodes[&frame].height, 93.5);
    let mut empty = Node::new(NodeKind::Frame, "Empty", 400.0, 0.0, 1.0, 1.0);
    empty.layout = Some(AutoLayout {
        width: SizingMode::Hug,
        height: SizingMode::Hug,
        ..AutoLayout::default()
    });
    let empty = editor.insert_node(editor.active_page, empty).unwrap();
    for field in LayoutVariableField::ALL.into_iter().take(4) {
        bind(&mut editor, empty, field, Some(spacing));
    }
    assert_eq!(editor.document.nodes[&empty].width, 28.5);
    assert_eq!(editor.document.nodes[&empty].height, 28.5);
}

#[test]
fn unchecked_legacy_spacing_bindings_cannot_inject_invalid_geometry() {
    let (mut editor, frame, _) = fixture();
    let spacing = add(&mut editor, "Spacing", VariableValue::Number(10.0));
    bind(&mut editor, frame, LayoutVariableField::Gap, Some(spacing));
    for value in [-20.0, f64::INFINITY, f64::NAN] {
        editor
            .document
            .variables
            .get_mut(&spacing)
            .unwrap()
            .values
            .insert("Default".into(), VariableValue::Number(value));
        assert!(editor.document.validate_variables().is_err());
        editor.document.apply_variables();
        assert_eq!(
            editor.document.nodes[&frame].layout.as_ref().unwrap().gap,
            10.0
        );
    }
}

#[test]
fn identical_binding_and_mode_do_not_arrange_unsettled_geometry_or_consume_redo() {
    let (mut editor, frame, [child, _]) = fixture();
    let spacing = add(&mut editor, "Spacing", VariableValue::Number(10.0));
    bind(
        &mut editor,
        frame,
        LayoutVariableField::PaddingLeft,
        Some(spacing),
    );
    save(&mut editor, spacing, "Default", 24.0).unwrap();
    let after = editor.document.to_json().unwrap();
    assert!(editor.undo());
    let restored = editor.document.to_json().unwrap();
    editor.document.nodes.get_mut(&child).unwrap().x = 123.5;
    editor.refresh();
    let before = editor.document.clone();
    let token = editor.render_change_token();
    let depth = editor.history_len();
    for edit in [
        VariableEdit::Bind {
            nodes: vec![frame],
            property: "layout.padding.left".into(),
            variable: Some(spacing),
        },
        VariableEdit::ActivateMode {
            mode: "Default".into(),
        },
    ] {
        assert!(!editor.edit_variables(edit).unwrap().changed);
        assert_eq!(editor.document, before);
        assert_eq!(editor.history_len(), depth);
        assert_eq!(editor.render_change_token(), token);
        assert!(editor.can_redo());
    }
    // Restore the external test-only perturbation before exercising the actual
    // retained redo entry, which is independent of the no-op command.
    editor.document.nodes.get_mut(&child).unwrap().x = 10.0;
    editor.refresh();
    assert_eq!(editor.document.to_json().unwrap(), restored);
    assert!(editor.redo());
    assert_eq!(editor.document.to_json().unwrap(), after);
}
