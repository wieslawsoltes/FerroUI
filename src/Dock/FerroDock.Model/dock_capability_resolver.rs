// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::controls::IRootDock;
use crate::core::*;
use std::rc::Rc;

/// Resolves effective capability values using dock/root policy precedence.
pub struct DockCapabilityResolver;

impl DockCapabilityResolver {
    /// Evaluates a capability using the dockable's operation dock as policy context.
    pub fn evaluate(
        dockable: Option<Rc<dyn IDockable>>,
        capability: DockCapability,
    ) -> Rc<DockCapabilityEvaluation> {
        let context = Self::resolve_operation_dock(dockable.clone());
        Self::evaluate_with_context(dockable, capability, context)
    }

    /// Evaluates a capability using the explicitly supplied dock context.
    pub fn evaluate_with_context(
        dockable: Option<Rc<dyn IDockable>>,
        capability: DockCapability,
        dock_context: Option<Rc<dyn IDock>>,
    ) -> Rc<DockCapabilityEvaluation> {
        let dockable = dockable.expect("source capability dockable flag access");
        let base_value = Self::get_dockable_capability(&*dockable, capability);
        let root = Self::resolve_root(dockable.clone(), dock_context.clone());
        let root_policy_value = root
            .and_then(|root| root.root_dock_capability_policy())
            .and_then(|policy| policy.get(capability));
        let dock_policy_value = dock_context
            .and_then(|dock| dock.dock_capability_policy())
            .and_then(|policy| policy.get(capability));
        let dockable_override_value = dockable
            .dock_capability_overrides()
            .and_then(|overrides| overrides.get(capability));
        let mut effective_value = base_value;
        let mut source = DockCapabilityValueSource::Dockable;
        if let Some(value) = root_policy_value {
            effective_value = value;
            source = DockCapabilityValueSource::RootPolicy;
        }
        if let Some(value) = dock_policy_value {
            effective_value = value;
            source = DockCapabilityValueSource::DockPolicy;
        }
        if let Some(value) = dockable_override_value {
            effective_value = value;
            source = DockCapabilityValueSource::DockableOverride;
        }
        let message = Self::build_diagnostic_message(capability, effective_value, source);
        DockCapabilityEvaluation::new(
            capability,
            base_value,
            root_policy_value,
            dock_policy_value,
            dockable_override_value,
            effective_value,
            source,
            message,
        )
    }

    /// Gets the effective capability flag using an optional explicit context.
    /// An absent context retains root/override resolution but does not implicitly
    /// select the operation dock's own policy, matching the source overload.
    pub fn is_enabled(
        dockable: Option<Rc<dyn IDockable>>,
        capability: DockCapability,
        dock_context: Option<Rc<dyn IDock>>,
    ) -> bool {
        Self::evaluate_with_context(dockable, capability, dock_context).effective_value()
    }

    /// Resolves this dockable as a dock, or its immediate owner as a dock.
    pub fn resolve_operation_dock(dockable: Option<Rc<dyn IDockable>>) -> Option<Rc<dyn IDock>> {
        let dockable = dockable.expect("source ResolveOperationDock dockable.Owner access");
        dockable
            .clone()
            .as_dock()
            .or_else(|| dockable.owner().and_then(|owner| owner.as_dock()))
    }

    /// Resolves a drop target as a dock, or its immediate owner as a dock.
    pub fn resolve_drop_target_dock(
        target_dockable: Option<Rc<dyn IDockable>>,
    ) -> Option<Rc<dyn IDock>> {
        let target_dockable =
            target_dockable.expect("source ResolveDropTargetDock target.Owner access");
        target_dockable
            .clone()
            .as_dock()
            .or_else(|| target_dockable.owner().and_then(|owner| owner.as_dock()))
    }

    fn resolve_root(
        dockable: Rc<dyn IDockable>,
        dock_context: Option<Rc<dyn IDock>>,
    ) -> Option<Rc<dyn IRootDock>> {
        if let Some(root) = dockable
            .factory()
            .and_then(|factory| factory.find_root(Some(dockable.clone()), Some(Rc::new(|_| true))))
        {
            return Some(root);
        }
        if let Some(context) = &dock_context {
            if let Some(root) = context.factory().and_then(|factory| {
                factory.find_root(Some(context.clone()), Some(Rc::new(|_| true)))
            }) {
                return Some(root);
            }
        }
        let mut current = Some(dockable);
        while let Some(dockable) = current {
            if let Some(root) = dockable.clone().as_root_dock() {
                return Some(root);
            }
            current = dockable.owner();
        }
        if let Some(context) = dock_context {
            let mut current: Option<Rc<dyn IDockable>> = Some(context);
            while let Some(dockable) = current {
                if let Some(root) = dockable.clone().as_root_dock() {
                    return Some(root);
                }
                current = dockable.owner();
            }
        }
        None
    }

    fn get_dockable_capability(dockable: &dyn IDockable, capability: DockCapability) -> bool {
        match capability {
            DockCapability::Close => dockable.can_close(),
            DockCapability::Pin => dockable.can_pin(),
            DockCapability::Float => dockable.can_float(),
            DockCapability::Drag => dockable.can_drag(),
            DockCapability::Drop => dockable.can_drop(),
            DockCapability::DockAsDocument => dockable.can_dock_as_document(),
        }
    }

    fn build_diagnostic_message(
        capability: DockCapability,
        effective_value: bool,
        source: DockCapabilityValueSource,
    ) -> String {
        let capability = match capability {
            DockCapability::Close => "CanClose",
            DockCapability::Pin => "CanPin",
            DockCapability::Float => "CanFloat",
            DockCapability::Drag => "CanDrag",
            DockCapability::Drop => "CanDrop",
            DockCapability::DockAsDocument => "CanDockAsDocument",
        };
        let source = match source {
            DockCapabilityValueSource::Dockable => "dockable flag",
            DockCapabilityValueSource::RootPolicy => "root capability policy",
            DockCapabilityValueSource::DockPolicy => "dock capability policy",
            DockCapabilityValueSource::DockableOverride => "dockable override",
        };
        format!(
            "Capability '{capability}' {} by {source}.",
            if effective_value {
                "allowed"
            } else {
                "blocked"
            }
        )
    }
}
