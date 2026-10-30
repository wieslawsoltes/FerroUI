// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::AdapterOwner;
use crate::core::{IDockWindow, IHostAdapter, IHostWindow};
use std::cell::Cell;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};

/// Saves and presents a dock window and coordinates the host's close lifecycle.
/// Standalone adapters retain the window; embedded adapters use a weak backlink.
pub struct HostAdapter {
    window: AdapterOwner<dyn IDockWindow>,
    exiting: Cell<bool>,
}

impl HostAdapter {
    /// Creates a standalone adapter retaining its window, matching the source constructor.
    pub fn new(window: Rc<dyn IDockWindow>) -> Self {
        Self {
            window: AdapterOwner::Strong(window),
            exiting: Cell::new(false),
        }
    }

    /// Constructs the adapter while its enclosing window's Rc is being created.
    /// The owner must outlive operations through this weak construction path.
    pub fn from_weak(window: Weak<dyn IDockWindow>) -> Self {
        Self {
            window: AdapterOwner::Weak(window),
            exiting: Cell::new(false),
        }
    }

    fn window(&self) -> Rc<dyn IDockWindow> {
        self.window
            .get()
            .expect("the containing window must be alive during host operations")
    }

    fn current_host(window: &dyn IDockWindow) -> Rc<dyn IHostWindow> {
        window
            .host()
            .expect("the source host access requires a non-null host")
    }

    fn is_closed(host: &dyn IHostWindow) -> Option<bool> {
        host.as_host_window_close_state()
            .map(|state| state.is_closed())
    }
}

impl HostAdapter {
    /// Saves the current host state into its window.
    pub fn save(&self) {
        let window = self.window();
        if window.host().is_some() {
            window.set_window_state(Self::current_host(window.as_ref()).get_window_state());
            let (x, y) = Self::current_host(window.as_ref()).get_position();
            window.set_x(x);
            window.set_y(y);
            let (width, height) = Self::current_host(window.as_ref()).get_size();
            window.set_width(width);
            window.set_height(height);
        }
    }

    /// Creates or configures a host and presents it.
    pub fn present(&self, is_dialog: bool) {
        let window = self.window();
        if window.layout().is_none() {
            return;
        }
        if window.host().is_none() {
            let host = if let Some(factory) = window
                .factory()
                .and_then(|factory| factory.as_dock_window_host_factory())
            {
                factory.create_host_window(Some(window.clone()))
            } else {
                window
                    .factory()
                    .and_then(|factory| factory.get_host_window(window.id()))
            };
            window.set_host(host);
            if window.host().is_some() {
                Self::current_host(window.as_ref()).set_window(Some(window.clone()));
            }
        }
        if window.host().is_some() {
            Self::current_host(window.as_ref()).set_position(window.x(), window.y());
            Self::current_host(window.as_ref()).set_size(window.width(), window.height());
            Self::current_host(window.as_ref()).set_window_state(window.window_state());
            Self::current_host(window.as_ref()).set_title(Some(window.title()));
            Self::current_host(window.as_ref()).set_layout(
                window
                    .layout()
                    .expect("the source SetLayout contract requires a non-null layout"),
            );
            Self::current_host(window.as_ref()).set_is_tracked(true);
            Self::current_host(window.as_ref()).present(is_dialog);
        }
    }

    /// Saves and closes the host while preserving cancellation and failure state.
    pub fn exit(&self) {
        if self.exiting.get() {
            return;
        }
        let window = self.window();
        let Some(host) = window.host() else {
            return;
        };
        let was_tracked = host.is_tracked();
        self.exiting.set(true);
        let _reset = ExitGuard(&self.exiting);
        // Native host failures unwind. No interior borrows cross callbacks;
        // restore the same tracked flag as the source catch/finally blocks.
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.save();
            host.set_is_tracked(false);
            if Self::is_closed(host.as_ref()) != Some(true) {
                host.exit();
            }
            if Self::is_closed(host.as_ref()) == Some(false) {
                host.set_is_tracked(was_tracked);
            } else if window
                .host()
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(current, &host))
            {
                window.set_host(None);
            }
        }));
        if let Err(error) = result {
            if Self::is_closed(host.as_ref()) != Some(true) {
                host.set_is_tracked(was_tracked);
            }
            resume_unwind(error);
        }
    }

    /// Activates the current host, if one exists.
    pub fn set_active(&self) {
        let window = self.window();
        if window.host().is_some() {
            Self::current_host(window.as_ref()).set_active();
        }
    }
}

struct ExitGuard<'a>(&'a Cell<bool>);
impl Drop for ExitGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl IHostAdapter for HostAdapter {
    fn save(&self) {
        HostAdapter::save(self)
    }
    fn present(&self, is_dialog: bool) {
        HostAdapter::present(self, is_dialog)
    }
    fn exit(&self) {
        HostAdapter::exit(self)
    }
    fn set_active(&self) {
        HostAdapter::set_active(self)
    }
}
