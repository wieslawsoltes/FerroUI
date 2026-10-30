// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::tracking::same;
use super::FactoryBase;
use crate::core::*;
use std::rc::Rc;

impl FactoryBase {
    pub(super) fn set_is_active(dockable: Option<Rc<dyn IDockable>>, active: bool) {
        if let Some(dock) = dockable.and_then(|item| item.as_dock()) {
            dock.set_is_active(active);
        }
    }

    /// Transfers focus, deactivates owners in other registered roots and reconciles selection flags.
    pub fn set_focused_dockable(
        &self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
    ) {
        let Some(root) = self.runtime().find_root(
            dock.clone().map(|dock| dock as Rc<dyn IDockable>),
            Some(Rc::new(|root| root.is_focusable_root())),
        ) else {
            return;
        };
        let previous = root.focused_dockable();
        let owner_was_active = previous
            .as_ref()
            .and_then(|item| item.owner())
            .and_then(|owner| owner.as_dock())
            .is_some_and(|owner| owner.is_active());
        if dockable.is_some() {
            for result in self
                .find(Some(Rc::new(|item| {
                    item.and_then(|item| item.as_root_dock()).is_some()
                })))
                .iter()
            {
                if let Some(other) = result.and_then(|item| item.as_root_dock()) {
                    if other.is_focusable_root()
                        && !Rc::ptr_eq(&other, &root)
                        && other
                            .focused_dockable()
                            .and_then(|item| item.owner())
                            .is_some()
                    {
                        Self::set_is_active(
                            other
                                .focused_dockable()
                                .expect("source focused item exists")
                                .owner(),
                            false,
                        );
                        self.runtime()
                            .on_dockable_deactivated(other.focused_dockable());
                        self.synchronize_docking_window_state(other.focused_dockable());
                        if let Some(owner) = other
                            .focused_dockable()
                            .expect("source focused item exists")
                            .owner()
                            .and_then(|owner| owner.as_dock())
                        {
                            self.synchronize_docking_window_selection(Some(owner));
                        }
                    }
                }
            }
        }
        if !same(&root.focused_dockable(), &dockable)
            && root
                .focused_dockable()
                .and_then(|item| item.owner())
                .is_some()
        {
            Self::set_is_active(
                root.focused_dockable()
                    .expect("source focused item exists")
                    .owner(),
                false,
            );
            self.runtime()
                .on_dockable_deactivated(root.focused_dockable());
        }
        if dockable.is_some() {
            if !same(&root.focused_dockable(), &dockable) {
                root.set_focused_dockable(dockable.clone());
            }
        } else {
            root.set_focused_dockable(None);
        }
        if root
            .focused_dockable()
            .and_then(|item| item.owner())
            .is_some()
        {
            Self::set_is_active(
                root.focused_dockable()
                    .expect("source focused item exists")
                    .owner(),
                true,
            );
        }
        if dockable.is_some() && same(&previous, &root.focused_dockable()) && !owner_was_active {
            self.runtime().on_focused_dockable_changed(dockable);
        }
        if let Some(previous) = previous {
            if !same(&Some(previous.clone()), &root.focused_dockable()) {
                self.synchronize_docking_window_state(Some(previous.clone()));
                if let Some(owner) = previous.owner().and_then(|owner| owner.as_dock()) {
                    self.synchronize_docking_window_selection(Some(owner));
                }
            }
        }
        if root.focused_dockable().is_some() {
            self.synchronize_docking_window_state(root.focused_dockable());
            if let Some(owner) = root
                .focused_dockable()
                .expect("source focused item exists")
                .owner()
                .and_then(|owner| owner.as_dock())
            {
                self.synchronize_docking_window_selection(Some(owner));
            }
        }
        self.synchronize_docking_window_selection(dock);
    }
}
