// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::controls::*;
use crate::core::*;
use std::rc::Rc;

/// Shared arguments for `FocusedDockableChangedEventArgs`. Clones refer to the same event instance.
#[derive(Clone)]
pub struct FocusedDockableChangedEventArgs {
    inner: Rc<Inner>,
}

struct Inner {
    dockable: Option<Rc<dyn IDockable>>,
    root_dock: Option<Rc<dyn IRootDock>>,
    window: Option<Rc<dyn IDockWindow>>,
}

impl FocusedDockableChangedEventArgs {
    /// Gets the identity of the shared source instance for native reference comparisons.
    #[doc(hidden)]
    pub fn reference_identity(&self) -> *const () {
        Rc::as_ptr(&self.inner).cast()
    }

    /// Creates arguments without an explicitly resolved root/window.
    pub fn new(dockable: Option<Rc<dyn IDockable>>) -> Self {
        Self::with_context(dockable, None, None)
    }
    /// Creates event arguments with the supplied context.
    pub fn with_context(
        dockable: Option<Rc<dyn IDockable>>,
        root_dock: Option<Rc<dyn IRootDock>>,
        window: Option<Rc<dyn IDockWindow>>,
    ) -> Self {
        Self {
            inner: Rc::new(Inner {
                dockable,
                root_dock,
                window,
            }),
        }
    }
    /// Gets `Dockable`.
    pub fn dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.inner.dockable.clone()
    }
    /// Gets `RootDock`.
    pub fn root_dock(&self) -> Option<Rc<dyn IRootDock>> {
        self.inner.root_dock.clone()
    }
    /// Gets `Window`.
    pub fn window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.inner.window.clone()
    }
    /// Gets the window's current host, rather than a captured host snapshot.
    pub fn host_window(&self) -> Option<Rc<dyn IHostWindow>> {
        self.window().and_then(|window| window.host())
    }
}

impl PartialEq for FocusedDockableChangedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
