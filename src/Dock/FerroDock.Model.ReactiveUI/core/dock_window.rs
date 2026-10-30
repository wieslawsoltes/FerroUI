// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::dockable_base::WeakLink;
use super::ReactiveBase;
use ferrodock_model::adapters::HostAdapter;
use ferrodock_model::controls::IRootDock;
use ferrodock_model::core::*;
use ferroui_reactiveui::{reactive_object, IReactiveObject, IReactiveObjectExt, ReactiveObject};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Object-safe dispatch of a composed window's typed virtual overrides.
#[doc(hidden)]
pub trait DockWindowCallbacks {
    /// Dispatches `on_close` to the concrete enclosing model.
    fn invoke_on_close(&self) -> bool;
    /// Dispatches `on_move_drag_begin` to the concrete enclosing model.
    fn invoke_on_move_drag_begin(&self) -> bool;
    /// Dispatches `on_move_drag` to the concrete enclosing model.
    fn invoke_on_move_drag(&self);
    /// Dispatches `on_move_drag_end` to the concrete enclosing model.
    fn invoke_on_move_drag_end(&self);
}
impl<T: DockWindowImpl> DockWindowCallbacks for T {
    fn invoke_on_close(&self) -> bool {
        <T as DockWindowImpl>::on_close(self)
    }
    fn invoke_on_move_drag_begin(&self) -> bool {
        <T as DockWindowImpl>::on_move_drag_begin(self)
    }
    fn invoke_on_move_drag(&self) {
        <T as DockWindowImpl>::on_move_drag(self)
    }
    fn invoke_on_move_drag_end(&self) {
        <T as DockWindowImpl>::on_move_drag_end(self)
    }
}

/// Virtual close and dragging callbacks for a model composed from DockWindow.
pub trait DockWindowImpl: IDockWindow + IReactiveObject + DockWindowCallbacks {
    /// Source default for `on_close`.
    fn on_close(_this: &Self) -> bool
    where
        Self: Sized,
    {
        true
    }
    /// Source default for `on_move_drag_begin`.
    fn on_move_drag_begin(_this: &Self) -> bool
    where
        Self: Sized,
    {
        true
    }
    /// Source default for `on_move_drag`.
    fn on_move_drag(_this: &Self)
    where
        Self: Sized,
    {
    }
    /// Source default for `on_move_drag_end`.
    fn on_move_drag_end(_this: &Self)
    where
        Self: Sized,
    {
    }
}

/// Reactive window model with source geometry, ownership and host operations.
/// Layout and host are owned; parent/owner/factory backlinks are nonowning.
pub struct DockWindow {
    base: ReactiveBase,
    runtime: Weak<dyn DockWindowImpl>,
    adapter: HostAdapter,
    id: RefCell<String>,
    x: RefCell<f64>,
    y: RefCell<f64>,
    width: RefCell<f64>,
    height: RefCell<f64>,
    window_state: RefCell<DockWindowState>,
    topmost: RefCell<bool>,
    title: RefCell<String>,
    owner_mode: RefCell<DockWindowOwnerMode>,
    parent_window: RefCell<Option<Rc<dyn IDockWindow>>>,
    is_modal: RefCell<bool>,
    show_in_taskbar: RefCell<Option<bool>>,
    owner: RefCell<WeakLink<dyn IDockable>>,
    factory: RefCell<WeakLink<dyn IFactory>>,
    layout: RefCell<Option<Rc<dyn IRootDock>>>,
    host: RefCell<Option<Rc<dyn IHostWindow>>>,
}
reactive_object!(DockWindow, base);
impl DockWindowImpl for DockWindow {}
impl AsRef<ReactiveObject> for DockWindow {
    fn as_ref(&self) -> &ReactiveObject {
        self.base.as_ref()
    }
}
impl PartialEq for DockWindow {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl DockWindow {
    /// Creates a shared window with the exact source defaults.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak: &Weak<Self>| Self::construct(weak.clone()))
    }
    /// Constructs a window component using the enclosing model's identity.
    pub fn construct<T: DockWindowImpl + 'static>(runtime: Weak<T>) -> Self {
        let window: Weak<dyn IDockWindow> = runtime.clone();
        let runtime: Weak<dyn DockWindowImpl> = runtime;
        Self {
            base: ReactiveBase::default(),
            runtime,
            adapter: HostAdapter::from_weak(window),
            id: RefCell::new("IDockWindow".to_owned()),
            x: RefCell::new(0.0),
            y: RefCell::new(0.0),
            width: RefCell::new(0.0),
            height: RefCell::new(0.0),
            window_state: RefCell::new(DockWindowState::Normal),
            topmost: RefCell::new(false),
            title: RefCell::new("IDockWindow".to_owned()),
            owner_mode: RefCell::new(DockWindowOwnerMode::Default),
            parent_window: RefCell::new(None),
            is_modal: RefCell::new(false),
            show_in_taskbar: RefCell::new(None),
            owner: RefCell::new(WeakLink::new(None)),
            factory: RefCell::new(WeakLink::new(None)),
            layout: RefCell::new(None),
            host: RefCell::new(None),
        }
    }
    fn runtime(&self) -> Rc<dyn DockWindowImpl> {
        self.runtime
            .upgrade()
            .expect("the enclosing window is alive while its component is borrowed")
    }
    /// The concrete window part used by inherited implementation macros.
    #[doc(hidden)]
    pub fn dock_window_base(&self) -> &Self {
        self
    }
    /// Gets `Id`.
    pub fn id(&self) -> String {
        self.id.borrow().clone()
    }
    /// Sets `Id`, notifying only on a value change.
    pub fn set_id(&self, value: String) {
        self.runtime()
            .raise_and_set_if_changed(&self.id, value, "Id");
    }
    /// Gets `X`.
    pub fn x(&self) -> f64 {
        *self.x.borrow()
    }
    /// Sets `X`, notifying only on a value change.
    pub fn set_x(&self, value: f64) {
        let current = *self.x.borrow();
        if current == value || (current.is_nan() && value.is_nan()) {
            return;
        }
        self.runtime().raise_and_set_if_changed(&self.x, value, "X");
    }
    /// Gets `Y`.
    pub fn y(&self) -> f64 {
        *self.y.borrow()
    }
    /// Sets `Y`, notifying only on a value change.
    pub fn set_y(&self, value: f64) {
        let current = *self.y.borrow();
        if current == value || (current.is_nan() && value.is_nan()) {
            return;
        }
        self.runtime().raise_and_set_if_changed(&self.y, value, "Y");
    }
    /// Gets `Width`.
    pub fn width(&self) -> f64 {
        *self.width.borrow()
    }
    /// Sets `Width`, notifying only on a value change.
    pub fn set_width(&self, value: f64) {
        let current = *self.width.borrow();
        if current == value || (current.is_nan() && value.is_nan()) {
            return;
        }
        self.runtime()
            .raise_and_set_if_changed(&self.width, value, "Width");
    }
    /// Gets `Height`.
    pub fn height(&self) -> f64 {
        *self.height.borrow()
    }
    /// Sets `Height`, notifying only on a value change.
    pub fn set_height(&self, value: f64) {
        let current = *self.height.borrow();
        if current == value || (current.is_nan() && value.is_nan()) {
            return;
        }
        self.runtime()
            .raise_and_set_if_changed(&self.height, value, "Height");
    }
    /// Gets `WindowState`.
    pub fn window_state(&self) -> DockWindowState {
        *self.window_state.borrow()
    }
    /// Sets `WindowState`, notifying only on a value change.
    pub fn set_window_state(&self, value: DockWindowState) {
        self.runtime()
            .raise_and_set_if_changed(&self.window_state, value, "WindowState");
    }
    /// Gets `Topmost`.
    pub fn topmost(&self) -> bool {
        *self.topmost.borrow()
    }
    /// Sets `Topmost`, notifying only on a value change.
    pub fn set_topmost(&self, value: bool) {
        self.runtime()
            .raise_and_set_if_changed(&self.topmost, value, "Topmost");
    }
    /// Gets `Title`.
    pub fn title(&self) -> String {
        self.title.borrow().clone()
    }
    /// Sets `Title`, notifying only on a value change.
    pub fn set_title(&self, value: String) {
        self.runtime()
            .raise_and_set_if_changed(&self.title, value, "Title");
    }
    /// Gets `OwnerMode`.
    pub fn owner_mode(&self) -> DockWindowOwnerMode {
        *self.owner_mode.borrow()
    }
    /// Sets `OwnerMode`, notifying only on a value change.
    pub fn set_owner_mode(&self, value: DockWindowOwnerMode) {
        self.runtime()
            .raise_and_set_if_changed(&self.owner_mode, value, "OwnerMode");
    }
    /// Gets `ParentWindow`.
    pub fn parent_window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.parent_window.borrow().clone()
    }
    /// Sets `ParentWindow`, notifying only on a value change.
    pub fn set_parent_window(&self, value: Option<Rc<dyn IDockWindow>>) {
        self.runtime()
            .raise_and_set_if_changed(&self.parent_window, value, "ParentWindow");
    }
    /// Gets `IsModal`.
    pub fn is_modal(&self) -> bool {
        *self.is_modal.borrow()
    }
    /// Sets `IsModal`, notifying only on a value change.
    pub fn set_is_modal(&self, value: bool) {
        self.runtime()
            .raise_and_set_if_changed(&self.is_modal, value, "IsModal");
    }
    /// Gets `ShowInTaskbar`.
    pub fn show_in_taskbar(&self) -> Option<bool> {
        *self.show_in_taskbar.borrow()
    }
    /// Sets `ShowInTaskbar`, notifying only on a value change.
    pub fn set_show_in_taskbar(&self, value: Option<bool>) {
        self.runtime()
            .raise_and_set_if_changed(&self.show_in_taskbar, value, "ShowInTaskbar");
    }
    /// Gets `Owner`.
    pub fn owner(&self) -> Option<Rc<dyn IDockable>> {
        self.owner.borrow().resolve()
    }
    /// Sets `Owner`, notifying only on a value change.
    pub fn set_owner(&self, value: Option<Rc<dyn IDockable>>) {
        self.runtime()
            .raise_and_set_if_changed(&self.owner, WeakLink::new(value), "Owner");
    }
    /// Gets `Factory`.
    pub fn factory(&self) -> Option<Rc<dyn IFactory>> {
        self.factory.borrow().resolve()
    }
    /// Sets `Factory`, notifying only on a value change.
    pub fn set_factory(&self, value: Option<Rc<dyn IFactory>>) {
        self.runtime()
            .raise_and_set_if_changed(&self.factory, WeakLink::new(value), "Factory");
    }
    /// Gets `Layout`.
    pub fn layout(&self) -> Option<Rc<dyn IRootDock>> {
        self.layout.borrow().clone()
    }
    /// Sets `Layout`, notifying only on a value change.
    pub fn set_layout(&self, value: Option<Rc<dyn IRootDock>>) {
        self.runtime()
            .raise_and_set_if_changed(&self.layout, value, "Layout");
    }
    /// Gets `Host`.
    pub fn host(&self) -> Option<Rc<dyn IHostWindow>> {
        self.host.borrow().clone()
    }
    /// Sets `Host`, notifying only on a value change.
    pub fn set_host(&self, value: Option<Rc<dyn IHostWindow>>) {
        self.runtime()
            .raise_and_set_if_changed(&self.host, value, "Host");
    }
    /// Runs the enclosing model's virtual `on_close` callback.
    pub fn on_close(&self) -> bool {
        self.runtime().invoke_on_close()
    }
    /// Runs the enclosing model's virtual `on_move_drag_begin` callback.
    pub fn on_move_drag_begin(&self) -> bool {
        self.runtime().invoke_on_move_drag_begin()
    }
    /// Runs the enclosing model's virtual `on_move_drag` callback.
    pub fn on_move_drag(&self) {
        self.runtime().invoke_on_move_drag()
    }
    /// Runs the enclosing model's virtual `on_move_drag_end` callback.
    pub fn on_move_drag_end(&self) {
        self.runtime().invoke_on_move_drag_end()
    }
    /// Delegates `Save` to the source host adapter.
    pub fn save(&self) {
        self.adapter.save();
    }
    /// Delegates `Present` to the source host adapter.
    pub fn present(&self, is_dialog: bool) {
        self.adapter.present(is_dialog);
    }
    /// Delegates `Exit` to the source host adapter.
    pub fn exit(&self) {
        self.adapter.exit();
    }
    /// Delegates `SetActive` to the source host adapter.
    pub fn set_active(&self) {
        self.adapter.set_active();
    }
}
impl IDockWindow for DockWindow {
    fn as_notify_property_changed(
        &self,
    ) -> Option<&dyn ferroui_system::component_model::INotifyPropertyChanged> {
        Some(self)
    }
    fn id(&self) -> String {
        self.id()
    }
    fn set_id(&self, value: String) {
        self.set_id(value);
    }
    fn x(&self) -> f64 {
        self.x()
    }
    fn set_x(&self, value: f64) {
        self.set_x(value);
    }
    fn y(&self) -> f64 {
        self.y()
    }
    fn set_y(&self, value: f64) {
        self.set_y(value);
    }
    fn width(&self) -> f64 {
        self.width()
    }
    fn set_width(&self, value: f64) {
        self.set_width(value);
    }
    fn height(&self) -> f64 {
        self.height()
    }
    fn set_height(&self, value: f64) {
        self.set_height(value);
    }
    fn window_state(&self) -> DockWindowState {
        self.window_state()
    }
    fn set_window_state(&self, value: DockWindowState) {
        self.set_window_state(value);
    }
    fn topmost(&self) -> bool {
        self.topmost()
    }
    fn set_topmost(&self, value: bool) {
        self.set_topmost(value);
    }
    fn title(&self) -> String {
        self.title()
    }
    fn set_title(&self, value: String) {
        self.set_title(value);
    }
    fn owner_mode(&self) -> DockWindowOwnerMode {
        self.owner_mode()
    }
    fn set_owner_mode(&self, value: DockWindowOwnerMode) {
        self.set_owner_mode(value);
    }
    fn parent_window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.parent_window()
    }
    fn set_parent_window(&self, value: Option<Rc<dyn IDockWindow>>) {
        self.set_parent_window(value);
    }
    fn is_modal(&self) -> bool {
        self.is_modal()
    }
    fn set_is_modal(&self, value: bool) {
        self.set_is_modal(value);
    }
    fn show_in_taskbar(&self) -> Option<bool> {
        self.show_in_taskbar()
    }
    fn set_show_in_taskbar(&self, value: Option<bool>) {
        self.set_show_in_taskbar(value);
    }
    fn owner(&self) -> Option<Rc<dyn IDockable>> {
        self.owner()
    }
    fn set_owner(&self, value: Option<Rc<dyn IDockable>>) {
        self.set_owner(value);
    }
    fn factory(&self) -> Option<Rc<dyn IFactory>> {
        self.factory()
    }
    fn set_factory(&self, value: Option<Rc<dyn IFactory>>) {
        self.set_factory(value);
    }
    fn layout(&self) -> Option<Rc<dyn IRootDock>> {
        self.layout()
    }
    fn set_layout(&self, value: Option<Rc<dyn IRootDock>>) {
        self.set_layout(value);
    }
    fn host(&self) -> Option<Rc<dyn IHostWindow>> {
        self.host()
    }
    fn set_host(&self, value: Option<Rc<dyn IHostWindow>>) {
        self.set_host(value);
    }
    fn on_close(&self) -> bool {
        self.on_close()
    }
    fn on_move_drag_begin(&self) -> bool {
        self.on_move_drag_begin()
    }
    fn on_move_drag(&self) {
        self.on_move_drag()
    }
    fn on_move_drag_end(&self) {
        self.on_move_drag_end()
    }
    fn save(&self) {
        self.save();
    }
    fn present(&self, is_dialog: bool) {
        self.present(is_dialog);
    }
    fn exit(&self) {
        self.exit();
    }
    fn set_active(&self) {
        self.set_active();
    }
}

/// Forwards the complete window contract from a composed window component.
/// Store a DockWindow part named `base` and implement DockWindowImpl for typed
/// virtual overrides. The @base form supports deeper composition.
#[macro_export]
macro_rules! impl_dock_window {
    ($type:ty $(, $extra:item)*) => { $crate::impl_dock_window!(@base $type, $crate::core::DockWindow $(, $extra)*); };
    (@base $type:ty, $base_type:ty $(, $extra:item)*) => {
        impl AsRef<$crate::__private::ReactiveObject> for $type {
            fn as_ref(&self) -> &$crate::__private::ReactiveObject { self.base.as_ref() }
        }
        impl std::ops::Deref for $type { type Target = $base_type; fn deref(&self) -> &Self::Target { &self.base } }
        impl PartialEq for $type { fn eq(&self, other: &Self) -> bool { std::ptr::eq(self, other) } }
        impl $crate::__private::IDockWindow for $type {
            fn as_notify_property_changed(&self) -> Option<&dyn $crate::__private::INotifyPropertyChanged> { Some(self) }
            fn id(&self) -> String { self.base.id() }
            fn set_id(&self, value: String) { self.base.set_id(value); }
            fn x(&self) -> f64 { self.base.x() }
            fn set_x(&self, value: f64) { self.base.set_x(value); }
            fn y(&self) -> f64 { self.base.y() }
            fn set_y(&self, value: f64) { self.base.set_y(value); }
            fn width(&self) -> f64 { self.base.width() }
            fn set_width(&self, value: f64) { self.base.set_width(value); }
            fn height(&self) -> f64 { self.base.height() }
            fn set_height(&self, value: f64) { self.base.set_height(value); }
            fn window_state(&self) -> $crate::__private::DockWindowState { self.base.window_state() }
            fn set_window_state(&self, value: $crate::__private::DockWindowState) { self.base.set_window_state(value); }
            fn topmost(&self) -> bool { self.base.topmost() }
            fn set_topmost(&self, value: bool) { self.base.set_topmost(value); }
            fn title(&self) -> String { self.base.title() }
            fn set_title(&self, value: String) { self.base.set_title(value); }
            fn owner_mode(&self) -> $crate::__private::DockWindowOwnerMode { self.base.owner_mode() }
            fn set_owner_mode(&self, value: $crate::__private::DockWindowOwnerMode) { self.base.set_owner_mode(value); }
            fn parent_window(&self) -> Option<std::rc::Rc<dyn $crate::__private::IDockWindow>> { self.base.parent_window() }
            fn set_parent_window(&self, value: Option<std::rc::Rc<dyn $crate::__private::IDockWindow>>) { self.base.set_parent_window(value); }
            fn is_modal(&self) -> bool { self.base.is_modal() }
            fn set_is_modal(&self, value: bool) { self.base.set_is_modal(value); }
            fn show_in_taskbar(&self) -> Option<bool> { self.base.show_in_taskbar() }
            fn set_show_in_taskbar(&self, value: Option<bool>) { self.base.set_show_in_taskbar(value); }
            fn owner(&self) -> Option<std::rc::Rc<dyn $crate::__private::IDockable>> { self.base.owner() }
            fn set_owner(&self, value: Option<std::rc::Rc<dyn $crate::__private::IDockable>>) { self.base.set_owner(value); }
            fn factory(&self) -> Option<std::rc::Rc<dyn $crate::__private::IFactory>> { self.base.factory() }
            fn set_factory(&self, value: Option<std::rc::Rc<dyn $crate::__private::IFactory>>) { self.base.set_factory(value); }
            fn layout(&self) -> Option<std::rc::Rc<dyn $crate::__private::IRootDock>> { self.base.layout() }
            fn set_layout(&self, value: Option<std::rc::Rc<dyn $crate::__private::IRootDock>>) { self.base.set_layout(value); }
            fn host(&self) -> Option<std::rc::Rc<dyn $crate::__private::IHostWindow>> { self.base.host() }
            fn set_host(&self, value: Option<std::rc::Rc<dyn $crate::__private::IHostWindow>>) { self.base.set_host(value); }
            fn on_close(&self) -> bool { self.base.dock_window_base().on_close() }
            fn on_move_drag_begin(&self) -> bool { self.base.dock_window_base().on_move_drag_begin() }
            fn on_move_drag(&self) { self.base.dock_window_base().on_move_drag() }
            fn on_move_drag_end(&self) { self.base.dock_window_base().on_move_drag_end() }
            fn save(&self) { self.base.save(); }
            fn present(&self, is_dialog: bool) { self.base.present(is_dialog); }
            fn exit(&self) { self.base.exit(); }
            fn set_active(&self) { self.base.set_active(); }
            $($extra)*
        }
    };
}
