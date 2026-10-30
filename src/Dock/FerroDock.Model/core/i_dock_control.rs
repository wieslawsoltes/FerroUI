// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use crate::BoxedValue;
use std::rc::Rc;

/// The `IDockControl` docking contract.
pub trait IDockControl {
    /// Resolves the native control view without adding a UI dependency to the model.
    #[doc(hidden)]
    fn boxed_interface(self: Rc<Self>, _target: std::any::TypeId) -> Option<BoxedValue> {
        None
    }

    /// Gets dock manager.
    fn dock_manager(&self) -> Rc<dyn IDockManager>;
    /// Gets dock control state.
    fn dock_control_state(&self) -> Rc<dyn IDockControlState>;
    /// Gets or sets the dock layout.
    fn layout(&self) -> Option<Rc<dyn IDock>>;
    /// Sets `Layout`.
    fn set_layout(&self, value: Option<Rc<dyn IDock>>);
    /// Gets or sets default context.
    fn default_context(&self) -> Option<BoxedValue>;
    /// Sets `DefaultContext`.
    fn set_default_context(&self, value: Option<BoxedValue>);
    /// Gets or sets the flag indicating whether to initialize layout.
    fn initialize_layout(&self) -> bool;
    /// Sets `InitializeLayout`.
    fn set_initialize_layout(&self, value: bool);
    /// Gets or sets the flag indicating whether to initialize factory.
    fn initialize_factory(&self) -> bool;
    /// Sets `InitializeFactory`.
    fn set_initialize_factory(&self, value: bool);
    /// Gets or sets the factory.
    fn factory(&self) -> Option<Rc<dyn IFactory>>;
    /// Sets `Factory`.
    fn set_factory(&self, value: Option<Rc<dyn IFactory>>);
}

impl PartialEq for dyn IDockControl {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
