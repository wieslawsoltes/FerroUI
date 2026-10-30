// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::controls::*;
use crate::core::*;
use crate::dock_outer_split as outer;
use crate::DockCapabilityResolver;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::rc::Rc;

#[path = "dock_manager_docking.rs"]
mod docking;
#[path = "dock_manager_pinned.rs"]
mod pinned;

/// Validates docking interactions and executes the source service pipeline with shared policy options.
pub struct DockManager {
    service: Rc<dyn IDockService>,
    options: Rc<DockManagerOptions>,
    position: Cell<DockPoint>,
    screen_position: Cell<DockPoint>,
    prevent_size_conflicts: Cell<bool>,
    last_capability: RefCell<Option<Rc<DockCapabilityEvaluation>>>,
}
impl DockManager {
    /// Creates a manager with fresh, enabled options and size-conflict prevention.
    pub fn new(service: Rc<dyn IDockService>) -> Rc<Self> {
        Self::with_options(service, DockManagerOptions::new())
    }
    /// Creates a manager retaining the supplied service and shared options.
    pub fn with_options(
        service: Rc<dyn IDockService>,
        options: Rc<DockManagerOptions>,
    ) -> Rc<Self> {
        Rc::new(Self {
            service,
            options,
            position: Cell::new(DockPoint::default()),
            screen_position: Cell::new(DockPoint::default()),
            prevent_size_conflicts: Cell::new(true),
            last_capability: RefCell::new(None),
        })
    }
    /// Gets the same shared options reference supplied at construction.
    pub fn options(&self) -> Rc<DockManagerOptions> {
        self.options.clone()
    }
    fn clear_capability(&self) {
        let previous = self.last_capability.replace(None);
        drop(previous);
    }
    fn capability_allowed(
        &self,
        item: Rc<dyn IDockable>,
        capability: DockCapability,
        context: Option<Rc<dyn IDock>>,
    ) -> bool {
        let evaluation =
            DockCapabilityResolver::evaluate_with_context(Some(item), capability, context);
        if evaluation.effective_value() {
            return true;
        }
        self.last_capability.replace(Some(evaluation));
        false
    }
    fn operation_allowed(item: &dyn IDockable, operation: DockOperation, source: bool) -> bool {
        item.as_docking_restrictions().is_none_or(|restrictions| {
            let mask = if source {
                restrictions.allowed_dock_operations()
            } else {
                restrictions.allowed_drop_operations()
            };
            mask.allows(operation)
        })
    }
    fn docking_allowed(
        &self,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDockable>,
        operation: DockOperation,
    ) -> bool {
        if !self.is_docking_enabled()
            || !Self::operation_allowed(source.as_ref(), operation, true)
            || !Self::operation_allowed(target.as_ref(), operation, false)
        {
            return false;
        }
        if let Some(policy) = source
            .factory()
            .and_then(|factory| factory.as_dock_drop_policy())
        {
            if !policy.can_drop(source.clone(), target.clone(), operation) {
                return false;
            }
        }
        let same = match (source.factory(), target.factory()) {
            (Some(a), Some(b)) => Rc::ptr_eq(&a, &b),
            (None, None) => true,
            _ => false,
        };
        if !same {
            if let Some(policy) = target
                .factory()
                .and_then(|factory| factory.as_dock_drop_policy())
            {
                if !policy.can_drop(source, target, operation) {
                    return false;
                }
            }
        }
        true
    }
    fn allows_document(&self, source: Rc<dyn IDockable>, target: Rc<dyn IDockable>) -> bool {
        if target.clone().as_document_dock().is_none() && target.as_document().is_none() {
            return true;
        }
        self.capability_allowed(
            source.clone(),
            DockCapability::DockAsDocument,
            DockCapabilityResolver::resolve_operation_dock(Some(source)),
        )
    }
    fn prerequisites(
        &self,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDockable>,
        operation: DockOperation,
    ) -> bool {
        self.clear_capability();
        self.docking_allowed(source.clone(), target.clone(), operation)
            && self.capability_allowed(
                source.clone(),
                DockCapability::Drag,
                DockCapabilityResolver::resolve_operation_dock(Some(source.clone())),
            )
            && self.capability_allowed(
                target.clone(),
                DockCapability::Drop,
                DockCapabilityResolver::resolve_drop_target_dock(Some(target.clone())),
            )
            && self.allows_document(source, target)
    }
    fn size_conflict(a: &dyn ITool, b: &dyn ITool) -> bool {
        fn fixed(min: f64, max: f64) -> bool {
            !min.is_nan() && !max.is_nan() && min == max
        }
        let width = fixed(a.min_width(), a.max_width())
            && fixed(b.min_width(), b.max_width())
            && a.min_width() != b.min_width();
        let height = fixed(a.min_height(), a.max_height())
            && fixed(b.min_height(), b.max_height())
            && a.min_height() != b.min_height();
        width || height
    }
    fn validate_children(
        &self,
        source: Rc<dyn IDock>,
        target: Rc<dyn IDockable>,
        action: DragAction,
        operation: DockOperation,
        execute: bool,
    ) -> bool {
        if source.visible_dockables().is_none() || source.visible_dockables().unwrap().count() == 0
        {
            return false;
        }
        let mut first = None;
        let mut all = true;
        let count = source.visible_dockables().unwrap().count();
        for index in (0..count).rev() {
            let child = source
                .visible_dockables()
                .expect("source visible list exists")
                .get(index);
            let Some(dock) = child.and_then(|item| item.as_dock()) else {
                continue;
            };
            let valid = self.validate_dockable(dock, target.clone(), action, operation, execute);
            if !valid && first.is_none() {
                first = self.last_capability_evaluation();
            }
            all &= valid;
        }
        if !all && first.is_some() {
            self.last_capability.replace(first);
        }
        all
    }
    fn validate_core(
        &self,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDockable>,
        action: DragAction,
        operation: DockOperation,
        execute: bool,
    ) -> bool {
        self.clear_capability();
        if outer::direction(operation) != DockOperation::None {
            return self.validate_outer(source, target, action, operation, execute);
        }
        if !self.docking_allowed(source.clone(), target.clone(), operation) {
            return false;
        }
        if let Some(dock) = source.clone().as_tool_dock() {
            return self.validate_dock(dock, target, action, operation, execute);
        }
        if let Some(dock) = source.clone().as_document_dock() {
            return self.validate_dock(dock, target, action, operation, execute);
        }
        if let Some(tool) = source.clone().as_tool() {
            return self.validate_tool(tool, target, action, operation, execute);
        }
        if let Some(document) = source.clone().as_document() {
            return self.validate_document(document, target, action, operation, execute);
        }
        let group: Option<Rc<dyn IDock>> = if let Some(dock) = source.clone().as_proportional_dock()
        {
            Some(dock)
        } else if let Some(dock) = source.clone().as_stack_dock() {
            Some(dock)
        } else if let Some(dock) = source.clone().as_grid_dock() {
            Some(dock)
        } else if let Some(dock) = source.clone().as_wrap_dock() {
            Some(dock)
        } else if let Some(dock) = source.as_uniform_grid_dock() {
            Some(dock)
        } else {
            None
        };
        group.is_some_and(|dock| self.validate_children(dock, target, action, operation, execute))
    }
    fn validate_outer(
        &self,
        source: Rc<dyn IDockable>,
        hovered: Rc<dyn IDockable>,
        action: DragAction,
        operation: DockOperation,
        execute: bool,
    ) -> bool {
        if action != DragAction::MOVE {
            return false;
        }
        let Some(owner) = source.owner().and_then(|owner| owner.as_dock()) else {
            return false;
        };
        if !owner
            .visible_dockables()
            .is_some_and(|items| items.contains(&Some((source).clone())))
        {
            return false;
        }
        let Some(policy) = hovered
            .factory()
            .and_then(|factory| factory.as_dock_outer_split_policy())
        else {
            return false;
        };
        let Some(target) =
            policy.get_outer_split_target(source.clone(), hovered.clone(), operation)
        else {
            return false;
        };
        let Some(parent) = target.owner().and_then(|owner| owner.as_dock()) else {
            return false;
        };
        if !parent.visible_dockables().is_some_and(|items| {
            items.contains(&Some((target.clone() as Rc<dyn IDockable>).clone()))
        }) {
            return false;
        }
        let Some(factory) = target.factory() else {
            return false;
        };
        if !source
            .factory()
            .is_some_and(|current| Rc::ptr_eq(&current, &factory))
            || !hovered
                .factory()
                .is_some_and(|current| Rc::ptr_eq(&current, &factory))
        {
            return false;
        }
        if !outer::contains(target.clone(), Some(hovered.clone()))
            || outer::contains(source.clone(), Some(target.clone()))
        {
            return false;
        }
        if outer::contains(target.clone(), Some(source.clone()))
            && !outer::has_other_content(Some(target.clone()), source.clone())
        {
            return false;
        }
        if !self.docking_allowed(source.clone(), hovered.clone(), operation)
            || !self.docking_allowed(source.clone(), target.clone(), operation)
            || !self.capability_allowed(
                hovered.clone(),
                DockCapability::Drop,
                DockCapabilityResolver::resolve_drop_target_dock(Some(hovered)),
            )
            || !self.capability_allowed(
                target.clone(),
                DockCapability::Drop,
                DockCapabilityResolver::resolve_drop_target_dock(Some(target.clone())),
            )
            || !self.validate_outer_source(source.clone(), target.clone(), operation)
        {
            return false;
        }
        if execute {
            outer::execute(source, target, outer::direction(operation));
        }
        true
    }
    fn validate_outer_source(
        &self,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDock>,
        operation: DockOperation,
    ) -> bool {
        if !self.docking_allowed(source.clone(), target.clone(), operation)
            || !self.capability_allowed(
                source.clone(),
                DockCapability::Drag,
                DockCapabilityResolver::resolve_operation_dock(Some(source.clone())),
            )
            || !DockGroupValidator::validate_docking_groups(source.clone(), target.clone())
        {
            return false;
        }
        let Some(dock) = source.clone().as_dock() else {
            return source.clone().as_document().is_some() || source.as_tool().is_some();
        };
        if dock.visible_dockables().is_none() {
            return false;
        }
        let mut content = false;
        let mut index = 0;
        while index
            < dock
                .visible_dockables()
                .expect("source visible list exists")
                .count()
        {
            let child = dock
                .visible_dockables()
                .expect("source visible list exists")
                .get(index);
            if child.clone().and_then(|item| item.as_splitter()).is_some() {
                index += 1;
                continue;
            }
            if !self.validate_outer_source(
                child.expect("source docking capability member dereference"),
                target.clone(),
                operation,
            ) {
                return false;
            }
            content = true;
            index += 1;
        }
        content
    }
}
impl IDockManager for DockManager {
    fn position(&self) -> DockPoint {
        self.position.get()
    }
    fn set_position(&self, value: DockPoint) {
        self.position.set(value);
    }
    fn screen_position(&self) -> DockPoint {
        self.screen_position.get()
    }
    fn set_screen_position(&self, value: DockPoint) {
        self.screen_position.set(value);
    }
    fn prevent_size_conflicts(&self) -> bool {
        self.prevent_size_conflicts.get()
    }
    fn set_prevent_size_conflicts(&self, value: bool) {
        self.prevent_size_conflicts.set(value);
    }
    fn is_docking_enabled(&self) -> bool {
        self.options.is_docking_enabled()
    }
    fn set_is_docking_enabled(&self, value: bool) {
        self.options.set_is_docking_enabled(value);
    }
    fn last_capability_evaluation(&self) -> Option<Rc<DockCapabilityEvaluation>> {
        self.last_capability.borrow().clone()
    }
    fn validate_tool(
        &self,
        source: Rc<dyn ITool>,
        target: Rc<dyn IDockable>,
        action: DragAction,
        operation: DockOperation,
        execute: bool,
    ) -> bool {
        if !self.prerequisites(source.clone(), target.clone(), operation) {
            return false;
        }
        if let Some(root) = target.clone().as_root_dock() {
            return self.dock_into_root(source, root, operation, execute);
        }
        if let Some(dock) = target.clone().as_tool_dock() {
            if self.prevent_size_conflicts() {
                if let Some(items) = dock.visible_dockables() {
                    for item in items.iter() {
                        if let Some(tool) = item.and_then(|item| item.as_tool()) {
                            if Self::size_conflict(source.as_ref(), tool.as_ref()) {
                                return false;
                            }
                        }
                    }
                }
            }
            return self.dock_into(source, dock, action, operation, execute);
        }
        if let Some(dock) = target.clone().as_document_dock() {
            return self.dock_into(source, dock, action, operation, execute);
        }
        if let Some(tool) = target.clone().as_tool() {
            return self.tool_to_tool(source, tool, action, execute);
        }
        if let Some(document) = target.clone().as_document() {
            return self
                .service
                .dock_dockable_into_dockable(source, document, action, execute);
        }
        if let Some(dock) = target.as_proportional_dock() {
            return self.dock_into(source, dock, action, operation, execute);
        }
        false
    }
    fn validate_document(
        &self,
        source: Rc<dyn IDocument>,
        target: Rc<dyn IDockable>,
        action: DragAction,
        operation: DockOperation,
        execute: bool,
    ) -> bool {
        if !self.prerequisites(source.clone(), target.clone(), operation) {
            return false;
        }
        if target.clone().as_tool().is_some() || target.clone().as_tool_dock().is_some() {
            return false;
        }
        if let Some(root) = target.clone().as_root_dock() {
            return self.dock_into_root(source, root, operation, execute);
        }
        if let Some(dock) = target.clone().as_document_dock() {
            return self.dock_into(source, dock, action, operation, execute);
        }
        if let Some(document) = target.clone().as_document() {
            return self
                .service
                .dock_dockable_into_dockable(source, document, action, execute);
        }
        if let Some(dock) = target.as_proportional_dock() {
            return self.dock_into(source, dock, action, operation, execute);
        }
        false
    }
    fn validate_dock(
        &self,
        source: Rc<dyn IDock>,
        target: Rc<dyn IDockable>,
        action: DragAction,
        operation: DockOperation,
        execute: bool,
    ) -> bool {
        if !self.prerequisites(source.clone(), target.clone(), operation) {
            return false;
        }
        if target.clone().as_root_dock().is_some() {
            return self.service.dock_dockable_into_window(
                source,
                target,
                self.screen_position(),
                execute,
            );
        }
        let dock: Option<Rc<dyn IDock>> = if let Some(dock) = target.clone().as_tool_dock() {
            Some(dock)
        } else if let Some(dock) = target
            .clone()
            .as_tool()
            .and_then(|tool| tool.owner())
            .and_then(|owner| owner.as_tool_dock())
        {
            Some(dock)
        } else if let Some(dock) = target.clone().as_document_dock() {
            Some(dock)
        } else if let Some(dock) = target
            .clone()
            .as_document()
            .and_then(|document| document.owner())
            .and_then(|owner| owner.as_document_dock())
        {
            Some(dock)
        } else {
            target
                .clone()
                .as_proportional_dock()
                .map(|dock| dock as Rc<dyn IDock>)
        };
        if let Some(dock) = dock {
            if !Rc::ptr_eq(&source, &dock) {
                return self.dock_into_target(source, target, dock, action, operation, execute);
            }
        }
        false
    }
    fn validate_dockable(
        &self,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDockable>,
        action: DragAction,
        operation: DockOperation,
        execute: bool,
    ) -> bool {
        if execute {
            if let Some(observer) = source
                .factory()
                .and_then(|factory| factory.as_dock_operation_observer())
            {
                if !self.validate_core(source.clone(), target.clone(), action, operation, false) {
                    return false;
                }
                let scope =
                    observer.begin_dock_operation(source.clone(), target.clone(), operation);
                let result = catch_unwind(AssertUnwindSafe(|| {
                    let result = self.validate_core(source, target, action, operation, true);
                    if result {
                        if let Some(scope) = &scope {
                            scope.complete();
                        }
                    }
                    result
                }));
                if let Some(scope) = scope {
                    scope.dispose();
                }
                return match result {
                    Ok(result) => result,
                    Err(error) => resume_unwind(error),
                };
            }
        }
        self.validate_core(source, target, action, operation, execute)
    }
    fn is_dock_target_visible(
        &self,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDockable>,
        operation: DockOperation,
    ) -> bool {
        if outer::direction(operation) != DockOperation::None {
            return self.validate_outer(source, target, DragAction::MOVE, operation, false);
        }
        if !self.docking_allowed(source.clone(), target.clone(), operation) {
            return false;
        }
        if operation != DockOperation::Fill {
            return true;
        }
        !Rc::ptr_eq(&source, &target)
            && !source
                .owner()
                .is_some_and(|owner| Rc::ptr_eq(&owner, &target))
    }
}
impl PartialEq for DockManager {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
