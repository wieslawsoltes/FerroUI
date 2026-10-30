// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::FactoryBase;
use crate::core::*;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::rc::Rc;

pub(super) struct StateSyncScope<'a>(&'a FactoryBase);
impl Drop for StateSyncScope<'_> {
    fn drop(&mut self) {
        self.0
            .state_sync_depth
            .set(self.0.state_sync_depth.get() - 1);
    }
}

struct CoalescingScope<'a> {
    factory: &'a FactoryBase,
    key: usize,
}
impl Drop for CoalescingScope<'_> {
    fn drop(&mut self) {
        self.factory.state_coalescing.borrow_mut().remove(&self.key);
    }
}

fn identity(item: &Rc<dyn IDockable>) -> usize {
    Rc::as_ptr(item).cast::<()>() as usize
}
fn same(item: &Option<Rc<dyn IDockable>>, other: &Rc<dyn IDockable>) -> bool {
    item.as_ref().is_some_and(|item| Rc::ptr_eq(item, other))
}

impl IDockingWindowStateSync for FactoryBase {
    fn on_docking_window_state_property_changed(
        &self,
        dockable: Rc<dyn IDockable>,
        property: DockingWindowStateProperty,
    ) {
        self.hooks()
            .on_docking_window_state_property_changed(dockable, property);
    }
}

impl FactoryBase {
    pub(super) fn enter_state_sync_scope(&self) -> StateSyncScope<'_> {
        self.state_sync_depth.set(self.state_sync_depth.get() + 1);
        StateSyncScope(self)
    }

    /// Applies an external state request once, suppressing nested reactive writes.
    /// Scope guards restore suppression and coalescing state even during unwinding.
    pub fn on_docking_window_state_property_changed(
        &self,
        dockable: Rc<dyn IDockable>,
        property: DockingWindowStateProperty,
    ) {
        let Some(window_state) = dockable.as_docking_window_state() else {
            return;
        };
        if self.state_sync_depth.get() > 0 {
            return;
        }
        let key = identity(&dockable);
        if !self.state_coalescing.borrow_mut().insert(key) {
            return;
        }
        let _coalescing = CoalescingScope { factory: self, key };
        {
            let _scope = self.enter_state_sync_scope();
            match property {
                DockingWindowStateProperty::IsOpen => {
                    self.apply_open_request(dockable.clone(), window_state.is_open())
                }
                DockingWindowStateProperty::IsSelected => {
                    self.apply_selected_request(dockable.clone(), window_state.is_selected())
                }
                DockingWindowStateProperty::IsActive => {
                    self.apply_active_request(dockable.clone(), window_state.is_active())
                }
                DockingWindowStateProperty::DockingState => {
                    self.apply_state_request(dockable.clone(), window_state.docking_state())
                }
            }
        }
        self.synchronize_docking_window_state(Some(dockable));
    }

    fn apply_open_request(&self, dockable: Rc<dyn IDockable>, is_open: bool) {
        if self.is_docking_window_open(dockable.clone()) == is_open {
            return;
        }
        if is_open {
            if self.is_dockable_hidden(Some(dockable.clone())) {
                self.runtime().restore_dockable(Some(dockable));
            }
            return;
        }
        self.runtime().hide_dockable(Some(dockable));
    }

    fn apply_selected_request(&self, dockable: Rc<dyn IDockable>, is_selected: bool) {
        let Some(owner) = dockable.owner().and_then(|owner| owner.as_dock()) else {
            return;
        };
        if is_selected {
            if !same(&owner.active_dockable(), &dockable) {
                owner.set_active_dockable(Some(dockable));
            }
            return;
        }
        if same(&owner.active_dockable(), &dockable) {
            let fallback = find_fallback(owner.clone(), &dockable);
            owner.set_active_dockable(fallback);
        }
    }

    fn apply_active_request(&self, dockable: Rc<dyn IDockable>, is_active: bool) {
        let Some(owner) = dockable.owner().and_then(|owner| owner.as_dock()) else {
            return;
        };
        let Some(root) = self.runtime().find_root(
            Some(owner.clone()),
            Some(Rc::new(|root| root.is_focusable_root())),
        ) else {
            return;
        };
        let currently_active = same(&root.focused_dockable(), &dockable) && owner.is_active();
        if currently_active == is_active {
            return;
        }
        if is_active {
            if !same(&owner.active_dockable(), &dockable)
                && dockable.clone().as_splitter().is_none()
            {
                owner.set_active_dockable(Some(dockable.clone()));
            }
            self.runtime()
                .set_focused_dockable(Some(owner), Some(dockable));
            return;
        }
        if same(&root.focused_dockable(), &dockable) {
            let mut fallback = owner.active_dockable();
            if same(&fallback, &dockable) {
                fallback = find_fallback(owner.clone(), &dockable);
            }
            self.runtime().set_focused_dockable(Some(owner), fallback);
        }
    }

    fn apply_state_request(&self, dockable: Rc<dyn IDockable>, requested: DockingWindowState) {
        if requested == DockingWindowState::NONE {
            return;
        }
        let mut current = self
            .hooks()
            .resolve_docking_window_state(Some(dockable.clone()));
        if requested.contains(DockingWindowState::HIDDEN) {
            if !current.contains(DockingWindowState::HIDDEN) {
                self.runtime().hide_dockable(Some(dockable));
            }
            return;
        }
        if current.contains(DockingWindowState::HIDDEN) {
            self.runtime().restore_dockable(Some(dockable.clone()));
            current = self
                .hooks()
                .resolve_docking_window_state(Some(dockable.clone()));
        }
        if requested.contains(DockingWindowState::DOCUMENT)
            && !current.contains(DockingWindowState::DOCUMENT)
        {
            self.runtime().dock_as_document(dockable.clone());
            current = self
                .hooks()
                .resolve_docking_window_state(Some(dockable.clone()));
        } else if requested.contains(DockingWindowState::PINNED)
            && !current.contains(DockingWindowState::PINNED)
        {
            self.runtime().pin_dockable(Some(dockable.clone()));
            current = self
                .hooks()
                .resolve_docking_window_state(Some(dockable.clone()));
        } else if requested.contains(DockingWindowState::DOCKED)
            && current.contains(DockingWindowState::PINNED)
        {
            self.unpin_dockable(Some(dockable.clone()));
            current = self
                .hooks()
                .resolve_docking_window_state(Some(dockable.clone()));
        } else if requested.contains(DockingWindowState::DOCKED)
            && current.contains(DockingWindowState::DOCUMENT)
        {
            self.dock_as_tool_if_possible(dockable.clone());
            current = self
                .hooks()
                .resolve_docking_window_state(Some(dockable.clone()));
        }
        if requested.contains(DockingWindowState::FLOATING)
            && !current.contains(DockingWindowState::FLOATING)
        {
            self.runtime().float_dockable(Some(dockable));
        }
    }

    fn dock_as_tool_if_possible(&self, dockable: Rc<dyn IDockable>) {
        if dockable.clone().as_tool().is_none() {
            return;
        }
        let Some(source) = dockable.owner().and_then(|owner| owner.as_dock()) else {
            return;
        };
        let Some(root) = self
            .runtime()
            .find_root(Some(source.clone()), Some(Rc::new(|_| true)))
        else {
            return;
        };
        let mut target = dockable
            .original_owner()
            .and_then(|owner| owner.as_dock())
            .filter(|owner| {
                !Rc::ptr_eq(owner, &source) && self.is_owner_attached(root.clone(), owner.clone())
            });
        if target.is_none() {
            target = self
                .find_in_dock(
                    Some(root.clone()),
                    Some(Rc::new(|item| {
                        item.and_then(|item| item.as_tool_dock()).is_some()
                    })),
                )
                .iter()
                .filter_map(|item| item.and_then(|item| item.as_dock()))
                .find(|item| {
                    !Rc::ptr_eq(item, &source)
                        && !root
                            .pinned_dock()
                            .is_some_and(|pinned| Rc::ptr_eq(item, &(pinned as Rc<dyn IDock>)))
                });
        }
        let Some(target) = target else { return };
        // IList LastOrDefault uses Count/indexing rather than enumerating the list.
        let last = target.visible_dockables().and_then(|items| {
            let count = items.count();
            (count > 0).then(|| items.get(count - 1)).flatten()
        });
        self.runtime().move_dockable_between_docks(
            Some(source),
            Some(target),
            Some(dockable),
            last,
        );
    }

    fn is_docking_window_open(&self, dockable: Rc<dyn IDockable>) -> bool {
        if self.is_dockable_hidden(Some(dockable.clone())) {
            return false;
        }
        if let Some(owner) = dockable.owner().and_then(|owner| owner.as_dock()) {
            if owner
                .visible_dockables()
                .is_some_and(|items| items.contains(&Some((dockable).clone())))
            {
                return true;
            }
            if let Some(split) = (owner as Rc<dyn IDockable>).as_split_view_dock() {
                if same(&split.pane_dockable(), &dockable)
                    || same(&split.content_dockable(), &dockable)
                {
                    return true;
                }
            }
        }
        let Some(root) = dockable.owner().and_then(|owner| owner.as_root_dock()) else {
            return false;
        };
        root.left_pinned_dockables()
            .is_some_and(|items| items.contains(&Some((dockable).clone())))
            || root
                .right_pinned_dockables()
                .is_some_and(|items| items.contains(&Some((dockable).clone())))
            || root
                .top_pinned_dockables()
                .is_some_and(|items| items.contains(&Some((dockable).clone())))
            || root
                .bottom_pinned_dockables()
                .is_some_and(|items| items.contains(&Some((dockable).clone())))
    }

    fn is_docking_window_selected(&self, dockable: Rc<dyn IDockable>) -> bool {
        !self.is_dockable_hidden(Some(dockable.clone()))
            && dockable
                .owner()
                .and_then(|owner| owner.as_dock())
                .is_some_and(|owner| same(&owner.active_dockable(), &dockable))
    }

    fn is_docking_window_active(&self, dockable: Rc<dyn IDockable>) -> bool {
        if self.is_dockable_hidden(Some(dockable.clone())) {
            return false;
        }
        let Some(owner) = dockable.owner().and_then(|owner| owner.as_dock()) else {
            return false;
        };
        if !owner.is_active() {
            return false;
        }
        self.runtime()
            .find_root(
                Some(dockable.clone()),
                Some(Rc::new(|root| root.is_focusable_root())),
            )
            .is_some_and(|root| same(&root.focused_dockable(), &dockable))
    }

    pub(super) fn set_docking_window_state_flags(
        &self,
        dockable: Rc<dyn IDockable>,
        resolved: DockingWindowState,
    ) {
        let Some(state) = dockable.as_docking_window_state() else {
            return;
        };
        let open = self.is_docking_window_open(dockable.clone());
        let selected = self.is_docking_window_selected(dockable.clone());
        let active = self.is_docking_window_active(dockable.clone());
        let _scope = self.enter_state_sync_scope();
        if state.docking_state() != resolved {
            state.set_docking_state(resolved);
        }
        if state.is_open() != open {
            state.set_is_open(open);
        }
        if state.is_selected() != selected {
            state.set_is_selected(selected);
        }
        if state.is_active() != active {
            state.set_is_active(active);
        }
    }

    pub(super) fn synchronize_docking_window_state(&self, dockable: Option<Rc<dyn IDockable>>) {
        if let Some(dockable) = dockable {
            let resolved = self
                .hooks()
                .resolve_docking_window_state(Some(dockable.clone()));
            self.set_docking_window_state_flags(dockable, resolved);
        }
    }

    pub(super) fn synchronize_docking_window_selection(&self, owner: Option<Rc<dyn IDock>>) {
        for child in
            enumerate_owned(owner.expect("source selection dereferences owner.VisibleDockables"))
        {
            self.synchronize_docking_window_state(child);
        }
    }
}

fn find_fallback(owner: Rc<dyn IDock>, current: &Rc<dyn IDockable>) -> Option<Rc<dyn IDockable>> {
    enumerate_owned(owner)
        .find(|item| {
            !same(item, current) && item.clone().and_then(|item| item.as_splitter()).is_none()
        })
        .flatten()
}

type OwnedDockables = Box<dyn Iterator<Item = Option<Rc<dyn IDockable>>>>;

fn enumerate_owned(dock: Rc<dyn IDock>) -> impl Iterator<Item = Option<Rc<dyn IDockable>>> {
    let mut stage = 0;
    let mut visible: Option<OwnedDockables> = None;
    // The source HashSet holds strong references for the enumeration lifetime.
    // Retain the items as well as their identity keys to prevent address reuse.
    let mut seen = HashMap::new();
    std::iter::from_fn(move || loop {
        match stage {
            0 => {
                visible = if dock.visible_dockables().is_some() {
                    Some(
                        dock.visible_dockables()
                            .expect("source visible list exists")
                            .iter(),
                    )
                } else {
                    None
                };
                stage = 1;
            }
            1 => {
                if let Some(item) = visible.as_mut().and_then(|items| items.next()) {
                    if let Entry::Vacant(entry) = seen.entry(item.as_ref().map_or(0, identity)) {
                        entry.insert(item.clone());
                        return Some(item);
                    }
                } else {
                    visible = None;
                    stage = 2;
                }
            }
            2 | 3 => {
                let Some(split) = (dock.clone() as Rc<dyn IDockable>).as_split_view_dock() else {
                    stage = 4;
                    continue;
                };
                let item = if stage == 2 {
                    split.pane_dockable()
                } else {
                    split.content_dockable()
                };
                stage += 1;
                if let Some(item) = item {
                    if let Entry::Vacant(entry) = seen.entry(identity(&item)) {
                        entry.insert(Some(item.clone()));
                        return Some(Some(item));
                    }
                }
            }
            _ => return None,
        }
    })
}
