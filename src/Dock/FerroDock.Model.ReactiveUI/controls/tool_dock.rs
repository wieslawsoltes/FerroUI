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

/// The reactive `ToolDock` container with the source properties and defaults.
pub struct ToolDock {
    base: DockBase,
    alignment: RefCell<Alignment>,
    is_expanded: RefCell<bool>,
    auto_hide: RefCell<bool>,
    grip_mode: RefCell<GripMode>,
}

reactive_object!(ToolDock, base);
impl DockableBaseImpl for ToolDock {}
impl_dock!(
    ToolDock,
    fn as_local_target(self: Rc<Self>) -> Option<Rc<dyn ILocalTarget>> {
        Some(self)
    },
    fn as_tool_dock(self: Rc<Self>) -> Option<Rc<dyn IToolDock>> {
        Some(self)
    }
);
impl ILocalTarget for ToolDock {}
impl ToolDock {
    /// Creates a shared container and attaches its source reactive observations.
    pub fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(Self::construct);
        model.base.initialize(&model);
        model
    }
    /// Constructs the component inside a model with the complete `IToolDock` contract.
    /// The enclosing model must initialize DockBase after Rc::new_cyclic returns.
    pub fn construct<T: DockableBaseImpl + IToolDock + 'static>(runtime: &Weak<T>) -> Self {
        let base = DockBase::construct(runtime.clone());
        Self {
            base,
            alignment: RefCell::new(Alignment::Unset),
            is_expanded: RefCell::new(false),
            auto_hide: RefCell::new(true),
            grip_mode: RefCell::new(GripMode::Visible),
        }
    }
    /// Gets `Alignment`.
    pub fn alignment(&self) -> Alignment {
        *self.alignment.borrow()
    }
    /// Sets `Alignment`, notifying only when changed.
    pub fn set_alignment(&self, value: Alignment) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.alignment, value, "Alignment");
    }
    /// Gets `IsExpanded`.
    pub fn is_expanded(&self) -> bool {
        *self.is_expanded.borrow()
    }
    /// Sets `IsExpanded`, notifying only when changed.
    pub fn set_is_expanded(&self, value: bool) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.is_expanded, value, "IsExpanded");
    }
    /// Gets `AutoHide`.
    pub fn auto_hide(&self) -> bool {
        *self.auto_hide.borrow()
    }
    /// Sets `AutoHide`, notifying only when changed.
    pub fn set_auto_hide(&self, value: bool) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.auto_hide, value, "AutoHide");
    }
    /// Gets `GripMode`.
    pub fn grip_mode(&self) -> GripMode {
        *self.grip_mode.borrow()
    }
    /// Sets `GripMode`, notifying only when changed.
    pub fn set_grip_mode(&self, value: GripMode) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.grip_mode, value, "GripMode");
    }
    /// Invokes the enclosing model's virtual AddTool method.
    pub fn add_tool(&self, tool: Rc<dyn IDockable>) {
        self.base
            .runtime()
            .as_tool_dock()
            .expect("tool dock membership")
            .add_tool(Some(tool));
    }
    /// Invokes the original AddTool virtual slot with a possibly null reference.
    pub fn add_tool_nullable(&self, tool: Option<Rc<dyn IDockable>>) {
        self.base
            .runtime()
            .as_tool_dock()
            .expect("tool dock membership")
            .add_tool(tool);
    }
    /// Implements the base AddTool behavior for nonnull source calls.
    pub fn add_tool_core(&self, tool: Rc<dyn IDockable>) {
        self.add_tool_nullable_core(Some(tool));
    }
    /// Implements the original AddTool body, preserving a null argument.
    pub fn add_tool_nullable_core(&self, tool: Option<Rc<dyn IDockable>>) {
        let runtime = self.base.runtime();
        if let Some(factory) = runtime.factory() {
            factory.add_dockable(
                Some(runtime.clone().as_dock().expect("dock membership")),
                tool.clone(),
            );
        }
        if let Some(factory) = runtime.factory() {
            factory.set_active_dockable(tool.clone());
        }
        if let Some(factory) = runtime.factory() {
            factory.set_focused_dockable(Some(runtime.as_dock().expect("dock membership")), tool);
        }
    }
    /// Boxes the container using the canonical erased dockable representation.
    pub fn boxed(self: &Rc<Self>) -> BoxedValue {
        Rc::new(self.clone() as Rc<dyn IDockable>)
    }
}
impl IToolDock for ToolDock {
    fn alignment(&self) -> Alignment {
        self.alignment()
    }
    fn set_alignment(&self, value: Alignment) {
        self.set_alignment(value);
    }
    fn is_expanded(&self) -> bool {
        self.is_expanded()
    }
    fn set_is_expanded(&self, value: bool) {
        self.set_is_expanded(value);
    }
    fn auto_hide(&self) -> bool {
        self.auto_hide()
    }
    fn set_auto_hide(&self, value: bool) {
        self.set_auto_hide(value);
    }
    fn grip_mode(&self) -> GripMode {
        self.grip_mode()
    }
    fn set_grip_mode(&self, value: GripMode) {
        self.set_grip_mode(value);
    }
    fn add_tool(&self, tool: Option<Rc<dyn IDockable>>) {
        self.add_tool_nullable_core(tool);
    }
}
