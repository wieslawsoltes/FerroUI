// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::{DockWindowOwnerMode, IDockWindow};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Options configuring a dock window's relationships and presentation.
#[derive(Default)]
pub struct DockWindowOptions {
    owner_mode: Cell<DockWindowOwnerMode>,
    parent_window: RefCell<Option<Rc<dyn IDockWindow>>>,
    is_modal: Cell<bool>,
    show_in_taskbar: Cell<Option<bool>>,
}

impl DockWindowOptions {
    /// Creates options with default ownership, no parent, and no taskbar override.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
    /// Gets the owner resolution mode.
    pub fn owner_mode(&self) -> DockWindowOwnerMode {
        self.owner_mode.get()
    }
    /// Sets the owner resolution mode.
    pub fn set_owner_mode(&self, value: DockWindowOwnerMode) {
        self.owner_mode.set(value);
    }
    /// Gets the parent window retained by these options.
    pub fn parent_window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.parent_window.borrow().clone()
    }
    /// Sets the parent window to use when these options are applied.
    pub fn set_parent_window(&self, value: Option<Rc<dyn IDockWindow>>) {
        let previous = self.parent_window.replace(value);
        drop(previous);
    }
    /// Returns whether to present the window modally.
    pub fn is_modal(&self) -> bool {
        self.is_modal.get()
    }
    /// Sets whether to present the window modally.
    pub fn set_is_modal(&self, value: bool) {
        self.is_modal.set(value);
    }
    /// Gets the optional taskbar visibility override.
    pub fn show_in_taskbar(&self) -> Option<bool> {
        self.show_in_taskbar.get()
    }
    /// Sets the optional taskbar visibility override.
    pub fn set_show_in_taskbar(&self, value: Option<bool>) {
        self.show_in_taskbar.set(value);
    }
    /// Applies ownership and modality, then applies taskbar visibility if configured.
    ///
    /// # Panics
    /// Rejects a null window before any setter, matching the source argument check.
    pub fn apply_to(&self, window: Option<&dyn IDockWindow>) {
        let window = window.expect("Value cannot be null. (Parameter 'window')");
        window.set_owner_mode(self.owner_mode());
        window.set_parent_window(self.parent_window());
        window.set_is_modal(self.is_modal());
        if let Some(value) = self.show_in_taskbar() {
            window.set_show_in_taskbar(Some(value));
        }
    }
}

impl PartialEq for DockWindowOptions {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
