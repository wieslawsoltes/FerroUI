// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::{DockBase, DockableBaseImpl};
use crate::impl_dock;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::BoxedValue;
use ferroui_reactiveui::{reactive_object, IReactiveObjectExt};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The reactive `DockDock` container with the source properties and defaults.
pub struct DockDock {
    base: DockBase,
    last_child_fill: RefCell<bool>,
}

reactive_object!(DockDock, base);
impl DockableBaseImpl for DockDock {}
impl_dock!(
    DockDock,
    fn as_dock_dock(self: Rc<Self>) -> Option<Rc<dyn IDockDock>> {
        Some(self)
    }
);
impl DockDock {
    /// Creates a shared container and attaches its source reactive observations.
    pub fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(Self::construct);
        model.base.initialize(&model);
        model
    }
    /// Constructs the component inside a model with the complete `IDockDock` contract.
    /// The enclosing model must initialize DockBase after Rc::new_cyclic returns.
    pub fn construct<T: DockableBaseImpl + IDockDock + 'static>(runtime: &Weak<T>) -> Self {
        let base = DockBase::construct(runtime.clone());
        Self {
            base,
            last_child_fill: RefCell::new(true),
        }
    }
    /// Gets `LastChildFill`.
    pub fn last_child_fill(&self) -> bool {
        *self.last_child_fill.borrow()
    }
    /// Sets `LastChildFill`, notifying only when changed.
    pub fn set_last_child_fill(&self, value: bool) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.last_child_fill, value, "LastChildFill");
    }
    /// Boxes the container using the canonical erased dockable representation.
    pub fn boxed(self: &Rc<Self>) -> BoxedValue {
        Rc::new(self.clone() as Rc<dyn IDockable>)
    }
}
impl IDockDock for DockDock {
    fn last_child_fill(&self) -> bool {
        self.last_child_fill()
    }
    fn set_last_child_fill(&self, value: bool) {
        self.set_last_child_fill(value);
    }
}
