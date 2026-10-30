// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::DockWorkspace;
use std::rc::Rc;

/// A shared snapshot of the workspace and dirty flag at notification time.
#[derive(Clone)]
pub struct DockWorkspaceDirtyChangedEventArgs {
    inner: Rc<Inner>,
}
struct Inner {
    workspace: Option<Rc<DockWorkspace>>,
    is_dirty: bool,
}
impl DockWorkspaceDirtyChangedEventArgs {
    /// Creates immutable event arguments with the supplied state.
    pub fn new(workspace: Option<Rc<DockWorkspace>>, is_dirty: bool) -> Self {
        Self {
            inner: Rc::new(Inner {
                workspace,
                is_dirty,
            }),
        }
    }
    /// Gets the workspace associated with this transition, if one was active.
    pub fn workspace(&self) -> Option<Rc<DockWorkspace>> {
        self.inner.workspace.clone()
    }
    /// Gets the dirty state at the time of the notification.
    pub fn is_dirty(&self) -> bool {
        self.inner.is_dirty
    }
}
impl PartialEq for DockWorkspaceDirtyChangedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
