// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use crate::BoxedValue;
use std::cell::RefCell;
use std::rc::Rc;

/// Shared arguments for `DockableInitEventArgs`. Clones refer to the same event instance.
#[derive(Clone)]
pub struct DockableInitEventArgs {
    inner: Rc<Inner>,
}

struct Inner {
    dockable: Option<Rc<dyn IDockable>>,
    context: RefCell<Option<BoxedValue>>,
}

impl DockableInitEventArgs {
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
                context: RefCell::new(None),
            }),
        }
    }
    /// Gets `Dockable`.
    pub fn dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.inner.dockable.clone()
    }
    /// Gets `Context`.
    pub fn context(&self) -> Option<BoxedValue> {
        self.inner.context.borrow().clone()
    }
    /// Sets `Context` on the shared arguments.
    pub fn set_context(&self, value: Option<BoxedValue>) {
        let previous = self.inner.context.replace(value);
        drop(previous);
    }
}

impl PartialEq for DockableInitEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
