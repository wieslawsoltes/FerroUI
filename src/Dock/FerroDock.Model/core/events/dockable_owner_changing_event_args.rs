// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

/// Shared arguments for `DockableOwnerChangingEventArgs`. Clones refer to the same event instance.
#[derive(Clone)]
pub struct DockableOwnerChangingEventArgs {
    inner: Rc<Inner>,
}

struct Inner {
    old_owner: Option<Rc<dyn IDockable>>,
    new_owner: Option<Rc<dyn IDockable>>,
}

impl DockableOwnerChangingEventArgs {
    /// Gets the identity of the shared source instance for native reference comparisons.
    #[doc(hidden)]
    pub fn reference_identity(&self) -> *const () {
        Rc::as_ptr(&self.inner).cast()
    }

    /// Creates event arguments with the supplied context.
    pub fn new(old_owner: Option<Rc<dyn IDockable>>, new_owner: Option<Rc<dyn IDockable>>) -> Self {
        Self {
            inner: Rc::new(Inner {
                old_owner,
                new_owner,
            }),
        }
    }
    /// Gets `OldOwner`.
    pub fn old_owner(&self) -> Option<Rc<dyn IDockable>> {
        self.inner.old_owner.clone()
    }
    /// Gets `NewOwner`.
    pub fn new_owner(&self) -> Option<Rc<dyn IDockable>> {
        self.inner.new_owner.clone()
    }
}

impl PartialEq for DockableOwnerChangingEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
