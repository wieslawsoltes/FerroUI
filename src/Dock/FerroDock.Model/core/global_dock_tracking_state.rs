// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::{IDockWindow, IDockable, IHostWindow};
use crate::controls::IRootDock;
use std::rc::{Rc, Weak};

/// A weak snapshot of global dock/window focus context.
#[derive(Clone)]
pub struct GlobalDockTrackingState {
    inner: Rc<Inner>,
}

struct Inner {
    dockable: Option<Weak<dyn IDockable>>,
    root_dock: Option<Weak<dyn IRootDock>>,
    window: Option<Weak<dyn IDockWindow>>,
}

impl GlobalDockTrackingState {
    /// Gets the identity of the shared source instance for native reference comparisons.
    #[doc(hidden)]
    pub fn reference_identity(&self) -> *const () {
        Rc::as_ptr(&self.inner).cast()
    }

    /// Creates a tracking context that does not retain any of its targets.
    pub fn new(
        dockable: Option<Rc<dyn IDockable>>,
        root_dock: Option<Rc<dyn IRootDock>>,
        window: Option<Rc<dyn IDockWindow>>,
    ) -> Self {
        Self {
            inner: Rc::new(Inner {
                dockable: dockable.as_ref().map(Rc::downgrade),
                root_dock: root_dock.as_ref().map(Rc::downgrade),
                window: window.as_ref().map(Rc::downgrade),
            }),
        }
    }

    /// Gets the shared empty tracking state of the calling UI thread.
    pub fn empty() -> Self {
        ferroui_system::ferro_thread_local! {
            static EMPTY: GlobalDockTrackingState = GlobalDockTrackingState::new(None, None, None);
        }
        EMPTY.with(Clone::clone)
    }

    /// Resolves the tracked dockable if it is still alive.
    pub fn dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.inner.dockable.as_ref().and_then(Weak::upgrade)
    }
    /// Resolves the tracked root dock if it is still alive.
    pub fn root_dock(&self) -> Option<Rc<dyn IRootDock>> {
        self.inner.root_dock.as_ref().and_then(Weak::upgrade)
    }
    /// Resolves the tracked window if it is still alive.
    pub fn window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.inner.window.as_ref().and_then(Weak::upgrade)
    }
    /// Gets the tracked window's current host, if the window is still alive.
    pub fn host_window(&self) -> Option<Rc<dyn IHostWindow>> {
        self.window().and_then(|window| window.host())
    }
}

impl PartialEq for GlobalDockTrackingState {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
