// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::ReactiveBase;
use ferrodock_model::adapters::TrackingAdapter;
use ferrodock_model::core::*;
use ferrodock_model::BoxedValue;
use ferroui_reactiveui::{reactive_object, IReactiveObject, IReactiveObjectExt, ReactiveObject};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Object-safe callback dispatch for composed model components.
/// Implemented automatically from DockableBaseImpl's typed overrides.
#[doc(hidden)]
pub trait DockableBaseCallbacks {
    /// Dispatches `on_close` to the concrete model override.
    fn invoke_on_close(&self) -> bool;
    /// Dispatches `on_selected` to the concrete model override.
    fn invoke_on_selected(&self);
    /// Dispatches `on_visible_bounds_changed` to the concrete model override.
    fn invoke_on_visible_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64);
    /// Dispatches `on_pinned_bounds_changed` to the concrete model override.
    fn invoke_on_pinned_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64);
    /// Dispatches `on_tab_bounds_changed` to the concrete model override.
    fn invoke_on_tab_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64);
    /// Dispatches `on_pointer_position_changed` to the concrete model override.
    fn invoke_on_pointer_position_changed(&self, x: f64, y: f64);
    /// Dispatches `on_pointer_screen_position_changed` to the concrete model override.
    fn invoke_on_pointer_screen_position_changed(&self, x: f64, y: f64);
}
impl<T: DockableBaseImpl> DockableBaseCallbacks for T {
    fn invoke_on_close(&self) -> bool {
        <T as DockableBaseImpl>::on_close(self)
    }
    fn invoke_on_selected(&self) {
        <T as DockableBaseImpl>::on_selected(self)
    }
    fn invoke_on_visible_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) {
        <T as DockableBaseImpl>::on_visible_bounds_changed(self, x, y, width, height)
    }
    fn invoke_on_pinned_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) {
        <T as DockableBaseImpl>::on_pinned_bounds_changed(self, x, y, width, height)
    }
    fn invoke_on_tab_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) {
        <T as DockableBaseImpl>::on_tab_bounds_changed(self, x, y, width, height)
    }
    fn invoke_on_pointer_position_changed(&self, x: f64, y: f64) {
        <T as DockableBaseImpl>::on_pointer_position_changed(self, x, y)
    }
    fn invoke_on_pointer_screen_position_changed(&self, x: f64, y: f64) {
        <T as DockableBaseImpl>::on_pointer_screen_position_changed(self, x, y)
    }
}

/// Virtual behavior of a concrete model composed from DockableBase.
/// Implementations inherit the source defaults and override only their custom behavior.
pub trait DockableBaseImpl: IDockable + IReactiveObject + DockableBaseCallbacks {
    /// Accepts a close by default.
    fn on_close(_this: &Self) -> bool
    where
        Self: Sized,
    {
        true
    }
    /// Called when selected; the base performs no action.
    fn on_selected(_this: &Self)
    where
        Self: Sized,
    {
    }
    /// Called after the visible bounds have been stored.
    fn on_visible_bounds_changed(_this: &Self, _x: f64, _y: f64, _width: f64, _height: f64)
    where
        Self: Sized,
    {
    }
    /// Called after the pinned bounds have been stored.
    fn on_pinned_bounds_changed(_this: &Self, _x: f64, _y: f64, _width: f64, _height: f64)
    where
        Self: Sized,
    {
    }
    /// Called after the tab bounds have been stored.
    fn on_tab_bounds_changed(_this: &Self, _x: f64, _y: f64, _width: f64, _height: f64)
    where
        Self: Sized,
    {
    }
    /// Called after the dockable pointer position has been stored.
    fn on_pointer_position_changed(_this: &Self, _x: f64, _y: f64)
    where
        Self: Sized,
    {
    }
    /// Called after the screen pointer position has been stored.
    fn on_pointer_screen_position_changed(_this: &Self, _x: f64, _y: f64)
    where
        Self: Sized,
    {
    }
}

pub(crate) struct WeakLink<T: ?Sized>(Option<Weak<T>>);
impl<T: ?Sized> Clone for WeakLink<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T: ?Sized> WeakLink<T> {
    pub(crate) fn new(value: Option<Rc<T>>) -> Self {
        Self(value.as_ref().map(Rc::downgrade))
    }
    pub(crate) fn resolve(&self) -> Option<Rc<T>> {
        self.0.as_ref().and_then(Weak::upgrade)
    }
}
impl<T: ?Sized> PartialEq for WeakLink<T> {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (Some(a), Some(b)) => Weak::ptr_eq(a, b),
            _ => false,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ContextValue(pub(crate) Option<BoxedValue>);
impl PartialEq for ContextValue {
    fn eq(&self, other: &Self) -> bool {
        let (left, right) = match (&self.0, &other.0) {
            (None, None) => return true,
            (Some(a), Some(b)) => (a, b),
            _ => return false,
        };
        ferrodock_model::value_semantics::boxed_values_equal(left, right)
    }
}

/// The abstract dockable model's state, composed into concrete reactive models.
/// Construct it with a weak handle to the enclosing model so virtual calls and
/// reactive notifications use that model's identity. Runtime/factory links are weak; source Owner references retain their model.
pub struct DockableBase {
    base: ReactiveBase,
    runtime: Weak<dyn DockableBaseImpl>,
    tracking: TrackingAdapter,
    docking_state: RefCell<DockingWindowState>,
    id: RefCell<String>,
    title: RefCell<String>,
    context: RefCell<ContextValue>,
    owner: RefCell<Option<Rc<dyn IDockable>>>,
    original_owner: RefCell<WeakLink<dyn IDockable>>,
    factory: RefCell<WeakLink<dyn IFactory>>,
    is_empty: RefCell<bool>,
    is_collapsable: RefCell<bool>,
    proportion: RefCell<f64>,
    dock: RefCell<DockMode>,
    column: RefCell<i32>,
    row: RefCell<i32>,
    column_span: RefCell<i32>,
    row_span: RefCell<i32>,
    is_shared_size_scope: RefCell<bool>,
    collapsed_proportion: RefCell<f64>,
    min_width: RefCell<f64>,
    max_width: RefCell<f64>,
    min_height: RefCell<f64>,
    max_height: RefCell<f64>,
    can_close: RefCell<bool>,
    can_pin: RefCell<bool>,
    keep_pinned_dockable_visible: RefCell<bool>,
    pinned_dock_display_mode_override: RefCell<Option<PinnedDockDisplayMode>>,
    can_float: RefCell<bool>,
    can_drag: RefCell<bool>,
    can_drop: RefCell<bool>,
    can_dock_as_document: RefCell<bool>,
    dock_capability_overrides: RefCell<Option<Rc<DockCapabilityOverrides>>>,
    is_modified: RefCell<bool>,
    dock_group: RefCell<Option<String>>,
    allowed_dock_operations: RefCell<DockOperationMask>,
    allowed_drop_operations: RefCell<DockOperationMask>,
    show_in_selector: RefCell<bool>,
    selector_title: RefCell<Option<String>>,
}

impl AsRef<ReactiveObject> for DockableBase {
    fn as_ref(&self) -> &ReactiveObject {
        self.base.as_ref()
    }
}
reactive_object!(DockableBase, base);

impl DockableBase {
    /// Initializes the base part after the enclosing class supplies its weak identity.
    pub fn construct(runtime: Weak<dyn DockableBaseImpl>) -> Self {
        Self {
            base: ReactiveBase::default(),
            runtime,
            tracking: TrackingAdapter::default(),
            docking_state: RefCell::new(DockingWindowState::DOCKED),
            id: RefCell::new(Default::default()),
            title: RefCell::new(Default::default()),
            context: RefCell::new(ContextValue(None)),
            owner: RefCell::new(None),
            original_owner: RefCell::new(WeakLink::new(None)),
            factory: RefCell::new(WeakLink::new(None)),
            is_empty: RefCell::new(Default::default()),
            is_collapsable: RefCell::new(true),
            proportion: RefCell::new(f64::NAN),
            dock: RefCell::new(Default::default()),
            column: RefCell::new(Default::default()),
            row: RefCell::new(Default::default()),
            column_span: RefCell::new(1),
            row_span: RefCell::new(1),
            is_shared_size_scope: RefCell::new(Default::default()),
            collapsed_proportion: RefCell::new(f64::NAN),
            min_width: RefCell::new(f64::NAN),
            max_width: RefCell::new(f64::NAN),
            min_height: RefCell::new(f64::NAN),
            max_height: RefCell::new(f64::NAN),
            can_close: RefCell::new(true),
            can_pin: RefCell::new(true),
            keep_pinned_dockable_visible: RefCell::new(Default::default()),
            pinned_dock_display_mode_override: RefCell::new(Default::default()),
            can_float: RefCell::new(true),
            can_drag: RefCell::new(true),
            can_drop: RefCell::new(true),
            can_dock_as_document: RefCell::new(true),
            dock_capability_overrides: RefCell::new(Default::default()),
            is_modified: RefCell::new(Default::default()),
            dock_group: RefCell::new(Default::default()),
            allowed_dock_operations: RefCell::new(DockOperationMask::ALL),
            allowed_drop_operations: RefCell::new(DockOperationMask::ALL),
            show_in_selector: RefCell::new(true),
            selector_title: RefCell::new(Default::default()),
        }
    }

    pub(crate) fn runtime(&self) -> Rc<dyn DockableBaseImpl> {
        self.runtime
            .upgrade()
            .expect("the enclosing model is alive while its base is borrowed")
    }
    /// Gets the lowest base component for inherited implementation macros.
    #[doc(hidden)]
    pub fn dockable_base(&self) -> &Self {
        self
    }
    /// Dispatches `on_close` through the enclosing model identity.
    #[doc(hidden)]
    pub fn dispatch_on_close(&self) -> bool {
        self.runtime().invoke_on_close()
    }
    /// Dispatches `on_selected` through the enclosing model identity.
    #[doc(hidden)]
    pub fn dispatch_on_selected(&self) {
        self.runtime().invoke_on_selected()
    }
    /// Dispatches `on_visible_bounds_changed` through the enclosing model identity.
    #[doc(hidden)]
    pub fn dispatch_on_visible_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) {
        self.runtime()
            .invoke_on_visible_bounds_changed(x, y, width, height)
    }
    /// Dispatches `on_pinned_bounds_changed` through the enclosing model identity.
    #[doc(hidden)]
    pub fn dispatch_on_pinned_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) {
        self.runtime()
            .invoke_on_pinned_bounds_changed(x, y, width, height)
    }
    /// Dispatches `on_tab_bounds_changed` through the enclosing model identity.
    #[doc(hidden)]
    pub fn dispatch_on_tab_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) {
        self.runtime()
            .invoke_on_tab_bounds_changed(x, y, width, height)
    }
    /// Dispatches `on_pointer_position_changed` through the enclosing model identity.
    #[doc(hidden)]
    pub fn dispatch_on_pointer_position_changed(&self, x: f64, y: f64) {
        self.runtime().invoke_on_pointer_position_changed(x, y)
    }
    /// Dispatches `on_pointer_screen_position_changed` through the enclosing model identity.
    #[doc(hidden)]
    pub fn dispatch_on_pointer_screen_position_changed(&self, x: f64, y: f64) {
        self.runtime()
            .invoke_on_pointer_screen_position_changed(x, y)
    }
    /// Gets `Id`.
    pub fn id(&self) -> String {
        self.id.borrow().clone()
    }
    /// Sets `Id`, notifying only when the value changes.
    pub fn set_id(&self, value: String) {
        self.runtime()
            .raise_and_set_if_changed(&self.id, value, "Id");
    }
    /// Gets `Title`.
    pub fn title(&self) -> String {
        self.title.borrow().clone()
    }
    /// Sets `Title`, notifying only when the value changes.
    pub fn set_title(&self, value: String) {
        self.runtime()
            .raise_and_set_if_changed(&self.title, value, "Title");
    }
    /// Gets `Context`.
    pub fn context(&self) -> Option<BoxedValue> {
        self.context.borrow().0.clone()
    }
    /// Sets `Context`, notifying only when the value changes.
    pub fn set_context(&self, value: Option<BoxedValue>) {
        self.runtime()
            .raise_and_set_if_changed(&self.context, ContextValue(value), "Context");
    }
    /// Gets `Owner`.
    pub fn owner(&self) -> Option<Rc<dyn IDockable>> {
        self.owner.borrow().clone()
    }
    /// Sets `Owner`, notifying only when the value changes.
    pub fn set_owner(&self, value: Option<Rc<dyn IDockable>>) {
        self.runtime()
            .raise_and_set_if_changed(&self.owner, value, "Owner");
    }
    /// Gets `OriginalOwner`.
    pub fn original_owner(&self) -> Option<Rc<dyn IDockable>> {
        self.original_owner.borrow().resolve()
    }
    /// Sets `OriginalOwner`, notifying only when the value changes.
    pub fn set_original_owner(&self, value: Option<Rc<dyn IDockable>>) {
        self.runtime().raise_and_set_if_changed(
            &self.original_owner,
            WeakLink::new(value),
            "OriginalOwner",
        );
    }
    /// Gets `Factory`.
    pub fn factory(&self) -> Option<Rc<dyn IFactory>> {
        self.factory.borrow().resolve()
    }
    /// Sets `Factory`, notifying only when the value changes.
    pub fn set_factory(&self, value: Option<Rc<dyn IFactory>>) {
        self.runtime()
            .raise_and_set_if_changed(&self.factory, WeakLink::new(value), "Factory");
    }
    /// Gets `IsEmpty`.
    pub fn is_empty(&self) -> bool {
        *self.is_empty.borrow()
    }
    /// Sets `IsEmpty`, notifying only when the value changes.
    pub fn set_is_empty(&self, value: bool) {
        self.runtime()
            .raise_and_set_if_changed(&self.is_empty, value, "IsEmpty");
    }
    /// Gets `IsCollapsable`.
    pub fn is_collapsable(&self) -> bool {
        *self.is_collapsable.borrow()
    }
    /// Sets `IsCollapsable`, notifying only when the value changes.
    pub fn set_is_collapsable(&self, value: bool) {
        self.runtime()
            .raise_and_set_if_changed(&self.is_collapsable, value, "IsCollapsable");
    }
    /// Gets `Proportion`.
    pub fn proportion(&self) -> f64 {
        *self.proportion.borrow()
    }
    /// Sets `Proportion`, notifying only when the value changes.
    pub fn set_proportion(&self, value: f64) {
        let current = *self.proportion.borrow();
        if current == value || (current.is_nan() && value.is_nan()) {
            return;
        }
        self.runtime()
            .raise_and_set_if_changed(&self.proportion, value, "Proportion");
    }
    /// Gets `Dock`.
    pub fn dock(&self) -> DockMode {
        *self.dock.borrow()
    }
    /// Sets `Dock`, notifying only when the value changes.
    pub fn set_dock(&self, value: DockMode) {
        self.runtime()
            .raise_and_set_if_changed(&self.dock, value, "Dock");
    }
    /// Gets `Column`.
    pub fn column(&self) -> i32 {
        *self.column.borrow()
    }
    /// Sets `Column`, notifying only when the value changes.
    pub fn set_column(&self, value: i32) {
        self.runtime()
            .raise_and_set_if_changed(&self.column, value, "Column");
    }
    /// Gets `Row`.
    pub fn row(&self) -> i32 {
        *self.row.borrow()
    }
    /// Sets `Row`, notifying only when the value changes.
    pub fn set_row(&self, value: i32) {
        self.runtime()
            .raise_and_set_if_changed(&self.row, value, "Row");
    }
    /// Gets `ColumnSpan`.
    pub fn column_span(&self) -> i32 {
        *self.column_span.borrow()
    }
    /// Sets `ColumnSpan`, notifying only when the value changes.
    pub fn set_column_span(&self, value: i32) {
        self.runtime()
            .raise_and_set_if_changed(&self.column_span, value, "ColumnSpan");
    }
    /// Gets `RowSpan`.
    pub fn row_span(&self) -> i32 {
        *self.row_span.borrow()
    }
    /// Sets `RowSpan`, notifying only when the value changes.
    pub fn set_row_span(&self, value: i32) {
        self.runtime()
            .raise_and_set_if_changed(&self.row_span, value, "RowSpan");
    }
    /// Gets `IsSharedSizeScope`.
    pub fn is_shared_size_scope(&self) -> bool {
        *self.is_shared_size_scope.borrow()
    }
    /// Sets `IsSharedSizeScope`, notifying only when the value changes.
    pub fn set_is_shared_size_scope(&self, value: bool) {
        self.runtime().raise_and_set_if_changed(
            &self.is_shared_size_scope,
            value,
            "IsSharedSizeScope",
        );
    }
    /// Gets `CollapsedProportion`.
    pub fn collapsed_proportion(&self) -> f64 {
        *self.collapsed_proportion.borrow()
    }
    /// Sets `CollapsedProportion`, notifying only when the value changes.
    pub fn set_collapsed_proportion(&self, value: f64) {
        let current = *self.collapsed_proportion.borrow();
        if current == value || (current.is_nan() && value.is_nan()) {
            return;
        }
        self.runtime().raise_and_set_if_changed(
            &self.collapsed_proportion,
            value,
            "CollapsedProportion",
        );
    }
    /// Gets `MinWidth`.
    pub fn min_width(&self) -> f64 {
        *self.min_width.borrow()
    }
    /// Sets `MinWidth`, notifying only when the value changes.
    pub fn set_min_width(&self, value: f64) {
        let current = *self.min_width.borrow();
        if current == value || (current.is_nan() && value.is_nan()) {
            return;
        }
        self.runtime()
            .raise_and_set_if_changed(&self.min_width, value, "MinWidth");
    }
    /// Gets `MaxWidth`.
    pub fn max_width(&self) -> f64 {
        *self.max_width.borrow()
    }
    /// Sets `MaxWidth`, notifying only when the value changes.
    pub fn set_max_width(&self, value: f64) {
        let current = *self.max_width.borrow();
        if current == value || (current.is_nan() && value.is_nan()) {
            return;
        }
        self.runtime()
            .raise_and_set_if_changed(&self.max_width, value, "MaxWidth");
    }
    /// Gets `MinHeight`.
    pub fn min_height(&self) -> f64 {
        *self.min_height.borrow()
    }
    /// Sets `MinHeight`, notifying only when the value changes.
    pub fn set_min_height(&self, value: f64) {
        let current = *self.min_height.borrow();
        if current == value || (current.is_nan() && value.is_nan()) {
            return;
        }
        self.runtime()
            .raise_and_set_if_changed(&self.min_height, value, "MinHeight");
    }
    /// Gets `MaxHeight`.
    pub fn max_height(&self) -> f64 {
        *self.max_height.borrow()
    }
    /// Sets `MaxHeight`, notifying only when the value changes.
    pub fn set_max_height(&self, value: f64) {
        let current = *self.max_height.borrow();
        if current == value || (current.is_nan() && value.is_nan()) {
            return;
        }
        self.runtime()
            .raise_and_set_if_changed(&self.max_height, value, "MaxHeight");
    }
    /// Gets `CanClose`.
    pub fn can_close(&self) -> bool {
        *self.can_close.borrow()
    }
    /// Sets `CanClose`, notifying only when the value changes.
    pub fn set_can_close(&self, value: bool) {
        self.runtime()
            .raise_and_set_if_changed(&self.can_close, value, "CanClose");
    }
    /// Gets `CanPin`.
    pub fn can_pin(&self) -> bool {
        *self.can_pin.borrow()
    }
    /// Sets `CanPin`, notifying only when the value changes.
    pub fn set_can_pin(&self, value: bool) {
        self.runtime()
            .raise_and_set_if_changed(&self.can_pin, value, "CanPin");
    }
    /// Gets `KeepPinnedDockableVisible`.
    pub fn keep_pinned_dockable_visible(&self) -> bool {
        *self.keep_pinned_dockable_visible.borrow()
    }
    /// Sets `KeepPinnedDockableVisible`, notifying only when the value changes.
    pub fn set_keep_pinned_dockable_visible(&self, value: bool) {
        self.runtime().raise_and_set_if_changed(
            &self.keep_pinned_dockable_visible,
            value,
            "KeepPinnedDockableVisible",
        );
    }
    /// Gets `PinnedDockDisplayModeOverride`.
    pub fn pinned_dock_display_mode_override(&self) -> Option<PinnedDockDisplayMode> {
        *self.pinned_dock_display_mode_override.borrow()
    }
    /// Sets `PinnedDockDisplayModeOverride`, notifying only when the value changes.
    pub fn set_pinned_dock_display_mode_override(&self, value: Option<PinnedDockDisplayMode>) {
        self.runtime().raise_and_set_if_changed(
            &self.pinned_dock_display_mode_override,
            value,
            "PinnedDockDisplayModeOverride",
        );
    }
    /// Gets `CanFloat`.
    pub fn can_float(&self) -> bool {
        *self.can_float.borrow()
    }
    /// Sets `CanFloat`, notifying only when the value changes.
    pub fn set_can_float(&self, value: bool) {
        self.runtime()
            .raise_and_set_if_changed(&self.can_float, value, "CanFloat");
    }
    /// Gets `CanDrag`.
    pub fn can_drag(&self) -> bool {
        *self.can_drag.borrow()
    }
    /// Sets `CanDrag`, notifying only when the value changes.
    pub fn set_can_drag(&self, value: bool) {
        self.runtime()
            .raise_and_set_if_changed(&self.can_drag, value, "CanDrag");
    }
    /// Gets `CanDrop`.
    pub fn can_drop(&self) -> bool {
        *self.can_drop.borrow()
    }
    /// Sets `CanDrop`, notifying only when the value changes.
    pub fn set_can_drop(&self, value: bool) {
        self.runtime()
            .raise_and_set_if_changed(&self.can_drop, value, "CanDrop");
    }
    /// Gets `CanDockAsDocument`.
    pub fn can_dock_as_document(&self) -> bool {
        *self.can_dock_as_document.borrow()
    }
    /// Sets `CanDockAsDocument`, notifying only when the value changes.
    pub fn set_can_dock_as_document(&self, value: bool) {
        self.runtime().raise_and_set_if_changed(
            &self.can_dock_as_document,
            value,
            "CanDockAsDocument",
        );
    }
    /// Gets `DockCapabilityOverrides`.
    pub fn dock_capability_overrides(&self) -> Option<Rc<DockCapabilityOverrides>> {
        self.dock_capability_overrides.borrow().clone()
    }
    /// Sets `DockCapabilityOverrides`, notifying only when the value changes.
    pub fn set_dock_capability_overrides(&self, value: Option<Rc<DockCapabilityOverrides>>) {
        self.runtime().raise_and_set_if_changed(
            &self.dock_capability_overrides,
            value,
            "DockCapabilityOverrides",
        );
    }
    /// Gets `IsModified`.
    pub fn is_modified(&self) -> bool {
        *self.is_modified.borrow()
    }
    /// Sets `IsModified`, notifying only when the value changes.
    pub fn set_is_modified(&self, value: bool) {
        self.runtime()
            .raise_and_set_if_changed(&self.is_modified, value, "IsModified");
    }
    /// Gets `DockGroup`.
    pub fn dock_group(&self) -> Option<String> {
        self.dock_group.borrow().clone()
    }
    /// Sets `DockGroup`, notifying only when the value changes.
    pub fn set_dock_group(&self, value: Option<String>) {
        self.runtime()
            .raise_and_set_if_changed(&self.dock_group, value, "DockGroup");
    }
    /// Gets `AllowedDockOperations`.
    pub fn allowed_dock_operations(&self) -> DockOperationMask {
        *self.allowed_dock_operations.borrow()
    }
    /// Sets `AllowedDockOperations`, notifying only when the value changes.
    pub fn set_allowed_dock_operations(&self, value: DockOperationMask) {
        self.runtime().raise_and_set_if_changed(
            &self.allowed_dock_operations,
            value,
            "AllowedDockOperations",
        );
    }
    /// Gets `AllowedDropOperations`.
    pub fn allowed_drop_operations(&self) -> DockOperationMask {
        *self.allowed_drop_operations.borrow()
    }
    /// Sets `AllowedDropOperations`, notifying only when the value changes.
    pub fn set_allowed_drop_operations(&self, value: DockOperationMask) {
        self.runtime().raise_and_set_if_changed(
            &self.allowed_drop_operations,
            value,
            "AllowedDropOperations",
        );
    }
    /// Gets `ShowInSelector`.
    pub fn show_in_selector(&self) -> bool {
        *self.show_in_selector.borrow()
    }
    /// Sets `ShowInSelector`, notifying only when the value changes.
    pub fn set_show_in_selector(&self, value: bool) {
        self.runtime()
            .raise_and_set_if_changed(&self.show_in_selector, value, "ShowInSelector");
    }
    /// Gets `SelectorTitle`.
    pub fn selector_title(&self) -> Option<String> {
        self.selector_title.borrow().clone()
    }
    /// Sets `SelectorTitle`, notifying only when the value changes.
    pub fn set_selector_title(&self, value: Option<String>) {
        self.runtime()
            .raise_and_set_if_changed(&self.selector_title, value, "SelectorTitle");
    }
    /// Gets the logical docking state.
    pub fn docking_state(&self) -> DockingWindowState {
        *self.docking_state.borrow()
    }
    /// Sets the logical docking state and notifies a supporting layout factory.
    pub fn set_docking_state(&self, value: DockingWindowState) {
        if self.docking_state() == value {
            return;
        }
        self.runtime()
            .raise_and_set_if_changed(&self.docking_state, value, "DockingState");
        self.notify_docking_window_state_changed(DockingWindowStateProperty::DockingState);
    }
    /// Notifies the current factory after a lifecycle state property has changed.
    pub fn notify_docking_window_state_changed(&self, property: DockingWindowStateProperty) {
        let runtime = self.runtime();
        if runtime.as_docking_window_state().is_none() {
            return;
        }
        if let Some(sync) = runtime
            .factory()
            .and_then(|factory| factory.as_docking_window_state_sync())
        {
            sync.on_docking_window_state_property_changed(runtime, property);
        }
    }
    /// Gets the optional stored preview rectangle when both dimensions are finite.
    /// Negative or zero finite dimensions remain valid, matching the source.
    pub fn pinned_bounds(&self) -> Option<DockRect> {
        let (x, y, width, height) = self.get_pinned_bounds();
        if width.is_finite() && height.is_finite() {
            Some(DockRect::new(x, y, width, height))
        } else {
            None
        }
    }
    /// Sets stored preview bounds, or clears all coordinates to NaN.
    pub fn set_pinned_bounds_rect(&self, value: Option<DockRect>) {
        let value = value.unwrap_or(DockRect::new(f64::NAN, f64::NAN, f64::NAN, f64::NAN));
        self.set_pinned_bounds(value.x, value.y, value.width, value.height);
    }
    /// Gets the identifier used for control recycling.
    pub fn get_control_recycling_id(&self) -> Option<String> {
        Some(self.id())
    }
    /// Runs the enclosing model's virtual close callback.
    pub fn on_close(&self) -> bool {
        self.dispatch_on_close()
    }
    /// Runs the enclosing model's virtual selection callback.
    pub fn on_selected(&self) {
        self.dispatch_on_selected();
    }
    /// Gets the tracked `VisibleBounds` values.
    pub fn get_visible_bounds(&self) -> (f64, f64, f64, f64) {
        self.tracking.get_visible_bounds()
    }
    /// Stores `VisibleBounds` before invoking the enclosing model's virtual callback.
    pub fn set_visible_bounds(&self, x: f64, y: f64, width: f64, height: f64) {
        self.tracking.set_visible_bounds(x, y, width, height);
        self.dispatch_on_visible_bounds_changed(x, y, width, height);
    }
    /// Runs the enclosing model's `VisibleBounds` change callback.
    pub fn on_visible_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) {
        self.dispatch_on_visible_bounds_changed(x, y, width, height);
    }
    /// Gets the tracked `PinnedBounds` values.
    pub fn get_pinned_bounds(&self) -> (f64, f64, f64, f64) {
        self.tracking.get_pinned_bounds()
    }
    /// Stores `PinnedBounds` before invoking the enclosing model's virtual callback.
    pub fn set_pinned_bounds(&self, x: f64, y: f64, width: f64, height: f64) {
        self.tracking.set_pinned_bounds(x, y, width, height);
        self.dispatch_on_pinned_bounds_changed(x, y, width, height);
    }
    /// Runs the enclosing model's `PinnedBounds` change callback.
    pub fn on_pinned_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) {
        self.dispatch_on_pinned_bounds_changed(x, y, width, height);
    }
    /// Gets the tracked `TabBounds` values.
    pub fn get_tab_bounds(&self) -> (f64, f64, f64, f64) {
        self.tracking.get_tab_bounds()
    }
    /// Stores `TabBounds` before invoking the enclosing model's virtual callback.
    pub fn set_tab_bounds(&self, x: f64, y: f64, width: f64, height: f64) {
        self.tracking.set_tab_bounds(x, y, width, height);
        self.dispatch_on_tab_bounds_changed(x, y, width, height);
    }
    /// Runs the enclosing model's `TabBounds` change callback.
    pub fn on_tab_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) {
        self.dispatch_on_tab_bounds_changed(x, y, width, height);
    }
    /// Gets the tracked `PointerPosition` values.
    pub fn get_pointer_position(&self) -> (f64, f64) {
        self.tracking.get_pointer_position()
    }
    /// Stores `PointerPosition` before invoking the enclosing model's virtual callback.
    pub fn set_pointer_position(&self, x: f64, y: f64) {
        self.tracking.set_pointer_position(x, y);
        self.dispatch_on_pointer_position_changed(x, y);
    }
    /// Runs the enclosing model's `PointerPosition` change callback.
    pub fn on_pointer_position_changed(&self, x: f64, y: f64) {
        self.dispatch_on_pointer_position_changed(x, y);
    }
    /// Gets the tracked `PointerScreenPosition` values.
    pub fn get_pointer_screen_position(&self) -> (f64, f64) {
        self.tracking.get_pointer_screen_position()
    }
    /// Stores `PointerScreenPosition` before invoking the enclosing model's virtual callback.
    pub fn set_pointer_screen_position(&self, x: f64, y: f64) {
        self.tracking.set_pointer_screen_position(x, y);
        self.dispatch_on_pointer_screen_position_changed(x, y);
    }
    /// Runs the enclosing model's `PointerScreenPosition` change callback.
    pub fn on_pointer_screen_position_changed(&self, x: f64, y: f64) {
        self.dispatch_on_pointer_screen_position_changed(x, y);
    }
}

/// Implements the inherited dockable, selector, restriction, and recycling contracts.
/// The enclosing reactive model stores a `DockableBase` field named `base` and
/// implements `DockableBaseImpl`. Extra items override the optional type-test hooks.
#[macro_export]
macro_rules! impl_dockable {
    ($type:ty $(, $extra:item)*) => {
        $crate::impl_dockable!(@base $type, $crate::core::DockableBase $(, $extra)*);
    };
    (@base $type:ty, $base_type:ty $(, $extra:item)*) => {
        impl AsRef<$crate::__private::ReactiveObject> for $type {
            fn as_ref(&self) -> &$crate::__private::ReactiveObject { self.base.as_ref() }
        }
        impl $crate::__private::IDockable for $type {
            fn as_notify_property_changed(&self) -> Option<&dyn $crate::__private::INotifyPropertyChanged> { Some(self) }

            fn id(&self) -> String { self.base.id() }
            fn set_id(&self, value: String) { self.base.set_id(value) }
            fn title(&self) -> String { self.base.title() }
            fn set_title(&self, value: String) { self.base.set_title(value) }
            fn context(&self) -> Option<$crate::__private::BoxedValue> { self.base.context() }
            fn set_context(&self, value: Option<$crate::__private::BoxedValue>) { self.base.set_context(value) }
            fn owner(&self) -> Option<std::rc::Rc<dyn $crate::__private::IDockable>> { self.base.owner() }
            fn set_owner(&self, value: Option<std::rc::Rc<dyn $crate::__private::IDockable>>) { self.base.set_owner(value) }
            fn original_owner(&self) -> Option<std::rc::Rc<dyn $crate::__private::IDockable>> { self.base.original_owner() }
            fn set_original_owner(&self, value: Option<std::rc::Rc<dyn $crate::__private::IDockable>>) { self.base.set_original_owner(value) }
            fn factory(&self) -> Option<std::rc::Rc<dyn $crate::__private::IFactory>> { self.base.factory() }
            fn set_factory(&self, value: Option<std::rc::Rc<dyn $crate::__private::IFactory>>) { self.base.set_factory(value) }
            fn is_empty(&self) -> bool { self.base.is_empty() }
            fn set_is_empty(&self, value: bool) { self.base.set_is_empty(value) }
            fn is_collapsable(&self) -> bool { self.base.is_collapsable() }
            fn set_is_collapsable(&self, value: bool) { self.base.set_is_collapsable(value) }
            fn proportion(&self) -> f64 { self.base.proportion() }
            fn set_proportion(&self, value: f64) { self.base.set_proportion(value) }
            fn dock(&self) -> $crate::__private::DockMode { self.base.dock() }
            fn set_dock(&self, value: $crate::__private::DockMode) { self.base.set_dock(value) }
            fn docking_state(&self) -> $crate::__private::DockingWindowState { self.base.docking_state() }
            fn set_docking_state(&self, value: $crate::__private::DockingWindowState) { self.base.set_docking_state(value) }
            fn column(&self) -> i32 { self.base.column() }
            fn set_column(&self, value: i32) { self.base.set_column(value) }
            fn row(&self) -> i32 { self.base.row() }
            fn set_row(&self, value: i32) { self.base.set_row(value) }
            fn column_span(&self) -> i32 { self.base.column_span() }
            fn set_column_span(&self, value: i32) { self.base.set_column_span(value) }
            fn row_span(&self) -> i32 { self.base.row_span() }
            fn set_row_span(&self, value: i32) { self.base.set_row_span(value) }
            fn is_shared_size_scope(&self) -> bool { self.base.is_shared_size_scope() }
            fn set_is_shared_size_scope(&self, value: bool) { self.base.set_is_shared_size_scope(value) }
            fn collapsed_proportion(&self) -> f64 { self.base.collapsed_proportion() }
            fn set_collapsed_proportion(&self, value: f64) { self.base.set_collapsed_proportion(value) }
            fn min_width(&self) -> f64 { self.base.min_width() }
            fn set_min_width(&self, value: f64) { self.base.set_min_width(value) }
            fn max_width(&self) -> f64 { self.base.max_width() }
            fn set_max_width(&self, value: f64) { self.base.set_max_width(value) }
            fn min_height(&self) -> f64 { self.base.min_height() }
            fn set_min_height(&self, value: f64) { self.base.set_min_height(value) }
            fn max_height(&self) -> f64 { self.base.max_height() }
            fn set_max_height(&self, value: f64) { self.base.set_max_height(value) }
            fn can_close(&self) -> bool { self.base.can_close() }
            fn set_can_close(&self, value: bool) { self.base.set_can_close(value) }
            fn can_pin(&self) -> bool { self.base.can_pin() }
            fn set_can_pin(&self, value: bool) { self.base.set_can_pin(value) }
            fn keep_pinned_dockable_visible(&self) -> bool { self.base.keep_pinned_dockable_visible() }
            fn set_keep_pinned_dockable_visible(&self, value: bool) { self.base.set_keep_pinned_dockable_visible(value) }
            fn pinned_dock_display_mode_override(&self) -> Option<$crate::__private::PinnedDockDisplayMode> { self.base.pinned_dock_display_mode_override() }
            fn set_pinned_dock_display_mode_override(&self, value: Option<$crate::__private::PinnedDockDisplayMode>) { self.base.set_pinned_dock_display_mode_override(value) }
            fn pinned_bounds(&self) -> Option<$crate::__private::DockRect> { self.base.pinned_bounds() }
            fn set_pinned_bounds_rect(&self, value: Option<$crate::__private::DockRect>) { self.base.set_pinned_bounds_rect(value) }
            fn can_float(&self) -> bool { self.base.can_float() }
            fn set_can_float(&self, value: bool) { self.base.set_can_float(value) }
            fn can_drag(&self) -> bool { self.base.can_drag() }
            fn set_can_drag(&self, value: bool) { self.base.set_can_drag(value) }
            fn can_drop(&self) -> bool { self.base.can_drop() }
            fn set_can_drop(&self, value: bool) { self.base.set_can_drop(value) }
            fn can_dock_as_document(&self) -> bool { self.base.can_dock_as_document() }
            fn set_can_dock_as_document(&self, value: bool) { self.base.set_can_dock_as_document(value) }
            fn dock_capability_overrides(&self) -> Option<std::rc::Rc<$crate::__private::DockCapabilityOverrides>> { self.base.dock_capability_overrides() }
            fn set_dock_capability_overrides(&self, value: Option<std::rc::Rc<$crate::__private::DockCapabilityOverrides>>) { self.base.set_dock_capability_overrides(value) }
            fn is_modified(&self) -> bool { self.base.is_modified() }
            fn set_is_modified(&self, value: bool) { self.base.set_is_modified(value) }
            fn dock_group(&self) -> Option<String> { self.base.dock_group() }
            fn set_dock_group(&self, value: Option<String>) { self.base.set_dock_group(value) }
            fn on_close(&self) -> bool { self.base.dockable_base().dispatch_on_close() }
            fn on_selected(&self) { self.base.dockable_base().dispatch_on_selected() }
            fn get_visible_bounds(&self) -> (f64, f64, f64, f64) { self.base.get_visible_bounds() }
            fn set_visible_bounds(&self, x: f64, y: f64, width: f64, height: f64) { self.base.set_visible_bounds(x, y, width, height) }
            fn on_visible_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) { self.base.dockable_base().dispatch_on_visible_bounds_changed(x, y, width, height) }
            fn get_pinned_bounds(&self) -> (f64, f64, f64, f64) { self.base.get_pinned_bounds() }
            fn set_pinned_bounds(&self, x: f64, y: f64, width: f64, height: f64) { self.base.set_pinned_bounds(x, y, width, height) }
            fn on_pinned_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) { self.base.dockable_base().dispatch_on_pinned_bounds_changed(x, y, width, height) }
            fn get_tab_bounds(&self) -> (f64, f64, f64, f64) { self.base.get_tab_bounds() }
            fn set_tab_bounds(&self, x: f64, y: f64, width: f64, height: f64) { self.base.set_tab_bounds(x, y, width, height) }
            fn on_tab_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) { self.base.dockable_base().dispatch_on_tab_bounds_changed(x, y, width, height) }
            fn get_pointer_position(&self) -> (f64, f64) { self.base.get_pointer_position() }
            fn set_pointer_position(&self, x: f64, y: f64) { self.base.set_pointer_position(x, y) }
            fn on_pointer_position_changed(&self, x: f64, y: f64) { self.base.dockable_base().dispatch_on_pointer_position_changed(x, y) }
            fn get_pointer_screen_position(&self) -> (f64, f64) { self.base.get_pointer_screen_position() }
            fn set_pointer_screen_position(&self, x: f64, y: f64) { self.base.set_pointer_screen_position(x, y) }
            fn on_pointer_screen_position_changed(&self, x: f64, y: f64) { self.base.dockable_base().dispatch_on_pointer_screen_position_changed(x, y) }
            fn as_dock_selector_info(&self) -> Option<&dyn $crate::__private::IDockSelectorInfo> { Some(self) }
            fn as_docking_restrictions(&self) -> Option<&dyn $crate::__private::IDockableDockingRestrictions> { Some(self) }
            $($extra)*
        }
        impl $crate::__private::IControlRecyclingIdProvider for $type {
            fn get_control_recycling_id(&self) -> Option<String> { self.base.get_control_recycling_id() }
        }
        impl $crate::__private::IDockSelectorInfo for $type {
            fn show_in_selector(&self) -> bool { self.base.show_in_selector() }
            fn set_show_in_selector(&self, value: bool) { self.base.set_show_in_selector(value); }
            fn selector_title(&self) -> Option<String> { self.base.selector_title() }
            fn set_selector_title(&self, value: Option<String>) { self.base.set_selector_title(value); }
        }
        impl $crate::__private::IDockableDockingRestrictions for $type {
            fn allowed_dock_operations(&self) -> $crate::__private::DockOperationMask { self.base.allowed_dock_operations() }
            fn set_allowed_dock_operations(&self, value: $crate::__private::DockOperationMask) { self.base.set_allowed_dock_operations(value); }
            fn allowed_drop_operations(&self) -> $crate::__private::DockOperationMask { self.base.allowed_drop_operations() }
            fn set_allowed_drop_operations(&self, value: $crate::__private::DockOperationMask) { self.base.set_allowed_drop_operations(value); }
        }
        impl std::ops::Deref for $type {
            type Target = $base_type;
            fn deref(&self) -> &Self::Target { &self.base }
        }
        impl PartialEq for $type {
            fn eq(&self, other: &Self) -> bool { std::ptr::eq(self, other) }
        }
    };
}
