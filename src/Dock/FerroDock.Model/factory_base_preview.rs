// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::tracking::same;
use super::FactoryBase;
use crate::controls::*;
use crate::core::*;
use std::rc::Rc;

impl FactoryBase {
    /// Hides previews except items explicitly marked to remain visible.
    pub fn hide_previewing_dockables(&self, root: Option<Rc<dyn IRootDock>>) {
        self.hide_previewing_dockables_internal(root, true);
    }

    pub(super) fn hide_previewing_dockables_internal(
        &self,
        root: Option<Rc<dyn IRootDock>>,
        respect_keep_visible: bool,
    ) {
        let root = root.expect("source hide root.PinnedDock access");
        let Some(preview) = root.pinned_dock() else {
            return;
        };
        let Some(visible) = preview.visible_dockables() else {
            return;
        };
        for dockable in visible.to_vec() {
            if respect_keep_visible
                && dockable
                    .as_ref()
                    .expect("source hide dockable.KeepPinnedDockableVisible access")
                    .keep_pinned_dockable_visible()
            {
                continue;
            }
            let dockable = dockable.expect("source hide dockable.OriginalOwner access");
            let restore = dockable
                .original_owner()
                .and_then(|owner| owner.as_dock())
                .filter(|owner| self.is_owner_attached(root.clone(), owner.clone()))
                .unwrap_or_else(|| root.clone());
            if self.is_dockable_pinned(Some(dockable.clone()), Some(root.clone()))
                && !same(
                    &self.runtime().find_root(Some(restore.clone()), None),
                    &Some(root.clone()),
                )
            {
                dockable.set_owner(Some(root.clone()));
            } else {
                dockable.set_owner(Some(restore));
                dockable.set_original_owner(None);
            }
            self.remove_visible_dockable(
                root.pinned_dock().map(|dock| dock as Rc<dyn IDock>),
                Some(dockable),
            );
        }
        if visible.count() == 0 {
            preview.set_active_dockable(None);
            root.set_pinned_dock(None);
        } else if preview.active_dockable().is_none()
            || preview
                .active_dockable()
                .is_some_and(|item| item.as_splitter().is_some())
            || !visible.contains(&Some(
                (preview
                    .active_dockable()
                    .expect("source active preview exists"))
                .clone(),
            ))
        {
            preview.set_active_dockable(
                visible
                    .iter()
                    .find(|item| item.clone().and_then(|item| item.as_splitter()).is_none())
                    .flatten(),
            );
        }
    }

    /// Opens a single pinned preview, deriving alignment from the strip before owner fallback.
    pub fn preview_pinned_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        let Some(root) = self
            .runtime()
            .find_root(dockable.clone(), Some(Rc::new(|_| true)))
        else {
            return;
        };
        self.hide_previewing_dockables_internal(Some(root.clone()), false);
        let dockable = dockable.expect("source preview dockable.Owner access");
        let owner = dockable.owner();
        let mut alignment = Self::get_pinned_dockable_alignment(dockable.clone(), root.clone());
        if alignment == Alignment::Unset {
            alignment = owner
                .clone()
                .and_then(|owner| owner.as_tool_dock())
                .map_or(Alignment::Unset, |owner| owner.alignment());
        }
        if root.pinned_dock().is_none() {
            root.set_pinned_dock(Some(self.runtime().create_tool_dock()));
        }
        root.pinned_dock()
            .expect("source preview exists")
            .set_alignment(alignment);
        root.pinned_dock()
            .expect("source preview exists")
            .set_can_drop(false);
        self.remove_all_visible_dockables(root.pinned_dock().map(|dock| dock as Rc<dyn IDock>));
        if !root
            .pinned_dock()
            .expect("source preview exists")
            .visible_dockables()
            .is_some_and(|items| items.contains(&Some((dockable).clone())))
        {
            if dockable.original_owner().is_none() {
                if let Some(owner) = owner.and_then(|owner| owner.as_dock()) {
                    dockable.set_original_owner(Some(owner));
                }
            }
            self.add_visible_dockable(
                root.pinned_dock().map(|dock| dock as Rc<dyn IDock>),
                Some(dockable.clone()),
            );
        }
        self.runtime().init_dockable(
            root.pinned_dock().map(|dock| dock as Rc<dyn IDockable>),
            Some(root.clone()),
        );
        root.pinned_dock()
            .expect("source preview.ActiveDockable access")
            .set_active_dockable(Some(dockable));
    }

    /// Closes the current item's preview or opens it, ignoring keep-visible during explicit toggling.
    pub fn toggle_preview_pinned_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        let Some(root) = self
            .runtime()
            .find_root(dockable.clone(), Some(Rc::new(|_| true)))
        else {
            return;
        };
        if root
            .pinned_dock()
            .and_then(|dock| dock.visible_dockables())
            .is_some_and(|items| items.contains(&dockable))
        {
            self.hide_previewing_dockables_internal(Some(root), false);
            return;
        }
        self.preview_pinned_dockable(dockable);
    }
}
