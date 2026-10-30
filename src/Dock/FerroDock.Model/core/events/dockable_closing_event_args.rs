// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::cell::Cell;
use std::rc::Rc;

/// Shared arguments for `DockableClosingEventArgs`. Clones refer to the same event instance.
#[derive(Clone)]
pub struct DockableClosingEventArgs {
    inner: Rc<Inner>,
}

struct Inner {
    dockable: Option<Rc<dyn IDockable>>,
    cancel: Cell<bool>,
}

impl DockableClosingEventArgs {
    /// Gets the identity of the shared source instance for native reference comparisons.
    #[doc(hidden)]
    pub fn reference_identity(&self) -> *const () {
        Rc::as_ptr(&self.inner).cast()
    }

    /// Creates event arguments with the supplied context.
    pub fn new(dockable: Option<Rc<dyn IDockable>>) -> Self {
        Self {
            inner: Rc::new(Inner {
                dockable,
                cancel: Cell::new(false),
            }),
        }
    }
    /// Gets `Dockable`.
    pub fn dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.inner.dockable.clone()
    }
    /// Gets `Cancel`.
    pub fn cancel(&self) -> bool {
        self.inner.cancel.get()
    }
    /// Sets `Cancel` on the shared arguments.
    pub fn set_cancel(&self, value: bool) {
        self.inner.cancel.set(value);
    }
}

impl PartialEq for DockableClosingEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
