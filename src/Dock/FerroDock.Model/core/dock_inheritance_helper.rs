// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::IDockable;
use std::rc::Rc;

/// Computes inherited dock properties through the ownership hierarchy.
pub struct DockInheritanceHelper;

impl DockInheritanceHelper {
    /// Returns false when this dock or any ancestor dock disables global docking.
    pub fn get_effective_enable_global_docking(dockable: Rc<dyn IDockable>) -> bool {
        let mut current = Some(dockable);
        while let Some(dockable) = current {
            if dockable
                .clone()
                .as_dock()
                .is_some_and(|dock| !dock.enable_global_docking())
            {
                return false;
            }
            current = dockable.owner();
        }
        true
    }
}
