use crate::controls::*;
use crate::engine_test_support::{list, PolicyFactory};
use crate::Factory;
use ferrodock_model::core::*;
use ferrodock_model::DockService;
use std::rc::Rc;

#[test]
fn service_global_empty_and_local_populated_moves_keep_distinct_group_rules_and_validation_side_effects(
) {
    let factory = Factory::new();
    let root = factory.create_root_dock();
    let source = factory.create_tool_dock();
    source.set_is_collapsable(false);
    let target = factory.create_tool_dock();
    target.set_dock_group(Some("group".into()));
    let tool = factory.create_tool();
    factory.add_dockable(Some(root.clone()), Some(source.clone()));
    factory.add_dockable(Some(root.clone()), Some(target.clone()));
    factory.add_dockable(Some(source.clone()), Some(tool.clone()));
    let service = DockService::new();
    assert!(service.move_dockable_between_docks(
        tool.clone(),
        source.clone(),
        target.clone(),
        false
    ));
    assert!(Rc::ptr_eq(
        &tool.owner().unwrap(),
        &(source.clone() as Rc<dyn IDockable>)
    ));
    factory.add_dockable(Some(target.clone()), Some(factory.create_tool()));
    assert!(!service.move_dockable_between_docks(
        tool.clone(),
        source.clone(),
        target.clone(),
        true
    ));
    tool.set_dock_group(Some("group".into()));
    assert!(service.move_dockable_between_docks(
        tool.clone(),
        source.clone(),
        target.clone(),
        true
    ));
    assert!(Rc::ptr_eq(
        &tool.owner().unwrap(),
        &(target as Rc<dyn IDockable>)
    ));
}

#[test]
fn service_recurses_only_into_last_container_and_uses_its_leaf_owner() {
    let factory = Factory::new();
    let root = factory.create_root_dock();
    let source = factory.create_document_dock();
    source.set_is_collapsable(false);
    let moved = factory.create_document();
    factory.add_dockable(Some(source.clone()), Some(moved.clone()));
    let target = factory.create_proportional_dock();
    let leaf = factory.create_document_dock();
    let existing = factory.create_document();
    factory.add_dockable(Some(leaf.clone()), Some(existing.clone()));
    factory.add_dockable(Some(target.clone()), Some(leaf.clone()));
    factory.add_dockable(Some(root.clone()), Some(source.clone()));
    factory.add_dockable(Some(root.clone()), Some(target.clone()));
    assert!(DockService::new().move_dockable_between_docks(moved.clone(), source, target, true));
    let items = leaf.visible_dockables().unwrap();
    assert_eq!(items.count(), 2);
    assert!(Rc::ptr_eq(
        (items.get(1)).as_ref().expect("nonnull identity fixture"),
        &(moved as Rc<dyn IDockable>)
    ));
}

#[test]
fn service_move_and_swap_validate_membership_active_target_and_same_singleton() {
    let service = DockService::new();
    let owner = ToolDock::new();
    let source = Tool::new();
    source.set_owner(Some(owner.clone()));
    owner.set_visible_dockables(list(vec![source.clone()]));
    assert!(!service.move_dockable_between_docks(
        source.clone(),
        owner.clone(),
        owner.clone(),
        false
    ));
    let target = ToolDock::new();
    target.set_visible_dockables(list(vec![]));
    assert!(!service.swap_dockable_between_docks(
        source.clone(),
        owner.clone(),
        target.clone(),
        false
    ));
    let active = Tool::new();
    target.set_active_dockable(Some(active.clone()));
    assert!(!service.swap_dockable_between_docks(
        source.clone(),
        owner.clone(),
        target.clone(),
        false
    ));
    target.set_visible_dockables(None);
    assert!(service.swap_dockable_between_docks(
        source.clone(),
        owner.clone(),
        target.clone(),
        true
    ));
    target.set_active_dockable(Some(DocumentDock::new()));
    assert!(!service.swap_dockable_between_docks(source, owner, target, false));
}

#[test]
fn service_split_after_resize_preserves_vertical_structure_and_splits_only_the_resized_share() {
    let factory = Factory::new();
    let service = DockService::new();
    let root = factory.create_root_dock();
    let horizontal = factory.create_proportional_dock();
    factory.add_dockable(Some(root.clone()), Some(horizontal.clone()));
    let initial = factory.create_document_dock();
    initial.set_proportion(0.7);
    factory.add_dockable(Some(horizontal.clone()), Some(initial.clone()));
    let existing = factory.create_document();
    let bottom = factory.create_document();
    factory.add_dockable(Some(initial.clone()), Some(existing.clone()));
    factory.add_dockable(Some(initial.clone()), Some(bottom.clone()));
    initial.set_active_dockable(Some(existing));
    factory.init_layout(root.clone());
    assert!(service.split_dockable(
        bottom,
        initial.clone(),
        initial,
        DockOperation::Bottom,
        true
    ));
    let vertical = horizontal
        .visible_dockables()
        .unwrap()
        .get(0)
        .and_then(|item| item.as_proportional_dock())
        .unwrap();
    assert_eq!(vertical.orientation(), Orientation::Vertical);
    let top = vertical
        .visible_dockables()
        .unwrap()
        .get(0)
        .and_then(|item| item.as_document_dock())
        .unwrap();
    let bottom = vertical
        .visible_dockables()
        .unwrap()
        .get(2)
        .and_then(|item| item.as_document_dock())
        .unwrap();
    top.set_proportion(0.72);
    bottom.set_proportion(0.28);
    let added = factory.create_document();
    factory.add_dockable(Some(top.clone()), Some(added.clone()));
    assert!(service.split_dockable(added, top.clone(), bottom.clone(), DockOperation::Top, true));
    let children = vertical.visible_dockables().unwrap();
    assert_eq!(children.count(), 5);
    assert!(Rc::ptr_eq(
        (children.get(0))
            .as_ref()
            .expect("nonnull identity fixture"),
        &(top.clone() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        (children.get(4))
            .as_ref()
            .expect("nonnull identity fixture"),
        &(bottom.clone() as Rc<dyn IDockable>)
    ));
    assert_eq!(top.proportion(), 0.72);
    assert_eq!(
        children
            .get(2)
            .as_ref()
            .expect("nonnull source fixture")
            .proportion(),
        0.14
    );
    assert_eq!(bottom.proportion(), 0.14);
}

#[test]
fn service_tool_split_policy_is_execution_only_and_can_choose_document_or_tool_wrappers() {
    for document in [false, true] {
        let factory = PolicyFactory::new();
        factory.split_document.set(document);
        let root = factory.create_root_dock();
        let owner = factory.create_tool_dock();
        owner.set_is_collapsable(false);
        let target = factory.create_document_dock();
        let source = factory.create_tool();
        factory.add_dockable(Some(root.clone()), Some(owner.clone()));
        factory.add_dockable(Some(root.clone()), Some(target.clone()));
        factory.add_dockable(Some(owner.clone()), Some(source.clone()));
        assert!(DockService::new().split_dockable(
            source.clone(),
            owner.clone(),
            target.clone(),
            DockOperation::Right,
            false
        ));
        assert_eq!(factory.split_calls.get(), 0);
        assert!(DockService::new().split_dockable(
            source.clone(),
            owner,
            target,
            DockOperation::Right,
            true
        ));
        assert_eq!(factory.split_calls.get(), 1);
        assert_eq!(
            source.owner().unwrap().as_document_dock().is_some(),
            document
        );
        if !document {
            assert_eq!(
                source.owner().unwrap().as_tool_dock().unwrap().alignment(),
                Alignment::Right
            );
        }
    }
}

#[test]
fn service_window_uses_only_nan_size_fallback_and_requires_root_active_dock() {
    let factory = Factory::new();
    let root = factory.create_root_dock();
    let owner = factory.create_tool_dock();
    let source = factory.create_tool();
    let target = factory.create_tool();
    factory.add_dockable(Some(root.clone()), Some(owner.clone()));
    factory.add_dockable(Some(owner.clone()), Some(source.clone()));
    factory.add_dockable(Some(owner.clone()), Some(target.clone()));
    let service = DockService::new();
    assert!(!service.dock_dockable_into_window(
        source.clone(),
        target.clone(),
        DockPoint::new(1.0, 2.0),
        false
    ));
    root.set_active_dockable(Some(owner.clone()));
    source.set_can_float(false);
    assert!(!service.dock_dockable_into_window(
        source.clone(),
        target.clone(),
        DockPoint::default(),
        false
    ));
    source.set_can_float(true);
    owner.set_visible_bounds(0.0, 0.0, 500.0, f64::NAN);
    source.set_visible_bounds(0.0, 0.0, f64::NAN, -3.0);
    assert!(service.dock_dockable_into_window(source, target, DockPoint::new(11.0, 22.0), true));
    let window = root
        .windows()
        .unwrap()
        .get(0)
        .expect("nonnull window fixture");
    assert_eq!(
        (window.x(), window.y(), window.width(), window.height()),
        (11.0, 22.0, 500.0, -3.0)
    );
}

#[test]
fn window_service_resolves_float_denial_from_root_policy_in_the_original_policy_scenario() {
    let factory = Factory::new();
    let root = factory.create_root_dock();
    let owner = factory.create_tool_dock();
    let target = factory.create_tool_dock();
    let source = factory.create_tool();
    let target_tool = factory.create_tool();
    factory.add_dockable(Some(root.clone()), Some(owner.clone()));
    factory.add_dockable(Some(root.clone()), Some(target.clone()));
    factory.add_dockable(Some(owner.clone()), Some(source.clone()));
    factory.add_dockable(Some(target), Some(target_tool.clone()));
    root.set_active_dockable(Some(owner));
    let service = DockService::new();
    assert!(service.dock_dockable_into_window(
        source.clone(),
        target_tool.clone(),
        DockPoint::new(100.0, 100.0),
        false
    ));
    let policy = DockCapabilityPolicy::new();
    policy.set_can_float(Some(false));
    root.set_root_dock_capability_policy(Some(policy));
    assert!(!service.dock_dockable_into_window(
        source,
        target_tool,
        DockPoint::new(100.0, 100.0),
        false
    ));
}

#[test]
fn service_same_owner_and_cross_owner_item_moves_and_swaps_dispatch_source_overloads() {
    for cross in [false, true] {
        for action in [DragAction::MOVE, DragAction::LINK] {
            let factory = Factory::new();
            let root = factory.create_root_dock();
            let owner = factory.create_document_dock();
            owner.set_is_collapsable(false);
            let target = if cross {
                factory.create_document_dock()
            } else {
                owner.clone()
            };
            target.set_is_collapsable(false);
            factory.add_dockable(Some(root.clone()), Some(owner.clone()));
            if cross {
                factory.add_dockable(Some(root.clone()), Some(target.clone()));
            }
            let source = factory.create_document();
            source.set_id("source".into());
            let anchor = factory.create_document();
            anchor.set_id("anchor".into());
            factory.add_dockable(Some(owner.clone()), Some(source.clone()));
            factory.add_dockable(Some(target.clone()), Some(anchor.clone()));
            assert!(DockService::new().dock_dockable_into_dockable(
                source.clone(),
                anchor.clone(),
                action,
                true
            ));
            assert!(Rc::ptr_eq(
                &source.owner().unwrap(),
                &(target.clone() as Rc<dyn IDockable>)
            ));
            if action == DragAction::LINK && cross {
                assert!(Rc::ptr_eq(
                    &anchor.owner().unwrap(),
                    &(owner as Rc<dyn IDockable>)
                ));
            }
            assert!(!DockService::new().dock_dockable_into_dockable(
                source,
                anchor,
                DragAction::COPY,
                false
            ));
        }
    }
}
