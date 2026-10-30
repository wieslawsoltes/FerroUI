use crate::controls::*;
use crate::engine_test_support::PolicyFactory;
use crate::Factory;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::{DockManager, DockService};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

struct Scene {
    factory: Rc<PolicyFactory>,
    manager: Rc<DockManager>,
    root: Rc<dyn IRootDock>,
    source: Rc<dyn IDocumentDock>,
    target: Rc<dyn IDocumentDock>,
    document: Rc<dyn IDocument>,
}
impl Scene {
    fn new() -> Self {
        let factory = PolicyFactory::new();
        let root = factory.create_root_dock();
        let panel = factory.create_proportional_dock();
        let source = factory.create_document_dock();
        let target = factory.create_document_dock();
        source.set_is_collapsable(false);
        target.set_is_collapsable(false);
        source.set_id("source-dock".into());
        target.set_id("target-dock".into());
        let document = factory.create_document();
        document.set_id("document".into());
        factory.add_dockable(Some(source.clone()), Some(document.clone()));
        factory.add_dockable(Some(target.clone()), Some(factory.create_document()));
        factory.add_dockable(Some(panel.clone()), Some(source.clone()));
        factory.add_dockable(Some(panel.clone()), Some(target.clone()));
        factory.add_dockable(Some(root.clone()), Some(panel.clone()));
        root.set_active_dockable(Some(panel));
        factory.init_layout(root.clone());
        Self {
            factory,
            manager: DockManager::new(DockService::new()),
            root,
            source,
            target,
            document,
        }
    }
}
#[test]
fn manager_defaults_and_shared_options_match_source_and_pointer_properties_retain_values() {
    let service = DockService::new();
    let options = DockManagerOptions::new();
    let first = DockManager::with_options(service.clone(), options.clone());
    let second = DockManager::with_options(service, options.clone());
    assert_eq!(first.position(), DockPoint::default());
    assert_eq!(first.screen_position(), DockPoint::default());
    assert!(first.prevent_size_conflicts() && first.is_docking_enabled());
    assert!(first.last_capability_evaluation().is_none());
    assert!(Rc::ptr_eq(&first.options(), &options));
    first.set_position(DockPoint::new(f64::NAN, 2.0));
    first.set_screen_position(DockPoint::new(5.0, 6.0));
    assert_eq!(first.position(), DockPoint::new(f64::NAN, 2.0));
    assert_eq!(first.screen_position(), DockPoint::new(5.0, 6.0));
    first.set_prevent_size_conflicts(false);
    assert!(!first.prevent_size_conflicts());
    first.set_is_docking_enabled(false);
    assert!(!second.is_docking_enabled());
}
#[test]
fn manager_capability_provenance_precedence_and_reset_are_preserved() {
    let scene = Scene::new();
    let policy = DockCapabilityPolicy::new();
    policy.set_can_drag(Some(false));
    scene.root.set_root_dock_capability_policy(Some(policy));
    assert!(!scene.manager.validate_document(
        scene.document.clone(),
        scene.target.clone(),
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    let evaluation = scene.manager.last_capability_evaluation().unwrap();
    assert_eq!(evaluation.capability(), DockCapability::Drag);
    assert_eq!(
        evaluation.effective_source(),
        DockCapabilityValueSource::RootPolicy
    );
    let policy = DockCapabilityPolicy::new();
    policy.set_can_drag(Some(true));
    scene.source.set_dock_capability_policy(Some(policy));
    assert!(scene.manager.validate_document(
        scene.document.clone(),
        scene.target.clone(),
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    assert!(scene.manager.last_capability_evaluation().is_none());
    scene.target.set_can_drop(false);
    assert!(!scene.manager.validate_dockable(
        scene.document.clone(),
        scene.target.clone(),
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    assert_eq!(
        scene
            .manager
            .last_capability_evaluation()
            .unwrap()
            .capability(),
        DockCapability::Drop
    );
    scene.manager.set_is_docking_enabled(false);
    assert!(!scene.manager.validate_dockable(
        scene.document,
        scene.target,
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    assert!(scene.manager.last_capability_evaluation().is_none());
}

#[test]
fn tool_validation_root_dock_and_dockable_capability_precedence_matches_upstream_tool_scenario() {
    let factory = Factory::new();
    let root = factory.create_root_dock();
    let source = factory.create_tool_dock();
    let target = factory.create_tool_dock();
    let tool = factory.create_tool();
    factory.add_dockable(Some(root.clone()), Some(source.clone()));
    factory.add_dockable(Some(root.clone()), Some(target.clone()));
    factory.add_dockable(Some(source.clone()), Some(tool.clone()));
    factory.add_dockable(Some(target.clone()), Some(factory.create_tool()));
    let manager = DockManager::new(DockService::new());
    let policy = DockCapabilityPolicy::new();
    policy.set_can_drag(Some(false));
    root.set_root_dock_capability_policy(Some(policy));
    assert!(!manager.validate_tool(
        tool.clone(),
        target.clone(),
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    assert_eq!(
        manager
            .last_capability_evaluation()
            .unwrap()
            .effective_source(),
        DockCapabilityValueSource::RootPolicy
    );
    let policy = DockCapabilityPolicy::new();
    policy.set_can_drag(Some(true));
    source.set_dock_capability_policy(Some(policy));
    assert!(manager.validate_tool(
        tool.clone(),
        target.clone(),
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    assert!(manager.last_capability_evaluation().is_none());
    let overrides = DockCapabilityOverrides::new();
    overrides.set_can_drag(Some(false));
    tool.set_dock_capability_overrides(Some(overrides));
    assert!(!manager.validate_tool(tool, target, DragAction::MOVE, DockOperation::Fill, false));
    assert_eq!(
        manager
            .last_capability_evaluation()
            .unwrap()
            .effective_source(),
        DockCapabilityValueSource::DockableOverride
    );
}
#[test]
fn manager_restrictions_and_drop_policy_apply_before_capability_and_same_factory_policy_is_not_duplicated(
) {
    let scene = Scene::new();
    scene.document.set_can_drag(false);
    scene
        .document
        .as_docking_restrictions()
        .unwrap()
        .set_allowed_dock_operations(DockOperationMask::RIGHT);
    assert!(!scene.manager.validate_document(
        scene.document.clone(),
        scene.target.clone(),
        DragAction::MOVE,
        DockOperation::Left,
        false
    ));
    assert!(scene.manager.last_capability_evaluation().is_none());
    assert!(scene.factory.drop_calls.borrow().is_empty());
    scene.document.set_can_drag(true);
    scene
        .document
        .as_docking_restrictions()
        .unwrap()
        .set_allowed_dock_operations(DockOperationMask::ALL);
    assert!(scene.manager.validate_document(
        scene.document.clone(),
        scene.target.clone(),
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    assert_eq!(scene.factory.drop_calls.borrow().len(), 1);
    scene.factory.drop_allowed.set(false);
    scene.document.set_can_drag(false);
    assert!(!scene.manager.validate_document(
        scene.document,
        scene.target,
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    assert!(scene.manager.last_capability_evaluation().is_none());
}
#[test]
fn manager_cross_factory_target_veto_is_consulted_after_source_policy() {
    let scene = Scene::new();
    let other = PolicyFactory::new();
    let root = other.create_root_dock();
    let target = other.create_document_dock();
    other.add_dockable(Some(root.clone()), Some(target.clone()));
    other.drop_allowed.set(false);
    assert!(!scene.manager.validate_document(
        scene.document,
        target,
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    assert_eq!(scene.factory.drop_calls.borrow().len(), 1);
    assert_eq!(other.drop_calls.borrow().len(), 1);
}
#[test]
fn document_targets_require_document_capability_and_documents_reject_tool_targets() {
    let scene = Scene::new();
    scene.document.set_can_dock_as_document(false);
    assert!(!scene.manager.validate_document(
        scene.document.clone(),
        scene.target.clone(),
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    assert_eq!(
        scene
            .manager
            .last_capability_evaluation()
            .unwrap()
            .capability(),
        DockCapability::DockAsDocument
    );
    scene.document.set_can_dock_as_document(true);
    let tool = scene.factory.create_tool_dock();
    scene
        .factory
        .add_dockable(Some(scene.root.clone()), Some(tool.clone()));
    assert!(!scene.manager.validate_document(
        scene.document,
        tool,
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    assert!(scene.manager.last_capability_evaluation().is_none());
}
#[test]
fn observer_brackets_actual_ownership_change_and_rechecks_after_begin() {
    for block in [false, true] {
        let scene = Scene::new();
        scene.factory.observer.set(true);
        let document = scene.document.clone();
        let source = scene.source.clone();
        let manager = scene.manager.clone();
        *scene.factory.before.borrow_mut() = Some(Rc::new(move || {
            assert!(Rc::ptr_eq(
                &document.owner().unwrap(),
                &(source.clone() as Rc<dyn IDockable>)
            ));
            if block {
                manager.set_is_docking_enabled(false);
            }
        }));
        let document = scene.document.clone();
        let target = scene.target.clone();
        *scene.factory.after.borrow_mut() = Some(Rc::new(move || {
            assert!(Rc::ptr_eq(
                &document.owner().unwrap(),
                &(target.clone() as Rc<dyn IDockable>)
            ))
        }));
        assert_eq!(
            scene.manager.validate_dockable(
                scene.document.clone(),
                scene.target.clone(),
                DragAction::MOVE,
                DockOperation::Fill,
                true
            ),
            !block
        );
        assert_eq!(
            *scene.factory.trace.borrow(),
            if block {
                vec!["begin", "dispose"]
            } else {
                vec!["begin", "complete", "dispose"]
            }
        );
    }
}
#[test]
fn observer_is_not_opened_for_validation_or_failed_preflight_and_null_scope_is_allowed() {
    let scene = Scene::new();
    scene.factory.observer.set(true);
    assert!(scene.manager.validate_dockable(
        scene.document.clone(),
        scene.target.clone(),
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    assert!(scene.factory.trace.borrow().is_empty());
    scene.document.set_can_float(false);
    assert!(!scene.manager.validate_dockable(
        scene.document.clone(),
        scene.target.clone(),
        DragAction::MOVE,
        DockOperation::Window,
        true
    ));
    assert!(scene.factory.trace.borrow().is_empty());
    scene.document.set_can_float(true);
    scene.factory.omit_scope.set(true);
    assert!(scene.manager.validate_dockable(
        scene.document,
        scene.target,
        DragAction::MOVE,
        DockOperation::Fill,
        true
    ));
    assert_eq!(*scene.factory.trace.borrow(), ["begin"]);
}
#[test]
fn observer_disposes_after_execution_or_completion_failure_and_disposal_failure_replaces_an_execution_failure(
) {
    for failure in 0..3 {
        let scene = Scene::new();
        scene.factory.observer.set(true);
        scene.factory.throw_move.set(failure != 1);
        scene.factory.throw_complete.set(failure == 1);
        scene.factory.throw_dispose.set(failure == 2);
        let error = catch_unwind(AssertUnwindSafe(|| {
            scene.manager.validate_dockable(
                scene.document.clone(),
                scene.target.clone(),
                DragAction::MOVE,
                DockOperation::Fill,
                true,
            )
        }))
        .unwrap_err();
        assert_eq!(
            *scene.factory.trace.borrow(),
            if failure == 1 {
                vec!["begin", "complete", "dispose"]
            } else {
                vec!["begin", "dispose"]
            }
        );
        if failure == 2 {
            let message = error
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied())
                .unwrap();
            assert!(message.contains("disposal failed"));
        }
    }
}
#[test]
fn root_fill_preflight_ignores_owner_factory_and_action_but_execution_uses_source_wrapper_rules() {
    let manager = DockManager::new(DockService::new());
    let source = Document::new();
    let root = RootDock::new();
    assert!(manager.validate_document(
        source.clone(),
        root.clone(),
        DragAction::COPY,
        DockOperation::Fill,
        false
    ));
    assert!(!manager.validate_document(
        source.clone(),
        root.clone(),
        DragAction::MOVE,
        DockOperation::Fill,
        true
    ));
    assert!(!manager.validate_document(source, root, DragAction::MOVE, DockOperation::Left, false));
    let scene = Scene::new();
    let empty = scene.factory.create_root_dock();
    scene.factory.init_layout(empty.clone());
    scene.document.set_dock_group(Some("source-group".into()));
    assert!(scene.manager.validate_document(
        scene.document.clone(),
        empty.clone(),
        DragAction::COPY,
        DockOperation::Fill,
        true
    ));
    let wrapper = scene.document.owner().unwrap().as_document_dock().unwrap();
    assert_eq!(wrapper.dock_group().as_deref(), Some("source-group"));
    assert!(Rc::ptr_eq(
        &empty.active_dockable().unwrap(),
        &(wrapper as Rc<dyn IDockable>)
    ));
}
#[test]
fn fixed_tool_sizes_include_infinities_and_nonfixed_sizes_do_not_conflict() {
    let factory = Factory::new();
    let root = factory.create_root_dock();
    let source = factory.create_tool_dock();
    let target = factory.create_tool_dock();
    let a = factory.create_tool();
    let b = factory.create_tool();
    factory.add_dockable(Some(root.clone()), Some(source.clone()));
    factory.add_dockable(Some(root.clone()), Some(target.clone()));
    factory.add_dockable(Some(source), Some(a.clone()));
    factory.add_dockable(Some(target.clone()), Some(b.clone()));
    let manager = DockManager::new(DockService::new());
    a.set_min_width(f64::INFINITY);
    a.set_max_width(f64::INFINITY);
    b.set_min_width(200.0);
    b.set_max_width(200.0);
    assert!(!manager.validate_tool(
        a.clone(),
        target.clone(),
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    assert!(manager.last_capability_evaluation().is_none());
    manager.set_prevent_size_conflicts(false);
    assert!(manager.validate_tool(
        a.clone(),
        target.clone(),
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    manager.set_prevent_size_conflicts(true);
    b.set_max_width(201.0);
    assert!(manager.validate_tool(a, target, DragAction::MOVE, DockOperation::Fill, false));
}
#[test]
fn pinned_targets_pin_new_sources_and_move_already_pinned_sources_between_strips() {
    let factory = Factory::new();
    let root = factory.create_root_dock();
    let a = factory.create_tool_dock();
    let b = factory.create_tool_dock();
    a.set_alignment(Alignment::Left);
    b.set_alignment(Alignment::Right);
    let first = factory.create_tool();
    let second = factory.create_tool();
    factory.add_dockable(Some(root.clone()), Some(a.clone()));
    factory.add_dockable(Some(root.clone()), Some(b.clone()));
    factory.add_dockable(Some(a.clone()), Some(first.clone()));
    factory.add_dockable(Some(b.clone()), Some(second.clone()));
    factory.pin_dockable(Some(second.clone()));
    let manager = DockManager::new(DockService::new());
    assert!(manager.validate_tool(
        first.clone(),
        second.clone(),
        DragAction::MOVE,
        DockOperation::Fill,
        true
    ));
    assert!(root
        .right_pinned_dockables()
        .unwrap()
        .contains(&Some((first.clone() as Rc<dyn IDockable>).clone())));
    assert_eq!(a.alignment(), Alignment::Right);
    assert!(!manager.validate_tool(
        first.clone(),
        second.clone(),
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    factory.unpin_dockable(Some(first.clone()));
    a.set_alignment(Alignment::Left);
    factory.pin_dockable(Some(first.clone()));
    let moved = Rc::new(Cell::new(0));
    let log = moved.clone();
    factory.dockable_moved(Rc::new(move |_| log.set(log.get() + 1)));
    assert!(manager.validate_tool(
        first.clone(),
        second,
        DragAction::MOVE,
        DockOperation::Fill,
        true
    ));
    assert_eq!(moved.get(), 1);
    assert!(!root
        .left_pinned_dockables()
        .unwrap()
        .contains(&Some((first.clone() as Rc<dyn IDockable>).clone())));
    assert!(root
        .right_pinned_dockables()
        .unwrap()
        .contains(&Some((first as Rc<dyn IDockable>).clone())));
    assert_eq!(a.alignment(), Alignment::Left);
}
#[test]
fn pane_split_keeps_tab_order_and_document_owned_tools_match_the_document_target_arm() {
    let scene = Scene::new();
    let second = scene.factory.create_document();
    second.set_id("second".into());
    scene
        .factory
        .add_dockable(Some(scene.source.clone()), Some(second.clone()));
    assert!(scene.manager.validate_dock(
        scene.source.clone(),
        scene.target.clone(),
        DragAction::MOVE,
        DockOperation::Right,
        true
    ));
    let owner = scene.document.owner().unwrap().as_dock().unwrap();
    let items = owner.visible_dockables().unwrap();
    assert_eq!(items.count(), 2);
    assert!(Rc::ptr_eq(
        (items.get(0)).as_ref().expect("nonnull identity fixture"),
        &(scene.document.clone() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        (items.get(1)).as_ref().expect("nonnull identity fixture"),
        &(second as Rc<dyn IDockable>)
    ));
    let tool = scene.factory.create_tool();
    scene
        .factory
        .add_dockable(Some(scene.target.clone()), Some(tool.clone()));
    assert!(scene
        .manager
        .validate_dock(owner, tool, DragAction::MOVE, DockOperation::Fill, false));
}
#[test]
fn indicator_visibility_does_not_clear_a_previous_capability_failure_and_only_fill_rejects_self_owner(
) {
    let scene = Scene::new();
    scene.document.set_can_drag(false);
    assert!(!scene.manager.validate_document(
        scene.document.clone(),
        scene.target.clone(),
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    let evaluation = scene.manager.last_capability_evaluation().unwrap();
    assert!(scene.manager.is_dock_target_visible(
        scene.document.clone(),
        scene.target.clone(),
        DockOperation::Left
    ));
    assert!(Rc::ptr_eq(
        &evaluation,
        &scene.manager.last_capability_evaluation().unwrap()
    ));
    assert!(!scene.manager.is_dock_target_visible(
        scene.document.clone(),
        scene.document.clone(),
        DockOperation::Fill
    ));
    assert!(!scene.manager.is_dock_target_visible(
        scene.document.clone(),
        scene.source.clone(),
        DockOperation::Fill
    ));
    assert!(scene.manager.is_dock_target_visible(
        scene.document.clone(),
        scene.document,
        DockOperation::Right
    ));
}

#[derive(Clone)]
struct Call {
    kind: &'static str,
    source: Rc<dyn IDockable>,
    target: Rc<dyn IDockable>,
}
type ServiceCallback = Rc<dyn Fn(&Call) -> bool>;
#[derive(Default)]
struct TraceService {
    calls: RefCell<Vec<Call>>,
    callback: RefCell<Option<ServiceCallback>>,
}
impl TraceService {
    fn record(
        &self,
        kind: &'static str,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDockable>,
    ) -> bool {
        let call = Call {
            kind,
            source,
            target,
        };
        self.calls.borrow_mut().push(call.clone());
        let callback = self.callback.borrow().clone();
        callback.is_none_or(|callback| callback(&call))
    }
}

#[test]
fn manager_preserves_null_empty_and_leaf_only_group_collection_results() {
    let manager = DockManager::new(Rc::new(TraceService::default()));
    let source = DocumentDock::new();
    let target = DocumentDock::new();
    for operation in [DockOperation::Fill, DockOperation::Left] {
        assert!(manager.validate_dock(
            source.clone(),
            target.clone(),
            DragAction::MOVE,
            operation,
            false
        ));
    }
    source.set_visible_dockables(Some(ferrodock_model::collections::ListHandle::new()));
    assert!(manager.validate_dock(
        source.clone(),
        target.clone(),
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    assert!(!manager.validate_dock(
        source,
        target.clone(),
        DragAction::MOVE,
        DockOperation::Left,
        false
    ));
    let group = StackDock::new();
    assert!(!manager.validate_dockable(
        group.clone(),
        target.clone(),
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    group.set_can_drag(false);
    group.set_visible_dockables(Some(ferrodock_model::collections::ListHandle::from_items(
        vec![Some(Document::new() as Rc<dyn IDockable>)],
    )));
    assert!(manager.validate_dockable(
        group,
        target.clone(),
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    assert!(!manager.validate_dockable(
        DockDock::new(),
        target,
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
}
impl IDockService for TraceService {
    fn move_dockable_between_docks(
        &self,
        s: Rc<dyn IDockable>,
        _o: Rc<dyn IDock>,
        t: Rc<dyn IDock>,
        _e: bool,
    ) -> bool {
        self.record("move", s, t)
    }
    fn swap_dockable_between_docks(
        &self,
        s: Rc<dyn IDockable>,
        _o: Rc<dyn IDock>,
        t: Rc<dyn IDock>,
        _e: bool,
    ) -> bool {
        self.record("swap", s, t)
    }
    fn split_dockable(
        &self,
        s: Rc<dyn IDockable>,
        _o: Rc<dyn IDock>,
        t: Rc<dyn IDock>,
        _op: DockOperation,
        _e: bool,
    ) -> bool {
        self.record("split", s, t)
    }
    fn dock_dockable_into_window(
        &self,
        s: Rc<dyn IDockable>,
        t: Rc<dyn IDockable>,
        _p: DockPoint,
        _e: bool,
    ) -> bool {
        self.record("window", s, t)
    }
    fn dock_dockable_into_dockable(
        &self,
        s: Rc<dyn IDockable>,
        t: Rc<dyn IDockable>,
        _a: DragAction,
        _e: bool,
    ) -> bool {
        self.record("item", s, t)
    }
}
#[test]
fn manager_snapshots_pane_contents_and_splits_first_then_inserts_after_preceding_items() {
    let scene = Scene::new();
    let second = scene.factory.create_document();
    second.set_id("second".into());
    scene
        .factory
        .add_dockable(Some(scene.source.clone()), Some(second.clone()));
    let trace = Rc::new(TraceService::default());
    let manager = DockManager::new(trace.clone());
    let source = scene.source.clone();
    *trace.callback.borrow_mut() = Some(Rc::new(move |call| {
        if call.kind == "split" {
            source.visible_dockables().unwrap().clear();
        }
        true
    }));
    assert!(manager.validate_dock(
        scene.source,
        scene.target,
        DragAction::MOVE,
        DockOperation::Left,
        true
    ));
    let calls = trace.calls.borrow();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].kind, "split");
    assert_eq!(calls[1].kind, "item");
    assert!(Rc::ptr_eq(&calls[1].source, &(second as Rc<dyn IDockable>)));
    assert!(Rc::ptr_eq(
        &calls[1].target,
        &(scene.document as Rc<dyn IDockable>)
    ));
}
#[test]
fn group_validation_visits_child_docks_in_reverse_without_short_circuit_and_restores_first_capability_failure(
) {
    let scene = Scene::new();
    let group = scene.factory.create_stack_dock();
    let a = scene.factory.create_tool_dock();
    let b = scene.factory.create_tool_dock();
    a.set_id("a".into());
    b.set_id("b".into());
    a.set_can_drag(false);
    b.set_can_drop(false);
    b.set_can_drag(false);
    let policy = DockCapabilityPolicy::new();
    policy.set_can_drag(Some(false));
    b.set_dock_capability_policy(Some(policy));
    scene.factory.add_dockable(Some(group.clone()), Some(a));
    scene.factory.add_dockable(Some(group.clone()), Some(b));
    scene
        .factory
        .add_dockable(Some(scene.root.clone()), Some(group.clone()));
    assert!(!scene.manager.validate_dockable(
        group,
        scene.target,
        DragAction::MOVE,
        DockOperation::Fill,
        false
    ));
    assert_eq!(
        scene
            .manager
            .last_capability_evaluation()
            .unwrap()
            .capability(),
        DockCapability::Drag
    );
    assert_eq!(
        scene
            .manager
            .last_capability_evaluation()
            .unwrap()
            .effective_source(),
        DockCapabilityValueSource::DockPolicy
    );
    let ids: Vec<_> = scene
        .factory
        .drop_calls
        .borrow()
        .iter()
        .map(|(id, _, _)| id.clone())
        .collect();
    assert!(
        ids.iter().position(|id| id == "b").unwrap() < ids.iter().position(|id| id == "a").unwrap()
    );
    assert!(ids.iter().any(|id| id == "a"));
}
