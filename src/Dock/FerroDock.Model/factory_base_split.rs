// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::collection::index_i32;
use super::FactoryBase;
use crate::controls::*;
use crate::core::DockGroupValidator;
use crate::core::*;
use std::rc::Rc;

fn assigned(value: f64) -> bool {
    !value.is_nan() && value > 0.0
}

impl FactoryBase {
    pub(super) fn copy_dock_group(source: Rc<dyn IDockable>, target: Rc<dyn IDockable>) {
        if let Some(group) =
            DockGroupValidator::get_effective_dock_group(source).filter(|group| !group.is_empty())
        {
            target.set_dock_group(Some(group));
        }
    }

    fn resolve_effective_proportion(owner: Rc<dyn IProportionalDock>, dock: Rc<dyn IDock>) -> f64 {
        if assigned(dock.proportion()) {
            return dock.proportion();
        }
        let Some(visible) = owner.visible_dockables().filter(|items| items.count() > 0) else {
            return f64::NAN;
        };
        let mut total = 0.0;
        let mut unassigned = 0;
        let mut non_splitters = 0;
        for item in visible.iter() {
            if item
                .clone()
                .and_then(|item| item.as_proportional_dock_splitter())
                .is_some()
            {
                continue;
            }
            non_splitters += 1;
            if assigned(
                item.as_ref()
                    .expect("source item.Proportion dereference")
                    .proportion(),
            ) {
                total += item
                    .as_ref()
                    .expect("source item.Proportion dereference")
                    .proportion();
            } else {
                unassigned += 1;
            }
        }
        if unassigned <= 0 {
            return f64::NAN;
        }
        let remaining = 1.0 - total;
        if remaining > 0.0 {
            remaining / f64::from(unassigned)
        } else if non_splitters > 0 {
            1.0 / f64::from(non_splitters)
        } else {
            f64::NAN
        }
    }

    /// Builds the source proportional split wrapper, retaining container share and balancing reused children.
    pub fn create_split_layout(
        &self,
        dock: Rc<dyn IDock>,
        dockable: Rc<dyn IDockable>,
        operation: DockOperation,
    ) -> Rc<dyn IDock> {
        let split: Rc<dyn IDock> = if let Some(split) = dockable.clone().as_dock() {
            split.set_proportion(f64::NAN);
            split.set_collapsed_proportion(f64::NAN);
            split
        } else {
            let split = self.runtime().create_proportional_dock();
            split.set_title("IProportionalDock".into());
            split.set_visible_dockables(Some((self.create_dockable_list)(Vec::new())));
            Self::copy_dock_group(dockable.clone(), split.clone());
            if split.visible_dockables().is_some() {
                self.add_visible_dockable(Some(split.clone()), Some(dockable.clone()));
                self.runtime().on_dockable_added(Some(dockable.clone()));
                split.set_active_dockable(Some(dockable));
            }
            split
        };
        let proportion = dock.proportion();
        dock.set_proportion(f64::NAN);
        dock.set_collapsed_proportion(f64::NAN);
        let layout = self.runtime().create_proportional_dock();
        layout.set_title("IProportionalDock".into());
        layout.set_visible_dockables(Some((self.create_dockable_list)(Vec::new())));
        layout.set_proportion(proportion);
        layout.set_collapsed_proportion(proportion);
        Self::copy_dock_group(dock.clone(), layout.clone());
        let splitter = self.runtime().create_proportional_dock_splitter();
        splitter.set_title("IProportionalDockSplitter".into());
        match operation {
            DockOperation::Left | DockOperation::Right => {
                layout.set_orientation(Orientation::Horizontal)
            }
            DockOperation::Top | DockOperation::Bottom => {
                layout.set_orientation(Orientation::Vertical)
            }
            _ => {}
        }
        let before = matches!(operation, DockOperation::Left | DockOperation::Top);
        let after = matches!(operation, DockOperation::Right | DockOperation::Bottom);
        if (before || after) && layout.visible_dockables().is_some() {
            let first: Rc<dyn IDockable> = if before { split.clone() } else { dock.clone() };
            self.add_visible_dockable(Some(layout.clone()), Some(first.clone()));
            self.runtime().on_dockable_added(Some(first.clone()));
            layout.set_active_dockable(Some(first));
        }
        self.add_visible_dockable(Some(layout.clone()), Some(splitter.clone()));
        self.runtime().on_dockable_added(Some(splitter));
        if (before || after) && layout.visible_dockables().is_some() {
            let last: Rc<dyn IDockable> = if before { dock } else { split };
            self.add_visible_dockable(Some(layout.clone()), Some(last.clone()));
            self.runtime().on_dockable_added(Some(last.clone()));
            layout.set_active_dockable(Some(last));
        }
        layout
    }

    /// Splits directly into a compatible proportional owner or replaces the dock with a split wrapper.
    pub fn split_to_dock(
        &self,
        dock: Rc<dyn IDock>,
        dockable: Rc<dyn IDockable>,
        operation: DockOperation,
    ) {
        let before = matches!(operation, DockOperation::Left | DockOperation::Top);
        let orientation = match operation {
            DockOperation::Left | DockOperation::Right => Orientation::Horizontal,
            DockOperation::Top | DockOperation::Bottom => Orientation::Vertical,
            _ => panic!("Not supported split operation: {operation:?}."),
        };
        let Some(owner) = dock.owner().and_then(|owner| owner.as_dock()) else {
            return;
        };
        if owner.visible_dockables().is_none() {
            return;
        }
        let index = index_i32(
            &owner.visible_dockables().expect("source owner list exists"),
            &Some(dock.clone() as Rc<dyn IDockable>),
        );
        if index < 0 {
            return;
        }
        if let Some(proportional) = (owner.clone() as Rc<dyn IDockable>).as_proportional_dock() {
            if proportional.orientation() == orientation {
                let split: Rc<dyn IDock> = if let Some(split) = dockable.clone().as_dock() {
                    split
                } else {
                    let split = self.runtime().create_proportional_dock();
                    split.set_title("IProportionalDock".into());
                    split.set_visible_dockables(Some((self.create_dockable_list)(Vec::new())));
                    if split.visible_dockables().is_some() {
                        self.add_visible_dockable(Some(split.clone()), Some(dockable.clone()));
                        self.runtime().on_dockable_added(Some(dockable.clone()));
                        split.set_active_dockable(Some(dockable.clone()));
                    }
                    split
                };
                let splitter = self.runtime().create_proportional_dock_splitter();
                splitter.set_title("IProportionalDockSplitter".into());
                let original =
                    Self::resolve_effective_proportion(proportional.clone(), dock.clone());
                let half = if original.is_nan() {
                    f64::NAN
                } else {
                    original / 2.0
                };
                dock.set_proportion(half);
                dock.set_collapsed_proportion(half);
                split.set_proportion(half);
                split.set_collapsed_proportion(half);
                if before {
                    self.insert_visible_dockable(
                        Some(proportional.clone()),
                        index,
                        Some(split.clone()),
                    );
                    self.runtime().on_dockable_added(Some(split.clone()));
                    self.insert_visible_dockable(
                        Some(proportional.clone()),
                        index + 1,
                        Some(splitter.clone()),
                    );
                    self.runtime().on_dockable_added(Some(splitter));
                } else {
                    self.insert_visible_dockable(
                        Some(proportional.clone()),
                        index + 1,
                        Some(splitter.clone()),
                    );
                    self.runtime().on_dockable_added(Some(splitter));
                    self.insert_visible_dockable(
                        Some(proportional.clone()),
                        index + 2,
                        Some(split.clone()),
                    );
                    self.runtime().on_dockable_added(Some(split.clone()));
                }
                self.runtime()
                    .init_dockable(Some(split), Some(proportional.clone()));
                self.cleanup_orphaned_splitters(Some(proportional));
                self.runtime()
                    .on_dockable_undocked(Some(dockable.clone()), operation);
                self.runtime().on_dockable_docked(Some(dockable), operation);
                return;
            }
        }
        let layout = self
            .runtime()
            .create_split_layout(dock.clone(), dockable.clone(), operation);
        let was_default = owner
            .default_dockable()
            .as_ref()
            .is_some_and(|item| Rc::ptr_eq(item, &(dock as Rc<dyn IDockable>)));
        self.remove_visible_dockable_at(Some(owner.clone()), index);
        self.runtime().on_dockable_removed(Some(dockable.clone()));
        self.runtime()
            .on_dockable_undocked(Some(dockable.clone()), operation);
        self.insert_visible_dockable(Some(owner.clone()), index, Some(layout.clone()));
        self.runtime().on_dockable_added(Some(dockable.clone()));
        owner.set_active_dockable(Some(layout.clone()));
        if was_default {
            owner.set_default_dockable(Some(layout.clone()));
        }
        self.runtime().init_dockable(Some(layout), Some(owner));
        self.runtime().on_dockable_docked(Some(dockable), operation);
    }
}
