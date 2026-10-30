use super::*;
use crate::controls::*;
use crate::test_support::{TestDock, TestNode, TestRoot, TestWindow};
use ferrodock_model::collections::ListHandle;
use ferroui_system::collections::generic::Dictionary;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn group_and_global_target_rules_preserve_their_distinct_asymmetry() {
    for source_group in [None, Some(""), Some(" "), Some("A"), Some("a")] {
        for target_group in [None, Some(""), Some(" "), Some("A"), Some("a")] {
            let source = TestNode::new(source_group);
            let target = TestDock::new(target_group);
            let source_group = source_group.filter(|group| !group.is_empty());
            let target_group = target_group.filter(|group| !group.is_empty());
            assert_eq!(
                DockGroupValidator::validate_docking_groups(source.clone(), target.clone()),
                source_group == target_group
            );
            assert_eq!(
                DockGroupValidator::validate_global_docking(source, target),
                source_group.is_none() || source_group == target_group
            );
        }
    }
}

#[test]
fn empty_groups_inherit_while_whitespace_groups_override_the_owner() {
    let parent = TestDock::new(Some("Parent"));
    let child = TestNode::new(Some(""));
    child.set_owner(Some(parent));
    assert_eq!(
        DockGroupValidator::get_effective_dock_group(child.clone()).as_deref(),
        Some("Parent")
    );
    child.set_dock_group(Some(" ".into()));
    assert_eq!(
        DockGroupValidator::get_effective_dock_group(child).as_deref(),
        Some(" ")
    );
}

#[test]
fn disabled_global_docking_cascades_through_leaf_and_dock_owners() {
    let root = TestRoot::new(None);
    let intermediate = TestNode::new(None);
    let dock = TestDock::new(None);
    let leaf = TestNode::new(None);
    intermediate.set_owner(Some(root.clone()));
    dock.set_owner(Some(intermediate));
    leaf.set_owner(Some(dock.clone()));
    assert!(DockInheritanceHelper::get_effective_enable_global_docking(
        leaf.clone()
    ));
    root.set_enable_global_docking(false);
    assert!(!DockInheritanceHelper::get_effective_enable_global_docking(
        leaf.clone()
    ));
    assert!(!DockInheritanceHelper::get_effective_enable_global_docking(
        root.clone()
    ));
    root.set_enable_global_docking(true);
    dock.set_enable_global_docking(false);
    assert!(!DockInheritanceHelper::get_effective_enable_global_docking(
        leaf.clone()
    ));
    dock.set_enable_global_docking(true);
    assert!(DockInheritanceHelper::get_effective_enable_global_docking(
        leaf
    ));
    assert!(DockInheritanceHelper::get_effective_enable_global_docking(
        TestNode::new(None)
    ));
}

#[test]
fn shared_visible_lists_keep_identity_observable_mutation_and_upcasts() {
    let root: Rc<dyn IRootDock> = TestRoot::new(None);
    let list = ListHandle::<Option<Rc<dyn IDockable>>>::new();
    root.set_visible_dockables(Some(list.clone()));
    let from_contract = root.visible_dockables().unwrap();
    assert!(from_contract == list);
    let child: Rc<dyn IDockable> = TestNode::new(None);
    list.add(Some(child.clone()));
    assert!(Rc::ptr_eq(
        (from_contract.get(0))
            .as_ref()
            .expect("nonnull identity fixture"),
        &child
    ));
    let upcast: Rc<dyn IDockable> = root.clone();
    assert!(Rc::ptr_eq(&upcast.clone().as_root_dock().unwrap(), &root));
    assert!(upcast.as_dock().is_some());
    assert!(child.as_dock().is_none());
}

#[test]
fn plain_visible_list_enumerator_fails_when_a_group_getter_mutates_the_list() {
    let source = TestNode::new(None);
    let target = TestDock::new(None);
    let child = TestNode::new(None);
    let visible = ListHandle::from_plain(vec![Some(child.clone() as Rc<dyn IDockable>)]);
    target.set_visible_dockables(Some(visible.clone()));
    let weak = Rc::downgrade(&visible.as_list());
    *child.node.on_group_read.borrow_mut() = Some(Rc::new(move || weak.upgrade().unwrap().clear()));
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        DockGroupValidator::validate_docking_groups_in_dock(source, target)
    }))
    .is_err());
    assert!(visible.is_empty());
}

#[test]
fn factory_dictionary_keys_use_reference_identity_even_when_ids_match() {
    let first: Rc<dyn IDockable> = TestNode::new(None);
    let second: Rc<dyn IDockable> = TestNode::new(None);
    first.set_id("same".into());
    second.set_id("same".into());
    let map: DockableObjectMap = Rc::new(RefCell::new(Dictionary::new()));
    map.borrow_mut().insert(first.clone(), Rc::new(1_i32));
    map.borrow_mut().insert(second.clone(), Rc::new(2_i32));
    assert_eq!(map.borrow().len(), 2);
    let value = map.borrow().get(&first).unwrap().clone();
    assert_eq!(value.downcast_ref::<i32>(), Some(&1));
    assert!(first == first.clone());
    assert!(first != second);
}

#[test]
fn window_options_apply_in_source_order_and_preserve_an_absent_taskbar_override() {
    let options = DockWindowOptions::new();
    let window = TestWindow::new();
    window.set_show_in_taskbar(Some(false));
    window.window.calls.borrow_mut().clear();
    options.apply_to(Some(&*window));
    assert_eq!(
        *window.window.calls.borrow(),
        ["OwnerMode", "ParentWindow", "IsModal"]
    );
    assert_eq!(window.owner_mode(), DockWindowOwnerMode::Default);
    assert!(window.parent_window().is_none());
    assert!(!window.is_modal());
    assert_eq!(window.show_in_taskbar(), Some(false));
    let parent: Rc<dyn IDockWindow> = TestWindow::new();
    options.set_owner_mode(DockWindowOwnerMode::ParentWindow);
    options.set_parent_window(Some(parent.clone()));
    options.set_is_modal(true);
    for value in [false, true] {
        window.window.calls.borrow_mut().clear();
        options.set_show_in_taskbar(Some(value));
        options.apply_to(Some(&*window));
        assert_eq!(
            *window.window.calls.borrow(),
            ["OwnerMode", "ParentWindow", "IsModal", "ShowInTaskbar"]
        );
        assert_eq!(window.owner_mode(), DockWindowOwnerMode::ParentWindow);
        assert!(Rc::ptr_eq(&window.parent_window().unwrap(), &parent));
        assert!(window.is_modal());
        assert_eq!(window.show_in_taskbar(), Some(value));
    }
    assert!(options == options.clone());
    assert!(options != DockWindowOptions::new());
}

#[test]
fn window_options_retain_the_parent_until_cleared() {
    let options = DockWindowOptions::new();
    let parent: Rc<dyn IDockWindow> = TestWindow::new();
    let weak = Rc::downgrade(&parent);
    options.set_parent_window(Some(parent));
    assert!(weak.upgrade().is_some());
    options.set_parent_window(None);
    assert!(weak.upgrade().is_none());
    assert!(options.parent_window().is_none());
}
