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

/// The reactive `StackDock` container with the source properties and defaults.
pub struct StackDock {
    base: DockBase,
    orientation: RefCell<Orientation>,
    spacing: RefCell<f64>,
}

reactive_object!(StackDock, base);
impl DockableBaseImpl for StackDock {}
impl_dock!(
    StackDock,
    fn as_stack_dock(self: Rc<Self>) -> Option<Rc<dyn IStackDock>> {
        Some(self)
    }
);
impl StackDock {
    /// Creates a shared container and attaches its source reactive observations.
    pub fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(Self::construct);
        model.base.initialize(&model);
        model
    }
    /// Constructs the component inside a model with the complete `IStackDock` contract.
    /// The enclosing model must initialize DockBase after Rc::new_cyclic returns.
    pub fn construct<T: DockableBaseImpl + IStackDock + 'static>(runtime: &Weak<T>) -> Self {
        let base = DockBase::construct(runtime.clone());
        Self {
            base,
            orientation: RefCell::new(Orientation::Horizontal),
            spacing: RefCell::new(0.0),
        }
    }
    /// Gets `Orientation`.
    pub fn orientation(&self) -> Orientation {
        *self.orientation.borrow()
    }
    /// Sets `Orientation`, notifying only when changed.
    pub fn set_orientation(&self, value: Orientation) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.orientation, value, "Orientation");
    }
    /// Gets `Spacing`.
    pub fn spacing(&self) -> f64 {
        *self.spacing.borrow()
    }
    /// Sets `Spacing`, notifying only when changed.
    pub fn set_spacing(&self, value: f64) {
        let current = *self.spacing.borrow();
        if current == value || (current.is_nan() && value.is_nan()) {
            return;
        }
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.spacing, value, "Spacing");
    }
    /// Boxes the container using the canonical erased dockable representation.
    pub fn boxed(self: &Rc<Self>) -> BoxedValue {
        Rc::new(self.clone() as Rc<dyn IDockable>)
    }
}
impl IStackDock for StackDock {
    fn orientation(&self) -> Orientation {
        self.orientation()
    }
    fn set_orientation(&self, value: Orientation) {
        self.set_orientation(value);
    }
    fn spacing(&self) -> f64 {
        self.spacing()
    }
    fn set_spacing(&self, value: f64) {
        self.set_spacing(value);
    }
}
