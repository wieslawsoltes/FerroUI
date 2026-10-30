// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::{IDock, IDockable};
use std::rc::Rc;

/// Shared docking group validation used by the docking service and manager.
pub struct DockGroupValidator;

impl DockGroupValidator {
    /// Validates two dockables using inherited, case-sensitive group identifiers.
    /// Ungrouped dockables are compatible with each other, and grouped dockables
    /// are compatible only when their effective groups match.
    pub fn validate_docking_groups(
        source_dockable: Rc<dyn IDockable>,
        target_dockable: Rc<dyn IDockable>,
    ) -> bool {
        let source_group = Self::get_effective_dock_group(source_dockable);
        let target_group = Self::get_effective_dock_group(target_dockable);
        Self::groups_match(source_group, target_group)
    }
    fn groups_match(source_group: Option<String>, target_group: Option<String>) -> bool {
        let source_has_group = source_group.as_ref().is_some_and(|group| !group.is_empty());
        let target_has_group = target_group.as_ref().is_some_and(|group| !group.is_empty());
        if !source_has_group && !target_has_group {
            return true;
        }
        if !source_has_group && target_has_group {
            return false;
        }
        if source_has_group && !target_has_group {
            return false;
        }
        source_group == target_group
    }

    /// Validates against every visible item in a target dock. Empty or absent
    /// visible collections accept the operation regardless of the dock's own group.
    pub fn validate_docking_groups_in_dock(
        source_dockable: Rc<dyn IDockable>,
        target_dock: Rc<dyn IDock>,
    ) -> bool {
        if target_dock
            .visible_dockables()
            .is_some_and(|items| items.is_empty())
        {
            return true;
        }
        if target_dock.visible_dockables().is_some() {
            let items = target_dock
                .visible_dockables()
                .expect("the visible collection remains present during enumeration");
            for dockable in items.iter() {
                let source_group = Self::get_effective_dock_group(source_dockable.clone());
                let target_group = Self::get_effective_nullable_dock_group(dockable);
                if !Self::groups_match(source_group, target_group) {
                    return false;
                }
            }
        }
        true
    }

    /// Validates a global target. An ungrouped source may enter any target;
    /// a grouped source requires the same effective target group.
    pub fn validate_global_docking(
        source_dockable: Rc<dyn IDockable>,
        target_dock: Rc<dyn IDock>,
    ) -> bool {
        let source_group = Self::get_effective_dock_group(source_dockable);
        let target_group = Self::get_effective_dock_group(target_dock);
        Self::global_groups_match(source_group, target_group)
    }
    fn global_groups_match(source_group: Option<String>, target_group: Option<String>) -> bool {
        let source_has_group = source_group.as_ref().is_some_and(|group| !group.is_empty());
        let target_has_group = target_group.as_ref().is_some_and(|group| !group.is_empty());
        if !source_has_group && !target_has_group {
            return true;
        }
        if !source_has_group && target_has_group {
            return true;
        }
        if source_has_group && !target_has_group {
            return false;
        }
        source_group == target_group
    }

    /// Walks the ownership hierarchy to find the nearest nonempty group.
    /// Whitespace identifiers are retained, matching the source's empty check.
    pub fn get_effective_dock_group(dockable: Rc<dyn IDockable>) -> Option<String> {
        Self::get_effective_nullable_dock_group(Some(dockable))
    }
    fn get_effective_nullable_dock_group(mut current: Option<Rc<dyn IDockable>>) -> Option<String> {
        while let Some(dockable) = current {
            if dockable.dock_group().is_some_and(|group| !group.is_empty()) {
                return dockable.dock_group();
            }
            current = dockable.owner();
        }
        None
    }
}
