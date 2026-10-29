use super::*;
use ferroui_base::threading::Dispatcher;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{Button, CheckBox, Control, TextBlock, TextBox};

fn state_of(control: &Control) -> NodeState {
    let peer = ControlAutomationPeer::create_peer_for_element(control);
    let mut state = NodeState::default();
    read_attributes(&peer, &mut state);
    state
}

fn aria(state: &NodeState, name: &str) -> String {
    state.states.iter().find(|(state_name, _)| *state_name == name).map(|(_, value)| value.clone()).unwrap_or_default()
}

#[test]
fn the_roles_of_the_common_control_types() {
    assert_eq!(aria_role(AutomationControlType::Button, false), Some("button"));
    assert_eq!(aria_role(AutomationControlType::CheckBox, true), Some("checkbox"));
    assert_eq!(aria_role(AutomationControlType::Edit, false), Some("textbox"));
    assert_eq!(aria_role(AutomationControlType::ListItem, false), Some("option"));
    assert_eq!(aria_role(AutomationControlType::MenuItem, true), Some("menuitemcheckbox"));
    assert_eq!(aria_role(AutomationControlType::TreeItem, false), Some("treeitem"));
    // A text is read by its text, without a role; so is a part of a control without one.
    assert_eq!(aria_role(AutomationControlType::Text, false), None);
    assert_eq!(aria_role(AutomationControlType::Thumb, false), None);
}

#[test]
fn a_button_has_its_role_name_and_type() {
    let button = Button::new();
    AutomationProperties::set_name(&button, Some("Save"));
    AutomationProperties::set_automation_id(&button, Some("SaveButton"));
    let state = state_of(&button);
    assert_eq!(state.role, "button");
    assert_eq!(state.name, "Save");
    assert_eq!(state.text, "");
    assert_eq!(state.class_name, "Button");
    assert_eq!(state.automation_id, "SaveButton");
    assert_eq!(aria(&state, "aria-disabled"), "");
    assert_eq!(aria(&state, "aria-checked"), "");

    button.set_is_enabled(false);
    assert_eq!(aria(&state_of(&button), "aria-disabled"), "true");
}

#[test]
fn a_check_box_is_checked_through_its_toggle_state() {
    let check_box = CheckBox::new();
    AutomationProperties::set_name(&check_box, Some("Remember me"));
    assert_eq!(aria(&state_of(&check_box), "aria-checked"), "false");
    check_box.set_is_checked(Some(true));
    assert_eq!(aria(&state_of(&check_box), "aria-checked"), "true");
    check_box.set_is_three_state(true);
    check_box.set_is_checked(None);
    assert_eq!(aria(&state_of(&check_box), "aria-checked"), "mixed");
}

#[test]
fn a_text_box_shows_its_value_as_its_text() {
    let _scope = Dispatcher::unit_test_scope();
    let text_box = TextBox::new();
    AutomationProperties::set_name(&text_box, Some("User name"));
    text_box.set_text(Some("ada"));
    let state = state_of(&text_box);
    assert_eq!(state.role, "textbox");
    assert_eq!(state.name, "User name");
    assert_eq!(state.text, "ada");
}

#[test]
fn a_text_block_is_read_by_its_text() {
    let text_block = TextBlock::new();
    text_block.set_text(Some("Hello"));
    let state = state_of(&text_block);
    assert_eq!(state.role, "");
    assert_eq!(state.name, "");
    assert_eq!(state.text, "Hello");
}

#[test]
fn a_new_element_gets_its_fields_and_no_empty_states() {
    let state = NodeState {
        role: "button".into(),
        name: "Save".into(),
        bounds: [1.0, 2.0, 30.0, 40.0],
        states: vec![("aria-disabled", String::new()), ("aria-checked", "true".into())],
        class_name: "Button".into(),
        ..NodeState::default()
    };
    let json = change_json(7, Some((3, 1)), None, &state).unwrap();
    assert_eq!(
        json,
        r#"{"i":7,"p":3,"x":1,"r":"button","n":"Save","h":"","t":"","ph":"","ct":"Button","ci":"","b":[1,2,30,40],"s":{"aria-checked":"true"}}"#
    );
}

#[test]
fn a_change_sends_only_the_fields_that_changed() {
    let before = NodeState {
        role: "checkbox".into(),
        name: "Remember me".into(),
        bounds: [0.0, 0.0, 100.0, 20.0],
        states: vec![("aria-disabled", String::new()), ("aria-checked", "false".into())],
        ..NodeState::default()
    };
    let mut after = before.clone();
    assert_eq!(change_json(4, None, Some(&before), &after), None);

    after.states[1].1 = "true".into();
    after.bounds = [0.0, 10.5, 100.0, 20.0];
    assert_eq!(change_json(4, None, Some(&before), &after).unwrap(), r#"{"i":4,"b":[0,10.5,100,20],"s":{"aria-checked":"true"}}"#);

    // A state that is no longer set is sent empty: the page removes the attribute.
    let cleared = NodeState { states: vec![("aria-disabled", String::new()), ("aria-checked", String::new())], ..before.clone() };
    assert_eq!(change_json(4, None, Some(&before), &cleared).unwrap(), r#"{"i":4,"s":{"aria-checked":""}}"#);
}

#[test]
fn strings_are_escaped_for_json() {
    let mut out = String::new();
    push_json_string(&mut out, "a \"quote\", a \\ and\na line\u{1}");
    assert_eq!(out, r#""a \"quote\", a \\ and\na line\u0001""#);
}
