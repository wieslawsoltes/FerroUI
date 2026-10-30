// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::{DockableBase, DockableBaseImpl};
use crate::impl_dockable;
use ferrodock_model::controls::*;
use ferroui_reactiveui::{reactive_object, IReactiveObjectExt};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A reactive proportional dock splitter model.
pub struct ProportionalDockSplitter {
    base: DockableBase,
    can_resize: RefCell<bool>,
    resize_preview: RefCell<bool>,
}
reactive_object!(ProportionalDockSplitter, base);
impl DockableBaseImpl for ProportionalDockSplitter {}
impl_dockable!(
    ProportionalDockSplitter,
    fn as_splitter(self: Rc<Self>) -> Option<Rc<dyn ISplitter>> {
        Some(self)
    },
    fn as_proportional_dock_splitter(self: Rc<Self>) -> Option<Rc<dyn IProportionalDockSplitter>> {
        Some(self)
    }
);
impl ISplitter for ProportionalDockSplitter {}
impl ProportionalDockSplitter {
    /// Creates a shared splitter with the source defaults.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak: &Weak<Self>| Self::construct(weak.clone()))
    }
    /// Constructs this component with the enclosing model's reactive identity.
    pub fn construct<T: DockableBaseImpl + IProportionalDockSplitter + 'static>(
        runtime: Weak<T>,
    ) -> Self {
        let runtime: Weak<dyn DockableBaseImpl> = runtime;
        Self {
            base: DockableBase::construct(runtime),
            can_resize: RefCell::new(true),
            resize_preview: RefCell::new(false),
        }
    }
    /// Gets `CanResize`.
    pub fn can_resize(&self) -> bool {
        *self.can_resize.borrow()
    }
    /// Sets `CanResize`, notifying only when changed.
    pub fn set_can_resize(&self, value: bool) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.can_resize, value, "CanResize");
    }
    /// Gets `ResizePreview`.
    pub fn resize_preview(&self) -> bool {
        *self.resize_preview.borrow()
    }
    /// Sets `ResizePreview`, notifying only when changed.
    pub fn set_resize_preview(&self, value: bool) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.resize_preview, value, "ResizePreview");
    }
}
impl IProportionalDockSplitter for ProportionalDockSplitter {
    fn can_resize(&self) -> bool {
        self.can_resize()
    }
    fn set_can_resize(&self, value: bool) {
        self.set_can_resize(value);
    }
    fn resize_preview(&self) -> bool {
        self.resize_preview()
    }
    fn set_resize_preview(&self, value: bool) {
        self.set_resize_preview(value);
    }
}
