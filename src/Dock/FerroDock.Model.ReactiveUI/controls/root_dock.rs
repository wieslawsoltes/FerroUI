// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::{DockBase, DockableBaseImpl};
use crate::impl_dock;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::BoxedValue;
use ferroui_reactiveui::{reactive_object, IReactiveObjectExt};
use ferroui_reactiveui::{ICommand, ReactiveCommand};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

type DockableList = ListHandle<Option<Rc<dyn IDockable>>>;
type WindowList = ListHandle<Option<Rc<dyn IDockWindow>>>;

/// The reactive `RootDock` container with the source properties and defaults.
pub struct RootDock {
    base: DockBase,
    is_focusable_root: RefCell<bool>,
    hidden_dockables: RefCell<Option<DockableList>>,
    left_pinned_dockables: RefCell<Option<DockableList>>,
    right_pinned_dockables: RefCell<Option<DockableList>>,
    top_pinned_dockables: RefCell<Option<DockableList>>,
    bottom_pinned_dockables: RefCell<Option<DockableList>>,
    pinned_dock: RefCell<Option<Rc<dyn IToolDock>>>,
    pinned_dock_display_mode: RefCell<PinnedDockDisplayMode>,
    window: RefCell<Option<Rc<dyn IDockWindow>>>,
    windows: RefCell<Option<WindowList>>,
    floating_window_host_mode: RefCell<DockFloatingWindowHostMode>,
    root_dock_capability_policy: RefCell<Option<Rc<dyn DockCapabilityPolicyImpl>>>,
    enable_adaptive_global_dock_targets: RefCell<bool>,
    show_windows: Rc<dyn ICommand>,
    exit_windows: Rc<dyn ICommand>,
}

reactive_object!(RootDock, base);
impl DockableBaseImpl for RootDock {}
impl_dock!(
    RootDock,
    fn as_root_dock(self: Rc<Self>) -> Option<Rc<dyn IRootDock>> {
        Some(self)
    }
);
impl RootDock {
    /// Creates a shared container and attaches its source reactive observations.
    pub fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(Self::construct);
        model.base.initialize(&model);
        model
    }
    /// Constructs the component inside a model with the complete `IRootDock` contract.
    /// The enclosing model must initialize DockBase after Rc::new_cyclic returns.
    pub fn construct<T: DockableBaseImpl + IRootDock + 'static>(runtime: &Weak<T>) -> Self {
        let base = DockBase::construct(runtime.clone());
        let weak = Rc::downgrade(base.navigation_adapter());
        let show_windows = Rc::new(ReactiveCommand::create(move || {
            if let Some(adapter) = weak.upgrade() {
                adapter.show_windows();
            }
        }));
        let weak = Rc::downgrade(base.navigation_adapter());
        let exit_windows = Rc::new(ReactiveCommand::create(move || {
            if let Some(adapter) = weak.upgrade() {
                adapter.exit_windows();
            }
        }));
        Self {
            base,
            is_focusable_root: RefCell::new(true),
            hidden_dockables: RefCell::new(None),
            left_pinned_dockables: RefCell::new(None),
            right_pinned_dockables: RefCell::new(None),
            top_pinned_dockables: RefCell::new(None),
            bottom_pinned_dockables: RefCell::new(None),
            pinned_dock: RefCell::new(None),
            pinned_dock_display_mode: RefCell::new(PinnedDockDisplayMode::Overlay),
            window: RefCell::new(None),
            windows: RefCell::new(None),
            floating_window_host_mode: RefCell::new(DockFloatingWindowHostMode::Default),
            root_dock_capability_policy: RefCell::new(None),
            enable_adaptive_global_dock_targets: RefCell::new(false),
            show_windows,
            exit_windows,
        }
    }
    /// Gets `IsFocusableRoot`.
    pub fn is_focusable_root(&self) -> bool {
        *self.is_focusable_root.borrow()
    }
    /// Sets `IsFocusableRoot`, notifying only when changed.
    pub fn set_is_focusable_root(&self, value: bool) {
        self.base.runtime().raise_and_set_if_changed(
            &self.is_focusable_root,
            value,
            "IsFocusableRoot",
        );
    }
    /// Gets `HiddenDockables`.
    pub fn hidden_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
        self.hidden_dockables.borrow().clone()
    }
    /// Sets `HiddenDockables`, notifying only when changed.
    pub fn set_hidden_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
        self.base.runtime().raise_and_set_if_changed(
            &self.hidden_dockables,
            value,
            "HiddenDockables",
        );
    }
    /// Gets `LeftPinnedDockables`.
    pub fn left_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
        self.left_pinned_dockables.borrow().clone()
    }
    /// Sets `LeftPinnedDockables`, notifying only when changed.
    pub fn set_left_pinned_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
        self.base.runtime().raise_and_set_if_changed(
            &self.left_pinned_dockables,
            value,
            "LeftPinnedDockables",
        );
    }
    /// Gets `RightPinnedDockables`.
    pub fn right_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
        self.right_pinned_dockables.borrow().clone()
    }
    /// Sets `RightPinnedDockables`, notifying only when changed.
    pub fn set_right_pinned_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
        self.base.runtime().raise_and_set_if_changed(
            &self.right_pinned_dockables,
            value,
            "RightPinnedDockables",
        );
    }
    /// Gets `TopPinnedDockables`.
    pub fn top_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
        self.top_pinned_dockables.borrow().clone()
    }
    /// Sets `TopPinnedDockables`, notifying only when changed.
    pub fn set_top_pinned_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
        self.base.runtime().raise_and_set_if_changed(
            &self.top_pinned_dockables,
            value,
            "TopPinnedDockables",
        );
    }
    /// Gets `BottomPinnedDockables`.
    pub fn bottom_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
        self.bottom_pinned_dockables.borrow().clone()
    }
    /// Sets `BottomPinnedDockables`, notifying only when changed.
    pub fn set_bottom_pinned_dockables(
        &self,
        value: Option<ListHandle<Option<Rc<dyn IDockable>>>>,
    ) {
        self.base.runtime().raise_and_set_if_changed(
            &self.bottom_pinned_dockables,
            value,
            "BottomPinnedDockables",
        );
    }
    /// Gets `PinnedDock`.
    pub fn pinned_dock(&self) -> Option<Rc<dyn IToolDock>> {
        self.pinned_dock.borrow().clone()
    }
    /// Sets `PinnedDock`, notifying only when changed.
    pub fn set_pinned_dock(&self, value: Option<Rc<dyn IToolDock>>) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.pinned_dock, value, "PinnedDock");
    }
    /// Gets `PinnedDockDisplayMode`.
    pub fn pinned_dock_display_mode(&self) -> PinnedDockDisplayMode {
        *self.pinned_dock_display_mode.borrow()
    }
    /// Sets `PinnedDockDisplayMode`, notifying only when changed.
    pub fn set_pinned_dock_display_mode(&self, value: PinnedDockDisplayMode) {
        self.base.runtime().raise_and_set_if_changed(
            &self.pinned_dock_display_mode,
            value,
            "PinnedDockDisplayMode",
        );
    }
    /// Gets `Window`.
    pub fn window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.window.borrow().clone()
    }
    /// Sets `Window`, notifying only when changed.
    pub fn set_window(&self, value: Option<Rc<dyn IDockWindow>>) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.window, value, "Window");
    }
    /// Gets `Windows`.
    pub fn windows(&self) -> Option<ListHandle<Option<Rc<dyn IDockWindow>>>> {
        self.windows.borrow().clone()
    }
    /// Sets `Windows`, notifying only when changed.
    pub fn set_windows(&self, value: Option<ListHandle<Option<Rc<dyn IDockWindow>>>>) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.windows, value, "Windows");
    }
    /// Gets `FloatingWindowHostMode`.
    pub fn floating_window_host_mode(&self) -> DockFloatingWindowHostMode {
        *self.floating_window_host_mode.borrow()
    }
    /// Sets `FloatingWindowHostMode`, notifying only when changed.
    pub fn set_floating_window_host_mode(&self, value: DockFloatingWindowHostMode) {
        self.base.runtime().raise_and_set_if_changed(
            &self.floating_window_host_mode,
            value,
            "FloatingWindowHostMode",
        );
    }
    /// Gets `RootDockCapabilityPolicy`.
    pub fn root_dock_capability_policy(&self) -> Option<Rc<dyn DockCapabilityPolicyImpl>> {
        self.root_dock_capability_policy.borrow().clone()
    }
    /// Sets `RootDockCapabilityPolicy`, notifying only when changed.
    pub fn set_root_dock_capability_policy(&self, value: Option<Rc<dyn DockCapabilityPolicyImpl>>) {
        self.base.runtime().raise_and_set_if_changed(
            &self.root_dock_capability_policy,
            value,
            "RootDockCapabilityPolicy",
        );
    }
    /// Gets `EnableAdaptiveGlobalDockTargets`.
    pub fn enable_adaptive_global_dock_targets(&self) -> bool {
        *self.enable_adaptive_global_dock_targets.borrow()
    }
    /// Sets `EnableAdaptiveGlobalDockTargets`, notifying only when changed.
    pub fn set_enable_adaptive_global_dock_targets(&self, value: bool) {
        self.base.runtime().raise_and_set_if_changed(
            &self.enable_adaptive_global_dock_targets,
            value,
            "EnableAdaptiveGlobalDockTargets",
        );
    }
    /// Gets the `ShowWindows` ReactiveCommand.
    pub fn show_windows(&self) -> Rc<dyn ICommand> {
        self.show_windows.clone()
    }
    /// Gets the `ExitWindows` ReactiveCommand.
    pub fn exit_windows(&self) -> Rc<dyn ICommand> {
        self.exit_windows.clone()
    }
    /// Boxes the container using the canonical erased dockable representation.
    pub fn boxed(self: &Rc<Self>) -> BoxedValue {
        Rc::new(self.clone() as Rc<dyn IDockable>)
    }
}
impl IRootDock for RootDock {
    fn is_focusable_root(&self) -> bool {
        self.is_focusable_root()
    }
    fn set_is_focusable_root(&self, value: bool) {
        self.set_is_focusable_root(value);
    }
    fn hidden_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
        self.hidden_dockables()
    }
    fn set_hidden_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
        self.set_hidden_dockables(value);
    }
    fn left_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
        self.left_pinned_dockables()
    }
    fn set_left_pinned_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
        self.set_left_pinned_dockables(value);
    }
    fn right_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
        self.right_pinned_dockables()
    }
    fn set_right_pinned_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
        self.set_right_pinned_dockables(value);
    }
    fn top_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
        self.top_pinned_dockables()
    }
    fn set_top_pinned_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
        self.set_top_pinned_dockables(value);
    }
    fn bottom_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
        self.bottom_pinned_dockables()
    }
    fn set_bottom_pinned_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
        self.set_bottom_pinned_dockables(value);
    }
    fn pinned_dock(&self) -> Option<Rc<dyn IToolDock>> {
        self.pinned_dock()
    }
    fn set_pinned_dock(&self, value: Option<Rc<dyn IToolDock>>) {
        self.set_pinned_dock(value);
    }
    fn pinned_dock_display_mode(&self) -> PinnedDockDisplayMode {
        self.pinned_dock_display_mode()
    }
    fn set_pinned_dock_display_mode(&self, value: PinnedDockDisplayMode) {
        self.set_pinned_dock_display_mode(value);
    }
    fn window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.window()
    }
    fn set_window(&self, value: Option<Rc<dyn IDockWindow>>) {
        self.set_window(value);
    }
    fn windows(&self) -> Option<ListHandle<Option<Rc<dyn IDockWindow>>>> {
        self.windows()
    }
    fn set_windows(&self, value: Option<ListHandle<Option<Rc<dyn IDockWindow>>>>) {
        self.set_windows(value);
    }
    fn floating_window_host_mode(&self) -> DockFloatingWindowHostMode {
        self.floating_window_host_mode()
    }
    fn set_floating_window_host_mode(&self, value: DockFloatingWindowHostMode) {
        self.set_floating_window_host_mode(value);
    }
    fn root_dock_capability_policy(&self) -> Option<Rc<dyn DockCapabilityPolicyImpl>> {
        self.root_dock_capability_policy()
    }
    fn set_root_dock_capability_policy(&self, value: Option<Rc<dyn DockCapabilityPolicyImpl>>) {
        self.set_root_dock_capability_policy(value);
    }
    fn enable_adaptive_global_dock_targets(&self) -> bool {
        self.enable_adaptive_global_dock_targets()
    }
    fn set_enable_adaptive_global_dock_targets(&self, value: bool) {
        self.set_enable_adaptive_global_dock_targets(value);
    }
    fn show_windows(&self) -> Rc<dyn ICommand> {
        self.show_windows()
    }
    fn exit_windows(&self) -> Rc<dyn ICommand> {
        self.exit_windows()
    }
}
