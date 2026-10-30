// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

/// The `IHostWindow` docking contract.
pub trait IHostWindow {
    /// Projects the original runtime class name without reflection.
    #[doc(hidden)]
    fn runtime_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
            .rsplit("::")
            .next()
            .expect("compiler type name")
    }

    /// Resolves a native plain-object interface without a UI dependency or reflection.
    #[doc(hidden)]
    fn boxed_interface(self: Rc<Self>, _target: std::any::TypeId) -> Option<crate::BoxedValue> {
        None
    }

    /// Resolves optional close completion reporting without reflection.
    fn as_host_window_close_state(&self) -> Option<&dyn IHostWindowCloseState> {
        None
    }
    /// Gets host window state.
    fn host_window_state(&self) -> Option<Rc<dyn IHostWindowState>>;
    /// Gets or sets value that indicates whether host size and position is tracked.
    fn is_tracked(&self) -> bool;
    /// Sets `IsTracked`.
    fn set_is_tracked(&self, value: bool);
    /// Gets or sets dock window.
    fn window(&self) -> Option<Rc<dyn IDockWindow>>;
    /// Sets `Window`.
    fn set_window(&self, value: Option<Rc<dyn IDockWindow>>);
    /// Presents host.
    fn present(&self, is_dialog: bool);
    /// Exits host.
    fn exit(&self);
    /// Sets host position.
    fn set_position(&self, x: f64, y: f64);
    /// Gets host position.
    fn get_position(&self) -> (f64, f64);
    /// Sets host size.
    fn set_size(&self, width: f64, height: f64);
    /// Gets host size.
    fn get_size(&self) -> (f64, f64);
    /// Sets host window state.
    fn set_window_state(&self, window_state: DockWindowState);
    /// Gets host window state.
    fn get_window_state(&self) -> DockWindowState;
    /// Sets host title.
    fn set_title(&self, title: Option<String>);
    /// Sets host layout.
    fn set_layout(&self, layout: Rc<dyn IDock>);
    /// Activates the host window.
    fn set_active(&self);
}

impl PartialEq for dyn IHostWindow {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
