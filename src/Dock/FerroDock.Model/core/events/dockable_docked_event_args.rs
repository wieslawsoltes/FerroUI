// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

/// Shared arguments for `DockableDockedEventArgs`. Clones refer to the same event instance.
#[derive(Clone)]
pub struct DockableDockedEventArgs {
    inner: Rc<Inner>,
}

struct Inner {
    dockable: Option<Rc<dyn IDockable>>,
    operation: DockOperation,
}

impl DockableDockedEventArgs {
    /// Gets the identity of the shared source instance for native reference comparisons.
    #[doc(hidden)]
    pub fn reference_identity(&self) -> *const () {
        Rc::as_ptr(&self.inner).cast()
    }

    /// Creates event arguments with the supplied context.
    pub fn new(dockable: Option<Rc<dyn IDockable>>, operation: DockOperation) -> Self {
        Self {
            inner: Rc::new(Inner {
                dockable,
                operation,
            }),
        }
    }
    /// Gets `Dockable`.
    pub fn dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.inner.dockable.clone()
    }
    /// Gets `Operation`.
    pub fn operation(&self) -> DockOperation {
        self.inner.operation
    }
}

impl PartialEq for DockableDockedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
