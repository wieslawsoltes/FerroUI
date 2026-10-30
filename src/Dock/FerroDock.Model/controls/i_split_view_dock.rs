// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

/// The `ISplitViewDock` docking contract.
pub trait ISplitViewDock: IDock {
    /// Gets or sets the width of the pane when closed in compact display modes.
    fn compact_pane_length(&self) -> f64;
    /// Sets `CompactPaneLength`.
    fn set_compact_pane_length(&self, value: f64);
    /// Gets or sets how the pane is displayed in both open and closed states.
    fn display_mode(&self) -> SplitViewDisplayMode;
    /// Sets `DisplayMode`.
    fn set_display_mode(&self, value: SplitViewDisplayMode);
    /// Gets or sets whether the pane is currently open.
    fn is_pane_open(&self) -> bool;
    /// Sets `IsPaneOpen`.
    fn set_is_pane_open(&self, value: bool);
    /// Gets or sets the width of the pane when it is open.
    fn open_pane_length(&self) -> f64;
    /// Sets `OpenPaneLength`.
    fn set_open_pane_length(&self, value: f64);
    /// Gets or sets the position of the pane (Left, Right, Top, or Bottom).
    fn pane_placement(&self) -> SplitViewPanePlacement;
    /// Sets `PanePlacement`.
    fn set_pane_placement(&self, value: SplitViewPanePlacement);
    /// Gets or sets whether to use light dismiss overlay mode.
    /// When enabled in overlay modes, a light dismiss overlay is shown
    /// and clicking outside the pane can close it.
    fn use_light_dismiss_overlay_mode(&self) -> bool;
    /// Sets `UseLightDismissOverlayMode`.
    fn set_use_light_dismiss_overlay_mode(&self, value: bool);
    /// Gets or sets the dockable displayed in the pane area.
    /// This can be a single dockable or a container dock (like ToolDock).
    fn pane_dockable(&self) -> Option<Rc<dyn IDockable>>;
    /// Sets `PaneDockable`.
    fn set_pane_dockable(&self, value: Option<Rc<dyn IDockable>>);
    /// Gets or sets the dockable displayed in the content area.
    /// This is typically the main content (like a DocumentDock).
    fn content_dockable(&self) -> Option<Rc<dyn IDockable>>;
    /// Sets `ContentDockable`.
    fn set_content_dockable(&self, value: Option<Rc<dyn IDockable>>);
    /// Opens the pane.
    fn open_pane(&self);
    /// Closes the pane.
    fn close_pane(&self);
    /// Toggles the pane open/closed state.
    fn toggle_pane(&self);
}

impl PartialEq for dyn ISplitViewDock {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
