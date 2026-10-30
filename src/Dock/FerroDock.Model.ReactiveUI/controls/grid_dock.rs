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

/// The reactive `GridDock` container with the source properties and defaults.
pub struct GridDock {
    base: DockBase,
    column_definitions: RefCell<Option<String>>,
    row_definitions: RefCell<Option<String>>,
}

reactive_object!(GridDock, base);
impl DockableBaseImpl for GridDock {}
impl_dock!(
    GridDock,
    fn as_grid_dock(self: Rc<Self>) -> Option<Rc<dyn IGridDock>> {
        Some(self)
    }
);
impl GridDock {
    /// Creates a shared container and attaches its source reactive observations.
    pub fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(Self::construct);
        model.base.initialize(&model);
        model
    }
    /// Constructs the component inside a model with the complete `IGridDock` contract.
    /// The enclosing model must initialize DockBase after Rc::new_cyclic returns.
    pub fn construct<T: DockableBaseImpl + IGridDock + 'static>(runtime: &Weak<T>) -> Self {
        let base = DockBase::construct(runtime.clone());
        Self {
            base,
            column_definitions: RefCell::new(None),
            row_definitions: RefCell::new(None),
        }
    }
    /// Gets `ColumnDefinitions`.
    pub fn column_definitions(&self) -> Option<String> {
        self.column_definitions.borrow().clone()
    }
    /// Sets `ColumnDefinitions`, notifying only when changed.
    pub fn set_column_definitions(&self, value: Option<String>) {
        self.base.runtime().raise_and_set_if_changed(
            &self.column_definitions,
            value,
            "ColumnDefinitions",
        );
    }
    /// Gets `RowDefinitions`.
    pub fn row_definitions(&self) -> Option<String> {
        self.row_definitions.borrow().clone()
    }
    /// Sets `RowDefinitions`, notifying only when changed.
    pub fn set_row_definitions(&self, value: Option<String>) {
        self.base.runtime().raise_and_set_if_changed(
            &self.row_definitions,
            value,
            "RowDefinitions",
        );
    }
    /// Boxes the container using the canonical erased dockable representation.
    pub fn boxed(self: &Rc<Self>) -> BoxedValue {
        Rc::new(self.clone() as Rc<dyn IDockable>)
    }
}
impl IGridDock for GridDock {
    fn column_definitions(&self) -> Option<String> {
        self.column_definitions()
    }
    fn set_column_definitions(&self, value: Option<String>) {
        self.set_column_definitions(value);
    }
    fn row_definitions(&self) -> Option<String> {
        self.row_definitions()
    }
    fn set_row_definitions(&self, value: Option<String>) {
        self.set_row_definitions(value);
    }
}
