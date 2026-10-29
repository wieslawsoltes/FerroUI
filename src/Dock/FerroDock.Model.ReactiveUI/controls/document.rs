// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::{DockableBase, DockableBaseImpl};
use crate::impl_dockable;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::BoxedValue;
use ferroui_reactiveui::{reactive_object, IReactiveObjectExt};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A reactive document model with MDI and layout-backed lifecycle state.
pub struct Document {
    base: DockableBase,
    mdi_bounds: RefCell<DockRect>,
    mdi_state: RefCell<MdiWindowState>,
    mdi_z_index: RefCell<i32>,
    is_open: RefCell<bool>,
    is_active: RefCell<bool>,
    is_selected: RefCell<bool>,
}

reactive_object!(Document, base);
impl DockableBaseImpl for Document {}
impl_dockable!(
    Document,
    fn as_local_target(self: Rc<Self>) -> Option<Rc<dyn ILocalTarget>> {
        Some(self)
    },
    fn as_document(self: Rc<Self>) -> Option<Rc<dyn IDocument>> {
        Some(self)
    },
    fn as_mdi_document(self: Rc<Self>) -> Option<Rc<dyn IMdiDocument>> {
        Some(self)
    },
    fn as_docking_window_state(&self) -> Option<&dyn IDockingWindowState> {
        Some(self)
    }
);
impl ILocalTarget for Document {}
impl IDocument for Document {}

impl Document {
    /// Creates a shared model with the source defaults.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak: &Weak<Self>| Self::construct(weak.clone()))
    }
    /// Constructs this component with the enclosing model's reactive identity.
    pub fn construct<T: DockableBaseImpl + IMdiDocument + IDockingWindowState + 'static>(
        runtime: Weak<T>,
    ) -> Self {
        let runtime: Weak<dyn DockableBaseImpl> = runtime;
        Self {
            base: DockableBase::construct(runtime),
            mdi_bounds: RefCell::new(DockRect::default()),
            mdi_state: RefCell::new(MdiWindowState::Normal),
            mdi_z_index: RefCell::new(0),
            is_open: RefCell::new(false),
            is_active: RefCell::new(false),
            is_selected: RefCell::new(false),
        }
    }
    /// Gets `MdiBounds`.
    pub fn mdi_bounds(&self) -> DockRect {
        *self.mdi_bounds.borrow()
    }
    /// Sets `MdiBounds`, notifying only when changed.
    pub fn set_mdi_bounds(&self, value: DockRect) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.mdi_bounds, value, "MdiBounds");
    }
    /// Gets `MdiState`.
    pub fn mdi_state(&self) -> MdiWindowState {
        *self.mdi_state.borrow()
    }
    /// Sets `MdiState`, notifying only when changed.
    pub fn set_mdi_state(&self, value: MdiWindowState) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.mdi_state, value, "MdiState");
    }
    /// Gets `MdiZIndex`.
    pub fn mdi_z_index(&self) -> i32 {
        *self.mdi_z_index.borrow()
    }
    /// Sets `MdiZIndex`, notifying only when changed.
    pub fn set_mdi_z_index(&self, value: i32) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.mdi_z_index, value, "MdiZIndex");
    }
    /// Gets `IsOpen`.
    pub fn is_open(&self) -> bool {
        *self.is_open.borrow()
    }
    /// Sets `IsOpen` and then requests synchronization from a supporting factory.
    pub fn set_is_open(&self, value: bool) {
        if self.is_open() == value {
            return;
        }
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.is_open, value, "IsOpen");
        self.base
            .notify_docking_window_state_changed(DockingWindowStateProperty::IsOpen);
    }
    /// Gets `IsActive`.
    pub fn is_active(&self) -> bool {
        *self.is_active.borrow()
    }
    /// Sets `IsActive` and then requests synchronization from a supporting factory.
    pub fn set_is_active(&self, value: bool) {
        if self.is_active() == value {
            return;
        }
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.is_active, value, "IsActive");
        self.base
            .notify_docking_window_state_changed(DockingWindowStateProperty::IsActive);
    }
    /// Gets `IsSelected`.
    pub fn is_selected(&self) -> bool {
        *self.is_selected.borrow()
    }
    /// Sets `IsSelected` and then requests synchronization from a supporting factory.
    pub fn set_is_selected(&self, value: bool) {
        if self.is_selected() == value {
            return;
        }
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.is_selected, value, "IsSelected");
        self.base
            .notify_docking_window_state_changed(DockingWindowStateProperty::IsSelected);
    }
    /// Gets the inherited logical docking state.
    pub fn docking_state(&self) -> DockingWindowState {
        self.base.docking_state()
    }
    /// Sets the inherited logical docking state.
    pub fn set_docking_state(&self, value: DockingWindowState) {
        self.base.set_docking_state(value);
    }
    /// Boxes the model using the canonical erased dockable representation.
    pub fn boxed(self: &Rc<Self>) -> BoxedValue {
        Rc::new(self.clone() as Rc<dyn IDockable>)
    }
}
impl IMdiDocument for Document {
    fn mdi_bounds(&self) -> DockRect {
        self.mdi_bounds()
    }
    fn set_mdi_bounds(&self, value: DockRect) {
        self.set_mdi_bounds(value);
    }
    fn mdi_state(&self) -> MdiWindowState {
        self.mdi_state()
    }
    fn set_mdi_state(&self, value: MdiWindowState) {
        self.set_mdi_state(value);
    }
    fn mdi_z_index(&self) -> i32 {
        self.mdi_z_index()
    }
    fn set_mdi_z_index(&self, value: i32) {
        self.set_mdi_z_index(value);
    }
}
impl IDockingWindowState for Document {
    fn is_open(&self) -> bool {
        self.is_open()
    }
    fn set_is_open(&self, value: bool) {
        self.set_is_open(value);
    }
    fn is_active(&self) -> bool {
        self.is_active()
    }
    fn set_is_active(&self, value: bool) {
        self.set_is_active(value);
    }
    fn is_selected(&self) -> bool {
        self.is_selected()
    }
    fn set_is_selected(&self, value: bool) {
        self.set_is_selected(value);
    }
    fn docking_state(&self) -> DockingWindowState {
        self.docking_state()
    }
    fn set_docking_state(&self, value: DockingWindowState) {
        self.set_docking_state(value);
    }
}
