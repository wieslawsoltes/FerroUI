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

/// The reactive `SplitViewDock` container with the source properties and defaults.
pub struct SplitViewDock {
    base: DockBase,
    compact_pane_length: RefCell<f64>,
    display_mode: RefCell<SplitViewDisplayMode>,
    is_pane_open: RefCell<bool>,
    open_pane_length: RefCell<f64>,
    pane_placement: RefCell<SplitViewPanePlacement>,
    use_light_dismiss_overlay_mode: RefCell<bool>,
    pane_dockable: RefCell<Option<Rc<dyn IDockable>>>,
    content_dockable: RefCell<Option<Rc<dyn IDockable>>>,
}

reactive_object!(SplitViewDock, base);
impl DockableBaseImpl for SplitViewDock {}
impl_dock!(
    SplitViewDock,
    fn as_split_view_dock(self: Rc<Self>) -> Option<Rc<dyn ISplitViewDock>> {
        Some(self)
    }
);
impl SplitViewDock {
    /// Creates a shared container and attaches its source reactive observations.
    pub fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(Self::construct);
        model.base.initialize(&model);
        model
    }
    /// Constructs the component inside a model with the complete `ISplitViewDock` contract.
    /// The enclosing model must initialize DockBase after Rc::new_cyclic returns.
    pub fn construct<T: DockableBaseImpl + ISplitViewDock + 'static>(runtime: &Weak<T>) -> Self {
        let base = DockBase::construct(runtime.clone());
        Self {
            base,
            compact_pane_length: RefCell::new(48.0),
            display_mode: RefCell::new(SplitViewDisplayMode::Overlay),
            is_pane_open: RefCell::new(false),
            open_pane_length: RefCell::new(320.0),
            pane_placement: RefCell::new(SplitViewPanePlacement::Left),
            use_light_dismiss_overlay_mode: RefCell::new(false),
            pane_dockable: RefCell::new(None),
            content_dockable: RefCell::new(None),
        }
    }
    /// Gets `CompactPaneLength`.
    pub fn compact_pane_length(&self) -> f64 {
        *self.compact_pane_length.borrow()
    }
    /// Sets `CompactPaneLength`, notifying only when changed.
    pub fn set_compact_pane_length(&self, value: f64) {
        let current = *self.compact_pane_length.borrow();
        if current == value || (current.is_nan() && value.is_nan()) {
            return;
        }
        self.base.runtime().raise_and_set_if_changed(
            &self.compact_pane_length,
            value,
            "CompactPaneLength",
        );
    }
    /// Gets `DisplayMode`.
    pub fn display_mode(&self) -> SplitViewDisplayMode {
        *self.display_mode.borrow()
    }
    /// Sets `DisplayMode`, notifying only when changed.
    pub fn set_display_mode(&self, value: SplitViewDisplayMode) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.display_mode, value, "DisplayMode");
    }
    /// Gets `IsPaneOpen`.
    pub fn is_pane_open(&self) -> bool {
        *self.is_pane_open.borrow()
    }
    /// Sets `IsPaneOpen`, notifying only when changed.
    pub fn set_is_pane_open(&self, value: bool) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.is_pane_open, value, "IsPaneOpen");
    }
    /// Gets `OpenPaneLength`.
    pub fn open_pane_length(&self) -> f64 {
        *self.open_pane_length.borrow()
    }
    /// Sets `OpenPaneLength`, notifying only when changed.
    pub fn set_open_pane_length(&self, value: f64) {
        let current = *self.open_pane_length.borrow();
        if current == value || (current.is_nan() && value.is_nan()) {
            return;
        }
        self.base.runtime().raise_and_set_if_changed(
            &self.open_pane_length,
            value,
            "OpenPaneLength",
        );
    }
    /// Gets `PanePlacement`.
    pub fn pane_placement(&self) -> SplitViewPanePlacement {
        *self.pane_placement.borrow()
    }
    /// Sets `PanePlacement`, notifying only when changed.
    pub fn set_pane_placement(&self, value: SplitViewPanePlacement) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.pane_placement, value, "PanePlacement");
    }
    /// Gets `UseLightDismissOverlayMode`.
    pub fn use_light_dismiss_overlay_mode(&self) -> bool {
        *self.use_light_dismiss_overlay_mode.borrow()
    }
    /// Sets `UseLightDismissOverlayMode`, notifying only when changed.
    pub fn set_use_light_dismiss_overlay_mode(&self, value: bool) {
        self.base.runtime().raise_and_set_if_changed(
            &self.use_light_dismiss_overlay_mode,
            value,
            "UseLightDismissOverlayMode",
        );
    }
    /// Gets `PaneDockable`.
    pub fn pane_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.pane_dockable.borrow().clone()
    }
    /// Sets `PaneDockable`, notifying only when changed.
    pub fn set_pane_dockable(&self, value: Option<Rc<dyn IDockable>>) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.pane_dockable, value, "PaneDockable");
    }
    /// Gets `ContentDockable`.
    pub fn content_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.content_dockable.borrow().clone()
    }
    /// Sets `ContentDockable`, notifying only when changed.
    pub fn set_content_dockable(&self, value: Option<Rc<dyn IDockable>>) {
        self.base.runtime().raise_and_set_if_changed(
            &self.content_dockable,
            value,
            "ContentDockable",
        );
    }
    /// Invokes the enclosing model's virtual `open_pane` method.
    pub fn open_pane(&self) {
        self.base
            .runtime()
            .as_split_view_dock()
            .expect("split view membership")
            .open_pane();
    }
    /// Implements the base `open_pane` behavior for composed subclasses.
    pub fn open_pane_core(&self) {
        self.set_is_pane_open(true);
    }
    /// Invokes the enclosing model's virtual `close_pane` method.
    pub fn close_pane(&self) {
        self.base
            .runtime()
            .as_split_view_dock()
            .expect("split view membership")
            .close_pane();
    }
    /// Implements the base `close_pane` behavior for composed subclasses.
    pub fn close_pane_core(&self) {
        self.set_is_pane_open(false);
    }
    /// Invokes the enclosing model's virtual `toggle_pane` method.
    pub fn toggle_pane(&self) {
        self.base
            .runtime()
            .as_split_view_dock()
            .expect("split view membership")
            .toggle_pane();
    }
    /// Implements the base `toggle_pane` behavior for composed subclasses.
    pub fn toggle_pane_core(&self) {
        self.set_is_pane_open(!self.is_pane_open());
    }
    /// Boxes the container using the canonical erased dockable representation.
    pub fn boxed(self: &Rc<Self>) -> BoxedValue {
        Rc::new(self.clone() as Rc<dyn IDockable>)
    }
}
impl ISplitViewDock for SplitViewDock {
    fn compact_pane_length(&self) -> f64 {
        self.compact_pane_length()
    }
    fn set_compact_pane_length(&self, value: f64) {
        self.set_compact_pane_length(value);
    }
    fn display_mode(&self) -> SplitViewDisplayMode {
        self.display_mode()
    }
    fn set_display_mode(&self, value: SplitViewDisplayMode) {
        self.set_display_mode(value);
    }
    fn is_pane_open(&self) -> bool {
        self.is_pane_open()
    }
    fn set_is_pane_open(&self, value: bool) {
        self.set_is_pane_open(value);
    }
    fn open_pane_length(&self) -> f64 {
        self.open_pane_length()
    }
    fn set_open_pane_length(&self, value: f64) {
        self.set_open_pane_length(value);
    }
    fn pane_placement(&self) -> SplitViewPanePlacement {
        self.pane_placement()
    }
    fn set_pane_placement(&self, value: SplitViewPanePlacement) {
        self.set_pane_placement(value);
    }
    fn use_light_dismiss_overlay_mode(&self) -> bool {
        self.use_light_dismiss_overlay_mode()
    }
    fn set_use_light_dismiss_overlay_mode(&self, value: bool) {
        self.set_use_light_dismiss_overlay_mode(value);
    }
    fn pane_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.pane_dockable()
    }
    fn set_pane_dockable(&self, value: Option<Rc<dyn IDockable>>) {
        self.set_pane_dockable(value);
    }
    fn content_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.content_dockable()
    }
    fn set_content_dockable(&self, value: Option<Rc<dyn IDockable>>) {
        self.set_content_dockable(value);
    }
    fn open_pane(&self) {
        self.open_pane_core();
    }
    fn close_pane(&self) {
        self.close_pane_core();
    }
    fn toggle_pane(&self) {
        self.toggle_pane_core();
    }
}
