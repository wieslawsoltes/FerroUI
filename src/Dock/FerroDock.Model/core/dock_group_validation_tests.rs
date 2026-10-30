// Port of the source group validation tests; calls the public validator directly.
use super::*;
use crate::test_support::{TestDock, TestNode};
use ferrodock_model::collections::ListHandle;
use std::rc::Rc;

#[test]
fn get_effective_dock_group_should_return_own_group() {
    let parent = TestNode::new(Some("ParentGroup"));
    let child = TestNode::new(Some("ChildGroup"));
    child.set_owner(Some(parent.clone()));
    let result = DockGroupValidator::get_effective_dock_group(child.clone());
    assert_eq!(result.as_deref(), Some("ChildGroup"));
}

#[test]
fn get_effective_dock_group_should_inherit_from_parent() {
    let parent = TestNode::new(Some("ParentGroup"));
    let child = TestNode::new(None);
    child.set_owner(Some(parent.clone()));
    let result = DockGroupValidator::get_effective_dock_group(child.clone());
    assert_eq!(result.as_deref(), Some("ParentGroup"));
}

#[test]
fn get_effective_dock_group_should_walk_up_hierarchy() {
    let grandparent = TestNode::new(Some("GrandparentGroup"));
    let parent = TestNode::new(None);
    parent.set_owner(Some(grandparent.clone()));
    let child = TestNode::new(None);
    child.set_owner(Some(parent.clone()));
    let result = DockGroupValidator::get_effective_dock_group(child.clone());
    assert_eq!(result.as_deref(), Some("GrandparentGroup"));
}

#[test]
fn get_effective_dock_group_no_group_in_hierarchy_should_return_null() {
    let parent = TestNode::new(None);
    let child = TestNode::new(None);
    child.set_owner(Some(parent.clone()));
    let result = DockGroupValidator::get_effective_dock_group(child.clone());
    assert!(result.is_none());
}

#[test]
fn validate_docking_groups_same_group_should_allow() {
    let source = TestNode::new(Some("GroupA"));
    let target = TestNode::new(Some("GroupA"));
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert!(result);
}

#[test]
fn validate_docking_groups_different_group_should_reject() {
    let source = TestNode::new(Some("GroupA"));
    let target = TestNode::new(Some("GroupB"));
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert!(!result);
}

#[test]
fn validate_docking_groups_source_null_group_should_reject() {
    let source = TestNode::new(None);
    let target = TestNode::new(Some("GroupA"));
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert!(!result);
}

#[test]
fn validate_docking_groups_target_null_group_should_reject() {
    let source = TestNode::new(Some("GroupA"));
    let target = TestNode::new(None);
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert!(!result);
}

#[test]
fn validate_docking_groups_both_null_groups_should_allow() {
    let source = TestNode::new(None);
    let target = TestNode::new(None);
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert!(result);
}

#[test]
fn validate_docking_groups_empty_string_groups_should_allow() {
    let source = TestNode::new(Some(""));
    let target = TestNode::new(Some(""));
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert!(result);
}

#[test]
fn validate_docking_groups_case_sensitive_should_reject() {
    let source = TestNode::new(Some("GroupA"));
    let target = TestNode::new(Some("groupa"));
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert!(!result);
}

#[test]
fn validate_docking_groups_in_dock_empty_dock_should_allow() {
    let source = TestNode::new(Some("GroupA"));
    let target_dock = TestDock::new(Some("GroupB"));
    let result =
        DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target_dock.clone());
    assert!(result);
}

#[test]
fn validate_docking_groups_in_dock_same_group_should_allow() {
    let source = TestNode::new(Some("GroupA"));
    let existing = TestNode::new(Some("GroupA"));
    let target_dock = TestDock::new(Some("GroupA"));
    target_dock
        .visible_dockables()
        .unwrap()
        .add(Some(existing.clone()));
    let result =
        DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target_dock.clone());
    assert!(result);
}

#[test]
fn validate_docking_groups_in_dock_different_group_should_reject() {
    let source = TestNode::new(Some("GroupA"));
    let existing = TestNode::new(Some("GroupB"));
    let target_dock = TestDock::new(Some("GroupB"));
    target_dock
        .visible_dockables()
        .unwrap()
        .add(Some(existing.clone()));
    let result =
        DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target_dock.clone());
    assert!(!result);
}

#[test]
fn dock_group_validator_validate_docking_groups_same_group_should_allow() {
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(Some("GroupA".into()));
    let target = TestNode::new(None);
    target.set_id("Target".into());
    target.set_dock_group(Some("GroupA".into()));
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert!(result);
}

#[test]
fn dock_group_validator_validate_docking_groups_different_group_should_reject() {
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(Some("GroupA".into()));
    let target = TestNode::new(None);
    target.set_id("Target".into());
    target.set_dock_group(Some("GroupB".into()));
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert!(!result);
}

#[test]
fn dock_group_validator_validate_docking_groups_both_null_groups_should_allow() {
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(None);
    let target = TestNode::new(None);
    target.set_id("Target".into());
    target.set_dock_group(None);
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert!(result);
}

#[test]
fn dock_group_validator_validate_docking_groups_mixed_states_should_reject() {
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(Some("GroupA".into()));
    let target = TestNode::new(None);
    target.set_id("Target".into());
    target.set_dock_group(None);
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert!(!result);
}

#[test]
fn dock_group_validator_validate_docking_groups_in_dock_empty_dock_should_allow() {
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(Some("GroupA".into()));
    let target_dock = TestDock::new(None);
    target_dock.set_id("TargetDock".into());
    target_dock.set_visible_dockables(Some(ListHandle::from_items(vec![])));
    let result =
        DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target_dock.clone());
    assert!(result);
}

#[test]
fn dock_group_validator_validate_docking_groups_in_dock_compatible_group_should_allow() {
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(Some("GroupA".into()));
    let existing = TestNode::new(None);
    existing.set_id("Existing".into());
    existing.set_dock_group(Some("GroupA".into()));
    let target_dock = TestDock::new(None);
    target_dock.set_id("TargetDock".into());
    target_dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        existing.clone() as Rc<dyn IDockable>
    )])));
    let result =
        DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target_dock.clone());
    assert!(result);
}

#[test]
fn dock_group_validator_validate_docking_groups_in_dock_incompatible_group_should_reject() {
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(Some("GroupA".into()));
    let existing = TestNode::new(None);
    existing.set_id("Existing".into());
    existing.set_dock_group(Some("GroupB".into()));
    let target_dock = TestDock::new(None);
    target_dock.set_id("TargetDock".into());
    target_dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        existing.clone() as Rc<dyn IDockable>
    )])));
    let result =
        DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target_dock.clone());
    assert!(!result);
}

#[test]
fn dock_group_validator_validate_docking_groups_all_scenarios_case_1() {
    let source_group: Option<&str> = None;
    let target_group: Option<&str> = None;
    let expected: bool = true;
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(source_group.map(str::to_owned));
    let target = TestNode::new(None);
    target.set_id("Target".into());
    target.set_dock_group(target_group.map(str::to_owned));
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert_eq!(result, expected);
}

#[test]
fn dock_group_validator_validate_docking_groups_all_scenarios_case_2() {
    let source_group: Option<&str> = Some("");
    let target_group: Option<&str> = Some("");
    let expected: bool = true;
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(source_group.map(str::to_owned));
    let target = TestNode::new(None);
    target.set_id("Target".into());
    target.set_dock_group(target_group.map(str::to_owned));
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert_eq!(result, expected);
}

#[test]
fn dock_group_validator_validate_docking_groups_all_scenarios_case_3() {
    let source_group: Option<&str> = Some("GroupA");
    let target_group: Option<&str> = Some("GroupA");
    let expected: bool = true;
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(source_group.map(str::to_owned));
    let target = TestNode::new(None);
    target.set_id("Target".into());
    target.set_dock_group(target_group.map(str::to_owned));
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert_eq!(result, expected);
}

#[test]
fn dock_group_validator_validate_docking_groups_all_scenarios_case_4() {
    let source_group: Option<&str> = Some("GroupA");
    let target_group: Option<&str> = Some("GroupB");
    let expected: bool = false;
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(source_group.map(str::to_owned));
    let target = TestNode::new(None);
    target.set_id("Target".into());
    target.set_dock_group(target_group.map(str::to_owned));
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert_eq!(result, expected);
}

#[test]
fn dock_group_validator_validate_docking_groups_all_scenarios_case_5() {
    let source_group: Option<&str> = None;
    let target_group: Option<&str> = Some("GroupA");
    let expected: bool = false;
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(source_group.map(str::to_owned));
    let target = TestNode::new(None);
    target.set_id("Target".into());
    target.set_dock_group(target_group.map(str::to_owned));
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert_eq!(result, expected);
}

#[test]
fn dock_group_validator_validate_docking_groups_all_scenarios_case_6() {
    let source_group: Option<&str> = Some("GroupA");
    let target_group: Option<&str> = None;
    let expected: bool = false;
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(source_group.map(str::to_owned));
    let target = TestNode::new(None);
    target.set_id("Target".into());
    target.set_dock_group(target_group.map(str::to_owned));
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert_eq!(result, expected);
}

#[test]
fn dock_group_validator_validate_docking_groups_all_scenarios_case_7() {
    let source_group: Option<&str> = Some("");
    let target_group: Option<&str> = Some("GroupA");
    let expected: bool = false;
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(source_group.map(str::to_owned));
    let target = TestNode::new(None);
    target.set_id("Target".into());
    target.set_dock_group(target_group.map(str::to_owned));
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert_eq!(result, expected);
}

#[test]
fn dock_group_validator_validate_docking_groups_all_scenarios_case_8() {
    let source_group: Option<&str> = Some("GroupA");
    let target_group: Option<&str> = Some("");
    let expected: bool = false;
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(source_group.map(str::to_owned));
    let target = TestNode::new(None);
    target.set_id("Target".into());
    target.set_dock_group(target_group.map(str::to_owned));
    let result = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
    assert_eq!(result, expected);
}

#[test]
fn dock_group_validator_validate_docking_groups_in_dock_all_scenarios_case_1() {
    let source_group: Option<&str> = None;
    let existing_group: Option<&str> = None;
    let expected: bool = true;
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(source_group.map(str::to_owned));
    let existing = TestNode::new(None);
    existing.set_id("Existing".into());
    existing.set_dock_group(existing_group.map(str::to_owned));
    let target_dock = TestDock::new(None);
    target_dock.set_id("TargetDock".into());
    target_dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        existing.clone() as Rc<dyn IDockable>
    )])));
    let result =
        DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target_dock.clone());
    assert_eq!(result, expected);
}

#[test]
fn dock_group_validator_validate_docking_groups_in_dock_all_scenarios_case_2() {
    let source_group: Option<&str> = Some("");
    let existing_group: Option<&str> = Some("");
    let expected: bool = true;
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(source_group.map(str::to_owned));
    let existing = TestNode::new(None);
    existing.set_id("Existing".into());
    existing.set_dock_group(existing_group.map(str::to_owned));
    let target_dock = TestDock::new(None);
    target_dock.set_id("TargetDock".into());
    target_dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        existing.clone() as Rc<dyn IDockable>
    )])));
    let result =
        DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target_dock.clone());
    assert_eq!(result, expected);
}

#[test]
fn dock_group_validator_validate_docking_groups_in_dock_all_scenarios_case_3() {
    let source_group: Option<&str> = Some("GroupA");
    let existing_group: Option<&str> = Some("GroupA");
    let expected: bool = true;
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(source_group.map(str::to_owned));
    let existing = TestNode::new(None);
    existing.set_id("Existing".into());
    existing.set_dock_group(existing_group.map(str::to_owned));
    let target_dock = TestDock::new(None);
    target_dock.set_id("TargetDock".into());
    target_dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        existing.clone() as Rc<dyn IDockable>
    )])));
    let result =
        DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target_dock.clone());
    assert_eq!(result, expected);
}

#[test]
fn dock_group_validator_validate_docking_groups_in_dock_all_scenarios_case_4() {
    let source_group: Option<&str> = Some("GroupA");
    let existing_group: Option<&str> = Some("GroupB");
    let expected: bool = false;
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(source_group.map(str::to_owned));
    let existing = TestNode::new(None);
    existing.set_id("Existing".into());
    existing.set_dock_group(existing_group.map(str::to_owned));
    let target_dock = TestDock::new(None);
    target_dock.set_id("TargetDock".into());
    target_dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        existing.clone() as Rc<dyn IDockable>
    )])));
    let result =
        DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target_dock.clone());
    assert_eq!(result, expected);
}

#[test]
fn dock_group_validator_validate_docking_groups_in_dock_all_scenarios_case_5() {
    let source_group: Option<&str> = None;
    let existing_group: Option<&str> = Some("GroupA");
    let expected: bool = false;
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(source_group.map(str::to_owned));
    let existing = TestNode::new(None);
    existing.set_id("Existing".into());
    existing.set_dock_group(existing_group.map(str::to_owned));
    let target_dock = TestDock::new(None);
    target_dock.set_id("TargetDock".into());
    target_dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        existing.clone() as Rc<dyn IDockable>
    )])));
    let result =
        DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target_dock.clone());
    assert_eq!(result, expected);
}

#[test]
fn dock_group_validator_validate_docking_groups_in_dock_all_scenarios_case_6() {
    let source_group: Option<&str> = Some("GroupA");
    let existing_group: Option<&str> = None;
    let expected: bool = false;
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(source_group.map(str::to_owned));
    let existing = TestNode::new(None);
    existing.set_id("Existing".into());
    existing.set_dock_group(existing_group.map(str::to_owned));
    let target_dock = TestDock::new(None);
    target_dock.set_id("TargetDock".into());
    target_dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        existing.clone() as Rc<dyn IDockable>
    )])));
    let result =
        DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target_dock.clone());
    assert_eq!(result, expected);
}

#[test]
fn dock_group_validator_validate_docking_groups_in_dock_all_scenarios_case_7() {
    let source_group: Option<&str> = Some("");
    let existing_group: Option<&str> = Some("GroupA");
    let expected: bool = false;
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(source_group.map(str::to_owned));
    let existing = TestNode::new(None);
    existing.set_id("Existing".into());
    existing.set_dock_group(existing_group.map(str::to_owned));
    let target_dock = TestDock::new(None);
    target_dock.set_id("TargetDock".into());
    target_dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        existing.clone() as Rc<dyn IDockable>
    )])));
    let result =
        DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target_dock.clone());
    assert_eq!(result, expected);
}

#[test]
fn dock_group_validator_validate_docking_groups_in_dock_all_scenarios_case_8() {
    let source_group: Option<&str> = Some("GroupA");
    let existing_group: Option<&str> = Some("");
    let expected: bool = false;
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(source_group.map(str::to_owned));
    let existing = TestNode::new(None);
    existing.set_id("Existing".into());
    existing.set_dock_group(existing_group.map(str::to_owned));
    let target_dock = TestDock::new(None);
    target_dock.set_id("TargetDock".into());
    target_dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        existing.clone() as Rc<dyn IDockable>
    )])));
    let result =
        DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target_dock.clone());
    assert_eq!(result, expected);
}

#[test]
fn dock_group_validator_validate_docking_groups_in_dock_null_visible_dockables_should_allow() {
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(Some("GroupA".into()));
    let target_dock = TestDock::new(None);
    target_dock.set_id("TargetDock".into());
    target_dock.set_visible_dockables(None);
    let result =
        DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target_dock.clone());
    assert!(result);
}

#[test]
fn dock_group_validator_validate_docking_groups_in_dock_multiple_compatible_dockables_should_allow()
{
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(Some("GroupA".into()));
    let existing1 = TestNode::new(None);
    existing1.set_id("Existing1".into());
    existing1.set_dock_group(Some("GroupA".into()));
    let existing2 = TestNode::new(None);
    existing2.set_id("Existing2".into());
    existing2.set_dock_group(Some("GroupA".into()));
    let target_dock = TestDock::new(None);
    target_dock.set_id("TargetDock".into());
    target_dock.set_visible_dockables(Some(ListHandle::from_items(vec![
        Some(existing1.clone() as Rc<dyn IDockable>),
        Some(existing2.clone() as Rc<dyn IDockable>),
    ])));
    let result =
        DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target_dock.clone());
    assert!(result);
}

#[test]
fn dock_group_validator_validate_docking_groups_in_dock_one_incompatible_dockable_should_reject() {
    let source = TestNode::new(None);
    source.set_id("Source".into());
    source.set_dock_group(Some("GroupA".into()));
    let existing1 = TestNode::new(None);
    existing1.set_id("Existing1".into());
    existing1.set_dock_group(Some("GroupA".into()));
    let existing2 = TestNode::new(None);
    existing2.set_id("Existing2".into());
    existing2.set_dock_group(Some("GroupB".into()));
    let target_dock = TestDock::new(None);
    target_dock.set_id("TargetDock".into());
    target_dock.set_visible_dockables(Some(ListHandle::from_items(vec![
        Some(existing1.clone() as Rc<dyn IDockable>),
        Some(existing2.clone() as Rc<dyn IDockable>),
    ])));
    let result =
        DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target_dock.clone());
    assert!(!result);
}

#[test]
fn dock_group_validator_get_effective_dock_group_inheritance_works_correctly_case_1() {
    let child_group: Option<&str> = None;
    let parent_group: Option<&str> = None;
    let grandparent_group: Option<&str> = None;
    let grandparent = TestDock::new(None);
    grandparent.set_id("Grandparent".into());
    grandparent.set_dock_group(grandparent_group.map(str::to_owned));
    let parent = TestDock::new(None);
    parent.set_id("Parent".into());
    parent.set_dock_group(parent_group.map(str::to_owned));
    parent.set_owner(Some(grandparent.clone()));
    let child = TestNode::new(None);
    child.set_id("Child".into());
    child.set_dock_group(child_group.map(str::to_owned));
    child.set_owner(Some(parent.clone()));
    let effective_group = DockGroupValidator::get_effective_dock_group(child.clone());
    let expected_group = child_group.or(parent_group).or(grandparent_group);
    assert_eq!(effective_group.as_deref(), expected_group);
}

#[test]
fn dock_group_validator_get_effective_dock_group_inheritance_works_correctly_case_2() {
    let child_group: Option<&str> = None;
    let parent_group: Option<&str> = Some("GroupA");
    let grandparent_group: Option<&str> = None;
    let grandparent = TestDock::new(None);
    grandparent.set_id("Grandparent".into());
    grandparent.set_dock_group(grandparent_group.map(str::to_owned));
    let parent = TestDock::new(None);
    parent.set_id("Parent".into());
    parent.set_dock_group(parent_group.map(str::to_owned));
    parent.set_owner(Some(grandparent.clone()));
    let child = TestNode::new(None);
    child.set_id("Child".into());
    child.set_dock_group(child_group.map(str::to_owned));
    child.set_owner(Some(parent.clone()));
    let effective_group = DockGroupValidator::get_effective_dock_group(child.clone());
    let expected_group = child_group.or(parent_group).or(grandparent_group);
    assert_eq!(effective_group.as_deref(), expected_group);
}

#[test]
fn dock_group_validator_get_effective_dock_group_inheritance_works_correctly_case_3() {
    let child_group: Option<&str> = Some("GroupB");
    let parent_group: Option<&str> = Some("GroupA");
    let grandparent_group: Option<&str> = None;
    let grandparent = TestDock::new(None);
    grandparent.set_id("Grandparent".into());
    grandparent.set_dock_group(grandparent_group.map(str::to_owned));
    let parent = TestDock::new(None);
    parent.set_id("Parent".into());
    parent.set_dock_group(parent_group.map(str::to_owned));
    parent.set_owner(Some(grandparent.clone()));
    let child = TestNode::new(None);
    child.set_id("Child".into());
    child.set_dock_group(child_group.map(str::to_owned));
    child.set_owner(Some(parent.clone()));
    let effective_group = DockGroupValidator::get_effective_dock_group(child.clone());
    let expected_group = child_group.or(parent_group).or(grandparent_group);
    assert_eq!(effective_group.as_deref(), expected_group);
}

#[test]
fn dock_group_validator_get_effective_dock_group_inheritance_works_correctly_case_4() {
    let child_group: Option<&str> = None;
    let parent_group: Option<&str> = None;
    let grandparent_group: Option<&str> = Some("GroupC");
    let grandparent = TestDock::new(None);
    grandparent.set_id("Grandparent".into());
    grandparent.set_dock_group(grandparent_group.map(str::to_owned));
    let parent = TestDock::new(None);
    parent.set_id("Parent".into());
    parent.set_dock_group(parent_group.map(str::to_owned));
    parent.set_owner(Some(grandparent.clone()));
    let child = TestNode::new(None);
    child.set_id("Child".into());
    child.set_dock_group(child_group.map(str::to_owned));
    child.set_owner(Some(parent.clone()));
    let effective_group = DockGroupValidator::get_effective_dock_group(child.clone());
    let expected_group = child_group.or(parent_group).or(grandparent_group);
    assert_eq!(effective_group.as_deref(), expected_group);
}
