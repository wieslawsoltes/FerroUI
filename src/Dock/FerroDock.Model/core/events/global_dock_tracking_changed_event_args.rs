// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

/// Shared arguments for `GlobalDockTrackingChangedEventArgs`. Clones refer to the same event instance.
#[derive(Clone)]
pub struct GlobalDockTrackingChangedEventArgs {
    inner: Rc<Inner>,
}

struct Inner {
    previous: GlobalDockTrackingState,
    current: GlobalDockTrackingState,
    reason: DockTrackingChangeReason,
}

impl GlobalDockTrackingChangedEventArgs {
    /// Gets the identity of the shared source instance for native reference comparisons.
    #[doc(hidden)]
    pub fn reference_identity(&self) -> *const () {
        Rc::as_ptr(&self.inner).cast()
    }

    /// Creates event arguments with the supplied context.
    pub fn new(
        previous: GlobalDockTrackingState,
        current: GlobalDockTrackingState,
        reason: DockTrackingChangeReason,
    ) -> Self {
        Self {
            inner: Rc::new(Inner {
                previous,
                current,
                reason,
            }),
        }
    }
    /// Gets `Previous`.
    pub fn previous(&self) -> GlobalDockTrackingState {
        self.inner.previous.clone()
    }
    /// Gets `Current`.
    pub fn current(&self) -> GlobalDockTrackingState {
        self.inner.current.clone()
    }
    /// Gets `Reason`.
    pub fn reason(&self) -> DockTrackingChangeReason {
        self.inner.reason
    }
}

impl PartialEq for GlobalDockTrackingChangedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
