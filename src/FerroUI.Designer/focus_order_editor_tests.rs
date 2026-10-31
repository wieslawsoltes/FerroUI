use super::*;
use crate::focus_order::{FocusLabelAssociation, FocusLabelTarget, FocusOrderScope};
use ferroui_base::ObjectType;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::testing::{TestServices, UnitTestApplication};

fn item(node: u64, label: &str, index: usize) -> FocusOrderItem {
    FocusOrderItem {
        runtime_id: node + 100,
        node: Some(node),
        label: label.into(),
        tab_index: i32::MAX,
        tree_order: index,
        focusable: true,
        is_tab_stop: true,
        visible: true,
        enabled: true,
        group: None,
        group_has_tab_stop: false,
    }
}
fn catalog() -> FocusOrderCatalog {
    FocusOrderCatalog {
        scopes: vec![FocusOrderScope {
            id: 100,
            node: Some(1),
            parent: None,
            label: "Form".into(),
            mode: KeyboardNavigationMode::Continue,
            blocked_by_ancestor: false,
            items: vec![item(2, "First", 0), item(3, "Second", 1)],
        }],
        labels: vec![FocusLabelAssociation {
            node: 4,
            label: "Caption".into(),
            current_target: None,
            candidates: vec![
                FocusLabelTarget {
                    node: 2,
                    label: "First".into(),
                },
                FocusLabelTarget {
                    node: 3,
                    label: "Second".into(),
                },
            ],
        }],
    }
}
fn named<T: ObjectType>(editor: &FocusOrderEditor, name: &str) -> Ref<T> {
    editor
        .control()
        .get_logical_descendants()
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("Missing focus editor control {name}"))
}
fn click(editor: &FocusOrderEditor, name: &str) {
    named::<Button>(editor, name).raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

#[test]
fn focus_editor_stages_order_eligibility_and_label_links_until_one_apply() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = FocusOrderEditor::new();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let captured = calls.clone();
    editor.on_apply(move |commands| {
        captured.borrow_mut().push(commands);
        Ok(())
    });
    editor.configure(catalog(), true);
    click(&editor, "FocusOrderDown");
    named::<CheckBox>(&editor, "FocusOrderTabStop").set_is_checked(Some(false));
    named::<ComboBox>(&editor, "FocusOrderTarget").set_selected_index(3);
    assert!(calls.borrow().is_empty());
    click(&editor, "FocusOrderApply");
    assert_eq!(calls.borrow().len(), 1);
    let commands = &calls.borrow()[0];
    assert!(commands.contains(&FocusOrderEdit::SetOrder {
        scope: 100,
        order: vec![3, 2]
    }));
    assert!(commands.contains(&FocusOrderEdit::SetTabStop {
        node: 2,
        value: false
    }));
    assert!(commands.contains(&FocusOrderEdit::SetLabelTarget {
        label: 4,
        target: Some(3)
    }));
}

#[test]
fn focus_editor_cancel_read_only_and_reconfigure_do_not_commit_drafts() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = FocusOrderEditor::new();
    let calls = Rc::new(Cell::new(0));
    let captured = calls.clone();
    editor.on_apply(move |_| {
        captured.set(captured.get() + 1);
        Err("Stale focus model".into())
    });
    editor.configure(catalog(), true);
    click(&editor, "FocusOrderDown");
    click(&editor, "FocusOrderCancel");
    click(&editor, "FocusOrderApply");
    assert_eq!(calls.get(), 0);
    editor.configure(catalog(), true);
    assert!(editor.draft().is_empty());
    click(&editor, "FocusOrderApply");
    assert_eq!(editor.error.text().as_deref(), Some("Stale focus model"));
    editor.configure(catalog(), false);
    click(&editor, "FocusOrderDown");
    click(&editor, "FocusOrderApply");
    assert_eq!(calls.get(), 1);
    assert!(editor.draft().is_empty());
}
