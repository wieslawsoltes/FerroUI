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

/// The reactive `UniformGridDock` container with the source properties and defaults.
pub struct UniformGridDock {
    base: DockBase,
    rows: RefCell<i32>,
    columns: RefCell<i32>,
}

reactive_object!(UniformGridDock, base);
impl DockableBaseImpl for UniformGridDock {}
impl_dock!(
    UniformGridDock,
    fn as_uniform_grid_dock(self: Rc<Self>) -> Option<Rc<dyn IUniformGridDock>> {
        Some(self)
    }
);
impl UniformGridDock {
    /// Creates a shared container and attaches its source reactive observations.
    pub fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(Self::construct);
        model.base.initialize(&model);
        model
    }
    /// Constructs the component inside a model with the complete `IUniformGridDock` contract.
    /// The enclosing model must initialize DockBase after Rc::new_cyclic returns.
    pub fn construct<T: DockableBaseImpl + IUniformGridDock + 'static>(runtime: &Weak<T>) -> Self {
        let base = DockBase::construct(runtime.clone());
        Self {
            base,
            rows: RefCell::new(0),
            columns: RefCell::new(0),
        }
    }
    /// Gets `Rows`.
    pub fn rows(&self) -> i32 {
        *self.rows.borrow()
    }
    /// Sets `Rows`, notifying only when changed.
    pub fn set_rows(&self, value: i32) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.rows, value, "Rows");
    }
    /// Gets `Columns`.
    pub fn columns(&self) -> i32 {
        *self.columns.borrow()
    }
    /// Sets `Columns`, notifying only when changed.
    pub fn set_columns(&self, value: i32) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.columns, value, "Columns");
    }
    /// Boxes the container using the canonical erased dockable representation.
    pub fn boxed(self: &Rc<Self>) -> BoxedValue {
        Rc::new(self.clone() as Rc<dyn IDockable>)
    }
}
impl IUniformGridDock for UniformGridDock {
    fn rows(&self) -> i32 {
        self.rows()
    }
    fn set_rows(&self, value: i32) {
        self.set_rows(value);
    }
    fn columns(&self) -> i32 {
        self.columns()
    }
    fn set_columns(&self, value: i32) {
        self.set_columns(value);
    }
}
