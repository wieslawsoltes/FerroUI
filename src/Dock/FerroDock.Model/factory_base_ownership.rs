// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::FactoryBase;
use crate::controls::*;
use crate::core::*;
use std::rc::Rc;

impl FactoryBase {
    /// Unpins an item by invoking the virtual pin toggle only when currently pinned.
    pub fn unpin_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        if self.is_dockable_pinned(dockable.clone(), None) {
            self.runtime().pin_dockable(dockable);
        }
    }

    /// Tests membership in each of the root's four auto-hide strips in source order.
    pub fn is_dockable_pinned(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        root: Option<Rc<dyn IRootDock>>,
    ) -> bool {
        let Some(root) = root.or_else(|| self.runtime().find_root(dockable.clone(), None)) else {
            return false;
        };
        // Read each property only when the preceding strip did not contain the item.
        for get in [
            IRootDock::left_pinned_dockables,
            IRootDock::right_pinned_dockables,
            IRootDock::top_pinned_dockables,
            IRootDock::bottom_pinned_dockables,
        ] {
            if get(root.as_ref()).is_some()
                && get(root.as_ref())
                    .expect("source pinned list exists")
                    .contains(&dockable)
            {
                return true;
            }
        }
        false
    }

    pub(super) fn is_owner_attached(&self, root: Rc<dyn IRootDock>, owner: Rc<dyn IDock>) -> bool {
        if Rc::ptr_eq(&(root.clone() as Rc<dyn IDock>), &owner) {
            return true;
        }
        let matches = |owner: Rc<dyn IDock>| {
            Rc::new(move |item: Option<Rc<dyn IDockable>>| {
                item.is_some_and(|item| Rc::ptr_eq(&item, &(owner.clone() as Rc<dyn IDockable>)))
            }) as DockablePredicate
        };
        if self
            .find_in_dock(Some(root.clone()), Some(matches(owner.clone())))
            .iter()
            .next()
            .is_some()
        {
            return true;
        }
        if root.hidden_dockables().is_some() {
            if root
                .hidden_dockables()
                .expect("source hidden list exists")
                .contains(&Some((owner.clone() as Rc<dyn IDockable>).clone()))
            {
                return true;
            }
            for hidden in root
                .hidden_dockables()
                .expect("source hidden list exists")
                .iter()
            {
                if let Some(hidden) = hidden.and_then(|item| item.as_dock()) {
                    if self
                        .find_in_dock(Some(hidden), Some(matches(owner.clone())))
                        .iter()
                        .next()
                        .is_some()
                    {
                        return true;
                    }
                }
            }
        }
        if root.windows().is_none() {
            return false;
        }
        for window in root.windows().expect("source window list exists").iter() {
            let window = window.expect("source window.Layout access");
            let Some(layout) = window.layout() else {
                continue;
            };
            if Rc::ptr_eq(
                &(layout.clone() as Rc<dyn IDockable>),
                &(owner.clone() as Rc<dyn IDockable>),
            ) {
                return true;
            }
            if let Some(dock) = layout.as_dock() {
                if self
                    .find_in_dock(Some(dock), Some(matches(owner.clone())))
                    .iter()
                    .next()
                    .is_some()
                {
                    return true;
                }
            }
        }
        false
    }
}
