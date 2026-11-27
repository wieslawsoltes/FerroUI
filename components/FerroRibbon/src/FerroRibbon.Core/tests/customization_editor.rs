//! Unchanged RibbonCustomizeDialog editing-member corpus; UI is verified separately.
use ferroribbon_core::collections::RibbonItems;
use ferroribbon_core::{
    RibbonString, customization_editor::*, localization::RibbonStrings, state::*,
};
use serde_json::{Value, json};
use std::rc::Rc;
fn text(value: &str) -> Option<RibbonString> {
    Some(value.into())
}
fn strings(values: &[&str]) -> RibbonStateStrings {
    RibbonItems::from_values(values.iter().map(|s| text(s)).collect())
}
fn utf8(value: Option<RibbonString>) -> Option<String> {
    value.map(|s| s.to_utf8().unwrap())
}
fn group(id: &str, header: &str, original: Option<&str>) -> RibbonCustomizationGroup {
    RibbonCustomizationGroup {
        id: text(id),
        header: text(header),
        original_header: original.and_then(text),
    }
}
fn build() -> RibbonCustomizationEditor {
    let tabs = vec![
        RibbonCustomizationTab {
            id: text("a"),
            header: text("Home"),
            original_header: text("Original Home"),
            is_custom: false,
            is_contextual: false,
            groups: vec![
                group("g1", "Clipboard", Some("Original Clipboard")),
                group("g2", "Edit", None),
                group("cg", "Applied custom", None),
            ],
        },
        RibbonCustomizationTab {
            id: text("b"),
            header: text("View"),
            original_header: None,
            is_custom: false,
            is_contextual: false,
            groups: vec![group("b1", "Zoom", None)],
        },
        RibbonCustomizationTab {
            id: text("context"),
            header: text("Context"),
            original_header: None,
            is_custom: false,
            is_contextual: true,
            groups: vec![],
        },
        RibbonCustomizationTab {
            id: text("ui-custom"),
            header: text("Custom UI"),
            original_header: None,
            is_custom: true,
            is_contextual: false,
            groups: vec![],
        },
    ];
    let state = RibbonState::new();
    state.set_quick_access_item_ids(Some(strings(&["open", "own", "missing", "open"])));
    let c = state.customization().unwrap();
    c.set_tab_order(Some(strings(&["ct", "b", "a"])));
    c.group_order()
        .unwrap()
        .insert("a".into(), Some(strings(&["g2", "g1"])));
    c.set_hidden_tab_ids(Some(strings(&["a"])));
    c.set_hidden_group_ids(Some(strings(&["g1", "ctg"])));
    c.labels().unwrap().insert("g2".into(), None);
    let tab = RibbonCustomTab::new();
    tab.set_id(text("ct"));
    tab.set_label(text("Custom"));
    let g = RibbonCustomGroup::new();
    g.set_id(text("ctg"));
    g.set_label(text("Custom group"));
    g.set_item_ids(Some(strings(&["open"])));
    tab.groups().unwrap().add(Some(g));
    c.custom_tabs().unwrap().add(Some(tab));
    let g = RibbonCustomGroup::new();
    g.set_id(text("cg"));
    g.set_label(text("Tools"));
    g.set_tab_id(text("a"));
    g.set_item_ids(Some(strings(&["save", "missing", "save"])));
    c.custom_groups().unwrap().add(Some(g));
    let commands = vec![
        RibbonCustomizationCommand {
            id: "save".into(),
            label: text("Save"),
            path: text("Home"),
            icon: None,
        },
        RibbonCustomizationCommand {
            id: "open".into(),
            label: text("Open"),
            path: text("File"),
            icon: None,
        },
    ];
    RibbonCustomizationEditor::new(
        state,
        tabs,
        commands,
        RibbonStrings::current().custom_group_suffix(),
    )
    .unwrap()
}
fn snapshot(editor: &RibbonCustomizationEditor, name: &str, qat_selected: Option<&str>) -> Value {
    let own = vec![RibbonCustomizationCommand {
        id: "own".into(),
        label: text("Own QAT"),
        path: None,
        icon: None,
    }];
    let state = RibbonStateSerializer::serialize_default(Some(&editor.working_state()))
        .unwrap()
        .to_utf8()
        .unwrap();
    json!({"name":name,"rows":editor.rows().iter().map(|r|json!({"Id":utf8(r.id.clone()),"Label":utf8(r.label.clone()),"Level":r.kind.level(),"Hidden":r.hidden,"IsCustom":r.is_custom,"TabId":utf8(r.parent_id.clone())})).collect::<Vec<_>>(),"selected":editor.selected().and_then(|r|utf8(r.id.clone())),"qat":editor.quick_access_rows(&own).unwrap().iter().map(|r|json!({"Id":utf8(r.id.clone()),"Label":utf8(r.label.clone()),"Detail":utf8(r.detail.clone())})).collect::<Vec<_>>(),"qatSelected":qat_selected,"state":serde_json::from_str::<Value>(&state).unwrap()})
}
#[test]
fn unchanged_dialog_structure_qat_editing_and_partial_state_corpus() {
    let mut editor = build();
    let mut cases = vec![snapshot(&editor, "initial", None)];
    editor.select(text("g1").as_ref());
    editor.move_selected(-1).unwrap();
    cases.push(snapshot(&editor, "builtin-group-up", None));
    editor.select(text("cg/save").as_ref());
    editor.move_selected(1).unwrap();
    cases.push(snapshot(&editor, "duplicate-command-move", None));
    editor.remove_selected().unwrap();
    cases.push(snapshot(&editor, "remove-first-command", None));
    let before = editor.rows()[0].clone();
    editor
        .add_command(&"save".into(), "unused".into(), text("Unused"), text("a"))
        .unwrap();
    assert!(
        Rc::ptr_eq(&before, &editor.rows()[0]),
        "duplicate insertion doesn't refill/reselect"
    );
    cases.push(snapshot(&editor, "add-duplicate-noop", None));
    editor.set_visible(text("a").as_ref(), true).unwrap();
    editor.set_visible(text("cg").as_ref(), false).unwrap();
    cases.push(snapshot(&editor, "visibility-without-rebuild", None));
    editor.refresh().unwrap();
    cases.push(snapshot(&editor, "visibility-rebuilt", None));
    editor.select(text("ct").as_ref());
    editor.move_selected(1).unwrap();
    cases.push(snapshot(&editor, "tab-down", None));
    editor.select(text("cg").as_ref());
    editor.remove_selected().unwrap();
    cases.push(snapshot(&editor, "remove-custom-group", None));
    assert!(
        !editor.selected().unwrap().is_custom,
        "same ID reappears from the live built-in snapshot"
    );
    editor.select(text("a").as_ref());
    editor
        .new_group(
            "new-group".into(),
            RibbonStrings::current().new_group(),
            text("a"),
        )
        .unwrap();
    editor
        .add_command(&"open".into(), "unused".into(), None, text("a"))
        .unwrap();
    cases.push(snapshot(&editor, "new-group-and-command", None));
    editor
        .new_tab(
            "new-tab".into(),
            RibbonStrings::current().new_tab(),
            "new-tab-group".into(),
            RibbonStrings::current().new_group(),
        )
        .unwrap();
    cases.push(snapshot(&editor, "new-tab", None));
    editor.move_quick_access(text("open"), 1).unwrap();
    cases.push(snapshot(&editor, "qat-duplicate-move", Some("open")));
    editor
        .working_state()
        .quick_access_item_ids()
        .unwrap()
        .clear();
    assert_eq!(
        editor.move_quick_access(text("open"), 1),
        Err(RibbonCustomizationEditError::EmptyMove)
    );
    let mut expected: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/customization-editor.json"
    ))
    .unwrap();
    for case in expected["cases"].as_array_mut().unwrap() {
        case["state"] = serde_json::from_str(case["state"].as_str().unwrap()).unwrap();
    }
    assert_eq!(json!({"cases":cases,"error":"empty-move"}), expected);
}
#[test]
fn native_editor_preserves_working_state_identity_and_import_field_scope() {
    let mut editor = build();
    let state = editor.working_state();
    assert!(Rc::ptr_eq(&state, &editor.working_state()));
    let old_ids = state.quick_access_item_ids().unwrap();
    editor.move_quick_access(text("open"), 1).unwrap();
    assert!(Rc::ptr_eq(
        &old_ids,
        &state.quick_access_item_ids().unwrap()
    ));
    state.set_is_quick_access_visible(false);
    let imported = RibbonState::new();
    let customization = RibbonCustomization::new();
    imported.set_customization(Some(customization.clone()));
    imported.set_quick_access_item_ids(None);
    imported.set_quick_access_position(ferroribbon_core::RibbonQuickAccessPosition::BelowRibbon);
    editor.import_fields(&imported);
    assert!(Rc::ptr_eq(&customization, &state.customization().unwrap()));
    assert!(Rc::ptr_eq(
        &old_ids,
        &state.quick_access_item_ids().unwrap()
    ));
    assert!(!state.is_quick_access_visible());
    editor.refresh().unwrap();
    editor.reset_quick_access(
        [text("save")],
        ferroribbon_core::RibbonQuickAccessPosition::AboveRibbon,
    );
    assert!(!Rc::ptr_eq(
        &old_ids,
        &state.quick_access_item_ids().unwrap()
    ));
    assert!(state.is_quick_access_visible());
}

#[test]
fn original_move_clamp_duplicate_null_and_wrapping_boundaries() {
    let expected: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/customization-editor-moves.json"
    ))
    .unwrap();
    let mut cases = Vec::new();
    for case in expected.as_array().unwrap() {
        let editor = build();
        let state = editor.working_state();
        let ids = case["initial"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().and_then(text))
            .collect();
        state.set_quick_access_item_ids(Some(RibbonItems::from_values(ids)));
        let id = case["id"].as_str().and_then(text);
        let delta = i32::try_from(case["delta"].as_i64().unwrap()).unwrap();
        let result = editor.move_quick_access(id, delta);
        let (moved, error) = match result {
            Ok(value) => (Some(value), None),
            Err(RibbonCustomizationEditError::EmptyMove) => (None, Some("empty-move")),
            Err(error) => panic!("unexpected move failure: {error}"),
        };
        cases.push(json!({"initial":case["initial"],"id":case["id"],"delta":delta,"moved":moved,"error":error,"ids":state.quick_access_item_ids().unwrap().iter().map(utf8).collect::<Vec<_>>()}));
    }
    assert_eq!(json!(cases), expected);
}

#[test]
fn rename_uses_captured_row_and_preserves_nullable_label_overrides() {
    let mut editor = build();
    editor.select(text("g1").as_ref());
    let row = editor.selected().unwrap();
    editor.select(text("a").as_ref());
    editor.rename_row(&row, "Renamed group".into()).unwrap();
    let labels = editor
        .working_state()
        .customization()
        .unwrap()
        .labels()
        .unwrap();
    assert_eq!(labels.get(&"g1".into()), Some(text("Renamed group")));
    assert_eq!(labels.get(&"g2".into()), Some(None));
    assert_eq!(editor.selected().unwrap().id, text("a"));
    editor.rename_row(&row, " \t\u{3000}".into()).unwrap();
    assert_eq!(labels.get(&"g1".into()), Some(text("Renamed group")));
}
