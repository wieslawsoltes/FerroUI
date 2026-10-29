// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::DockCommandBarMergingScope;
use crate::DockFloatingWindowOwnerPolicy;
use ferrodock_model::core::{DockFloatingWindowHostMode, DockWindowOwnerMode};
use ferroui_base::input::KeyGesture;
use std::cell::Cell;
use std::rc::Rc;
/// Optional configuration values for the original builder extension.
pub struct DockSettingsOptions {
    minimum_horizontal_drag_distance: Cell<Option<f64>>,
    minimum_vertical_drag_distance: Cell<Option<f64>>,
    use_floating_dock_adorner: Cell<Option<bool>>,
    use_pinned_dock_window: Cell<Option<bool>>,
    use_managed_windows: Cell<Option<bool>>,
    floating_window_host_mode: Cell<Option<DockFloatingWindowHostMode>>,
    use_owner_for_floating_windows: Cell<Option<bool>>,
    floating_window_owner_policy: Cell<Option<DockFloatingWindowOwnerPolicy>>,
    default_floating_window_owner_mode: Cell<Option<DockWindowOwnerMode>>,
    enable_window_magnetism: Cell<Option<bool>>,
    window_magnet_distance: Cell<Option<f64>>,
    bring_windows_to_front_on_drag: Cell<Option<bool>>,
    close_floating_windows_on_main_window_close: Cell<Option<bool>>,
    show_dockable_preview_on_drag: Cell<Option<bool>>,
    drag_preview_opacity: Cell<Option<f64>>,
    update_items_source_on_unregister: Cell<Option<bool>>,
    selector_enabled: Cell<Option<bool>>,
    document_selector_key_gesture: Cell<Option<KeyGesture>>,
    tool_selector_key_gesture: Cell<Option<KeyGesture>>,
    command_bar_merging_enabled: Cell<Option<bool>>,
    command_bar_merging_scope: Cell<Option<DockCommandBarMergingScope>>,
}
impl PartialEq for DockSettingsOptions {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl DockSettingsOptions {
    /// Creates options with every member unset.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            minimum_horizontal_drag_distance: Cell::new(None),
            minimum_vertical_drag_distance: Cell::new(None),
            use_floating_dock_adorner: Cell::new(None),
            use_pinned_dock_window: Cell::new(None),
            use_managed_windows: Cell::new(None),
            floating_window_host_mode: Cell::new(None),
            use_owner_for_floating_windows: Cell::new(None),
            floating_window_owner_policy: Cell::new(None),
            default_floating_window_owner_mode: Cell::new(None),
            enable_window_magnetism: Cell::new(None),
            window_magnet_distance: Cell::new(None),
            bring_windows_to_front_on_drag: Cell::new(None),
            close_floating_windows_on_main_window_close: Cell::new(None),
            show_dockable_preview_on_drag: Cell::new(None),
            drag_preview_opacity: Cell::new(None),
            update_items_source_on_unregister: Cell::new(None),
            selector_enabled: Cell::new(None),
            document_selector_key_gesture: Cell::new(None),
            tool_selector_key_gesture: Cell::new(None),
            command_bar_merging_enabled: Cell::new(None),
            command_bar_merging_scope: Cell::new(None),
        })
    }
    /// Gets the optional `MinimumHorizontalDragDistance` value.
    pub fn minimum_horizontal_drag_distance(&self) -> Option<f64> {
        self.minimum_horizontal_drag_distance.get()
    }
    /// Sets the optional `MinimumHorizontalDragDistance` value.
    pub fn set_minimum_horizontal_drag_distance(&self, value: Option<f64>) {
        self.minimum_horizontal_drag_distance.set(value);
    }
    /// Gets the optional `MinimumVerticalDragDistance` value.
    pub fn minimum_vertical_drag_distance(&self) -> Option<f64> {
        self.minimum_vertical_drag_distance.get()
    }
    /// Sets the optional `MinimumVerticalDragDistance` value.
    pub fn set_minimum_vertical_drag_distance(&self, value: Option<f64>) {
        self.minimum_vertical_drag_distance.set(value);
    }
    /// Gets the optional `UseFloatingDockAdorner` value.
    pub fn use_floating_dock_adorner(&self) -> Option<bool> {
        self.use_floating_dock_adorner.get()
    }
    /// Sets the optional `UseFloatingDockAdorner` value.
    pub fn set_use_floating_dock_adorner(&self, value: Option<bool>) {
        self.use_floating_dock_adorner.set(value);
    }
    /// Gets the optional `UsePinnedDockWindow` value.
    pub fn use_pinned_dock_window(&self) -> Option<bool> {
        self.use_pinned_dock_window.get()
    }
    /// Sets the optional `UsePinnedDockWindow` value.
    pub fn set_use_pinned_dock_window(&self, value: Option<bool>) {
        self.use_pinned_dock_window.set(value);
    }
    /// Gets the optional `UseManagedWindows` value.
    pub fn use_managed_windows(&self) -> Option<bool> {
        self.use_managed_windows.get()
    }
    /// Sets the optional `UseManagedWindows` value.
    pub fn set_use_managed_windows(&self, value: Option<bool>) {
        self.use_managed_windows.set(value);
    }
    /// Gets the optional `FloatingWindowHostMode` value.
    pub fn floating_window_host_mode(&self) -> Option<DockFloatingWindowHostMode> {
        self.floating_window_host_mode.get()
    }
    /// Sets the optional `FloatingWindowHostMode` value.
    pub fn set_floating_window_host_mode(&self, value: Option<DockFloatingWindowHostMode>) {
        self.floating_window_host_mode.set(value);
    }
    /// Gets the optional `UseOwnerForFloatingWindows` value.
    pub fn use_owner_for_floating_windows(&self) -> Option<bool> {
        self.use_owner_for_floating_windows.get()
    }
    /// Sets the optional `UseOwnerForFloatingWindows` value.
    pub fn set_use_owner_for_floating_windows(&self, value: Option<bool>) {
        self.use_owner_for_floating_windows.set(value);
    }
    /// Gets the optional `FloatingWindowOwnerPolicy` value.
    pub fn floating_window_owner_policy(&self) -> Option<DockFloatingWindowOwnerPolicy> {
        self.floating_window_owner_policy.get()
    }
    /// Sets the optional `FloatingWindowOwnerPolicy` value.
    pub fn set_floating_window_owner_policy(&self, value: Option<DockFloatingWindowOwnerPolicy>) {
        self.floating_window_owner_policy.set(value);
    }
    /// Gets the optional `DefaultFloatingWindowOwnerMode` value.
    pub fn default_floating_window_owner_mode(&self) -> Option<DockWindowOwnerMode> {
        self.default_floating_window_owner_mode.get()
    }
    /// Sets the optional `DefaultFloatingWindowOwnerMode` value.
    pub fn set_default_floating_window_owner_mode(&self, value: Option<DockWindowOwnerMode>) {
        self.default_floating_window_owner_mode.set(value);
    }
    /// Gets the optional `EnableWindowMagnetism` value.
    pub fn enable_window_magnetism(&self) -> Option<bool> {
        self.enable_window_magnetism.get()
    }
    /// Sets the optional `EnableWindowMagnetism` value.
    pub fn set_enable_window_magnetism(&self, value: Option<bool>) {
        self.enable_window_magnetism.set(value);
    }
    /// Gets the optional `WindowMagnetDistance` value.
    pub fn window_magnet_distance(&self) -> Option<f64> {
        self.window_magnet_distance.get()
    }
    /// Sets the optional `WindowMagnetDistance` value.
    pub fn set_window_magnet_distance(&self, value: Option<f64>) {
        self.window_magnet_distance.set(value);
    }
    /// Gets the optional `BringWindowsToFrontOnDrag` value.
    pub fn bring_windows_to_front_on_drag(&self) -> Option<bool> {
        self.bring_windows_to_front_on_drag.get()
    }
    /// Sets the optional `BringWindowsToFrontOnDrag` value.
    pub fn set_bring_windows_to_front_on_drag(&self, value: Option<bool>) {
        self.bring_windows_to_front_on_drag.set(value);
    }
    /// Gets the optional `CloseFloatingWindowsOnMainWindowClose` value.
    pub fn close_floating_windows_on_main_window_close(&self) -> Option<bool> {
        self.close_floating_windows_on_main_window_close.get()
    }
    /// Sets the optional `CloseFloatingWindowsOnMainWindowClose` value.
    pub fn set_close_floating_windows_on_main_window_close(&self, value: Option<bool>) {
        self.close_floating_windows_on_main_window_close.set(value);
    }
    /// Gets the optional `ShowDockablePreviewOnDrag` value.
    pub fn show_dockable_preview_on_drag(&self) -> Option<bool> {
        self.show_dockable_preview_on_drag.get()
    }
    /// Sets the optional `ShowDockablePreviewOnDrag` value.
    pub fn set_show_dockable_preview_on_drag(&self, value: Option<bool>) {
        self.show_dockable_preview_on_drag.set(value);
    }
    /// Gets the optional `DragPreviewOpacity` value.
    pub fn drag_preview_opacity(&self) -> Option<f64> {
        self.drag_preview_opacity.get()
    }
    /// Sets the optional `DragPreviewOpacity` value.
    pub fn set_drag_preview_opacity(&self, value: Option<f64>) {
        self.drag_preview_opacity.set(value);
    }
    /// Gets the optional `UpdateItemsSourceOnUnregister` value.
    pub fn update_items_source_on_unregister(&self) -> Option<bool> {
        self.update_items_source_on_unregister.get()
    }
    /// Sets the optional `UpdateItemsSourceOnUnregister` value.
    pub fn set_update_items_source_on_unregister(&self, value: Option<bool>) {
        self.update_items_source_on_unregister.set(value);
    }
    /// Gets the optional `SelectorEnabled` value.
    pub fn selector_enabled(&self) -> Option<bool> {
        self.selector_enabled.get()
    }
    /// Sets the optional `SelectorEnabled` value.
    pub fn set_selector_enabled(&self, value: Option<bool>) {
        self.selector_enabled.set(value);
    }
    /// Gets the optional `DocumentSelectorKeyGesture` value.
    pub fn document_selector_key_gesture(&self) -> Option<KeyGesture> {
        self.document_selector_key_gesture.get()
    }
    /// Sets the optional `DocumentSelectorKeyGesture` value.
    pub fn set_document_selector_key_gesture(&self, value: Option<KeyGesture>) {
        self.document_selector_key_gesture.set(value);
    }
    /// Gets the optional `ToolSelectorKeyGesture` value.
    pub fn tool_selector_key_gesture(&self) -> Option<KeyGesture> {
        self.tool_selector_key_gesture.get()
    }
    /// Sets the optional `ToolSelectorKeyGesture` value.
    pub fn set_tool_selector_key_gesture(&self, value: Option<KeyGesture>) {
        self.tool_selector_key_gesture.set(value);
    }
    /// Gets the optional `CommandBarMergingEnabled` value.
    pub fn command_bar_merging_enabled(&self) -> Option<bool> {
        self.command_bar_merging_enabled.get()
    }
    /// Sets the optional `CommandBarMergingEnabled` value.
    pub fn set_command_bar_merging_enabled(&self, value: Option<bool>) {
        self.command_bar_merging_enabled.set(value);
    }
    /// Gets the optional `CommandBarMergingScope` value.
    pub fn command_bar_merging_scope(&self) -> Option<DockCommandBarMergingScope> {
        self.command_bar_merging_scope.get()
    }
    /// Sets the optional `CommandBarMergingScope` value.
    pub fn set_command_bar_merging_scope(&self, value: Option<DockCommandBarMergingScope>) {
        self.command_bar_merging_scope.set(value);
    }
}
ferroui_base::ferro_markup_type!(class DockSettingsOptions{this:Rc<DockSettingsOptions>,handles:[Rc<DockSettingsOptions>,Option<Rc<DockSettingsOptions>>],constructors:[()=>DockSettingsOptions::new],properties:[
    MinimumHorizontalDragDistance:Option<f64>{get:|o:&Rc<DockSettingsOptions>|o.minimum_horizontal_drag_distance(),set:|o:&Rc<DockSettingsOptions>,v:Option<f64>|o.set_minimum_horizontal_drag_distance(v)},
    MinimumVerticalDragDistance:Option<f64>{get:|o:&Rc<DockSettingsOptions>|o.minimum_vertical_drag_distance(),set:|o:&Rc<DockSettingsOptions>,v:Option<f64>|o.set_minimum_vertical_drag_distance(v)},
    UseFloatingDockAdorner:Option<bool>{get:|o:&Rc<DockSettingsOptions>|o.use_floating_dock_adorner(),set:|o:&Rc<DockSettingsOptions>,v:Option<bool>|o.set_use_floating_dock_adorner(v)},
    UsePinnedDockWindow:Option<bool>{get:|o:&Rc<DockSettingsOptions>|o.use_pinned_dock_window(),set:|o:&Rc<DockSettingsOptions>,v:Option<bool>|o.set_use_pinned_dock_window(v)},
    UseManagedWindows:Option<bool>{get:|o:&Rc<DockSettingsOptions>|o.use_managed_windows(),set:|o:&Rc<DockSettingsOptions>,v:Option<bool>|o.set_use_managed_windows(v)},
    FloatingWindowHostMode:Option<DockFloatingWindowHostMode>{get:|o:&Rc<DockSettingsOptions>|o.floating_window_host_mode(),set:|o:&Rc<DockSettingsOptions>,v:Option<DockFloatingWindowHostMode>|o.set_floating_window_host_mode(v)},
    UseOwnerForFloatingWindows:Option<bool>{get:|o:&Rc<DockSettingsOptions>|o.use_owner_for_floating_windows(),set:|o:&Rc<DockSettingsOptions>,v:Option<bool>|o.set_use_owner_for_floating_windows(v)},
    FloatingWindowOwnerPolicy:Option<DockFloatingWindowOwnerPolicy>{get:|o:&Rc<DockSettingsOptions>|o.floating_window_owner_policy(),set:|o:&Rc<DockSettingsOptions>,v:Option<DockFloatingWindowOwnerPolicy>|o.set_floating_window_owner_policy(v)},
    DefaultFloatingWindowOwnerMode:Option<DockWindowOwnerMode>{get:|o:&Rc<DockSettingsOptions>|o.default_floating_window_owner_mode(),set:|o:&Rc<DockSettingsOptions>,v:Option<DockWindowOwnerMode>|o.set_default_floating_window_owner_mode(v)},
    EnableWindowMagnetism:Option<bool>{get:|o:&Rc<DockSettingsOptions>|o.enable_window_magnetism(),set:|o:&Rc<DockSettingsOptions>,v:Option<bool>|o.set_enable_window_magnetism(v)},
    WindowMagnetDistance:Option<f64>{get:|o:&Rc<DockSettingsOptions>|o.window_magnet_distance(),set:|o:&Rc<DockSettingsOptions>,v:Option<f64>|o.set_window_magnet_distance(v)},
    BringWindowsToFrontOnDrag:Option<bool>{get:|o:&Rc<DockSettingsOptions>|o.bring_windows_to_front_on_drag(),set:|o:&Rc<DockSettingsOptions>,v:Option<bool>|o.set_bring_windows_to_front_on_drag(v)},
    CloseFloatingWindowsOnMainWindowClose:Option<bool>{get:|o:&Rc<DockSettingsOptions>|o.close_floating_windows_on_main_window_close(),set:|o:&Rc<DockSettingsOptions>,v:Option<bool>|o.set_close_floating_windows_on_main_window_close(v)},
    ShowDockablePreviewOnDrag:Option<bool>{get:|o:&Rc<DockSettingsOptions>|o.show_dockable_preview_on_drag(),set:|o:&Rc<DockSettingsOptions>,v:Option<bool>|o.set_show_dockable_preview_on_drag(v)},
    DragPreviewOpacity:Option<f64>{get:|o:&Rc<DockSettingsOptions>|o.drag_preview_opacity(),set:|o:&Rc<DockSettingsOptions>,v:Option<f64>|o.set_drag_preview_opacity(v)},
    UpdateItemsSourceOnUnregister:Option<bool>{get:|o:&Rc<DockSettingsOptions>|o.update_items_source_on_unregister(),set:|o:&Rc<DockSettingsOptions>,v:Option<bool>|o.set_update_items_source_on_unregister(v)},
    SelectorEnabled:Option<bool>{get:|o:&Rc<DockSettingsOptions>|o.selector_enabled(),set:|o:&Rc<DockSettingsOptions>,v:Option<bool>|o.set_selector_enabled(v)},
    DocumentSelectorKeyGesture:Option<KeyGesture>{get:|o:&Rc<DockSettingsOptions>|o.document_selector_key_gesture(),set:|o:&Rc<DockSettingsOptions>,v:Option<KeyGesture>|o.set_document_selector_key_gesture(v)},
    ToolSelectorKeyGesture:Option<KeyGesture>{get:|o:&Rc<DockSettingsOptions>|o.tool_selector_key_gesture(),set:|o:&Rc<DockSettingsOptions>,v:Option<KeyGesture>|o.set_tool_selector_key_gesture(v)},
    CommandBarMergingEnabled:Option<bool>{get:|o:&Rc<DockSettingsOptions>|o.command_bar_merging_enabled(),set:|o:&Rc<DockSettingsOptions>,v:Option<bool>|o.set_command_bar_merging_enabled(v)},
    CommandBarMergingScope:Option<DockCommandBarMergingScope>{get:|o:&Rc<DockSettingsOptions>|o.command_bar_merging_scope(),set:|o:&Rc<DockSettingsOptions>,v:Option<DockCommandBarMergingScope>|o.set_command_bar_merging_scope(v)},
]});
