// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

/// Shared arguments for `WindowDeactivatedEventArgs`. Clones refer to the same event instance.
#[derive(Clone)]
pub struct WindowDeactivatedEventArgs {
    inner: Rc<Inner>,
}

struct Inner {
    window: Option<Rc<dyn IDockWindow>>,
}

impl WindowDeactivatedEventArgs {
    /// Gets the identity of the shared source instance for native reference comparisons.
    #[doc(hidden)]
    pub fn reference_identity(&self) -> *const () {
        Rc::as_ptr(&self.inner).cast()
    }

    /// Creates event arguments with the supplied context.
    pub fn new(window: Option<Rc<dyn IDockWindow>>) -> Self {
        Self {
            inner: Rc::new(Inner { window }),
        }
    }
    /// Gets `Window`.
    pub fn window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.inner.window.clone()
    }
}

impl PartialEq for WindowDeactivatedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
