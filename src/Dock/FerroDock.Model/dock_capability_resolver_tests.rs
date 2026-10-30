use crate::controls::*;
use crate::core::factory_parameter_tests::RecordingFactory;
use crate::core::*;
use crate::test_support::{TestDock, TestNode, TestRoot};
use crate::DockCapabilityResolver;
use std::rc::Rc;

fn hierarchy() -> (Rc<TestRoot>, Rc<TestDock>, Rc<TestNode>) {
    let root = TestRoot::new(None);
    let dock = TestDock::new(None);
    let tool = TestNode::new(None);
    dock.set_owner(Some(root.clone()));
    tool.set_owner(Some(dock.clone()));
    (root, dock, tool)
}

fn set_policy(policy: &dyn IDockCapabilityPolicy, capability: DockCapability, value: Option<bool>) {
    match capability {
        DockCapability::Close => policy.set_can_close(value),
        DockCapability::Pin => policy.set_can_pin(value),
        DockCapability::Float => policy.set_can_float(value),
        DockCapability::Drag => policy.set_can_drag(value),
        DockCapability::Drop => policy.set_can_drop(value),
        DockCapability::DockAsDocument => policy.set_can_dock_as_document(value),
    }
}

fn set_base(dockable: &dyn IDockable, capability: DockCapability, value: bool) {
    match capability {
        DockCapability::Close => dockable.set_can_close(value),
        DockCapability::Pin => dockable.set_can_pin(value),
        DockCapability::Float => dockable.set_can_float(value),
        DockCapability::Drag => dockable.set_can_drag(value),
        DockCapability::Drop => dockable.set_can_drop(value),
        DockCapability::DockAsDocument => dockable.set_can_dock_as_document(value),
    }
}

#[test]
fn all_capabilities_resolve_base_root_dock_override_precedence_and_keep_provenance() {
    for raw in 0..6 {
        let capability = DockCapability::try_from(raw).unwrap();
        for base in [false, true] {
            for root_value in [None, Some(false), Some(true)] {
                for dock_value in [None, Some(false), Some(true)] {
                    for override_value in [None, Some(false), Some(true)] {
                        let (root, dock, tool) = hierarchy();
                        let root_policy = DockCapabilityPolicy::new();
                        let dock_policy = DockCapabilityPolicy::new();
                        let overrides = DockCapabilityOverrides::new();
                        set_base(&*tool, capability, base);
                        set_policy(&*root_policy, capability, root_value);
                        set_policy(&*dock_policy, capability, dock_value);
                        set_policy(&*overrides, capability, override_value);
                        root.set_root_dock_capability_policy(Some(root_policy));
                        dock.set_dock_capability_policy(Some(dock_policy));
                        tool.set_dock_capability_overrides(Some(overrides));
                        let evaluation = DockCapabilityResolver::evaluate_with_context(
                            Some(tool.clone()),
                            capability,
                            Some(dock.clone()),
                        );
                        assert_eq!(
                            evaluation.effective_value(),
                            override_value.or(dock_value).or(root_value).unwrap_or(base)
                        );
                        assert_eq!(evaluation.base_value(), base);
                        assert_eq!(evaluation.capability(), capability);
                        assert_eq!(evaluation.root_policy_value(), root_value);
                        assert_eq!(evaluation.dock_policy_value(), dock_value);
                        assert_eq!(evaluation.dockable_override_value(), override_value);
                        let source = if override_value.is_some() {
                            DockCapabilityValueSource::DockableOverride
                        } else if dock_value.is_some() {
                            DockCapabilityValueSource::DockPolicy
                        } else if root_value.is_some() {
                            DockCapabilityValueSource::RootPolicy
                        } else {
                            DockCapabilityValueSource::Dockable
                        };
                        assert_eq!(evaluation.effective_source(), source);
                        assert_eq!(
                            DockCapabilityResolver::is_enabled(Some(tool), capability, Some(dock)),
                            evaluation.effective_value()
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn no_policy_context_retains_base_flags_and_emits_exact_diagnostics() {
    let tool = TestNode::new(None);
    tool.set_can_close(false);
    let evaluation = DockCapabilityResolver::evaluate(Some(tool.clone()), DockCapability::Close);
    assert!(!evaluation.effective_value());
    assert_eq!(
        evaluation.effective_source(),
        DockCapabilityValueSource::Dockable
    );
    assert_eq!(
        evaluation.diagnostic_message(),
        "Capability 'CanClose' blocked by dockable flag."
    );
    assert_eq!(evaluation.root_policy_value(), None);
    assert_eq!(evaluation.dock_policy_value(), None);
    assert_eq!(evaluation.dockable_override_value(), None);
    tool.set_can_close(true);
    assert_eq!(
        DockCapabilityResolver::evaluate(Some(tool), DockCapability::Close).diagnostic_message(),
        "Capability 'CanClose' allowed by dockable flag."
    );
}

#[test]
fn evaluate_resolves_the_root_through_the_owner_hierarchy() {
    let (root, _dock, tool) = hierarchy();
    let policy = DockCapabilityPolicy::new();
    policy.set_can_float(Some(false));
    root.set_root_dock_capability_policy(Some(policy));
    let evaluation = DockCapabilityResolver::evaluate(Some(tool), DockCapability::Float);
    assert!(!evaluation.effective_value());
    assert_eq!(
        evaluation.effective_source(),
        DockCapabilityValueSource::RootPolicy
    );
    assert_eq!(
        evaluation.diagnostic_message(),
        "Capability 'CanFloat' blocked by root capability policy."
    );
}

#[test]
fn explicit_context_falls_back_to_its_owners_for_a_detached_source() {
    let (root, dock, _) = hierarchy();
    let policy = DockCapabilityPolicy::new();
    policy.set_can_drop(Some(false));
    root.set_root_dock_capability_policy(Some(policy));
    let evaluation = DockCapabilityResolver::evaluate_with_context(
        Some(TestNode::new(None)),
        DockCapability::Drop,
        Some(dock),
    );
    assert!(!evaluation.effective_value());
    assert_eq!(
        evaluation.effective_source(),
        DockCapabilityValueSource::RootPolicy
    );
}

#[test]
fn root_resolution_prefers_source_factory_then_context_factory_then_owners() {
    let source = TestNode::new(None);
    let (owner_root, dock, _) = hierarchy();
    source.set_owner(Some(dock.clone()));
    let owner_policy = DockCapabilityPolicy::new();
    owner_policy.set_can_close(Some(true));
    owner_root.set_root_dock_capability_policy(Some(owner_policy));
    let source_factory = Rc::new(RecordingFactory::default());
    let context_factory = Rc::new(RecordingFactory::default());
    source.set_factory(Some(source_factory.clone()));
    dock.set_factory(Some(context_factory.clone()));
    let factory_root = TestRoot::new(None);
    let policy = DockCapabilityPolicy::new();
    policy.set_can_close(Some(false));
    factory_root.set_root_dock_capability_policy(Some(policy));
    *source_factory.resolved_root.borrow_mut() = Some(factory_root.clone());
    let evaluate = || {
        DockCapabilityResolver::evaluate_with_context(
            Some(source.clone()),
            DockCapability::Close,
            Some(dock.clone()),
        )
    };
    assert!(!evaluate().effective_value());
    assert_eq!(source_factory.find_root_calls.get(), 1);
    assert_eq!(context_factory.find_root_calls.get(), 0);
    *source_factory.resolved_root.borrow_mut() = None;
    *context_factory.resolved_root.borrow_mut() = Some(factory_root);
    assert!(!evaluate().effective_value());
    assert_eq!(context_factory.find_root_calls.get(), 1);
    source.set_factory(None);
    assert!(!evaluate().effective_value());
    *context_factory.resolved_root.borrow_mut() = None;
    assert!(evaluate().effective_value());
}

#[test]
fn operation_and_drop_target_contexts_use_the_dock_or_immediate_owner_only() {
    let dock: Rc<dyn IDock> = TestDock::new(None);
    let node: Rc<dyn IDockable> = TestNode::new(None);
    node.set_owner(Some(dock.clone()));
    for resolve in [
        DockCapabilityResolver::resolve_operation_dock,
        DockCapabilityResolver::resolve_drop_target_dock,
    ] {
        assert!(Rc::ptr_eq(&resolve(Some(dock.clone())).unwrap(), &dock));
        assert!(Rc::ptr_eq(&resolve(Some(node.clone())).unwrap(), &dock));
        assert!(resolve(Some(TestNode::new(None))).is_none());
        let intermediate: Rc<dyn IDockable> = TestNode::new(None);
        intermediate.set_owner(Some(dock.clone()));
        node.set_owner(Some(intermediate));
        assert!(resolve(Some(node.clone())).is_none());
        node.set_owner(Some(dock.clone()));
    }
}

#[test]
fn evaluate_infers_dock_policy_but_is_enabled_with_no_context_does_not() {
    let (_root, dock, tool) = hierarchy();
    let policy = DockCapabilityPolicy::new();
    policy.set_can_drag(Some(false));
    dock.set_dock_capability_policy(Some(policy));
    assert!(
        !DockCapabilityResolver::evaluate(Some(tool.clone()), DockCapability::Drag)
            .effective_value()
    );
    assert!(DockCapabilityResolver::is_enabled(
        Some(tool.clone()),
        DockCapability::Drag,
        None
    ));
    assert!(!DockCapabilityResolver::is_enabled(
        Some(tool),
        DockCapability::Drag,
        Some(dock)
    ));
}
