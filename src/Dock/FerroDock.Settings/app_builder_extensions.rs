// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{DockFloatingWindowOwnerPolicy, DockSettings, DockSettingsOptions};
use ferrodock_model::core::{DockFloatingWindowHostMode, DockWindowOwnerMode};
use ferroui_controls::AppBuilder;
/// Original docking settings extensions for the native shared app builder.
pub trait AppBuilderExtensions {
    /// Applies only supplied options, in source order, and returns the same builder.
    fn with_dock_settings(&self, options: Option<&DockSettingsOptions>) -> AppBuilder;
    /// Configures source `UseFloatingDockAdorner` and returns the same native builder.
    fn use_floating_dock_adorner(&self, enable: bool) -> AppBuilder;
    /// Configures source `UsePinnedDockWindow` and returns the same native builder.
    fn use_pinned_dock_window(&self, enable: bool) -> AppBuilder;
    /// Configures source `UseManagedWindows` and returns the same native builder.
    fn use_managed_windows(&self, enable: bool) -> AppBuilder;
    /// Configures source `UseFloatingWindowHostMode` and returns the same native builder.
    fn use_floating_window_host_mode(&self, mode: DockFloatingWindowHostMode) -> AppBuilder;
    /// Configures source `UseOwnerForFloatingWindows` and returns the same native builder.
    fn use_owner_for_floating_windows(&self, enable: bool) -> AppBuilder;
    /// Configures source `UseFloatingWindowOwnerPolicy` and returns the same native builder.
    fn use_floating_window_owner_policy(&self, policy: DockFloatingWindowOwnerPolicy)
        -> AppBuilder;
    /// Configures source `UseDefaultFloatingWindowOwnerMode` and returns the same native builder.
    fn use_default_floating_window_owner_mode(&self, owner_mode: DockWindowOwnerMode)
        -> AppBuilder;
    /// Configures source `EnableWindowMagnetism` and returns the same native builder.
    fn enable_window_magnetism(&self, enable: bool) -> AppBuilder;
    /// Configures source `SetWindowMagnetDistance` and returns the same native builder.
    fn set_window_magnet_distance(&self, distance: f64) -> AppBuilder;
    /// Configures source `BringWindowsToFrontOnDrag` and returns the same native builder.
    fn bring_windows_to_front_on_drag(&self, enable: bool) -> AppBuilder;
    /// Configures source `CloseFloatingWindowsOnMainWindowClose` and returns the same native builder.
    fn close_floating_windows_on_main_window_close(&self, enable: bool) -> AppBuilder;
    /// Configures source `ShowDockablePreviewOnDrag` and returns the same native builder.
    fn show_dockable_preview_on_drag(&self, enable: bool) -> AppBuilder;
    /// Configures source `SetDragPreviewOpacity` and returns the same native builder.
    fn set_drag_preview_opacity(&self, opacity: f64) -> AppBuilder;
    /// Configures source `UpdateItemsSourceOnUnregister` and returns the same native builder.
    fn update_items_source_on_unregister(&self, update: bool) -> AppBuilder;
}
impl AppBuilderExtensions for AppBuilder {
    fn with_dock_settings(&self, options: Option<&DockSettingsOptions>) -> AppBuilder {
        let Some(options) = options else {
            return self.clone();
        };
        if let Some(value) = options.minimum_horizontal_drag_distance() {
            DockSettings::set_minimum_horizontal_drag_distance(value);
        }
        if let Some(value) = options.minimum_vertical_drag_distance() {
            DockSettings::set_minimum_vertical_drag_distance(value);
        }
        if let Some(value) = options.use_floating_dock_adorner() {
            DockSettings::set_use_floating_dock_adorner(value);
        }
        if let Some(value) = options.use_pinned_dock_window() {
            DockSettings::set_use_pinned_dock_window(value);
        }
        if let Some(value) = options.use_managed_windows() {
            DockSettings::set_use_managed_windows(value);
        }
        if let Some(value) = options.floating_window_host_mode() {
            DockSettings::set_floating_window_host_mode(value);
        }
        if let Some(value) = options.use_owner_for_floating_windows() {
            DockSettings::set_use_owner_for_floating_windows(value);
        }
        if let Some(value) = options.floating_window_owner_policy() {
            DockSettings::set_floating_window_owner_policy(value);
        }
        if let Some(value) = options.default_floating_window_owner_mode() {
            DockSettings::set_default_floating_window_owner_mode(value);
        }
        if let Some(value) = options.enable_window_magnetism() {
            DockSettings::set_enable_window_magnetism(value);
        }
        if let Some(value) = options.window_magnet_distance() {
            DockSettings::set_window_magnet_distance(value);
        }
        if let Some(value) = options.bring_windows_to_front_on_drag() {
            DockSettings::set_bring_windows_to_front_on_drag(value);
        }
        if let Some(value) = options.close_floating_windows_on_main_window_close() {
            DockSettings::set_close_floating_windows_on_main_window_close(value);
        }
        if let Some(value) = options.show_dockable_preview_on_drag() {
            DockSettings::set_show_dockable_preview_on_drag(value);
        }
        if let Some(value) = options.drag_preview_opacity() {
            DockSettings::set_drag_preview_opacity(value);
        }
        if let Some(value) = options.update_items_source_on_unregister() {
            DockSettings::set_update_items_source_on_unregister(value);
        }
        if let Some(value) = options.selector_enabled() {
            DockSettings::set_selector_enabled(value);
        }
        if let Some(value) = options.document_selector_key_gesture() {
            DockSettings::set_document_selector_key_gesture(value);
        }
        if let Some(value) = options.tool_selector_key_gesture() {
            DockSettings::set_tool_selector_key_gesture(value);
        }
        if let Some(value) = options.command_bar_merging_enabled() {
            DockSettings::set_command_bar_merging_enabled(value);
        }
        if let Some(value) = options.command_bar_merging_scope() {
            DockSettings::set_command_bar_merging_scope(value);
        }
        self.clone()
    }
    fn use_floating_dock_adorner(&self, enable: bool) -> AppBuilder {
        DockSettings::set_use_floating_dock_adorner(enable);
        self.clone()
    }
    fn use_pinned_dock_window(&self, enable: bool) -> AppBuilder {
        DockSettings::set_use_pinned_dock_window(enable);
        self.clone()
    }
    fn use_managed_windows(&self, enable: bool) -> AppBuilder {
        DockSettings::set_use_managed_windows(enable);
        self.clone()
    }
    fn use_floating_window_host_mode(&self, mode: DockFloatingWindowHostMode) -> AppBuilder {
        DockSettings::set_floating_window_host_mode(mode);
        self.clone()
    }
    fn use_owner_for_floating_windows(&self, enable: bool) -> AppBuilder {
        DockSettings::set_use_owner_for_floating_windows(enable);
        self.clone()
    }
    fn use_floating_window_owner_policy(
        &self,
        policy: DockFloatingWindowOwnerPolicy,
    ) -> AppBuilder {
        DockSettings::set_floating_window_owner_policy(policy);
        self.clone()
    }
    fn use_default_floating_window_owner_mode(
        &self,
        owner_mode: DockWindowOwnerMode,
    ) -> AppBuilder {
        DockSettings::set_default_floating_window_owner_mode(owner_mode);
        self.clone()
    }
    fn enable_window_magnetism(&self, enable: bool) -> AppBuilder {
        DockSettings::set_enable_window_magnetism(enable);
        self.clone()
    }
    fn set_window_magnet_distance(&self, distance: f64) -> AppBuilder {
        DockSettings::set_window_magnet_distance(distance);
        self.clone()
    }
    fn bring_windows_to_front_on_drag(&self, enable: bool) -> AppBuilder {
        DockSettings::set_bring_windows_to_front_on_drag(enable);
        self.clone()
    }
    fn close_floating_windows_on_main_window_close(&self, enable: bool) -> AppBuilder {
        DockSettings::set_close_floating_windows_on_main_window_close(enable);
        self.clone()
    }
    fn show_dockable_preview_on_drag(&self, enable: bool) -> AppBuilder {
        DockSettings::set_show_dockable_preview_on_drag(enable);
        self.clone()
    }
    fn set_drag_preview_opacity(&self, opacity: f64) -> AppBuilder {
        DockSettings::set_drag_preview_opacity(opacity);
        self.clone()
    }
    fn update_items_source_on_unregister(&self, update: bool) -> AppBuilder {
        DockSettings::set_update_items_source_on_unregister(update);
        self.clone()
    }
}
