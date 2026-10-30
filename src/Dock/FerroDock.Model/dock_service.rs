// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use crate::{DockCapabilityResolver, FactoryBase};
use std::rc::Rc;

/// Executes the source move/swap/split/window service algorithms through layout factories.
#[derive(Default)]
pub struct DockService;

pub(crate) fn copy_group(source: Rc<dyn IDockable>, target: Rc<dyn IDockable>) {
    if let Some(group) =
        DockGroupValidator::get_effective_dock_group(source).filter(|group| !group.is_empty())
    {
        target.set_dock_group(Some(group));
    }
}

fn last(dock: &dyn IDock) -> Option<Rc<dyn IDockable>> {
    dock.visible_dockables().and_then(|items| {
        let count = items.count();
        (count > 0).then(|| items.get(count - 1)).flatten()
    })
}

impl DockService {
    /// Creates a service with no mutable/global state.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }

    fn valid_move(
        source: &Rc<dyn IDockable>,
        owner: &dyn IDock,
        target: &dyn IDock,
        item: &Rc<dyn IDockable>,
    ) -> bool {
        if item.clone().as_dock().is_some() {
            return false;
        }
        if target
            .visible_dockables()
            .is_some_and(|items| !items.contains(&Some((item).clone())))
        {
            return false;
        }
        if owner
            .visible_dockables()
            .is_some_and(|items| !items.contains(&Some((source).clone())))
        {
            return false;
        }
        true
    }

    fn get_dockable(dock: Rc<dyn IDock>) -> Option<Rc<dyn IDockable>> {
        let item = last(dock.as_ref())?;
        if let Some(child) = item.clone().as_dock() {
            Self::get_dockable(child)
        } else {
            Some(item)
        }
    }

    fn split_tool(
        source: Rc<dyn IDockable>,
        owner: Rc<dyn IDock>,
        target: Rc<dyn IDock>,
        operation: DockOperation,
    ) {
        let Some(factory) = target.factory() else {
            return;
        };
        let created = factory.create_tool_dock();
        created.set_title("IToolDock".into());
        created.set_alignment(operation.to_alignment());
        created.set_visible_dockables(Some(factory.create_dockable_list(Vec::new())));
        let settings = (target.clone() as Rc<dyn IDockable>)
            .as_tool_dock()
            .or_else(|| (owner.clone() as Rc<dyn IDockable>).as_tool_dock());
        if let Some(settings) = settings {
            created.set_is_expanded(settings.is_expanded());
            created.set_auto_hide(settings.auto_hide());
            created.set_grip_mode(settings.grip_mode());
            FactoryBase::copy_tool_extras(settings, created.clone());
        }
        copy_group(source.clone(), created.clone());
        if !DockGroupValidator::validate_docking_groups_in_dock(source.clone(), created.clone()) {
            return;
        }
        factory.move_dockable_between_docks(Some(owner), Some(created.clone()), Some(source), None);
        factory.split_to_dock(target, created, operation);
    }

    fn split_document(
        source: Rc<dyn IDockable>,
        owner: Rc<dyn IDock>,
        target: Rc<dyn IDock>,
        operation: DockOperation,
    ) {
        let Some(factory) = target.factory() else {
            return;
        };
        let created = factory.create_document_dock();
        created.set_title("IDocumentDock".into());
        created.set_visible_dockables(Some(factory.create_dockable_list(Vec::new())));
        copy_group(source.clone(), created.clone());
        if let Some(settings) = (owner.clone() as Rc<dyn IDockable>).as_document_dock() {
            created.set_id(settings.id());
            created.set_can_create_document(settings.can_create_document());
            created.set_enable_window_drag(settings.enable_window_drag());
            FactoryBase::copy_document_extras(settings, created.clone());
        }
        if !DockGroupValidator::validate_docking_groups_in_dock(source.clone(), created.clone()) {
            return;
        }
        factory.move_dockable_between_docks(Some(owner), Some(created.clone()), Some(source), None);
        factory.split_to_dock(target, created, operation);
    }
}

impl IDockService for DockService {
    fn move_dockable_between_docks(
        &self,
        source: Rc<dyn IDockable>,
        owner: Rc<dyn IDock>,
        mut target: Rc<dyn IDock>,
        execute: bool,
    ) -> bool {
        if Rc::ptr_eq(&owner, &target)
            && target
                .visible_dockables()
                .is_some_and(|items| items.count() == 1)
        {
            return false;
        }
        let item = Self::get_dockable(target.clone());
        let Some(item) = item else {
            if !DockGroupValidator::validate_global_docking(source.clone(), target.clone()) {
                return false;
            }
            if execute {
                if let Some(factory) = owner.factory() {
                    factory.move_dockable_between_docks(
                        Some(owner),
                        Some(target),
                        Some(source),
                        None,
                    );
                }
            }
            return true;
        };
        let Some(actual) = item.owner().and_then(|owner| owner.as_dock()) else {
            return false;
        };
        target = actual;
        if !Self::valid_move(&source, owner.as_ref(), target.as_ref(), &item)
            || !DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target.clone())
        {
            return false;
        }
        if execute {
            if let Some(factory) = owner.factory() {
                factory.move_dockable_between_docks(
                    Some(owner),
                    Some(target),
                    Some(source),
                    Some(item),
                );
            }
        }
        true
    }

    fn swap_dockable_between_docks(
        &self,
        source: Rc<dyn IDockable>,
        owner: Rc<dyn IDock>,
        target: Rc<dyn IDock>,
        execute: bool,
    ) -> bool {
        let Some(item) = target.active_dockable().or_else(|| last(target.as_ref())) else {
            return false;
        };
        if !Self::valid_move(&source, owner.as_ref(), target.as_ref(), &item)
            || !DockGroupValidator::validate_docking_groups_in_dock(source.clone(), target.clone())
        {
            return false;
        }
        if execute {
            if let Some(factory) = owner.factory() {
                factory.swap_dockable_between_docks(
                    Some(owner),
                    Some(target),
                    Some(source),
                    Some(item),
                );
            }
        }
        true
    }

    fn split_dockable(
        &self,
        source: Rc<dyn IDockable>,
        owner: Rc<dyn IDock>,
        target: Rc<dyn IDock>,
        operation: DockOperation,
        execute: bool,
    ) -> bool {
        let tool = source.clone().as_tool().is_some();
        if !tool && source.clone().as_document().is_none() {
            return false;
        }
        if Rc::ptr_eq(&owner, &target)
            && target
                .visible_dockables()
                .is_some_and(|items| items.count() == 1)
        {
            return false;
        }
        if execute {
            let as_document = tool
                && target
                    .factory()
                    .and_then(|factory| factory.as_dock_split_policy())
                    .is_some_and(|policy| {
                        policy.use_document_dock(source.clone(), target.clone(), operation)
                    });
            if !tool || as_document {
                Self::split_document(source, owner, target, operation);
            } else {
                Self::split_tool(source, owner, target, operation);
            }
        }
        true
    }

    fn dock_dockable_into_window(
        &self,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDockable>,
        position: DockPoint,
        execute: bool,
    ) -> bool {
        if Rc::ptr_eq(&source, &target) {
            return false;
        }
        if !DockCapabilityResolver::is_enabled(
            Some(source.clone()),
            DockCapability::Float,
            DockCapabilityResolver::resolve_operation_dock(Some(source.clone())),
        ) {
            return false;
        }
        let Some(owner) = source.owner().and_then(|owner| owner.as_dock()) else {
            return false;
        };
        let Some(factory) = owner.factory() else {
            return false;
        };
        let Some(window_owner) = factory
            .find_root(Some(source.clone()), Some(Rc::new(|_| true)))
            .and_then(|root| root.active_dockable())
            .and_then(|active| active.as_dock())
        else {
            return false;
        };
        if execute {
            let (_, _, owner_width, owner_height) = owner.get_visible_bounds();
            let (_, _, mut width, mut height) = source.get_visible_bounds();
            if width.is_nan() {
                width = if owner_width.is_nan() {
                    300.0
                } else {
                    owner_width
                };
            }
            if height.is_nan() {
                height = if owner_height.is_nan() {
                    400.0
                } else {
                    owner_height
                };
            }
            factory.split_to_window(
                Some(window_owner),
                Some(source),
                position.x,
                position.y,
                width,
                height,
            );
        }
        true
    }

    fn dock_dockable_into_dockable(
        &self,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDockable>,
        action: DragAction,
        execute: bool,
    ) -> bool {
        let Some(owner) = source.owner().and_then(|owner| owner.as_dock()) else {
            return false;
        };
        let Some(target_owner) = target.owner().and_then(|owner| owner.as_dock()) else {
            return false;
        };
        if action != DragAction::MOVE && action != DragAction::LINK {
            return false;
        }
        if !DockGroupValidator::validate_docking_groups(source.clone(), target.clone()) {
            return false;
        }
        if execute {
            if let Some(factory) = owner.factory() {
                if Rc::ptr_eq(&owner, &target_owner) {
                    if action == DragAction::MOVE {
                        factory.move_dockable(Some(owner), Some(source), Some(target));
                    } else {
                        factory.swap_dockable(Some(owner), Some(source), Some(target));
                    }
                } else if action == DragAction::MOVE {
                    factory.move_dockable_between_docks(
                        Some(owner),
                        Some(target_owner),
                        Some(source),
                        Some(target),
                    );
                } else {
                    factory.swap_dockable_between_docks(
                        Some(owner),
                        Some(target_owner),
                        Some(source),
                        Some(target),
                    );
                }
            }
        }
        true
    }
}

impl PartialEq for DockService {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
