// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::{DockOperation, IDockable};
use std::rc::Rc;
/// Original pinned-preview drag surface resolution and no-op policy.
pub(crate) struct DragDockableResolver;
impl DragDockableResolver {
    pub(crate) fn resolve(dockable: Rc<dyn IDockable>) -> Rc<dyn IDockable> {
        if let Some(preview) = dockable.clone().as_tool_dock() {
            if let Some(root) = preview.owner().and_then(|owner| owner.as_root_dock()) {
                if let Some(active) = preview.active_dockable() {
                    if root
                        .pinned_dock()
                        .is_some_and(|pinned| Rc::ptr_eq(&pinned, &preview))
                    {
                        return active;
                    }
                }
            }
        }
        dockable
    }
    pub(crate) fn is_self_drop(source: Rc<dyn IDockable>, target: Rc<dyn IDockable>) -> bool {
        let resolved = Self::resolve(source.clone());
        Rc::ptr_eq(&resolved, &target)
            || (!Rc::ptr_eq(&resolved, &source)
                && resolved
                    .owner()
                    .is_some_and(|owner| Rc::ptr_eq(&owner, &target)))
    }
    pub(crate) fn is_no_op_drop(
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDockable>,
        operation: DockOperation,
    ) -> bool {
        if !Self::is_self_drop(source.clone(), target) {
            return false;
        }
        operation == DockOperation::Fill || !Rc::ptr_eq(&Self::resolve(source.clone()), &source)
    }
}
