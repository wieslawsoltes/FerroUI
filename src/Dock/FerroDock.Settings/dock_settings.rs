// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{
    DockCommandBarMergingScope, DockFloatingWindowOwnerPolicy, DockGlobalDockingPreset,
    WindowDragDockScope,
};
use ferrodock_model::{
    controls::IRootDock,
    core::{DockFloatingWindowHostMode, DockWindowOwnerMode},
};
use ferroui_base::input::{Key, KeyGesture, KeyModifiers};
use ferroui_base::{PixelPoint, Vector};
use std::sync::{Arc, OnceLock, PoisonError, RwLock};
/// A process-wide diagnostics callback, invoked after releasing settings locks.
pub type DiagnosticsLogHandler = Arc<dyn Fn(String) + Send + Sync>;
#[derive(Clone)]
struct Values {
    minimum_horizontal_drag_distance: f64,
    minimum_vertical_drag_distance: f64,
    use_floating_dock_adorner: bool,
    use_pinned_dock_window: bool,
    use_managed_windows: bool,
    floating_window_host_mode: DockFloatingWindowHostMode,
    use_owner_for_floating_windows: bool,
    floating_window_owner_policy: DockFloatingWindowOwnerPolicy,
    default_floating_window_owner_mode: DockWindowOwnerMode,
    enable_window_magnetism: bool,
    window_magnet_distance: f64,
    bring_windows_to_front_on_drag: bool,
    document_window_drag_scope: WindowDragDockScope,
    close_floating_windows_on_main_window_close: bool,
    show_dockable_preview_on_drag: bool,
    drag_preview_opacity: f64,
    update_items_source_on_unregister: bool,
    global_docking_proportion: f64,
    global_docking_preset: DockGlobalDockingPreset,
    enable_diagnostics_logging: bool,
    diagnostics_log_handler: Option<DiagnosticsLogHandler>,
    selector_enabled: bool,
    document_selector_key_gesture: KeyGesture,
    tool_selector_key_gesture: KeyGesture,
    command_bar_merging_enabled: bool,
    command_bar_merging_scope: DockCommandBarMergingScope,
}
impl Default for Values {
    fn default() -> Self {
        Self {
            minimum_horizontal_drag_distance: 4.0,
            minimum_vertical_drag_distance: 4.0,
            use_floating_dock_adorner: false,
            use_pinned_dock_window: false,
            use_managed_windows: false,
            floating_window_host_mode: DockFloatingWindowHostMode::Default,
            use_owner_for_floating_windows: true,
            floating_window_owner_policy: DockFloatingWindowOwnerPolicy::Default,
            default_floating_window_owner_mode: DockWindowOwnerMode::Default,
            enable_window_magnetism: false,
            window_magnet_distance: 16.0,
            bring_windows_to_front_on_drag: true,
            document_window_drag_scope: WindowDragDockScope::WindowChrome,
            close_floating_windows_on_main_window_close: false,
            show_dockable_preview_on_drag: false,
            drag_preview_opacity: 1.0,
            update_items_source_on_unregister: true,
            global_docking_proportion: 0.5,
            global_docking_preset: DockGlobalDockingPreset::GlobalFirst,
            enable_diagnostics_logging: false,
            diagnostics_log_handler: None,
            selector_enabled: true,
            document_selector_key_gesture: KeyGesture::new(Key::Tab, KeyModifiers::CONTROL),
            tool_selector_key_gesture: KeyGesture::new(
                Key::Tab,
                KeyModifiers::CONTROL | KeyModifiers::ALT,
            ),
            command_bar_merging_enabled: false,
            command_bar_merging_scope: DockCommandBarMergingScope::ActiveDocument,
        }
    }
}
static VALUES: OnceLock<RwLock<Values>> = OnceLock::new();
fn values() -> &'static RwLock<Values> {
    VALUES.get_or_init(|| RwLock::new(Values::default()))
}
/// Original process-wide docking settings and resolution helpers.
pub struct DockSettings;
impl DockSettings {
    /// Gets the original `MinimumHorizontalDragDistance` setting.
    pub fn minimum_horizontal_drag_distance() -> f64 {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .minimum_horizontal_drag_distance
    }
    /// Sets `MinimumHorizontalDragDistance` without adding validation or coercion.
    pub fn set_minimum_horizontal_drag_distance(value: f64) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .minimum_horizontal_drag_distance = value;
    }
    /// Gets the original `MinimumVerticalDragDistance` setting.
    pub fn minimum_vertical_drag_distance() -> f64 {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .minimum_vertical_drag_distance
    }
    /// Sets `MinimumVerticalDragDistance` without adding validation or coercion.
    pub fn set_minimum_vertical_drag_distance(value: f64) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .minimum_vertical_drag_distance = value;
    }
    /// Gets the original `UseFloatingDockAdorner` setting.
    pub fn use_floating_dock_adorner() -> bool {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .use_floating_dock_adorner
    }
    /// Sets `UseFloatingDockAdorner` without adding validation or coercion.
    pub fn set_use_floating_dock_adorner(value: bool) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .use_floating_dock_adorner = value;
    }
    /// Gets the original `UsePinnedDockWindow` setting.
    pub fn use_pinned_dock_window() -> bool {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .use_pinned_dock_window
    }
    /// Sets `UsePinnedDockWindow` without adding validation or coercion.
    pub fn set_use_pinned_dock_window(value: bool) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .use_pinned_dock_window = value;
    }
    /// Gets the original `UseManagedWindows` setting.
    pub fn use_managed_windows() -> bool {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .use_managed_windows
    }
    /// Sets `UseManagedWindows` without adding validation or coercion.
    pub fn set_use_managed_windows(value: bool) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .use_managed_windows = value;
    }
    /// Gets the original `FloatingWindowHostMode` setting.
    pub fn floating_window_host_mode() -> DockFloatingWindowHostMode {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .floating_window_host_mode
    }
    /// Sets `FloatingWindowHostMode` without adding validation or coercion.
    pub fn set_floating_window_host_mode(value: DockFloatingWindowHostMode) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .floating_window_host_mode = value;
    }
    /// Gets the original `UseOwnerForFloatingWindows` setting.
    pub fn use_owner_for_floating_windows() -> bool {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .use_owner_for_floating_windows
    }
    /// Sets `UseOwnerForFloatingWindows` without adding validation or coercion.
    pub fn set_use_owner_for_floating_windows(value: bool) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .use_owner_for_floating_windows = value;
    }
    /// Gets the original `FloatingWindowOwnerPolicy` setting.
    pub fn floating_window_owner_policy() -> DockFloatingWindowOwnerPolicy {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .floating_window_owner_policy
    }
    /// Sets `FloatingWindowOwnerPolicy` without adding validation or coercion.
    pub fn set_floating_window_owner_policy(value: DockFloatingWindowOwnerPolicy) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .floating_window_owner_policy = value;
    }
    /// Gets the original `DefaultFloatingWindowOwnerMode` setting.
    pub fn default_floating_window_owner_mode() -> DockWindowOwnerMode {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .default_floating_window_owner_mode
    }
    /// Sets `DefaultFloatingWindowOwnerMode` without adding validation or coercion.
    pub fn set_default_floating_window_owner_mode(value: DockWindowOwnerMode) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .default_floating_window_owner_mode = value;
    }
    /// Gets the original `EnableWindowMagnetism` setting.
    pub fn enable_window_magnetism() -> bool {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .enable_window_magnetism
    }
    /// Sets `EnableWindowMagnetism` without adding validation or coercion.
    pub fn set_enable_window_magnetism(value: bool) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .enable_window_magnetism = value;
    }
    /// Gets the original `WindowMagnetDistance` setting.
    pub fn window_magnet_distance() -> f64 {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .window_magnet_distance
    }
    /// Sets `WindowMagnetDistance` without adding validation or coercion.
    pub fn set_window_magnet_distance(value: f64) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .window_magnet_distance = value;
    }
    /// Gets the original `BringWindowsToFrontOnDrag` setting.
    pub fn bring_windows_to_front_on_drag() -> bool {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .bring_windows_to_front_on_drag
    }
    /// Sets `BringWindowsToFrontOnDrag` without adding validation or coercion.
    pub fn set_bring_windows_to_front_on_drag(value: bool) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .bring_windows_to_front_on_drag = value;
    }
    /// Gets the original `DocumentWindowDragScope` setting.
    pub fn document_window_drag_scope() -> WindowDragDockScope {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .document_window_drag_scope
    }
    /// Sets `DocumentWindowDragScope` without adding validation or coercion.
    pub fn set_document_window_drag_scope(value: WindowDragDockScope) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .document_window_drag_scope = value;
    }
    /// Gets the original `CloseFloatingWindowsOnMainWindowClose` setting.
    pub fn close_floating_windows_on_main_window_close() -> bool {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .close_floating_windows_on_main_window_close
    }
    /// Sets `CloseFloatingWindowsOnMainWindowClose` without adding validation or coercion.
    pub fn set_close_floating_windows_on_main_window_close(value: bool) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .close_floating_windows_on_main_window_close = value;
    }
    /// Gets the original `ShowDockablePreviewOnDrag` setting.
    pub fn show_dockable_preview_on_drag() -> bool {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .show_dockable_preview_on_drag
    }
    /// Sets `ShowDockablePreviewOnDrag` without adding validation or coercion.
    pub fn set_show_dockable_preview_on_drag(value: bool) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .show_dockable_preview_on_drag = value;
    }
    /// Gets the original `DragPreviewOpacity` setting.
    pub fn drag_preview_opacity() -> f64 {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .drag_preview_opacity
    }
    /// Sets `DragPreviewOpacity` without adding validation or coercion.
    pub fn set_drag_preview_opacity(value: f64) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .drag_preview_opacity = value;
    }
    /// Gets the original `UpdateItemsSourceOnUnregister` setting.
    pub fn update_items_source_on_unregister() -> bool {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .update_items_source_on_unregister
    }
    /// Sets `UpdateItemsSourceOnUnregister` without adding validation or coercion.
    pub fn set_update_items_source_on_unregister(value: bool) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .update_items_source_on_unregister = value;
    }
    /// Gets the original `GlobalDockingProportion` setting.
    pub fn global_docking_proportion() -> f64 {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .global_docking_proportion
    }
    /// Sets `GlobalDockingProportion` without adding validation or coercion.
    pub fn set_global_docking_proportion(value: f64) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .global_docking_proportion = value;
    }
    /// Gets the original `GlobalDockingPreset` setting.
    pub fn global_docking_preset() -> DockGlobalDockingPreset {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .global_docking_preset
    }
    /// Sets `GlobalDockingPreset` without adding validation or coercion.
    pub fn set_global_docking_preset(value: DockGlobalDockingPreset) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .global_docking_preset = value;
    }
    /// Gets the original `EnableDiagnosticsLogging` setting.
    pub fn enable_diagnostics_logging() -> bool {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .enable_diagnostics_logging
    }
    /// Sets `EnableDiagnosticsLogging` without adding validation or coercion.
    pub fn set_enable_diagnostics_logging(value: bool) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .enable_diagnostics_logging = value;
    }
    /// Gets the original `DiagnosticsLogHandler` setting.
    pub fn diagnostics_log_handler() -> Option<DiagnosticsLogHandler> {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .diagnostics_log_handler
            .clone()
    }
    /// Sets `DiagnosticsLogHandler` without adding validation or coercion.
    pub fn set_diagnostics_log_handler(value: Option<DiagnosticsLogHandler>) {
        let previous = {
            let mut values = values().write().unwrap_or_else(PoisonError::into_inner);
            std::mem::replace(&mut values.diagnostics_log_handler, value)
        };
        drop(previous);
    }
    /// Gets the original `SelectorEnabled` setting.
    pub fn selector_enabled() -> bool {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .selector_enabled
    }
    /// Sets `SelectorEnabled` without adding validation or coercion.
    pub fn set_selector_enabled(value: bool) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .selector_enabled = value;
    }
    /// Gets the original `DocumentSelectorKeyGesture` setting.
    pub fn document_selector_key_gesture() -> KeyGesture {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .document_selector_key_gesture
    }
    /// Sets `DocumentSelectorKeyGesture` without adding validation or coercion.
    pub fn set_document_selector_key_gesture(value: KeyGesture) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .document_selector_key_gesture = value;
    }
    /// Gets the original `ToolSelectorKeyGesture` setting.
    pub fn tool_selector_key_gesture() -> KeyGesture {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .tool_selector_key_gesture
    }
    /// Sets `ToolSelectorKeyGesture` without adding validation or coercion.
    pub fn set_tool_selector_key_gesture(value: KeyGesture) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .tool_selector_key_gesture = value;
    }
    /// Gets the original `CommandBarMergingEnabled` setting.
    pub fn command_bar_merging_enabled() -> bool {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .command_bar_merging_enabled
    }
    /// Sets `CommandBarMergingEnabled` without adding validation or coercion.
    pub fn set_command_bar_merging_enabled(value: bool) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .command_bar_merging_enabled = value;
    }
    /// Gets the original `CommandBarMergingScope` setting.
    pub fn command_bar_merging_scope() -> DockCommandBarMergingScope {
        values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .command_bar_merging_scope
    }
    /// Sets `CommandBarMergingScope` without adding validation or coercion.
    pub fn set_command_bar_merging_scope(value: DockCommandBarMergingScope) {
        values()
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .command_bar_merging_scope = value;
    }
    /// Tests the floating-point drag delta using the original strict OR threshold.
    pub fn is_minimum_drag_distance(diff: Vector) -> bool {
        diff.x.abs() > Self::minimum_horizontal_drag_distance()
            || diff.y.abs() > Self::minimum_vertical_drag_distance()
    }
    /// Tests the pixel drag delta; minimum integer absolute-value overflow is preserved.
    pub fn is_minimum_drag_distance_pixels(diff: PixelPoint) -> bool {
        f64::from(
            diff.x
                .checked_abs()
                .expect("pixel drag absolute-value overflow"),
        ) > Self::minimum_horizontal_drag_distance()
            || f64::from(
                diff.y
                    .checked_abs()
                    .expect("pixel drag absolute-value overflow"),
            ) > Self::minimum_vertical_drag_distance()
    }
    /// Resolves root override, explicit global mode, then the managed-window flag.
    pub fn resolve_floating_window_host_mode(
        root: Option<&dyn IRootDock>,
    ) -> DockFloatingWindowHostMode {
        if let Some(root) = root {
            let mode = root.floating_window_host_mode();
            if mode != DockFloatingWindowHostMode::Default {
                return mode;
            }
        }
        if Self::floating_window_host_mode() != DockFloatingWindowHostMode::Default {
            return Self::floating_window_host_mode();
        }
        if Self::use_managed_windows() {
            DockFloatingWindowHostMode::Managed
        } else {
            DockFloatingWindowHostMode::Native
        }
    }
    /// Whether the effective root/global host mode is managed.
    pub fn is_managed_window_hosting_enabled(root: Option<&dyn IRootDock>) -> bool {
        Self::resolve_floating_window_host_mode(root) == DockFloatingWindowHostMode::Managed
    }
    /// Resolves the global owner policy before the original owner flag.
    pub fn should_use_owner_for_floating_windows() -> bool {
        match Self::floating_window_owner_policy() {
            DockFloatingWindowOwnerPolicy::AlwaysOwned => true,
            DockFloatingWindowOwnerPolicy::NeverOwned => false,
            _ => Self::use_owner_for_floating_windows(),
        }
    }
}
ferroui_base::ferro_markup_type!(static DockSettings {
fields:[
    MinimumHorizontalDragDistance:f64=>DockSettings::minimum_horizontal_drag_distance,
    MinimumVerticalDragDistance:f64=>DockSettings::minimum_vertical_drag_distance,
    UseFloatingDockAdorner:bool=>DockSettings::use_floating_dock_adorner,
    UsePinnedDockWindow:bool=>DockSettings::use_pinned_dock_window,
    UseManagedWindows:bool=>DockSettings::use_managed_windows,
    FloatingWindowHostMode:DockFloatingWindowHostMode=>DockSettings::floating_window_host_mode,
    UseOwnerForFloatingWindows:bool=>DockSettings::use_owner_for_floating_windows,
    FloatingWindowOwnerPolicy:DockFloatingWindowOwnerPolicy=>DockSettings::floating_window_owner_policy,
    DefaultFloatingWindowOwnerMode:DockWindowOwnerMode=>DockSettings::default_floating_window_owner_mode,
    EnableWindowMagnetism:bool=>DockSettings::enable_window_magnetism,
    WindowMagnetDistance:f64=>DockSettings::window_magnet_distance,
    BringWindowsToFrontOnDrag:bool=>DockSettings::bring_windows_to_front_on_drag,
    DocumentWindowDragScope:WindowDragDockScope=>DockSettings::document_window_drag_scope,
    CloseFloatingWindowsOnMainWindowClose:bool=>DockSettings::close_floating_windows_on_main_window_close,
    ShowDockablePreviewOnDrag:bool=>DockSettings::show_dockable_preview_on_drag,
    DragPreviewOpacity:f64=>DockSettings::drag_preview_opacity,
    UpdateItemsSourceOnUnregister:bool=>DockSettings::update_items_source_on_unregister,
    GlobalDockingProportion:f64=>DockSettings::global_docking_proportion,
    GlobalDockingPreset:DockGlobalDockingPreset=>DockSettings::global_docking_preset,
    EnableDiagnosticsLogging:bool=>DockSettings::enable_diagnostics_logging,
],static_properties:[
    SelectorEnabled:bool{get:DockSettings::selector_enabled,set:DockSettings::set_selector_enabled},
    DocumentSelectorKeyGesture:KeyGesture{get:DockSettings::document_selector_key_gesture,set:DockSettings::set_document_selector_key_gesture},
    ToolSelectorKeyGesture:KeyGesture{get:DockSettings::tool_selector_key_gesture,set:DockSettings::set_tool_selector_key_gesture},
    CommandBarMergingEnabled:bool{get:DockSettings::command_bar_merging_enabled,set:DockSettings::set_command_bar_merging_enabled},
    CommandBarMergingScope:DockCommandBarMergingScope{get:DockSettings::command_bar_merging_scope,set:DockSettings::set_command_bar_merging_scope},
]});
#[cfg(test)]
pub(crate) struct Scope {
    saved: Values,
    _guard: std::sync::MutexGuard<'static, ()>,
}
#[cfg(test)]
impl Scope {
    pub(crate) fn new() -> Self {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let guard = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let saved = values()
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        *values().write().unwrap_or_else(PoisonError::into_inner) = Values::default();
        Self {
            saved,
            _guard: guard,
        }
    }
}
#[cfg(test)]
impl Drop for Scope {
    fn drop(&mut self) {
        let previous = {
            let mut current = values().write().unwrap_or_else(PoisonError::into_inner);
            std::mem::replace(&mut *current, self.saved.clone())
        };
        drop(previous);
    }
}
