// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::controls::*;
use crate::core::*;
use ferrodock_model::collections::ListHandle;
use ferroui_reactiveui::ICommand;
use std::rc::Rc;

/// The `IRootDock` docking contract.
pub trait IRootDock: IDock {
    /// Gets or sets if root dock is focusable.
    fn is_focusable_root(&self) -> bool;
    /// Sets `IsFocusableRoot`.
    fn set_is_focusable_root(&self, value: bool);
    /// Gets or sets hidden dockables.
    fn hidden_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>>;
    /// Sets `HiddenDockables`.
    fn set_hidden_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>);
    /// Gets or sets left pinned dockables.
    fn left_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>>;
    /// Sets `LeftPinnedDockables`.
    fn set_left_pinned_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>);
    /// Gets or sets right pinned dockables.
    fn right_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>>;
    /// Sets `RightPinnedDockables`.
    fn set_right_pinned_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>);
    /// Gets or sets top pinned dockables.
    fn top_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>>;
    /// Sets `TopPinnedDockables`.
    fn set_top_pinned_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>);
    /// Gets or sets bottom pinned dockables.
    fn bottom_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>>;
    /// Sets `BottomPinnedDockables`.
    fn set_bottom_pinned_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>);
    /// Gets or sets pinned tool dock.
    fn pinned_dock(&self) -> Option<Rc<dyn IToolDock>>;
    /// Sets `PinnedDock`.
    fn set_pinned_dock(&self, value: Option<Rc<dyn IToolDock>>);
    /// Gets or sets pinned dock display mode.
    fn pinned_dock_display_mode(&self) -> PinnedDockDisplayMode;
    /// Sets `PinnedDockDisplayMode`.
    fn set_pinned_dock_display_mode(&self, value: PinnedDockDisplayMode);
    /// Gets or sets owner window.
    fn window(&self) -> Option<Rc<dyn IDockWindow>>;
    /// Sets `Window`.
    fn set_window(&self, value: Option<Rc<dyn IDockWindow>>);
    /// Gets or sets windows.
    fn windows(&self) -> Option<ListHandle<Option<Rc<dyn IDockWindow>>>>;
    /// Sets `Windows`.
    fn set_windows(&self, value: Option<ListHandle<Option<Rc<dyn IDockWindow>>>>);
    /// Gets or sets the floating window host mode override.
    fn floating_window_host_mode(&self) -> DockFloatingWindowHostMode;
    /// Sets `FloatingWindowHostMode`.
    fn set_floating_window_host_mode(&self, value: DockFloatingWindowHostMode);
    /// Gets or sets optional root-wide capability policy values.
    /// Null values inherit dockable base flag values.
    fn root_dock_capability_policy(&self) -> Option<Rc<dyn DockCapabilityPolicyImpl>>;
    /// Sets `RootDockCapabilityPolicy`.
    fn set_root_dock_capability_policy(&self, value: Option<Rc<dyn DockCapabilityPolicyImpl>>);
    /// Show windows.
    fn show_windows(&self) -> Rc<dyn ICommand>;
    /// Exit windows.
    fn exit_windows(&self) -> Rc<dyn ICommand>;
    /// Gets or sets a value indicating whether adaptive global dock targets are enabled.
    /// When enabled the global dock targets are reduced to only show options where the layout would change.
    /// This is typically used for dashboards and widget areas.
    fn enable_adaptive_global_dock_targets(&self) -> bool;
    /// Sets `EnableAdaptiveGlobalDockTargets`.
    fn set_enable_adaptive_global_dock_targets(&self, value: bool);
}

impl PartialEq for dyn IRootDock {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
