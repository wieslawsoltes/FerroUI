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

/// The reactive `WrapDock` container with the source properties and defaults.
pub struct WrapDock {
    base: DockBase,
    orientation: RefCell<Orientation>,
}

reactive_object!(WrapDock, base);
impl DockableBaseImpl for WrapDock {}
impl_dock!(
    WrapDock,
    fn as_wrap_dock(self: Rc<Self>) -> Option<Rc<dyn IWrapDock>> {
        Some(self)
    }
);
impl WrapDock {
    /// Creates a shared container and attaches its source reactive observations.
    pub fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(Self::construct);
        model.base.initialize(&model);
        model
    }
    /// Constructs the component inside a model with the complete `IWrapDock` contract.
    /// The enclosing model must initialize DockBase after Rc::new_cyclic returns.
    pub fn construct<T: DockableBaseImpl + IWrapDock + 'static>(runtime: &Weak<T>) -> Self {
        let base = DockBase::construct(runtime.clone());
        Self {
            base,
            orientation: RefCell::new(Orientation::Horizontal),
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
    /// Boxes the container using the canonical erased dockable representation.
    pub fn boxed(self: &Rc<Self>) -> BoxedValue {
        Rc::new(self.clone() as Rc<dyn IDockable>)
    }
}
impl IWrapDock for WrapDock {
    fn orientation(&self) -> Orientation {
        self.orientation()
    }
    fn set_orientation(&self, value: Orientation) {
        self.set_orientation(value);
    }
}
