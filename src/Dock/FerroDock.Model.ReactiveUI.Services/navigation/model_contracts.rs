// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
// Trait forwarding only: state/engine behavior remains in the real model components.
macro_rules! document_contracts {
    ($type:ty) => {
        impl IMdiDocument for $type {
            fn mdi_bounds(&self) -> DockRect {
                self.base.mdi_bounds()
            }
            fn set_mdi_bounds(&self, value: DockRect) {
                self.base.set_mdi_bounds(value);
            }
            fn mdi_state(&self) -> MdiWindowState {
                self.base.mdi_state()
            }
            fn set_mdi_state(&self, value: MdiWindowState) {
                self.base.set_mdi_state(value);
            }
            fn mdi_z_index(&self) -> i32 {
                self.base.mdi_z_index()
            }
            fn set_mdi_z_index(&self, value: i32) {
                self.base.set_mdi_z_index(value);
            }
        }
        impl IDockingWindowState for $type {
            fn is_open(&self) -> bool {
                self.base.is_open()
            }
            fn set_is_open(&self, value: bool) {
                self.base.set_is_open(value);
            }
            fn is_active(&self) -> bool {
                self.base.is_active()
            }
            fn set_is_active(&self, value: bool) {
                self.base.set_is_active(value);
            }
            fn is_selected(&self) -> bool {
                self.base.is_selected()
            }
            fn set_is_selected(&self, value: bool) {
                self.base.set_is_selected(value);
            }
            fn docking_state(&self) -> DockingWindowState {
                self.base.docking_state()
            }
            fn set_docking_state(&self, value: DockingWindowState) {
                self.base.set_docking_state(value);
            }
        }
    };
}
macro_rules! root_contracts {
    ($type:ty) => {
        impl IRootDock for $type {
            fn is_focusable_root(&self) -> bool {
                self.base.is_focusable_root()
            }
            fn set_is_focusable_root(&self, value: bool) {
                self.base.set_is_focusable_root(value);
            }
            fn hidden_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
                self.base.hidden_dockables()
            }
            fn set_hidden_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
                self.base.set_hidden_dockables(value);
            }
            fn left_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
                self.base.left_pinned_dockables()
            }
            fn set_left_pinned_dockables(
                &self,
                value: Option<ListHandle<Option<Rc<dyn IDockable>>>>,
            ) {
                self.base.set_left_pinned_dockables(value);
            }
            fn right_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
                self.base.right_pinned_dockables()
            }
            fn set_right_pinned_dockables(
                &self,
                value: Option<ListHandle<Option<Rc<dyn IDockable>>>>,
            ) {
                self.base.set_right_pinned_dockables(value);
            }
            fn top_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
                self.base.top_pinned_dockables()
            }
            fn set_top_pinned_dockables(
                &self,
                value: Option<ListHandle<Option<Rc<dyn IDockable>>>>,
            ) {
                self.base.set_top_pinned_dockables(value);
            }
            fn bottom_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
                self.base.bottom_pinned_dockables()
            }
            fn set_bottom_pinned_dockables(
                &self,
                value: Option<ListHandle<Option<Rc<dyn IDockable>>>>,
            ) {
                self.base.set_bottom_pinned_dockables(value);
            }
            fn pinned_dock(&self) -> Option<Rc<dyn IToolDock>> {
                self.base.pinned_dock()
            }
            fn set_pinned_dock(&self, value: Option<Rc<dyn IToolDock>>) {
                self.base.set_pinned_dock(value);
            }
            fn pinned_dock_display_mode(&self) -> PinnedDockDisplayMode {
                self.base.pinned_dock_display_mode()
            }
            fn set_pinned_dock_display_mode(&self, value: PinnedDockDisplayMode) {
                self.base.set_pinned_dock_display_mode(value);
            }
            fn window(&self) -> Option<Rc<dyn IDockWindow>> {
                self.base.window()
            }
            fn set_window(&self, value: Option<Rc<dyn IDockWindow>>) {
                self.base.set_window(value);
            }
            fn windows(&self) -> Option<ListHandle<Option<Rc<dyn IDockWindow>>>> {
                self.base.windows()
            }
            fn set_windows(&self, value: Option<ListHandle<Option<Rc<dyn IDockWindow>>>>) {
                self.base.set_windows(value);
            }
            fn floating_window_host_mode(&self) -> DockFloatingWindowHostMode {
                self.base.floating_window_host_mode()
            }
            fn set_floating_window_host_mode(&self, value: DockFloatingWindowHostMode) {
                self.base.set_floating_window_host_mode(value);
            }
            fn root_dock_capability_policy(&self) -> Option<Rc<dyn DockCapabilityPolicyImpl>> {
                self.base.root_dock_capability_policy()
            }
            fn set_root_dock_capability_policy(
                &self,
                value: Option<Rc<dyn DockCapabilityPolicyImpl>>,
            ) {
                self.base.set_root_dock_capability_policy(value);
            }
            fn enable_adaptive_global_dock_targets(&self) -> bool {
                self.base.enable_adaptive_global_dock_targets()
            }
            fn set_enable_adaptive_global_dock_targets(&self, value: bool) {
                self.base.set_enable_adaptive_global_dock_targets(value);
            }
            fn show_windows(&self) -> Rc<dyn ICommand> {
                self.base.show_windows()
            }
            fn exit_windows(&self) -> Rc<dyn ICommand> {
                self.base.exit_windows()
            }
        }
    };
}

pub(crate) use root_contracts;
