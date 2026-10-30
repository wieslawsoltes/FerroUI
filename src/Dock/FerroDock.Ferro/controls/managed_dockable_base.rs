// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::{adapters::TrackingAdapter, core::*, BoxedValue};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::any::Any;
use std::cell::{OnceCell, RefCell};
use std::rc::{Rc, Weak};

/// Object-safe callback dispatch for composed model components.
/// Implemented automatically from ManagedDockableBaseImpl's typed overrides.
#[doc(hidden)]
pub trait ManagedDockableBaseCallbacks {
    /// Dispatches the source virtual property notification.
    fn invoke_on_property_changed(&self, name: &str);
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
impl<T: ManagedDockableBaseImpl> ManagedDockableBaseCallbacks for T {
    fn invoke_on_property_changed(&self, name: &str) {
        <T as ManagedDockableBaseImpl>::on_property_changed(self, name);
    }
    fn invoke_on_close(&self) -> bool {
        <T as ManagedDockableBaseImpl>::on_close(self)
    }
    fn invoke_on_selected(&self) {
        <T as ManagedDockableBaseImpl>::on_selected(self)
    }
    fn invoke_on_visible_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) {
        <T as ManagedDockableBaseImpl>::on_visible_bounds_changed(self, x, y, width, height)
    }
    fn invoke_on_pinned_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) {
        <T as ManagedDockableBaseImpl>::on_pinned_bounds_changed(self, x, y, width, height)
    }
    fn invoke_on_tab_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) {
        <T as ManagedDockableBaseImpl>::on_tab_bounds_changed(self, x, y, width, height)
    }
    fn invoke_on_pointer_position_changed(&self, x: f64, y: f64) {
        <T as ManagedDockableBaseImpl>::on_pointer_position_changed(self, x, y)
    }
    fn invoke_on_pointer_screen_position_changed(&self, x: f64, y: f64) {
        <T as ManagedDockableBaseImpl>::on_pointer_screen_position_changed(self, x, y)
    }
}

/// Virtual behavior of a concrete model composed from ManagedDockableBase.
/// Implementations inherit the source defaults and override only their custom behavior.
pub trait ManagedDockableBaseImpl:
    IDockable
    + IDockSelectorInfo
    + IDockableDockingRestrictions
    + INotifyPropertyChanged
    + AsRef<ManagedDockableBase>
    + ManagedDockableBaseCallbacks
{
    /// Raises the original event. Derived implementations call this base body first.
    fn on_property_changed(this: &Self, name: &str)
    where
        Self: Sized,
    {
        this.as_ref().raise_property_changed(name);
    }
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

#[derive(Clone)]
pub(super) struct ContextValue(pub(super) Option<BoxedValue>);
impl PartialEq for ContextValue {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                if ferrodock_model::value_semantics::boxed_values_equal(a, b) {
                    return true;
                }
                let left = ferrodock_controls_recycling::from_recycling_value(a);
                let right = ferrodock_controls_recycling::from_recycling_value(b);
                if let (Some(a), Some(b)) =
                    (left.downcast_ref::<f64>(), right.downcast_ref::<f64>())
                {
                    return a == b || (a.is_nan() && b.is_nan());
                }
                if let (Some(a), Some(b)) =
                    (left.downcast_ref::<f32>(), right.downcast_ref::<f32>())
                {
                    return a == b || (a.is_nan() && b.is_nan());
                }
                match (
                    ferroui_base::data::core::ValueTypes::as_object(&*left),
                    ferroui_base::data::core::ValueTypes::as_object(&*right),
                ) {
                    (Some(a), Some(b)) => a.ptr_eq(&b),
                    _ => (*left).value_eq(&*right),
                }
            }
            _ => false,
        }
    }
}
/// Original abstract managed dockable state, composed into concrete managed models.
/// Notifications and virtual callbacks keep the enclosing model's identity. Owner,
/// original owner, factory and context retain their original strong references.
pub struct ManagedDockableBase {
    runtime: Weak<dyn ManagedDockableBaseImpl>,
    property_changed: Event<str>,
    tracking_adapter: OnceCell<Rc<TrackingAdapter>>,
    id: RefCell<String>,
    title: RefCell<String>,
    context: RefCell<ContextValue>,
    owner: RefCell<Option<Rc<dyn IDockable>>>,
    original_owner: RefCell<Option<Rc<dyn IDockable>>>,
    factory: RefCell<Option<Rc<dyn IFactory>>>,
    is_empty: RefCell<bool>,
    is_collapsable: RefCell<bool>,
    proportion: RefCell<f64>,
    dock: RefCell<DockMode>,
    docking_state: RefCell<DockingWindowState>,
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
impl ManagedDockableBase {
    /// Initializes the abstract base with the enclosing model identity.
    pub fn construct(runtime: Weak<dyn ManagedDockableBaseImpl>) -> Self {
        Self {
            runtime,
            property_changed: Event::new(),
            tracking_adapter: OnceCell::new(),
            id: RefCell::new(String::new()),
            title: RefCell::new(String::new()),
            context: RefCell::new(ContextValue(None)),
            owner: RefCell::new(None),
            original_owner: RefCell::new(None),
            factory: RefCell::new(None),
            is_empty: RefCell::new(false),
            is_collapsable: RefCell::new(true),
            proportion: RefCell::new(f64::NAN),
            dock: RefCell::new(DockMode::Center),
            docking_state: RefCell::new(DockingWindowState::DOCKED),
            column: RefCell::new(0),
            row: RefCell::new(0),
            column_span: RefCell::new(1),
            row_span: RefCell::new(1),
            is_shared_size_scope: RefCell::new(false),
            collapsed_proportion: RefCell::new(f64::NAN),
            min_width: RefCell::new(f64::NAN),
            max_width: RefCell::new(f64::NAN),
            min_height: RefCell::new(f64::NAN),
            max_height: RefCell::new(f64::NAN),
            can_close: RefCell::new(true),
            can_pin: RefCell::new(true),
            keep_pinned_dockable_visible: RefCell::new(false),
            pinned_dock_display_mode_override: RefCell::new(None),
            can_float: RefCell::new(true),
            can_drag: RefCell::new(true),
            can_drop: RefCell::new(true),
            can_dock_as_document: RefCell::new(true),
            dock_capability_overrides: RefCell::new(None),
            is_modified: RefCell::new(false),
            dock_group: RefCell::new(None),
            allowed_dock_operations: RefCell::new(DockOperationMask::ALL),
            allowed_drop_operations: RefCell::new(DockOperationMask::ALL),
            show_in_selector: RefCell::new(true),
            selector_title: RefCell::new(None),
        }
    }
    fn runtime(&self) -> Rc<dyn ManagedDockableBaseImpl> {
        self.runtime
            .upgrade()
            .expect("the enclosing managed model is alive while its base is borrowed")
    }
    fn tracking_adapter(&self) -> &TrackingAdapter {
        self.tracking_adapter.get_or_init(TrackingAdapter::new)
    }
    /// Raises the base PropertyChanged event without repeating virtual dispatch.
    /// Call this from a derived OnPropertyChanged override to preserve base ordering.
    pub fn raise_property_changed(&self, property_name: &str) {
        self.property_changed.raise(property_name);
    }
    /// Sets a source backing field and invokes the original virtual notification
    /// only when its value changes. None explicitly names the empty property.
    /// All source property setters supply their original caller-member name.
    pub fn set_property<T: Clone + PartialEq + 'static>(
        &self,
        field: &RefCell<T>,
        value: T,
        property_name: Option<&str>,
    ) -> bool {
        let previous = field.borrow().clone();
        if source_equals(&previous, &value) {
            return false;
        }
        let replaced = field.replace(value);
        drop(replaced);
        drop(previous);
        self.runtime()
            .invoke_on_property_changed(property_name.unwrap_or(""));
        true
    }
    /// Invokes the source virtual PropertyChanged method for the enclosing model.
    pub fn on_property_changed(&self, name: &str) {
        self.runtime().invoke_on_property_changed(name);
    }
    /// Gets the abstract base part for original inherited contract implementations.
    #[doc(hidden)]
    pub fn managed_dockable_base(&self) -> &Self {
        self
    }
    /// Gets `Id`.
    pub fn id(&self) -> String {
        self.id.borrow().clone()
    }
    /// Sets `Id`, notifying only when the original value changes.
    pub fn set_id(&self, value: String) {
        self.set_property(&self.id, value, Some("Id"));
    }
    /// Gets `Title`.
    pub fn title(&self) -> String {
        self.title.borrow().clone()
    }
    /// Sets `Title`, notifying only when the original value changes.
    pub fn set_title(&self, value: String) {
        self.set_property(&self.title, value, Some("Title"));
    }
    /// Gets the original Context object.
    pub fn context(&self) -> Option<BoxedValue> {
        self.context.borrow().0.clone()
    }
    /// Sets Context using the original boxed object equality boundary.
    pub fn set_context(&self, value: Option<BoxedValue>) {
        self.set_property(&self.context, ContextValue(value), Some("Context"));
    }
    /// Gets `Owner`.
    pub fn owner(&self) -> Option<Rc<dyn IDockable>> {
        self.owner.borrow().clone()
    }
    /// Sets `Owner`, notifying only when the original value changes.
    pub fn set_owner(&self, value: Option<Rc<dyn IDockable>>) {
        self.set_property(&self.owner, value, Some("Owner"));
    }
    /// Gets `OriginalOwner`.
    pub fn original_owner(&self) -> Option<Rc<dyn IDockable>> {
        self.original_owner.borrow().clone()
    }
    /// Sets `OriginalOwner`, notifying only when the original value changes.
    pub fn set_original_owner(&self, value: Option<Rc<dyn IDockable>>) {
        self.set_property(&self.original_owner, value, Some("OriginalOwner"));
    }
    /// Gets `Factory`.
    pub fn factory(&self) -> Option<Rc<dyn IFactory>> {
        self.factory.borrow().clone()
    }
    /// Sets `Factory`, notifying only when the original value changes.
    pub fn set_factory(&self, value: Option<Rc<dyn IFactory>>) {
        self.set_property(&self.factory, value, Some("Factory"));
    }
    /// Gets `IsEmpty`.
    pub fn is_empty(&self) -> bool {
        *self.is_empty.borrow()
    }
    /// Sets `IsEmpty`, notifying only when the original value changes.
    pub fn set_is_empty(&self, value: bool) {
        self.set_property(&self.is_empty, value, Some("IsEmpty"));
    }
    /// Gets `IsCollapsable`.
    pub fn is_collapsable(&self) -> bool {
        *self.is_collapsable.borrow()
    }
    /// Sets `IsCollapsable`, notifying only when the original value changes.
    pub fn set_is_collapsable(&self, value: bool) {
        self.set_property(&self.is_collapsable, value, Some("IsCollapsable"));
    }
    /// Gets `Proportion`.
    pub fn proportion(&self) -> f64 {
        *self.proportion.borrow()
    }
    /// Sets `Proportion`, notifying only when the original value changes.
    pub fn set_proportion(&self, value: f64) {
        self.set_property(&self.proportion, value, Some("Proportion"));
    }
    /// Gets `Dock`.
    pub fn dock(&self) -> DockMode {
        *self.dock.borrow()
    }
    /// Sets `Dock`, notifying only when the original value changes.
    pub fn set_dock(&self, value: DockMode) {
        self.set_property(&self.dock, value, Some("Dock"));
    }
    /// Gets `DockingState`.
    pub fn docking_state(&self) -> DockingWindowState {
        *self.docking_state.borrow()
    }
    /// Sets `DockingState`, notifying only when the original value changes.
    pub fn set_docking_state(&self, value: DockingWindowState) {
        self.set_property(&self.docking_state, value, Some("DockingState"));
    }
    /// Gets `Column`.
    pub fn column(&self) -> i32 {
        *self.column.borrow()
    }
    /// Sets `Column`, notifying only when the original value changes.
    pub fn set_column(&self, value: i32) {
        self.set_property(&self.column, value, Some("Column"));
    }
    /// Gets `Row`.
    pub fn row(&self) -> i32 {
        *self.row.borrow()
    }
    /// Sets `Row`, notifying only when the original value changes.
    pub fn set_row(&self, value: i32) {
        self.set_property(&self.row, value, Some("Row"));
    }
    /// Gets `ColumnSpan`.
    pub fn column_span(&self) -> i32 {
        *self.column_span.borrow()
    }
    /// Sets `ColumnSpan`, notifying only when the original value changes.
    pub fn set_column_span(&self, value: i32) {
        self.set_property(&self.column_span, value, Some("ColumnSpan"));
    }
    /// Gets `RowSpan`.
    pub fn row_span(&self) -> i32 {
        *self.row_span.borrow()
    }
    /// Sets `RowSpan`, notifying only when the original value changes.
    pub fn set_row_span(&self, value: i32) {
        self.set_property(&self.row_span, value, Some("RowSpan"));
    }
    /// Gets `IsSharedSizeScope`.
    pub fn is_shared_size_scope(&self) -> bool {
        *self.is_shared_size_scope.borrow()
    }
    /// Sets `IsSharedSizeScope`, notifying only when the original value changes.
    pub fn set_is_shared_size_scope(&self, value: bool) {
        self.set_property(&self.is_shared_size_scope, value, Some("IsSharedSizeScope"));
    }
    /// Gets `CollapsedProportion`.
    pub fn collapsed_proportion(&self) -> f64 {
        *self.collapsed_proportion.borrow()
    }
    /// Sets `CollapsedProportion`, notifying only when the original value changes.
    pub fn set_collapsed_proportion(&self, value: f64) {
        self.set_property(
            &self.collapsed_proportion,
            value,
            Some("CollapsedProportion"),
        );
    }
    /// Gets `MinWidth`.
    pub fn min_width(&self) -> f64 {
        *self.min_width.borrow()
    }
    /// Sets `MinWidth`, notifying only when the original value changes.
    pub fn set_min_width(&self, value: f64) {
        self.set_property(&self.min_width, value, Some("MinWidth"));
    }
    /// Gets `MaxWidth`.
    pub fn max_width(&self) -> f64 {
        *self.max_width.borrow()
    }
    /// Sets `MaxWidth`, notifying only when the original value changes.
    pub fn set_max_width(&self, value: f64) {
        self.set_property(&self.max_width, value, Some("MaxWidth"));
    }
    /// Gets `MinHeight`.
    pub fn min_height(&self) -> f64 {
        *self.min_height.borrow()
    }
    /// Sets `MinHeight`, notifying only when the original value changes.
    pub fn set_min_height(&self, value: f64) {
        self.set_property(&self.min_height, value, Some("MinHeight"));
    }
    /// Gets `MaxHeight`.
    pub fn max_height(&self) -> f64 {
        *self.max_height.borrow()
    }
    /// Sets `MaxHeight`, notifying only when the original value changes.
    pub fn set_max_height(&self, value: f64) {
        self.set_property(&self.max_height, value, Some("MaxHeight"));
    }
    /// Gets `CanClose`.
    pub fn can_close(&self) -> bool {
        *self.can_close.borrow()
    }
    /// Sets `CanClose`, notifying only when the original value changes.
    pub fn set_can_close(&self, value: bool) {
        self.set_property(&self.can_close, value, Some("CanClose"));
    }
    /// Gets `CanPin`.
    pub fn can_pin(&self) -> bool {
        *self.can_pin.borrow()
    }
    /// Sets `CanPin`, notifying only when the original value changes.
    pub fn set_can_pin(&self, value: bool) {
        self.set_property(&self.can_pin, value, Some("CanPin"));
    }
    /// Gets `KeepPinnedDockableVisible`.
    pub fn keep_pinned_dockable_visible(&self) -> bool {
        *self.keep_pinned_dockable_visible.borrow()
    }
    /// Sets `KeepPinnedDockableVisible`, notifying only when the original value changes.
    pub fn set_keep_pinned_dockable_visible(&self, value: bool) {
        self.set_property(
            &self.keep_pinned_dockable_visible,
            value,
            Some("KeepPinnedDockableVisible"),
        );
    }
    /// Gets `PinnedDockDisplayModeOverride`.
    pub fn pinned_dock_display_mode_override(&self) -> Option<PinnedDockDisplayMode> {
        *self.pinned_dock_display_mode_override.borrow()
    }
    /// Sets `PinnedDockDisplayModeOverride`, notifying only when the original value changes.
    pub fn set_pinned_dock_display_mode_override(&self, value: Option<PinnedDockDisplayMode>) {
        self.set_property(
            &self.pinned_dock_display_mode_override,
            value,
            Some("PinnedDockDisplayModeOverride"),
        );
    }
    /// Gets `CanFloat`.
    pub fn can_float(&self) -> bool {
        *self.can_float.borrow()
    }
    /// Sets `CanFloat`, notifying only when the original value changes.
    pub fn set_can_float(&self, value: bool) {
        self.set_property(&self.can_float, value, Some("CanFloat"));
    }
    /// Gets `CanDrag`.
    pub fn can_drag(&self) -> bool {
        *self.can_drag.borrow()
    }
    /// Sets `CanDrag`, notifying only when the original value changes.
    pub fn set_can_drag(&self, value: bool) {
        self.set_property(&self.can_drag, value, Some("CanDrag"));
    }
    /// Gets `CanDrop`.
    pub fn can_drop(&self) -> bool {
        *self.can_drop.borrow()
    }
    /// Sets `CanDrop`, notifying only when the original value changes.
    pub fn set_can_drop(&self, value: bool) {
        self.set_property(&self.can_drop, value, Some("CanDrop"));
    }
    /// Gets `CanDockAsDocument`.
    pub fn can_dock_as_document(&self) -> bool {
        *self.can_dock_as_document.borrow()
    }
    /// Sets `CanDockAsDocument`, notifying only when the original value changes.
    pub fn set_can_dock_as_document(&self, value: bool) {
        self.set_property(&self.can_dock_as_document, value, Some("CanDockAsDocument"));
    }
    /// Gets `DockCapabilityOverrides`.
    pub fn dock_capability_overrides(&self) -> Option<Rc<DockCapabilityOverrides>> {
        self.dock_capability_overrides.borrow().clone()
    }
    /// Sets `DockCapabilityOverrides`, notifying only when the original value changes.
    pub fn set_dock_capability_overrides(&self, value: Option<Rc<DockCapabilityOverrides>>) {
        self.set_property(
            &self.dock_capability_overrides,
            value,
            Some("DockCapabilityOverrides"),
        );
    }
    /// Gets `IsModified`.
    pub fn is_modified(&self) -> bool {
        *self.is_modified.borrow()
    }
    /// Sets `IsModified`, notifying only when the original value changes.
    pub fn set_is_modified(&self, value: bool) {
        self.set_property(&self.is_modified, value, Some("IsModified"));
    }
    /// Gets `DockGroup`.
    pub fn dock_group(&self) -> Option<String> {
        self.dock_group.borrow().clone()
    }
    /// Sets `DockGroup`, notifying only when the original value changes.
    pub fn set_dock_group(&self, value: Option<String>) {
        self.set_property(&self.dock_group, value, Some("DockGroup"));
    }
    /// Gets `AllowedDockOperations`.
    pub fn allowed_dock_operations(&self) -> DockOperationMask {
        *self.allowed_dock_operations.borrow()
    }
    /// Sets `AllowedDockOperations`, notifying only when the original value changes.
    pub fn set_allowed_dock_operations(&self, value: DockOperationMask) {
        self.set_property(
            &self.allowed_dock_operations,
            value,
            Some("AllowedDockOperations"),
        );
    }
    /// Gets `AllowedDropOperations`.
    pub fn allowed_drop_operations(&self) -> DockOperationMask {
        *self.allowed_drop_operations.borrow()
    }
    /// Sets `AllowedDropOperations`, notifying only when the original value changes.
    pub fn set_allowed_drop_operations(&self, value: DockOperationMask) {
        self.set_property(
            &self.allowed_drop_operations,
            value,
            Some("AllowedDropOperations"),
        );
    }
    /// Gets `ShowInSelector`.
    pub fn show_in_selector(&self) -> bool {
        *self.show_in_selector.borrow()
    }
    /// Sets `ShowInSelector`, notifying only when the original value changes.
    pub fn set_show_in_selector(&self, value: bool) {
        self.set_property(&self.show_in_selector, value, Some("ShowInSelector"));
    }
    /// Gets `SelectorTitle`.
    pub fn selector_title(&self) -> Option<String> {
        self.selector_title.borrow().clone()
    }
    /// Sets `SelectorTitle`, notifying only when the original value changes.
    pub fn set_selector_title(&self, value: Option<String>) {
        self.set_property(&self.selector_title, value, Some("SelectorTitle"));
    }
    /// Gets the stored pinned rectangle when both dimensions are finite.
    /// Positions and negative or zero dimensions retain source behavior.
    pub fn pinned_bounds(&self) -> Option<DockRect> {
        let (x, y, width, height) = self.get_pinned_bounds();
        if width.is_finite() && height.is_finite() {
            Some(DockRect::new(x, y, width, height))
        } else {
            None
        }
    }
    /// Sets the original optional pinned rectangle; None clears all coordinates to NaN.
    pub fn set_pinned_bounds_rect(&self, value: Option<DockRect>) {
        if let Some(value) = value {
            self.set_pinned_bounds(value.x, value.y, value.width, value.height);
        } else {
            self.set_pinned_bounds(f64::NAN, f64::NAN, f64::NAN, f64::NAN);
        }
    }
    /// Returns the current source Id for control recycling, including empty strings.
    pub fn get_control_recycling_id(&self) -> Option<String> {
        Some(self.id())
    }
    /// Dispatches the original close callback; the abstract base accepts closing.
    pub fn on_close(&self) -> bool {
        self.runtime().invoke_on_close()
    }
    /// Dispatches the original selection callback.
    pub fn on_selected(&self) {
        self.runtime().invoke_on_selected();
    }
    /// Gets the original tracked visible bounds.
    pub fn get_visible_bounds(&self) -> (f64, f64, f64, f64) {
        self.tracking_adapter().get_visible_bounds()
    }
    /// Stores visible bounds before invoking the original virtual callback.
    pub fn set_visible_bounds(&self, x: f64, y: f64, width: f64, height: f64) {
        self.tracking_adapter()
            .set_visible_bounds(x, y, width, height);
        self.runtime()
            .invoke_on_visible_bounds_changed(x, y, width, height);
    }
    /// Dispatches the original virtual visible bounds callback.
    pub fn on_visible_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) {
        self.runtime()
            .invoke_on_visible_bounds_changed(x, y, width, height);
    }
    /// Gets the original tracked pinned bounds.
    pub fn get_pinned_bounds(&self) -> (f64, f64, f64, f64) {
        self.tracking_adapter().get_pinned_bounds()
    }
    /// Stores pinned bounds before invoking the original virtual callback.
    pub fn set_pinned_bounds(&self, x: f64, y: f64, width: f64, height: f64) {
        self.tracking_adapter()
            .set_pinned_bounds(x, y, width, height);
        self.runtime()
            .invoke_on_pinned_bounds_changed(x, y, width, height);
    }
    /// Dispatches the original virtual pinned bounds callback.
    pub fn on_pinned_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) {
        self.runtime()
            .invoke_on_pinned_bounds_changed(x, y, width, height);
    }
    /// Gets the original tracked tab bounds.
    pub fn get_tab_bounds(&self) -> (f64, f64, f64, f64) {
        self.tracking_adapter().get_tab_bounds()
    }
    /// Stores tab bounds before invoking the original virtual callback.
    pub fn set_tab_bounds(&self, x: f64, y: f64, width: f64, height: f64) {
        self.tracking_adapter().set_tab_bounds(x, y, width, height);
        self.runtime()
            .invoke_on_tab_bounds_changed(x, y, width, height);
    }
    /// Dispatches the original virtual tab bounds callback.
    pub fn on_tab_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) {
        self.runtime()
            .invoke_on_tab_bounds_changed(x, y, width, height);
    }
    /// Gets the original tracked pointer position.
    pub fn get_pointer_position(&self) -> (f64, f64) {
        self.tracking_adapter().get_pointer_position()
    }
    /// Stores pointer position before invoking the original virtual callback.
    pub fn set_pointer_position(&self, x: f64, y: f64) {
        self.tracking_adapter().set_pointer_position(x, y);
        self.runtime().invoke_on_pointer_position_changed(x, y);
    }
    /// Dispatches the original virtual pointer position callback.
    pub fn on_pointer_position_changed(&self, x: f64, y: f64) {
        self.runtime().invoke_on_pointer_position_changed(x, y);
    }
    /// Gets the original tracked pointer screen position.
    pub fn get_pointer_screen_position(&self) -> (f64, f64) {
        self.tracking_adapter().get_pointer_screen_position()
    }
    /// Stores pointer screen position before invoking the original virtual callback.
    pub fn set_pointer_screen_position(&self, x: f64, y: f64) {
        self.tracking_adapter().set_pointer_screen_position(x, y);
        self.runtime()
            .invoke_on_pointer_screen_position_changed(x, y);
    }
    /// Dispatches the original virtual pointer screen position callback.
    pub fn on_pointer_screen_position_changed(&self, x: f64, y: f64) {
        self.runtime()
            .invoke_on_pointer_screen_position_changed(x, y);
    }
}
impl INotifyPropertyChanged for ManagedDockableBase {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}
fn source_equals<T: PartialEq + 'static>(left: &T, right: &T) -> bool {
    if let (Some(a), Some(b)) = (
        (left as &dyn Any).downcast_ref::<f64>(),
        (right as &dyn Any).downcast_ref::<f64>(),
    ) {
        return a == b || (a.is_nan() && b.is_nan());
    }
    if let (Some(a), Some(b)) = (
        (left as &dyn Any).downcast_ref::<f32>(),
        (right as &dyn Any).downcast_ref::<f32>(),
    ) {
        return a == b || (a.is_nan() && b.is_nan());
    }
    left == right
}
impl PartialEq for dyn ManagedDockableBaseImpl {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

/// Implements the inherited dockable, selector, restriction, and recycling contracts.
/// The enclosing managed model stores a `ManagedDockableBase` field named `base` and
/// implements `ManagedDockableBaseImpl`. Extra items override the optional type-test hooks.
#[macro_export]
macro_rules! impl_managed_dockable {
    ($type:ty $(, $extra:item)*) => {
        $crate::impl_managed_dockable!(@base $type, $crate::controls::ManagedDockableBase $(, $extra)*);
    };
    (@base $type:ty, $base_type:ty $(, $extra:item)*) => {
        impl AsRef<$crate::controls::ManagedDockableBase> for $type {
            fn as_ref(&self) -> &$crate::controls::ManagedDockableBase { self.base.managed_dockable_base() }
        }
        impl $crate::__private::INotifyPropertyChanged for $type {
            fn property_changed(&self) -> &$crate::__private::Event<str> { $crate::__private::INotifyPropertyChanged::property_changed(&self.base) }
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
            fn on_close(&self) -> bool { self.base.managed_dockable_base().on_close() }
            fn on_selected(&self) { self.base.managed_dockable_base().on_selected() }
            fn get_visible_bounds(&self) -> (f64, f64, f64, f64) { self.base.get_visible_bounds() }
            fn set_visible_bounds(&self, x: f64, y: f64, width: f64, height: f64) { self.base.set_visible_bounds(x, y, width, height) }
            fn on_visible_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) { self.base.managed_dockable_base().on_visible_bounds_changed(x, y, width, height) }
            fn get_pinned_bounds(&self) -> (f64, f64, f64, f64) { self.base.get_pinned_bounds() }
            fn set_pinned_bounds(&self, x: f64, y: f64, width: f64, height: f64) { self.base.set_pinned_bounds(x, y, width, height) }
            fn on_pinned_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) { self.base.managed_dockable_base().on_pinned_bounds_changed(x, y, width, height) }
            fn get_tab_bounds(&self) -> (f64, f64, f64, f64) { self.base.get_tab_bounds() }
            fn set_tab_bounds(&self, x: f64, y: f64, width: f64, height: f64) { self.base.set_tab_bounds(x, y, width, height) }
            fn on_tab_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64) { self.base.managed_dockable_base().on_tab_bounds_changed(x, y, width, height) }
            fn get_pointer_position(&self) -> (f64, f64) { self.base.get_pointer_position() }
            fn set_pointer_position(&self, x: f64, y: f64) { self.base.set_pointer_position(x, y) }
            fn on_pointer_position_changed(&self, x: f64, y: f64) { self.base.managed_dockable_base().on_pointer_position_changed(x, y) }
            fn get_pointer_screen_position(&self) -> (f64, f64) { self.base.get_pointer_screen_position() }
            fn set_pointer_screen_position(&self, x: f64, y: f64) { self.base.set_pointer_screen_position(x, y) }
            fn on_pointer_screen_position_changed(&self, x: f64, y: f64) { self.base.managed_dockable_base().on_pointer_screen_position_changed(x, y) }
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

/// A native notification-capable reference to the original managed model.
/// This only carries its Rc identity across the native boxed-value boundary;
/// all state and callbacks remain on the same original model.
#[derive(Clone)]
pub struct ManagedDockableBaseReference(Rc<dyn ManagedDockableBaseImpl>);
impl ManagedDockableBaseReference {
    /// Carries the original managed model without constructing a replacement model.
    pub fn new(model: Rc<dyn ManagedDockableBaseImpl>) -> Self {
        Self(model)
    }
    /// Gets the same original managed model.
    pub fn model(&self) -> Rc<dyn ManagedDockableBaseImpl> {
        self.0.clone()
    }
}
impl std::ops::Deref for ManagedDockableBaseReference {
    type Target = dyn ManagedDockableBaseImpl;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
impl PartialEq for ManagedDockableBaseReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl INotifyPropertyChanged for ManagedDockableBaseReference {
    fn property_changed(&self) -> &Event<str> {
        self.0.property_changed()
    }
}

ferroui_base::ferro_markup_type!(class ManagedDockableBase {
    this: ManagedDockableBaseReference,
    handles: [ManagedDockableBaseReference, Rc<dyn ManagedDockableBaseImpl>, Option<Rc<dyn ManagedDockableBaseImpl>>],
    notify_property_changed: ManagedDockableBaseReference,
    properties: [
        Id: String {get: |this: &ManagedDockableBaseReference| this.id(), set: |this: &ManagedDockableBaseReference, value| this.set_id(value)},
        Title: String {get: |this: &ManagedDockableBaseReference| this.title(), set: |this: &ManagedDockableBaseReference, value| this.set_title(value)},
        Context: Option<ferroui_base::BoxedValue> {
            get: |this: &ManagedDockableBaseReference| this.context().as_ref().map(ferrodock_controls_recycling::from_recycling_value),
            set: |this: &ManagedDockableBaseReference, value: Option<ferroui_base::BoxedValue>| this.set_context(value.map(ferrodock_controls_recycling::to_recycling_value))
        },
        Owner: Option<Rc<dyn IDockable>> {get: |this: &ManagedDockableBaseReference| this.owner(), set: |this: &ManagedDockableBaseReference, value| this.set_owner(value)},
        OriginalOwner: Option<Rc<dyn IDockable>> {get: |this: &ManagedDockableBaseReference| this.original_owner(), set: |this: &ManagedDockableBaseReference, value| this.set_original_owner(value)},
        Factory: Option<Rc<dyn IFactory>> {get: |this: &ManagedDockableBaseReference| this.factory(), set: |this: &ManagedDockableBaseReference, value| this.set_factory(value)},
        IsEmpty: bool {get: |this: &ManagedDockableBaseReference| this.is_empty(), set: |this: &ManagedDockableBaseReference, value| this.set_is_empty(value)},
        IsCollapsable: bool {get: |this: &ManagedDockableBaseReference| this.is_collapsable(), set: |this: &ManagedDockableBaseReference, value| this.set_is_collapsable(value)},
        Proportion: f64 {get: |this: &ManagedDockableBaseReference| this.proportion(), set: |this: &ManagedDockableBaseReference, value| this.set_proportion(value)},
        Dock: DockMode {get: |this: &ManagedDockableBaseReference| this.dock(), set: |this: &ManagedDockableBaseReference, value| this.set_dock(value)},
        DockingState: DockingWindowState {get: |this: &ManagedDockableBaseReference| this.docking_state(), set: |this: &ManagedDockableBaseReference, value| this.set_docking_state(value)},
        Column: i32 {get: |this: &ManagedDockableBaseReference| this.column(), set: |this: &ManagedDockableBaseReference, value| this.set_column(value)},
        Row: i32 {get: |this: &ManagedDockableBaseReference| this.row(), set: |this: &ManagedDockableBaseReference, value| this.set_row(value)},
        ColumnSpan: i32 {get: |this: &ManagedDockableBaseReference| this.column_span(), set: |this: &ManagedDockableBaseReference, value| this.set_column_span(value)},
        RowSpan: i32 {get: |this: &ManagedDockableBaseReference| this.row_span(), set: |this: &ManagedDockableBaseReference, value| this.set_row_span(value)},
        IsSharedSizeScope: bool {get: |this: &ManagedDockableBaseReference| this.is_shared_size_scope(), set: |this: &ManagedDockableBaseReference, value| this.set_is_shared_size_scope(value)},
        CollapsedProportion: f64 {get: |this: &ManagedDockableBaseReference| this.collapsed_proportion(), set: |this: &ManagedDockableBaseReference, value| this.set_collapsed_proportion(value)},
        MinWidth: f64 {get: |this: &ManagedDockableBaseReference| this.min_width(), set: |this: &ManagedDockableBaseReference, value| this.set_min_width(value)},
        MaxWidth: f64 {get: |this: &ManagedDockableBaseReference| this.max_width(), set: |this: &ManagedDockableBaseReference, value| this.set_max_width(value)},
        MinHeight: f64 {get: |this: &ManagedDockableBaseReference| this.min_height(), set: |this: &ManagedDockableBaseReference, value| this.set_min_height(value)},
        MaxHeight: f64 {get: |this: &ManagedDockableBaseReference| this.max_height(), set: |this: &ManagedDockableBaseReference, value| this.set_max_height(value)},
        CanClose: bool {get: |this: &ManagedDockableBaseReference| this.can_close(), set: |this: &ManagedDockableBaseReference, value| this.set_can_close(value)},
        CanPin: bool {get: |this: &ManagedDockableBaseReference| this.can_pin(), set: |this: &ManagedDockableBaseReference, value| this.set_can_pin(value)},
        KeepPinnedDockableVisible: bool {get: |this: &ManagedDockableBaseReference| this.keep_pinned_dockable_visible(), set: |this: &ManagedDockableBaseReference, value| this.set_keep_pinned_dockable_visible(value)},
        PinnedDockDisplayModeOverride: Option<PinnedDockDisplayMode> {get: |this: &ManagedDockableBaseReference| this.pinned_dock_display_mode_override(), set: |this: &ManagedDockableBaseReference, value| this.set_pinned_dock_display_mode_override(value)},
        CanFloat: bool {get: |this: &ManagedDockableBaseReference| this.can_float(), set: |this: &ManagedDockableBaseReference, value| this.set_can_float(value)},
        CanDrag: bool {get: |this: &ManagedDockableBaseReference| this.can_drag(), set: |this: &ManagedDockableBaseReference, value| this.set_can_drag(value)},
        CanDrop: bool {get: |this: &ManagedDockableBaseReference| this.can_drop(), set: |this: &ManagedDockableBaseReference, value| this.set_can_drop(value)},
        CanDockAsDocument: bool {get: |this: &ManagedDockableBaseReference| this.can_dock_as_document(), set: |this: &ManagedDockableBaseReference, value| this.set_can_dock_as_document(value)},
        DockCapabilityOverrides: Option<Rc<DockCapabilityOverrides>> {get: |this: &ManagedDockableBaseReference| this.dock_capability_overrides(), set: |this: &ManagedDockableBaseReference, value| this.set_dock_capability_overrides(value)},
        IsModified: bool {get: |this: &ManagedDockableBaseReference| this.is_modified(), set: |this: &ManagedDockableBaseReference, value| this.set_is_modified(value)},
        DockGroup: Option<String> {get: |this: &ManagedDockableBaseReference| this.dock_group(), set: |this: &ManagedDockableBaseReference, value| this.set_dock_group(value)},
        AllowedDockOperations: DockOperationMask {get: |this: &ManagedDockableBaseReference| this.allowed_dock_operations(), set: |this: &ManagedDockableBaseReference, value| this.set_allowed_dock_operations(value)},
        AllowedDropOperations: DockOperationMask {get: |this: &ManagedDockableBaseReference| this.allowed_drop_operations(), set: |this: &ManagedDockableBaseReference, value| this.set_allowed_drop_operations(value)},
        ShowInSelector: bool {get: |this: &ManagedDockableBaseReference| this.show_in_selector(), set: |this: &ManagedDockableBaseReference, value| this.set_show_in_selector(value)},
        SelectorTitle: Option<String> {get: |this: &ManagedDockableBaseReference| this.selector_title(), set: |this: &ManagedDockableBaseReference, value| this.set_selector_title(value)},
        PinnedBounds: Option<DockRect> {
            get: |this: &ManagedDockableBaseReference| this.pinned_bounds(),
            set: |this: &ManagedDockableBaseReference, value| this.set_pinned_bounds_rect(value)
        },
    ],
    methods: [
        fn GetControlRecyclingId() -> Option<String> => (|this: &ManagedDockableBaseReference| this.get_control_recycling_id()),
        fn OnClose() -> bool => (|this: &ManagedDockableBaseReference| this.on_close()),
        fn OnSelected() => (|this: &ManagedDockableBaseReference| this.on_selected()),
        fn GetVisibleBounds() -> (f64, f64, f64, f64) => (|this: &ManagedDockableBaseReference| this.get_visible_bounds()),
        fn SetVisibleBounds(f64, f64, f64, f64) => (|this: &ManagedDockableBaseReference, x, y, width, height| this.set_visible_bounds(x, y, width, height)),
        fn OnVisibleBoundsChanged(f64, f64, f64, f64) => (|this: &ManagedDockableBaseReference, x, y, width, height| this.on_visible_bounds_changed(x, y, width, height)),
        fn GetPinnedBounds() -> (f64, f64, f64, f64) => (|this: &ManagedDockableBaseReference| this.get_pinned_bounds()),
        fn SetPinnedBounds(f64, f64, f64, f64) => (|this: &ManagedDockableBaseReference, x, y, width, height| this.set_pinned_bounds(x, y, width, height)),
        fn OnPinnedBoundsChanged(f64, f64, f64, f64) => (|this: &ManagedDockableBaseReference, x, y, width, height| this.on_pinned_bounds_changed(x, y, width, height)),
        fn GetTabBounds() -> (f64, f64, f64, f64) => (|this: &ManagedDockableBaseReference| this.get_tab_bounds()),
        fn SetTabBounds(f64, f64, f64, f64) => (|this: &ManagedDockableBaseReference, x, y, width, height| this.set_tab_bounds(x, y, width, height)),
        fn OnTabBoundsChanged(f64, f64, f64, f64) => (|this: &ManagedDockableBaseReference, x, y, width, height| this.on_tab_bounds_changed(x, y, width, height)),
        fn GetPointerPosition() -> (f64, f64) => (|this: &ManagedDockableBaseReference| this.get_pointer_position()),
        fn SetPointerPosition(f64, f64) => (|this: &ManagedDockableBaseReference, x, y| this.set_pointer_position(x, y)),
        fn OnPointerPositionChanged(f64, f64) => (|this: &ManagedDockableBaseReference, x, y| this.on_pointer_position_changed(x, y)),
        fn GetPointerScreenPosition() -> (f64, f64) => (|this: &ManagedDockableBaseReference| this.get_pointer_screen_position()),
        fn SetPointerScreenPosition(f64, f64) => (|this: &ManagedDockableBaseReference, x, y| this.set_pointer_screen_position(x, y)),
        fn OnPointerScreenPositionChanged(f64, f64) => (|this: &ManagedDockableBaseReference, x, y| this.on_pointer_screen_position_changed(x, y)),
    ],
});

#[cfg(test)]
#[path = "managed_dockable_base_tests.rs"]
mod tests;
