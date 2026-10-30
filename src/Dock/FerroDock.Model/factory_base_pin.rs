// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::tracking::same;
use super::FactoryBase;
use crate::collections::ListHandle;
use crate::controls::*;
use crate::core::*;
use crate::DockCapabilityResolver;
use std::rc::Rc;

pub(super) fn is_valid_size(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

pub(super) fn pinned_list(
    root: &dyn IRootDock,
    alignment: Alignment,
) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
    match alignment {
        Alignment::Unset | Alignment::Left => root.left_pinned_dockables(),
        Alignment::Right => root.right_pinned_dockables(),
        Alignment::Top => root.top_pinned_dockables(),
        Alignment::Bottom => root.bottom_pinned_dockables(),
    }
}
fn set_pinned_list(
    root: &dyn IRootDock,
    alignment: Alignment,
    list: ListHandle<Option<Rc<dyn IDockable>>>,
) {
    match alignment {
        Alignment::Unset | Alignment::Left => root.set_left_pinned_dockables(Some(list)),
        Alignment::Right => root.set_right_pinned_dockables(Some(list)),
        Alignment::Top => root.set_top_pinned_dockables(Some(list)),
        Alignment::Bottom => root.set_bottom_pinned_dockables(Some(list)),
    }
}

impl FactoryBase {
    /// Toggles pinning when the source Pin capability allows the command.
    pub fn pin_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        if !DockCapabilityResolver::is_enabled(
            dockable.clone(),
            DockCapability::Pin,
            DockCapabilityResolver::resolve_operation_dock(dockable.clone()),
        ) {
            return;
        }
        self.hooks().pin_dockable_core(dockable);
    }

    /// Toggles pinning without command eligibility checks, using the current owner root.
    pub fn pin_dockable_core(&self, dockable: Option<Rc<dyn IDockable>>) {
        if let Some(root) = self
            .runtime()
            .find_root(dockable.clone(), Some(Rc::new(|_| true)))
        {
            self.hooks()
                .pin_dockable_core_with_root(dockable, Some(root), None);
        }
    }

    /// Toggles native pin state in an explicit root, preserving source ownership and notification order.
    pub fn pin_dockable_core_with_root(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        root: Option<Rc<dyn IRootDock>>,
        unpin: Option<Rc<dyn IToolDock>>,
    ) {
        let dockable = dockable.expect("source pin dockable.Owner access");
        let Some(owner) = dockable.owner().and_then(|owner| owner.as_tool_dock()) else {
            if self.is_dockable_pinned(Some(dockable.clone()), root.clone()) {
                self.unpin_dockable_internal(
                    dockable,
                    root.expect("source unpin root.PinnedDock access"),
                    None,
                    false,
                    unpin,
                );
            }
            return;
        };
        let visible = owner
            .visible_dockables()
            .is_some_and(|items| items.contains(&Some((dockable).clone())));
        let pinned = self.is_dockable_pinned(Some(dockable.clone()), root.clone());
        let original = dockable
            .original_owner()
            .and_then(|owner| owner.as_tool_dock());
        let alignment = original
            .as_ref()
            .map_or_else(|| owner.alignment(), |owner| owner.alignment());
        if visible && !pinned {
            Self::update_pinned_bounds_from_visible(dockable.clone(), owner.clone());
            let root = root.expect("source pin root pinned collection access");
            if pinned_list(root.as_ref(), alignment).is_none() {
                set_pinned_list(
                    root.as_ref(),
                    alignment,
                    (self.create_dockable_list)(Vec::new()),
                );
            }
            if owner.visible_dockables().is_some() {
                self.remove_visible_dockable(Some(owner.clone()), Some(dockable.clone()));
                self.runtime().on_dockable_removed(Some(dockable.clone()));
            }
            if !same(
                &self.runtime().find_root(Some(owner.clone()), None),
                &Some(root.clone()),
            ) {
                dockable.set_original_owner(Some(owner));
                self.runtime()
                    .init_dockable(Some(dockable.clone()), Some(root.clone()));
            }
            if pinned_list(root.as_ref(), alignment).is_some() {
                pinned_list(root.as_ref(), alignment)
                    .expect("source pinned list exists")
                    .add(Some(dockable.clone()));
                self.runtime().on_dockable_pinned(Some(dockable.clone()));
            }
            self.hooks().update_docking_window_state(Some(dockable));
        } else if pinned {
            self.unpin_dockable_internal(
                dockable.clone(),
                root.expect("source unpin root.PinnedDock access"),
                Some(owner),
                visible,
                unpin,
            );
            self.hooks().update_docking_window_state(Some(dockable));
        }
        // Source leaves active/expanded/autohide and invalid-state handling unspecified.
    }

    fn unpin_dockable_internal(
        &self,
        dockable: Rc<dyn IDockable>,
        root: Rc<dyn IRootDock>,
        current: Option<Rc<dyn IToolDock>>,
        visible: bool,
        unpin: Option<Rc<dyn IToolDock>>,
    ) {
        let preview = root.pinned_dock();
        let original = dockable
            .original_owner()
            .and_then(|owner| owner.as_tool_dock());
        let mut alignment = Self::get_pinned_dockable_alignment(dockable.clone(), root.clone());
        if alignment == Alignment::Unset {
            alignment = original
                .as_ref()
                .map(|owner| owner.alignment())
                .or_else(|| current.as_ref().map(|owner| owner.alignment()))
                .unwrap_or(Alignment::Left);
        }
        self.remove_pinned_dockable(root.clone(), dockable.clone(), alignment);
        if visible {
            self.hide_previewing_dockables_internal(Some(root.clone()), false);
        }
        let mut target = unpin.clone().or(original);
        if unpin.is_none()
            && target.as_ref().is_some_and(|target| {
                !self.is_owner_attached(root.clone(), target.clone())
                    || !Self::is_owner_aligned(target.clone(), alignment)
            })
        {
            target = None;
        }
        if target.is_none() {
            if let Some(current) = current.filter(|current| !same(&Some(current.clone()), &preview))
            {
                if self.is_owner_attached(root.clone(), current.clone())
                    && Self::is_owner_aligned(current.clone(), alignment)
                {
                    target = Some(current);
                }
            }
        }
        let target = target
            .or_else(|| self.find_tool_dock_by_alignment(root.clone(), alignment))
            .unwrap_or_else(|| self.create_tool_dock_for_unpin(root, alignment));
        self.add_visible_dockable(Some(target.clone()), Some(dockable.clone()));
        self.runtime().on_dockable_added(Some(dockable.clone()));
        self.runtime()
            .init_dockable(Some(dockable.clone()), Some(target.clone()));
        target.set_active_dockable(Some(dockable.clone()));
        dockable.set_original_owner(None);
        self.hooks().update_docking_window_state(Some(dockable));
    }

    fn is_owner_aligned(owner: Rc<dyn IDock>, alignment: Alignment) -> bool {
        alignment == Alignment::Unset
            || (owner as Rc<dyn IDockable>)
                .as_tool_dock()
                .is_some_and(|owner| owner.alignment() == alignment)
    }

    fn remove_pinned_dockable(
        &self,
        root: Rc<dyn IRootDock>,
        dockable: Rc<dyn IDockable>,
        alignment: Alignment,
    ) {
        let mut removed = false;
        if alignment != Alignment::Unset {
            removed |= pinned_list(root.as_ref(), alignment)
                .is_some_and(|items| items.remove(&Some((dockable).clone())));
        }
        for get in [
            IRootDock::left_pinned_dockables,
            IRootDock::right_pinned_dockables,
            IRootDock::top_pinned_dockables,
            IRootDock::bottom_pinned_dockables,
        ] {
            removed |=
                get(root.as_ref()).is_some_and(|items| items.remove(&Some((dockable).clone())));
        }
        if removed {
            self.runtime().on_dockable_unpinned(Some(dockable));
        }
    }

    fn create_tool_dock_for_unpin(
        &self,
        root: Rc<dyn IRootDock>,
        alignment: Alignment,
    ) -> Rc<dyn IToolDock> {
        let target = self.runtime().create_tool_dock();
        target.set_title("IToolDock".into());
        target.set_alignment(alignment);
        target.set_visible_dockables(Some((self.create_dockable_list)(Vec::new())));
        let anchor = self
            .get_preferred_anchor_dock(root.clone())
            .or_else(|| root.active_dockable().and_then(|item| item.as_dock()));
        if let Some(anchor) = anchor {
            let operation = match alignment {
                Alignment::Right => DockOperation::Right,
                Alignment::Top => DockOperation::Top,
                Alignment::Bottom => DockOperation::Bottom,
                _ => DockOperation::Left,
            };
            self.runtime()
                .split_to_dock(anchor, target.clone(), operation);
        } else {
            if root.visible_dockables().is_none() {
                root.set_visible_dockables(Some((self.create_dockable_list)(Vec::new())));
            }
            self.add_visible_dockable(Some(root.clone()), Some(target.clone()));
            self.runtime().on_dockable_added(Some(target.clone()));
            self.runtime()
                .init_dockable(Some(target.clone()), Some(root));
        }
        target
    }

    fn get_preferred_anchor_dock(&self, root: Rc<dyn IRootDock>) -> Option<Rc<dyn IDock>> {
        let excluded_root = root.clone();
        self.find_in_dock(
            Some(root.clone()),
            Some(Rc::new(move |item| {
                item.clone().and_then(|item| item.as_dock()).is_some()
                    && !same(&item, &Some(excluded_root.clone() as Rc<dyn IDockable>))
                    && !same(
                        &item,
                        &excluded_root
                            .pinned_dock()
                            .map(|dock| dock as Rc<dyn IDockable>),
                    )
            })),
        )
        .iter()
        .next()
        .map(|item| {
            item.and_then(|item| item.as_dock())
                .expect("source anchor implements IDock")
        })
    }

    fn find_tool_dock_by_alignment(
        &self,
        root: Rc<dyn IRootDock>,
        alignment: Alignment,
    ) -> Option<Rc<dyn IToolDock>> {
        self.find_in_dock(
            Some(root),
            Some(Rc::new(move |item| {
                item.and_then(|item| item.as_tool_dock())
                    .is_some_and(|dock| dock.alignment() == alignment)
            })),
        )
        .iter()
        .find_map(|item| item.and_then(|item| item.as_tool_dock()))
    }

    pub(super) fn get_pinned_dockable_alignment(
        dockable: Rc<dyn IDockable>,
        root: Rc<dyn IRootDock>,
    ) -> Alignment {
        for side in [
            Alignment::Left,
            Alignment::Right,
            Alignment::Top,
            Alignment::Bottom,
        ] {
            if pinned_list(root.as_ref(), side)
                .is_some_and(|items| items.contains(&Some((dockable).clone())))
            {
                return side;
            }
        }
        Alignment::Unset
    }

    fn update_pinned_bounds_from_visible(dockable: Rc<dyn IDockable>, owner: Rc<dyn IDock>) {
        let (_, _, mut width, mut height) = dockable.get_visible_bounds();
        if !is_valid_size(width) || !is_valid_size(height) {
            (_, _, width, height) = owner.get_visible_bounds();
        }
        if is_valid_size(width) && is_valid_size(height) {
            dockable.set_pinned_bounds(0.0, 0.0, width, height);
        }
    }
}
