// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;

impl DockManager {
    pub(super) fn dock_into(
        &self,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDock>,
        action: DragAction,
        operation: DockOperation,
        execute: bool,
    ) -> bool {
        let Some(owner) = source.owner().and_then(|owner| owner.as_dock()) else {
            return false;
        };
        if action == DragAction::LINK {
            return self
                .service
                .swap_dockable_between_docks(source, owner, target, execute);
        }
        if action != DragAction::MOVE {
            return false;
        }
        match operation {
            DockOperation::Fill => self
                .service
                .move_dockable_between_docks(source, owner, target, execute),
            DockOperation::Left
            | DockOperation::Right
            | DockOperation::Top
            | DockOperation::Bottom => self
                .service
                .split_dockable(source, owner, target, operation, execute),
            DockOperation::Window => self.service.dock_dockable_into_window(
                source,
                target,
                self.screen_position(),
                execute,
            ),
            _ => false,
        }
    }
    fn visible_into(
        &self,
        source: Rc<dyn IDock>,
        target: Rc<dyn IDock>,
        action: DragAction,
        operation: DockOperation,
        execute: bool,
    ) -> bool {
        let Some(visible) = source.visible_dockables().map(|items| items.to_vec()) else {
            return true;
        };
        for item in visible {
            if !self.dock_into(
                item.expect("source sourceDockable.Owner dereference"),
                target.clone(),
                action,
                operation,
                execute,
            ) {
                return false;
            }
        }
        true
    }
    fn pane_into(
        &self,
        source: Rc<dyn IDock>,
        target: Rc<dyn IDock>,
        action: DragAction,
        operation: DockOperation,
        execute: bool,
    ) -> bool {
        let Some(visible) = source.visible_dockables().map(|items| items.to_vec()) else {
            return true;
        };
        if visible.is_empty()
            || !self.dock_into(
                visible[0]
                    .clone()
                    .expect("source sourceDockable.Owner dereference"),
                target,
                action,
                operation,
                execute,
            )
        {
            return false;
        }
        for index in 1..visible.len() {
            if !self.service.dock_dockable_into_dockable(
                visible[index]
                    .clone()
                    .expect("source sourceDockable.Owner dereference"),
                visible[index - 1]
                    .clone()
                    .expect("source targetDockable.Owner dereference"),
                action,
                execute,
            ) {
                return false;
            }
        }
        true
    }
    pub(super) fn dock_into_target(
        &self,
        source: Rc<dyn IDock>,
        hovered: Rc<dyn IDockable>,
        target: Rc<dyn IDock>,
        action: DragAction,
        operation: DockOperation,
        execute: bool,
    ) -> bool {
        match operation {
            DockOperation::Fill => self.visible_into(source, target, action, operation, execute),
            DockOperation::Window => self.service.dock_dockable_into_window(
                source,
                hovered,
                self.screen_position(),
                execute,
            ),
            _ => self.pane_into(source, target, action, operation, execute),
        }
    }
    pub(super) fn dock_into_root(
        &self,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IRootDock>,
        operation: DockOperation,
        execute: bool,
    ) -> bool {
        if operation != DockOperation::Fill {
            return false;
        }
        if !execute {
            return true;
        }
        let Some(factory) = target.factory() else {
            return false;
        };
        let Some(owner) = source.owner().and_then(|owner| owner.as_dock()) else {
            return false;
        };
        let wrapper: Rc<dyn IDock> = if source.clone().as_tool().is_some() {
            factory.create_tool_dock()
        } else if source.clone().as_document().is_some() {
            factory.create_document_dock()
        } else {
            return false;
        };
        crate::dock_service::copy_group(source.clone(), wrapper.clone());
        factory.add_dockable(Some(target.clone()), Some(wrapper.clone()));
        factory.move_dockable_between_docks(
            Some(owner),
            Some(wrapper.clone()),
            Some(source.clone()),
            None,
        );
        wrapper.set_active_dockable(Some(source));
        if target.active_dockable().is_none() {
            target.set_active_dockable(Some(wrapper));
        }
        true
    }
}
