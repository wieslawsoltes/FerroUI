//! Tests of the state mappers of the upstream-ported controls (#443, VSM-5), against WinUI's
//! `ChangeVisualState` of the button family and the list item, and the `UpdateVisualState` of the
//! text box.

use super::state_mappers::is_installed;
use crate::primitives::{TemplatedControl, ToggleButton};
use crate::presenters::TextPresenter;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::test_scope;
use crate::{Border, Button, CheckBox, Control, ListBoxItem, RadioButton, TextBox, VisualState, VisualStateGroup};
use crate::VisualStateManager;
use ferroui_base::{Ref, Visual};
use std::cell::RefCell;
use std::rc::Rc;

type Groups = Rc<RefCell<Vec<Ref<VisualStateGroup>>>>;

/// Applies a template to `control` whose root is a border with `groups` (a group name and its
/// state names); returns the groups as the template built them.
fn templated(control: &TemplatedControl, groups: &[(&str, &[&str])]) -> Groups {
    let built: Groups = Rc::default();
    let slot = built.clone();
    let spec: Vec<(String, Vec<String>)> = groups
        .iter()
        .map(|(name, states)| (name.to_string(), states.iter().map(|state| state.to_string()).collect()))
        .collect();
    control.set_template(Some(FuncControlTemplate::new(move |_, scope| {
        let root = Border::new();
        // The part a text box requires; the other controls ignore it.
        let presenter = TextPresenter::new();
        presenter.set_name(Some("PART_TextPresenter".to_string()));
        root.set_child(presenter.register_in_name_scope(&**scope));
        let collection = VisualStateManager::get_visual_state_groups(&root);
        slot.borrow_mut().clear();
        for (name, states) in &spec {
            let group = VisualStateGroup::with_name(name.clone());
            for state in states {
                group.states().add(VisualState::with_name(state.clone()));
            }
            collection.add(group.clone());
            slot.borrow_mut().push(group);
        }
        root.upcast()
    })));
    control.apply_template();
    built
}

/// The current state of each group, by name.
fn states(groups: &Groups) -> Vec<String> {
    groups
        .borrow()
        .iter()
        .map(|group| group.current_state().map(|state| state.name()).unwrap_or_default())
        .collect()
}

const COMMON: (&str, &[&str]) = ("CommonStates", &["Normal", "PointerOver", "Pressed", "Disabled"]);
const FOCUS: (&str, &[&str]) = ("FocusStates", &["Focused", "Unfocused", "PointerFocused"]);

#[test]
fn a_button_follows_its_pseudo_classes_and_focus() {
    let _scope = test_scope();
    let button = Button::new();
    let groups = templated(&button, &[COMMON, FOCUS]);
    assert!(is_installed(&button));
    assert_eq!(states(&groups), ["Normal", "Unfocused"]);
    button.set_pseudo_class(":pointerover", true);
    assert_eq!(states(&groups), ["PointerOver", "Unfocused"]);
    button.set_pseudo_class(":pressed", true);
    assert_eq!(states(&groups), ["Pressed", "Unfocused"]);
    button.set_pseudo_class(":focus", true);
    assert_eq!(states(&groups), ["Pressed", "PointerFocused"]);
    button.set_pseudo_class(":focus-visible", true);
    assert_eq!(states(&groups), ["Pressed", "Focused"]);
    // Disabled wins, and a disabled control is not shown focused.
    button.set_pseudo_class(":disabled", true);
    assert_eq!(states(&groups), ["Disabled", "Unfocused"]);
}

#[test]
fn no_mapper_runs_without_groups_and_a_new_template_replaces_it() {
    let _scope = test_scope();
    let button = Button::new();
    let groups = templated(&button, &[]);
    assert!(groups.borrow().is_empty());
    assert!(!is_installed(&button));
    let classes_before = button.classes().listener_count();
    let groups = templated(&button, &[COMMON]);
    assert!(is_installed(&button));
    assert_eq!(button.classes().listener_count(), classes_before + 1);
    assert_eq!(states(&groups), ["Normal"]);
    templated(&button, &[]);
    assert!(!is_installed(&button));
    assert_eq!(button.classes().listener_count(), classes_before);
}

#[test]
fn a_toggle_button_combines_the_check_and_the_common_state() {
    let _scope = test_scope();
    let toggle = ToggleButton::new();
    let groups = templated(
        &toggle,
        &[(
            "CommonStates",
            &[
                "Normal", "PointerOver", "Pressed", "Disabled", "Checked", "CheckedPointerOver", "CheckedPressed",
                "CheckedDisabled", "Indeterminate", "IndeterminatePointerOver", "IndeterminatePressed",
                "IndeterminateDisabled",
            ],
        )],
    );
    assert_eq!(states(&groups), ["Normal"]);
    toggle.set_is_checked(Some(true));
    assert_eq!(states(&groups), ["Checked"]);
    toggle.set_pseudo_class(":pointerover", true);
    assert_eq!(states(&groups), ["CheckedPointerOver"]);
    toggle.set_is_checked(None);
    assert_eq!(states(&groups), ["IndeterminatePointerOver"]);
}

#[test]
fn a_check_box_uses_the_combined_states_and_else_the_older_ones() {
    let _scope = test_scope();
    let combined = CheckBox::new();
    let groups = templated(
        &combined,
        &[("CombinedStates", &["UncheckedNormal", "UncheckedPressed", "CheckedNormal", "CheckedPressed"])],
    );
    assert_eq!(states(&groups), ["UncheckedNormal"]);
    combined.set_is_checked(Some(true));
    combined.set_pseudo_class(":pressed", true);
    assert_eq!(states(&groups), ["CheckedPressed"]);

    let older = CheckBox::new();
    let groups = templated(&older, &[COMMON, ("CheckStates", &["Checked", "Unchecked", "Indeterminate"])]);
    assert_eq!(states(&groups), ["Normal", "Unchecked"]);
    older.set_is_checked(None);
    assert_eq!(states(&groups), ["Normal", "Indeterminate"]);
}

#[test]
fn a_radio_button_has_common_and_check_states() {
    let _scope = test_scope();
    let radio = RadioButton::new();
    let groups = templated(&radio, &[COMMON, ("CheckStates", &["Checked", "Unchecked", "Indeterminate"])]);
    assert_eq!(states(&groups), ["Normal", "Unchecked"]);
    radio.set_is_checked(Some(true));
    radio.set_pseudo_class(":pointerover", true);
    assert_eq!(states(&groups), ["PointerOver", "Checked"]);
}

#[test]
fn a_list_box_item_has_common_selection_and_focus_states() {
    let _scope = test_scope();
    let item = ListBoxItem::new();
    let groups = templated(
        &item,
        &[
            COMMON,
            (
                "SelectionStates",
                &[
                    "Unselected",
                    "Selected",
                    "SelectedUnfocused",
                    "SelectedDisabled",
                    "SelectedPointerOver",
                    "SelectedPressed",
                ],
            ),
            FOCUS,
        ],
    );
    assert_eq!(states(&groups), ["Normal", "Unselected", "Unfocused"]);
    item.set_is_selected(true);
    // No selector with the keyboard focus around it.
    assert_eq!(states(&groups), ["Normal", "SelectedUnfocused", "Unfocused"]);
    item.set_pseudo_class(":pointerover", true);
    assert_eq!(states(&groups), ["PointerOver", "SelectedPointerOver", "Unfocused"]);
    // A disabled item whose content is not a control shows its own disabled state.
    item.set_pseudo_class(":disabled", true);
    assert_eq!(states(&groups), ["Disabled", "SelectedDisabled", "Unfocused"]);
    let content: Ref<Control> = Border::new().upcast();
    item.set_content(Some(Rc::new(content) as ferroui_base::BoxedValue));
    assert_eq!(states(&groups)[0], "Normal");
}

#[test]
fn a_text_box_shows_focus_and_its_delete_button() {
    let _scope = test_scope();
    let text_box = TextBox::new();
    let groups = templated(
        &text_box,
        &[COMMON, ("ButtonStates", &["ButtonVisible", "ButtonCollapsed"]), ("FocusStates", &["Focused"])],
    );
    // `Focused` is in the common group of WinUI's text box; it is found in any group.
    assert_eq!(states(&groups)[..2], ["Normal", "ButtonCollapsed"]);
    text_box.set_pseudo_class(":focus", true);
    assert_eq!(states(&groups), ["Normal", "ButtonCollapsed", "Focused"]);
    text_box.set_text(Some("abc"));
    assert_eq!(states(&groups), ["Normal", "ButtonVisible", "Focused"]);
    text_box.set_is_read_only(true);
    assert_eq!(states(&groups)[1], "ButtonCollapsed");
    text_box.set_pseudo_class(":focus", false);
    text_box.set_pseudo_class(":pointerover", true);
    assert_eq!(states(&groups)[0], "PointerOver");
    // The opacity of the control is not touched by any of it.
    assert_eq!(text_box.get_base_value(Visual::opacity_property()), None);
}
