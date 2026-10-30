use crate::engine_test_support::PolicyFactory;
use crate::Factory;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::{DockManager, DockService};
use std::rc::Rc;

struct Layout {
    factory: Rc<PolicyFactory>,
    engine: Rc<DockManager>,
    root: Rc<dyn IRootDock>,
    first: Rc<dyn ITool>,
    second: Rc<dyn ITool>,
    pane: Rc<dyn IToolDock>,
    target: Rc<dyn IDocumentDock>,
    region: Rc<dyn IProportionalDock>,
    source: Rc<dyn IDockable>,
}
impl Layout {
    fn new(shape: u32) -> Self {
        let factory = PolicyFactory::new();
        let root = factory.create_root_dock();
        let panel = factory.create_proportional_dock();
        let pane = factory.create_tool_dock();
        let first = factory.create_tool();
        let second = factory.create_tool();
        first.set_can_dock_as_document(false);
        second.set_can_dock_as_document(false);
        factory.add_dockable(Some(pane.clone()), Some(first.clone()));
        factory.add_dockable(Some(pane.clone()), Some(second.clone()));
        pane.set_active_dockable(Some(first.clone()));
        let source: Rc<dyn IDockable> = match shape {
            0 => first.clone(),
            1 => pane.clone(),
            _ => {
                let group = factory.create_proportional_dock();
                group.set_orientation(Orientation::Vertical);
                factory.add_dockable(Some(group.clone()), Some(pane.clone()));
                let extra = factory.create_tool_dock();
                factory.add_dockable(Some(extra.clone()), Some(factory.create_tool()));
                factory.add_dockable(Some(group.clone()), Some(extra));
                group.set_active_dockable(Some(pane.clone()));
                group
            }
        };
        let target = factory.create_document_dock();
        factory.add_dockable(Some(target.clone()), Some(factory.create_document()));
        let region = factory.create_proportional_dock();
        factory.add_dockable(Some(region.clone()), Some(target.clone()));
        let extra = factory.create_document_dock();
        factory.add_dockable(Some(extra.clone()), Some(factory.create_document()));
        factory.add_dockable(Some(region.clone()), Some(extra));
        *factory.region.borrow_mut() = Some(region.clone());
        factory.add_dockable(Some(panel.clone()), Some(region.clone()));
        factory.add_dockable(
            Some(panel.clone()),
            if shape == 0 {
                Some(pane.clone() as Rc<dyn IDockable>)
            } else {
                Some(source.clone())
            },
        );
        factory.add_dockable(Some(root.clone()), Some(panel));
        factory.init_layout(root.clone());
        Self {
            factory,
            engine: DockManager::new(DockService::new()),
            root,
            first,
            second,
            pane,
            target,
            region,
            source,
        }
    }
}
#[test]
fn outer_splits_keep_panes_and_groups_intact_in_all_directions_and_restore_focus() {
    for shape in 0..3 {
        for (operation, orientation, before) in [
            (DockOperation::OuterLeft, Orientation::Horizontal, true),
            (DockOperation::OuterRight, Orientation::Horizontal, false),
            (DockOperation::OuterTop, Orientation::Vertical, true),
            (DockOperation::OuterBottom, Orientation::Vertical, false),
        ] {
            let layout = Layout::new(shape);
            layout
                .factory
                .set_focused_dockable(Some(layout.root.clone()), Some(layout.first.clone()));
            let old = layout.source.owner().unwrap();
            assert!(layout.engine.is_dock_target_visible(
                layout.source.clone(),
                layout.target.clone(),
                operation
            ));
            assert!(layout.engine.validate_dockable(
                layout.source.clone(),
                layout.target.clone(),
                DragAction::MOVE,
                operation,
                false
            ));
            assert!(Rc::ptr_eq(&old, &layout.source.owner().unwrap()));
            assert!(layout.engine.validate_dockable(
                layout.source.clone(),
                layout.target.clone(),
                DragAction::MOVE,
                operation,
                true
            ));
            let moved: Rc<dyn IDockable> = if shape == 0 {
                layout.source.owner().unwrap()
            } else {
                layout.source.clone()
            };
            let parent = moved.owner().unwrap().as_proportional_dock().unwrap();
            assert_eq!(parent.orientation(), orientation);
            assert!(Rc::ptr_eq(
                &layout.region.owner().unwrap(),
                &(parent.clone() as Rc<dyn IDockable>)
            ));
            assert!(Rc::ptr_eq(
                &layout.target.owner().unwrap(),
                &(layout.region.clone() as Rc<dyn IDockable>)
            ));
            let items = parent.visible_dockables().unwrap();
            let region_index = items
                .index_of(&Some((layout.region.clone() as Rc<dyn IDockable>).clone()))
                .unwrap();
            let moved_index = items.index_of(&Some((moved).clone())).unwrap();
            assert_eq!(moved_index < region_index, before);
            if shape > 0 {
                let children = layout.pane.visible_dockables().unwrap();
                assert_eq!(children.count(), 2);
                assert!(Rc::ptr_eq(
                    (children.get(0))
                        .as_ref()
                        .expect("nonnull identity fixture"),
                    &(layout.first.clone() as Rc<dyn IDockable>)
                ));
                assert!(Rc::ptr_eq(
                    (children.get(1))
                        .as_ref()
                        .expect("nonnull identity fixture"),
                    &(layout.second.clone() as Rc<dyn IDockable>)
                ));
            }
            assert!(Rc::ptr_eq(
                &layout.root.focused_dockable().unwrap(),
                &(layout.first as Rc<dyn IDockable>)
            ));
        }
    }
}
#[test]
fn outer_rejections_leave_source_and_target_structure_untouched() {
    for reason in 0..8 {
        let layout = Layout::new(2);
        match reason {
            0 => *layout.factory.region.borrow_mut() = None,
            1 => layout.source.set_can_drag(false),
            2 => layout.target.set_can_drop(false),
            3 => layout.second.set_can_drag(false),
            4 => layout.source.set_dock_group(Some("different".into())),
            5 => layout
                .source
                .as_docking_restrictions()
                .unwrap()
                .set_allowed_dock_operations(DockOperationMask::RIGHT),
            6 => *layout.factory.region.borrow_mut() = Some(layout.pane.clone()),
            _ => {
                layout.region.set_factory(None);
                layout.source.set_factory(None);
            }
        }
        let owner = layout.source.owner().unwrap();
        assert!(!layout.engine.is_dock_target_visible(
            layout.source.clone(),
            layout.target.clone(),
            DockOperation::OuterLeft
        ));
        assert!(!layout.engine.validate_dockable(
            layout.source.clone(),
            layout.target.clone(),
            DragAction::MOVE,
            DockOperation::OuterLeft,
            true
        ));
        assert!(Rc::ptr_eq(&owner, &layout.source.owner().unwrap()));
        assert_eq!(layout.pane.visible_dockables().unwrap().count(), 2);
        assert!(Rc::ptr_eq(
            &layout.target.owner().unwrap(),
            &(layout.region as Rc<dyn IDockable>)
        ));
    }
}
#[test]
fn outer_policy_opt_in_factory_identity_and_move_action_are_required() {
    let layout = Layout::new(0);
    layout.target.set_factory(Some(Factory::new()));
    assert!(!layout.engine.is_dock_target_visible(
        layout.source.clone(),
        layout.target.clone(),
        DockOperation::OuterLeft
    ));
    for action in [
        DragAction::COPY,
        DragAction::LINK,
        DragAction::MOVE | DragAction::LINK,
    ] {
        let layout = Layout::new(0);
        assert!(!layout.engine.validate_dockable(
            layout.source.clone(),
            layout.target.clone(),
            action,
            DockOperation::OuterRight,
            true
        ));
    }
}
#[test]
fn outer_split_inside_its_own_region_requires_other_content_and_can_change_single_parent_orientation(
) {
    for nested in [false, true] {
        for sibling in [false, true] {
            let factory = PolicyFactory::new();
            let root = factory.create_root_dock();
            let panel = factory.create_proportional_dock();
            panel.set_orientation(Orientation::Vertical);
            let pane = factory.create_tool_dock();
            let source = factory.create_tool();
            factory.add_dockable(Some(pane.clone()), Some(source.clone()));
            let other = factory.create_tool();
            if sibling {
                factory.add_dockable(Some(pane.clone()), Some(other.clone()));
            }
            let region: Rc<dyn IDock> = if nested {
                let group = factory.create_proportional_dock();
                factory.add_dockable(Some(group.clone()), Some(pane.clone()));
                group
            } else {
                pane.clone()
            };
            *factory.region.borrow_mut() = Some(region.clone());
            factory.add_dockable(Some(panel.clone()), Some(region.clone()));
            factory.add_dockable(Some(root.clone()), Some(panel.clone()));
            factory.init_layout(root.clone());
            let manager = DockManager::new(DockService::new());
            assert_eq!(
                manager.is_dock_target_visible(
                    source.clone(),
                    pane.clone(),
                    DockOperation::OuterLeft
                ),
                sibling
            );
            assert_eq!(
                manager.validate_dockable(
                    source.clone(),
                    pane.clone(),
                    DragAction::MOVE,
                    DockOperation::OuterLeft,
                    true
                ),
                sibling
            );
            if sibling {
                assert!(!pane
                    .visible_dockables()
                    .unwrap()
                    .contains(&Some((source.clone() as Rc<dyn IDockable>).clone())));
                assert!(Rc::ptr_eq(
                    &other.owner().unwrap(),
                    &(pane as Rc<dyn IDockable>)
                ));
                assert_eq!(
                    source
                        .owner()
                        .unwrap()
                        .owner()
                        .unwrap()
                        .as_proportional_dock()
                        .unwrap()
                        .orientation(),
                    Orientation::Horizontal
                );
            } else {
                assert!(Rc::ptr_eq(
                    &source.owner().unwrap(),
                    &(pane as Rc<dyn IDockable>)
                ));
            }
        }
    }
}
