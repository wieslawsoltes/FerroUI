// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

/// Shared arguments for `DockableDeactivatedEventArgs`. Clones refer to the same event instance.
#[derive(Clone)]
pub struct DockableDeactivatedEventArgs {
    inner: Rc<Inner>,
}

struct Inner {
    dockable: Option<Rc<dyn IDockable>>,
}

impl DockableDeactivatedEventArgs {
    /// Gets the identity of the shared source instance for native reference comparisons.
    #[doc(hidden)]
    pub fn reference_identity(&self) -> *const () {
        Rc::as_ptr(&self.inner).cast()
    }

    /// Creates event arguments with the supplied context.
    pub fn new(dockable: Option<Rc<dyn IDockable>>) -> Self {
        Self {
            inner: Rc::new(Inner { dockable }),
        }
    }
    /// Gets `Dockable`.
    pub fn dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.inner.dockable.clone()
    }
}

impl PartialEq for DockableDeactivatedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
