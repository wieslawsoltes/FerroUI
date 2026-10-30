// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{DockControl, ManagedDockWindowDocument, ManagedWindowDock};
use crate::internal::{
    dock_window_state_helper::DockWindowStateHelper as States, event_type::EventType,
    managed_host_window_access::ManagedHostWindowAccess,
    managed_host_window_state::ManagedHostWindowState,
    managed_window_registry::ManagedWindowRegistry,
};
use ferrodock_controls_recycling::to_recycling_value;
use ferrodock_model::{
    core::{
        DockManagerOptions, DockRect, DockWindowState, IDock, IDockWindow, IDockable, IHostWindow,
        IHostWindowCloseState, IHostWindowState, MdiWindowState,
    },
    BoxedValue, DockManager, DockService,
};
use ferroui_base::PixelPoint;
use ferroui_controls::Control;
use std::{
    any::TypeId,
    cell::{Cell, OnceCell, RefCell},
    rc::{Rc, Weak},
};
/// Original managed in-app host window implementation.
pub struct ManagedHostWindow {
    runtime: Weak<Self>,
    host_window_state: OnceCell<Rc<ManagedHostWindowState>>,
    document: RefCell<Option<Rc<ManagedDockWindowDocument>>>,
    dock: RefCell<Option<Rc<ManagedWindowDock>>>,
    layout: RefCell<Option<Rc<dyn IDock>>>,
    title: RefCell<Option<String>>,
    x: Cell<f64>,
    y: Cell<f64>,
    width: Cell<f64>,
    height: Cell<f64>,
    window_state: Cell<DockWindowState>,
    closed: Cell<bool>,
    closing: Cell<bool>,
    last_close_canceled: Cell<bool>,
    tracked: Cell<bool>,
    window: RefCell<Option<Rc<dyn IDockWindow>>>,
}
impl ManagedHostWindow {
    /// Creates the source host with fresh manager options.
    pub fn new() -> Rc<Self> {
        Self::with_options(DockManagerOptions::new())
    }
    /// Creates the source host retaining the supplied shared manager options.
    pub fn with_options(options: Rc<DockManagerOptions>) -> Rc<Self> {
        let this = Rc::new_cyclic(|runtime| Self {
            runtime: runtime.clone(),
            host_window_state: OnceCell::new(),
            document: RefCell::new(None),
            dock: RefCell::new(None),
            layout: RefCell::new(None),
            title: RefCell::new(None),
            x: Cell::new(0.0),
            y: Cell::new(0.0),
            width: Cell::new(400.0),
            height: Cell::new(300.0),
            window_state: Cell::new(DockWindowState::Normal),
            closed: Cell::new(false),
            closing: Cell::new(false),
            last_close_canceled: Cell::new(false),
            tracked: Cell::new(false),
            window: RefCell::new(None),
        });
        // OnceCell supports the source's constructor-time mutual strong references.
        // Do not weaken the state -> host edge to hide native Rc/CLR GC differences.
        let state = ManagedHostWindowState::new(
            DockManager::with_options(DockService::new(), options),
            this.clone(),
            None,
        );
        assert!(
            this.host_window_state.set(state).is_ok(),
            "source single state initialization"
        );
        this
    }
    fn this(&self) -> Rc<Self> {
        self.runtime.upgrade().expect("live source managed host")
    }
    fn document(&self) -> Option<Rc<ManagedDockWindowDocument>> {
        self.document.borrow().clone()
    }
    fn dock(&self) -> Option<Rc<ManagedWindowDock>> {
        self.dock.borrow().clone()
    }
    /// Gets the current document z-index, or zero without a document.
    pub fn managed_z_index(&self) -> i32 {
        self.document().map_or(0, |d| d.mdi_z_index())
    }
    pub(crate) fn last_close_canceled(&self) -> bool {
        self.last_close_canceled.get()
    }
    fn exit_core(&self) {
        self.last_close_canceled.set(false);
        let closing_window = self.window();
        let closing_factory = closing_window.as_ref().and_then(|w| w.factory());
        let mut preserve = false;
        if let Some(window) = &closing_window {
            if let Some(factory) = window.factory() {
                if !factory.on_window_closing(Some(window.clone())) {
                    self.last_close_canceled.set(true);
                    return;
                }
            } else if !window.on_close() {
                self.last_close_canceled.set(true);
                return;
            }
            preserve = closing_factory
                .clone()
                .and_then(|f| f.as_dock_window_close_policy())
                .is_some_and(|p| p.preserve_layout_on_host_close(window.clone()));
            if self.is_tracked() {
                window.save();
                if !preserve {
                    if let Some(root) = window.layout() {
                        if root.close().can_execute(None) {
                            root.close().execute(None);
                        }
                    }
                }
            }
        }
        self.closed.set(true);
        if preserve {
            let factory = closing_factory.expect("source preservation policy factory");
            let window = closing_window.expect("source preservation policy window");
            let this: Rc<dyn IHostWindow> = self.this();
            factory.host_windows().remove(&this);
            self.set_is_tracked(false);
            if window.host().as_ref().is_some_and(|h| Rc::ptr_eq(h, &this)) {
                window.set_host(None);
            }
            self.release_presentation();
            factory.on_window_closed(Some(window));
            return;
        }
        if let Some(factory) = self.window().and_then(|w| w.factory()) {
            let this: Rc<dyn IHostWindow> = self.this();
            factory.host_windows().remove(&this);
            if self.window().is_some()
                && self
                    .window()
                    .expect("source window re-read")
                    .layout()
                    .is_some()
            {
                factory.close_window(Some(
                    self.window().expect("source window after layout read"),
                ));
            }
            factory.on_window_closed(self.window());
            if self.window().is_some() && self.is_tracked() {
                factory.remove_window(Some(self.window().expect("source tracked window")));
            }
        }
        self.release_presentation();
        if self.window().is_some() {
            let this: Rc<dyn IHostWindow> = self.this();
            if self
                .window()
                .expect("source window after release")
                .host()
                .as_ref()
                .is_some_and(|h| Rc::ptr_eq(h, &this))
            {
                self.window().expect("source host window").set_host(None);
            }
        }
        self.set_is_tracked(false);
    }
    fn release_presentation(&self) {
        let dock = self.dock();
        let document = self.document();
        self.dock.replace(None);
        self.document.replace(None);
        if let (Some(dock), Some(document)) = (dock, document) {
            dock.remove_window(Some(document.clone()));
            document.dispose();
        }
    }
    fn update_bounds(&self) {
        if let Some(document) = self.document() {
            document.set_mdi_bounds(DockRect::new(
                self.x.get(),
                self.y.get(),
                self.width.get(),
                self.height.get(),
            ));
        }
    }
    pub(crate) fn update_bounds_from_document(&self, bounds: DockRect) {
        self.x.set(bounds.x);
        self.y.set(bounds.y);
        self.width.set(bounds.width);
        self.height.set(bounds.height);
    }
    pub(crate) fn process_drag(&self, point: PixelPoint, phase: EventType) {
        self.host_window_state
            .get()
            .expect("source state initialized")
            .process(point, phase);
    }
}
impl IHostWindowCloseState for ManagedHostWindow {
    fn is_closed(&self) -> bool {
        self.closed.get()
    }
}
impl ManagedHostWindowAccess for ManagedHostWindow {
    fn managed_z_index(&self) -> i32 {
        self.managed_z_index()
    }
    fn last_close_canceled(&self) -> bool {
        self.last_close_canceled()
    }
    fn process_drag(&self, point: PixelPoint, phase: EventType) {
        self.process_drag(point, phase);
    }
    fn update_bounds_from_document(&self, bounds: DockRect) {
        self.update_bounds_from_document(bounds);
    }
}
impl IHostWindow for ManagedHostWindow {
    fn boxed_interface(self: Rc<Self>, target: TypeId) -> Option<BoxedValue> {
        if target == TypeId::of::<Rc<Self>>() {
            return Some(Rc::new(self));
        }
        if target == TypeId::of::<Rc<dyn ManagedHostWindowAccess>>() {
            return Some(Rc::new(self as Rc<dyn ManagedHostWindowAccess>));
        }
        if target == TypeId::of::<Rc<dyn IHostWindowCloseState>>() {
            return Some(Rc::new(self as Rc<dyn IHostWindowCloseState>));
        }
        None
    }
    fn as_host_window_close_state(&self) -> Option<&dyn IHostWindowCloseState> {
        Some(self)
    }
    fn host_window_state(&self) -> Option<Rc<dyn IHostWindowState>> {
        Some(
            self.host_window_state
                .get()
                .expect("source state initialized")
                .clone(),
        )
    }
    fn is_tracked(&self) -> bool {
        self.tracked.get()
    }
    fn set_is_tracked(&self, value: bool) {
        self.tracked.set(value);
    }
    fn window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.window.borrow().clone()
    }
    fn set_window(&self, value: Option<Rc<dyn IDockWindow>>) {
        let previous = self.window.replace(value);
        drop(previous);
    }
    fn present(&self, _is_dialog: bool) {
        let Some(factory) = self.window().and_then(|w| w.factory()) else {
            return;
        };
        self.dock
            .replace(Some(ManagedWindowRegistry::get_or_create_dock(&factory)));
        if self.layout.borrow().is_none() {
            self.layout.replace(
                self.window()
                    .expect("source current Window")
                    .layout()
                    .map(|l| l as Rc<dyn IDock>),
            );
        }
        if self.title.borrow().is_none() {
            self.title
                .replace(Some(self.window().expect("source current Window").title()));
        }
        if self.document().is_none() {
            let document =
                ManagedDockWindowDocument::new(self.window().expect("source current Window"));
            self.document.replace(Some(document));
            let title = self
                .title
                .borrow()
                .clone()
                .unwrap_or_else(|| self.document().expect("source document").title());
            self.document().expect("source document").set_title(title);
            self.document()
                .expect("source document")
                .set_mdi_bounds(DockRect::new(
                    self.x.get(),
                    self.y.get(),
                    self.width.get(),
                    self.height.get(),
                ));
            self.document()
                .expect("source document")
                .set_mdi_state(States::to_mdi_window_state(self.window_state.get()));
            let layout = self.layout.borrow().clone();
            if let Some(layout) = layout {
                self.document()
                    .expect("source document")
                    .set_content(ManagedDockWindowDocumentContent::create(Some(layout)));
            }
        } else {
            self.document()
                .expect("source document")
                .set_mdi_state(States::to_mdi_window_state(self.window_state.get()));
        }
        let dock = self.dock().expect("source dock");
        dock.add_window(self.document());
        self.window()
            .expect("source window after add callback")
            .set_host(Some(self.this()));
        self.set_is_tracked(true);
        self.closed.set(false);
        let this: Rc<dyn IHostWindow> = self.this();
        if !factory.host_windows().contains(&this) {
            factory.host_windows().add(this);
        }
        factory.on_window_opened(self.window());
    }
    fn exit(&self) {
        if self.closed.get() || self.closing.get() {
            return;
        }
        self.closing.set(true);
        let _guard = ClosingGuard(&self.closing);
        self.exit_core();
    }
    fn set_position(&self, x: f64, y: f64) {
        if x.is_nan() || y.is_nan() {
            return;
        }
        self.x.set(x);
        self.y.set(y);
        self.update_bounds();
    }
    fn get_position(&self) -> (f64, f64) {
        (self.x.get(), self.y.get())
    }
    fn set_size(&self, width: f64, height: f64) {
        if width.is_nan() || height.is_nan() {
            return;
        }
        self.width.set(width);
        self.height.set(height);
        self.update_bounds();
    }
    fn get_size(&self) -> (f64, f64) {
        (self.width.get(), self.height.get())
    }
    fn set_window_state(&self, state: DockWindowState) {
        self.window_state.set(state);
        if let Some(window) = self.window() {
            window.set_window_state(state);
        }
        if let Some(document) = self.document() {
            document.set_mdi_state(States::to_mdi_window_state(state));
        }
    }
    fn get_window_state(&self) -> DockWindowState {
        if let Some(document) = self.document() {
            let mapped = States::to_dock_window_state_from_mdi(document.mdi_state());
            if !(self.window_state.get() == DockWindowState::FullScreen
                && mapped == DockWindowState::Normal)
            {
                self.window_state.set(mapped);
            }
        }
        self.window_state.get()
    }
    fn set_title(&self, title: Option<String>) {
        self.title.replace(title.clone());
        if let Some(document) = self.document() {
            document.set_title(title.unwrap_or_default());
        }
    }
    fn set_layout(&self, layout: Rc<dyn IDock>) {
        self.layout.replace(Some(layout.clone()));
        if let Some(document) = self.document() {
            document.set_content(ManagedDockWindowDocumentContent::create(Some(layout)));
        }
    }
    fn set_active(&self) {
        if self.dock().is_some() && self.document().is_some() {
            if self.document().expect("source document").mdi_state() == MdiWindowState::Minimized {
                self.document()
                    .expect("source document")
                    .set_mdi_state(MdiWindowState::Normal);
            }
            let dock = self.dock().expect("source dock after state callback");
            dock.set_active_dockable(self.document().map(|d| d as Rc<dyn IDockable>));
        }
    }
}
struct ManagedDockWindowDocumentContent;
impl ManagedDockWindowDocumentContent {
    fn create(layout: Option<Rc<dyn IDock>>) -> Option<BoxedValue> {
        let root = layout?.as_root_dock()?;
        let control = DockControl::new();
        control.set_layout(Some(root));
        control.set_initialize_factory(false);
        control.set_initialize_layout(false);
        control.set_enable_managed_window_layer(false);
        Some(to_recycling_value(Control::boxed(
            control.upcast::<Control>(),
        )))
    }
}
struct ClosingGuard<'a>(&'a Cell<bool>);
impl Drop for ClosingGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
