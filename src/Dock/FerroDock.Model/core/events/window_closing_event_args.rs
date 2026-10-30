// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::cell::Cell;
use std::rc::Rc;

/// Shared arguments for `WindowClosingEventArgs`. Clones refer to the same event instance.
#[derive(Clone)]
pub struct WindowClosingEventArgs {
    inner: Rc<Inner>,
}

struct Inner {
    window: Option<Rc<dyn IDockWindow>>,
    cancel: Cell<bool>,
}

impl WindowClosingEventArgs {
    /// Gets the identity of the shared source instance for native reference comparisons.
    #[doc(hidden)]
    pub fn reference_identity(&self) -> *const () {
        Rc::as_ptr(&self.inner).cast()
    }

    /// Creates event arguments with the supplied context.
    pub fn new(window: Option<Rc<dyn IDockWindow>>) -> Self {
        Self {
            inner: Rc::new(Inner {
                window,
                cancel: Cell::new(false),
            }),
        }
    }
    /// Gets `Window`.
    pub fn window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.inner.window.clone()
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

impl PartialEq for WindowClosingEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
