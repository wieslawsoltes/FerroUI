use super::*;
use crate::{Color, Document, Editor, LayoutGridAxis, LayoutGridPattern, LocalStyleKind, NodeId,
    NodeKind, VariableEdit};

fn fixture() -> (Editor, NodeId) {
    let mut document = Document::new("Guide variables");
    let mut frame = Node::new(NodeKind::Frame, "Responsive", 0.0, 0.0, 320.0, 240.0);
    frame.set_layout_guides(&[LayoutGrid::default(), LayoutGrid {
        pattern: LayoutGridPattern::Columns { axis: LayoutGridAxis::default() }, ..LayoutGrid::default()
    }]);
    let frame = document.insert_node(1, frame).unwrap();
    (Editor::new(document), frame)
}
fn variable(editor: &mut Editor, name: &str, value: VariableValue) -> VariableId {
    editor.edit_variables(VariableEdit::Upsert { id: None, name: name.into(), collection: "Guides".into(),
        mode: "Default".into(), value, replace_all_modes: false }).unwrap().variable.unwrap()
}
fn bind(editor: &mut Editor, node: NodeId, index: usize, field: LayoutGuideVariableField, variable: Option<VariableId>) {
    editor.edit_variables(VariableEdit::Bind { nodes: vec![node], property: field.key(index), variable }).unwrap();
}

#[test]
fn guide_bindings_resolve_aliases_modes_and_exact_undo_redo() {
    let (mut editor, frame) = fixture();
    let count = variable(&mut editor, "Columns", VariableValue::Number(4.0));
    let alias = variable(&mut editor, "Count alias", VariableValue::Alias(count));
    let color = variable(&mut editor, "Ink", VariableValue::Color(Color::BLACK));
    let visible = variable(&mut editor, "Show", VariableValue::Boolean(false));
    for (field, value) in [(LayoutGuideVariableField::Count, alias), (LayoutGuideVariableField::Color, color), (LayoutGuideVariableField::Visible, visible)] {
        let before = editor.to_json().unwrap(); let depth = editor.history_len();
        bind(&mut editor, frame, 1, field, Some(value));
        let after = editor.to_json().unwrap();
        assert_eq!(editor.history_len(), depth + 1);
        assert!(editor.undo()); assert_eq!(editor.to_json().unwrap(), before);
        assert!(editor.redo()); assert_eq!(editor.to_json().unwrap(), after);
    }
    let grid = editor.document.nodes[&frame].layout_guide(1).unwrap();
    assert_eq!(grid.pattern.axis().unwrap().count, Some(4));
    assert_eq!(grid.color, Color::BLACK); assert!(!grid.visible);
    editor.edit_variables(VariableEdit::AddMode { name: "Wide".into(), copy_from: "Default".into() }).unwrap();
    editor.edit_variables(VariableEdit::Upsert { id: Some(count), name: "Columns".into(), collection: "Guides".into(),
        mode: "Wide".into(), value: VariableValue::Number(8.0), replace_all_modes: false }).unwrap();
    // Adding a mode activates it. Restore Default so the following activation
    // is a real edit whose own history entry must restore both mode and value.
    editor.edit_variables(VariableEdit::ActivateMode { mode: "Default".into() }).unwrap();
    assert_eq!(editor.document.nodes[&frame].layout_guide(1).unwrap().pattern.axis().unwrap().count, Some(4));
    let before = editor.to_json().unwrap();
    let depth = editor.history_len();
    editor.edit_variables(VariableEdit::ActivateMode { mode: "Wide".into() }).unwrap();
    assert_eq!(editor.history_len(), depth + 1);
    assert_eq!(editor.document.nodes[&frame].layout_guide(1).unwrap().pattern.axis().unwrap().count, Some(8));
    let after = editor.to_json().unwrap();
    assert!(editor.undo()); assert_eq!(editor.to_json().unwrap(), before);
    assert!(editor.redo()); assert_eq!(editor.to_json().unwrap(), after);
    assert_eq!(Document::from_json(&after).unwrap().to_json().unwrap(), after);
}

#[test]
fn bad_ranges_in_any_mode_wrong_targets_and_locks_are_atomic() {
    let (mut editor, frame) = fixture();
    let count = variable(&mut editor, "Count", VariableValue::Number(4.0));
    bind(&mut editor, frame, 1, LayoutGuideVariableField::Count, Some(count));
    editor.edit_variables(VariableEdit::AddMode { name: "Wide".into(), copy_from: "Default".into() }).unwrap();
    let before = editor.to_json().unwrap(); let revision = editor.revision; let history = editor.history_len();
    for value in [0.0, -1.0, 2.5, 1_000_001.0, f64::INFINITY] {
        assert!(editor.edit_variables(VariableEdit::Upsert { id: Some(count), name: "Count".into(), collection: "Guides".into(),
            mode: "Wide".into(), value: VariableValue::Number(value), replace_all_modes: false }).is_err());
        assert_eq!(editor.to_json().unwrap(), before); assert_eq!(editor.revision, revision); assert_eq!(editor.history_len(), history);
    }
    for property in ["layout_grid.0.count", "layout_grid.2.count", "layout_grid.01.count", "layout_grid.64.count", "layout_grid.1.alignment"] {
        assert!(editor.edit_variables(VariableEdit::Bind { nodes: vec![frame], property: property.into(), variable: Some(count) }).is_err());
        assert_eq!(editor.to_json().unwrap(), before);
    }
    editor.document.nodes.get_mut(&frame).unwrap().locked = true;
    let before = editor.to_json().unwrap();
    assert!(editor.edit_variables(VariableEdit::Bind { nodes: vec![frame], property: LayoutGuideVariableField::Count.key(1), variable: None }).is_err());
    assert_eq!(editor.to_json().unwrap(), before);
}

#[test]
fn reordering_duplicate_remove_and_orientation_keep_binding_ownership_exact() {
    let (mut editor, frame) = fixture();
    let count = variable(&mut editor, "Count", VariableValue::Number(4.0));
    let size = variable(&mut editor, "Spacing", VariableValue::Number(16.0));
    bind(&mut editor, frame, 1, LayoutGuideVariableField::Count, Some(count));
    bind(&mut editor, frame, 0, LayoutGuideVariableField::Size, Some(size));
    for edit in [LayoutGuideEdit::Move { index: 1, to: 0 }, LayoutGuideEdit::Duplicate(0), LayoutGuideEdit::Remove(1)] {
        let before = editor.to_json().unwrap();
        editor.edit_layout_guides(&[frame], edit).unwrap();
        let after = editor.to_json().unwrap();
        assert!(editor.undo()); assert_eq!(editor.to_json().unwrap(), before);
        assert!(editor.redo()); assert_eq!(editor.to_json().unwrap(), after);
        editor.document.validate().unwrap();
    }
    let node = &editor.document.nodes[&frame];
    assert_eq!(node.variable_bindings.get("layout_grid.0.count"), Some(&count));
    assert_eq!(node.variable_bindings.get("layout_grid.1.size"), Some(&size));
    editor.edit_layout_guides(&[frame], LayoutGuideEdit::Set { index: 0, property: LayoutGuideProperty::Kind(LayoutGridKind::Rows) }).unwrap();
    assert_eq!(editor.document.nodes[&frame].variable_bindings.get("layout_grid.0.count"), Some(&count));
    editor.edit_layout_guides(&[frame], LayoutGuideEdit::Set { index: 0, property: LayoutGuideProperty::Kind(LayoutGridKind::Square) }).unwrap();
    assert!(!editor.document.nodes[&frame].variable_bindings.contains_key("layout_grid.0.count"));
    editor.edit_layout_guides(&[frame], LayoutGuideEdit::Clear).unwrap();
    assert!(editor.document.nodes[&frame].variable_bindings.is_empty());
    editor.document.validate().unwrap();
}

#[test]
fn literal_equal_value_takes_ownership_and_unbind_freezes_resolved_value() {
    let (mut editor, frame) = fixture();
    let gutter = variable(&mut editor, "Gutter", VariableValue::Number(12.0));
    bind(&mut editor, frame, 1, LayoutGuideVariableField::Gutter, Some(gutter));
    let before = editor.to_json().unwrap(); let depth = editor.history_len();
    assert_eq!(editor.edit_layout_guides(&[frame], LayoutGuideEdit::Set { index: 1, property: LayoutGuideProperty::Gutter(12.0) }).unwrap(), 1);
    assert!(!editor.document.nodes[&frame].variable_bindings.contains_key("layout_grid.1.gutter"));
    assert_eq!(editor.history_len(), depth + 1);
    assert!(editor.undo()); assert_eq!(editor.to_json().unwrap(), before);
    bind(&mut editor, frame, 1, LayoutGuideVariableField::Gutter, None);
    assert_eq!(editor.document.nodes[&frame].layout_guide(1).unwrap().pattern.axis().unwrap().gutter, 12.0);
    assert!(!editor.document.nodes[&frame].variable_bindings.contains_key("layout_grid.1.gutter"));
}

#[test]
fn styles_own_stack_values_but_local_visibility_binding_survives_propagation() {
    let (mut editor, frame) = fixture();
    let style = editor.create_local_style_for_nodes(&[frame], LocalStyleKind::Grid, "Responsive", "").unwrap();
    let visible = variable(&mut editor, "Show", VariableValue::Boolean(false));
    bind(&mut editor, frame, 1, LayoutGuideVariableField::Visible, Some(visible));
    assert_eq!(editor.document.nodes[&frame].grid_style, Some(style));
    let mut definition = editor.document.styles[&style].clone();
    let crate::StyleValue::GridStack { grids } = &mut definition.value else { panic!("stack expected") };
    grids[1].size = 33.0;
    editor.update_local_style(style, &definition.name, &definition.description, definition.value).unwrap();
    assert_eq!(editor.document.nodes[&frame].variable_bindings.get("layout_grid.1.visible"), Some(&visible));
    assert!(!editor.document.nodes[&frame].layout_guide(1).unwrap().visible);
    let before = editor.to_json().unwrap();
    editor.update_local_style(style, "Responsive", "", crate::StyleValue::GridStack { grids: vec![LayoutGrid::default()] }).unwrap();
    assert!(!editor.document.nodes[&frame].variable_bindings.contains_key("layout_grid.1.visible"));
    editor.document.validate().unwrap();
    assert!(editor.undo()); assert_eq!(editor.to_json().unwrap(), before);
    let count = variable(&mut editor, "Count", VariableValue::Number(4.0));
    bind(&mut editor, frame, 1, LayoutGuideVariableField::Count, Some(count));
    assert_eq!(editor.document.nodes[&frame].grid_style, None);
    editor.apply_local_style(&[frame], style).unwrap();
    assert!(editor.document.nodes[&frame].variable_bindings.is_empty());
    assert_eq!(editor.document.nodes[&frame].grid_style, Some(style));
}

#[test]
fn component_refresh_preserves_overridden_guide_bindings_and_prunes_missing_visibility_slots() {
    let (mut editor, frame) = fixture();
    editor.selection = vec![frame];
    let component = editor.create_component().unwrap();
    let instance = editor.create_instance(component, 400.0, 0.0).unwrap();
    let visible = variable(&mut editor, "Show", VariableValue::Boolean(false));
    bind(&mut editor, instance, 1, LayoutGuideVariableField::Visible, Some(visible));
    editor.edit_layout_guides(&[component], LayoutGuideEdit::Remove(1)).unwrap();
    editor.refresh_component_instances(component).unwrap();
    assert!(!editor.document.nodes[&instance].variable_bindings.contains_key("layout_grid.1.visible"));
    let size = variable(&mut editor, "Spacing", VariableValue::Number(16.0));
    bind(&mut editor, instance, 0, LayoutGuideVariableField::Size, Some(size));
    editor.edit_layout_guides(&[component], LayoutGuideEdit::Clear).unwrap();
    editor.refresh_component_instances(component).unwrap();
    assert_eq!(editor.document.nodes[&instance].variable_bindings.get("layout_grid.0.size"), Some(&size));
    assert_eq!(editor.document.nodes[&instance].layout_guide(0).unwrap().size, 16.0);
    editor.document.validate().unwrap();
}

#[test]
fn legacy_square_commands_detach_literal_binding_and_clear_dangling_slots() {
    let (mut editor, frame) = fixture();
    let size = variable(&mut editor, "Spacing", VariableValue::Number(16.0));
    bind(&mut editor, frame, 0, LayoutGuideVariableField::Size, Some(size));
    let before = editor.to_json().unwrap();
    assert_eq!(editor.edit_layout_grids(&[frame], crate::LayoutGridEdit::Size(16.0)).unwrap(), 1);
    assert!(!editor.document.nodes[&frame].variable_bindings.contains_key("layout_grid.0.size"));
    assert!(editor.undo()); assert_eq!(editor.to_json().unwrap(), before);
    editor.edit_layout_grids(&[frame], crate::LayoutGridEdit::Remove).unwrap();
    assert!(editor.document.nodes[&frame].variable_bindings.is_empty());
    editor.document.validate().unwrap();
}
