use ferrodock_model::core::{
    DockCapabilityOverrides, DockMode, DockOperationMask, DockRect, DockingWindowState,
    IDockSelectorInfo, IDockable, IDockableDockingRestrictions, IFactory, PinnedDockDisplayMode,
};
// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::ReactiveBaseReference;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
};
use ferroui_reactiveui::IReactiveObject;
use ferroui_system::{
    component_model::{INotifyPropertyChanged, INotifyPropertyChanging},
    events::Event,
};
use std::rc::Rc;
/// Native reference to the original DockableBase class, retaining the enclosing object.
#[derive(Clone)]
pub struct DockableBaseReference {
    model: Rc<dyn IDockable>,
    base: ReactiveBaseReference,
}
impl DockableBaseReference {
    /// Creates the source base-class view without copying or replacing its model.
    pub fn new<T: IDockable + IReactiveObject>(model: Rc<T>) -> Self {
        Self {
            base: super::ReactiveBaseReference::new(model.clone()),
            model,
        }
    }
    fn restrictions(&self) -> Result<&dyn IDockableDockingRestrictions, MarkupInvokeError> {
        self.model.as_docking_restrictions().ok_or_else(|| {
            MarkupInvokeError::Failed(
                "The source DockableBase requires docking restrictions.".into(),
            )
        })
    }
    fn selector(&self) -> Result<&dyn IDockSelectorInfo, MarkupInvokeError> {
        self.model.as_dock_selector_info().ok_or_else(|| {
            MarkupInvokeError::Failed(
                "The source DockableBase requires selector information.".into(),
            )
        })
    }
    /// Gets the actual model behind this class reference.
    pub fn model(&self) -> Rc<dyn IDockable> {
        self.model.clone()
    }
    /// Gets the source base-class reference to the same enclosing model.
    pub fn base(&self) -> ReactiveBaseReference {
        self.base.clone()
    }
}
impl PartialEq for DockableBaseReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
impl INotifyPropertyChanged for DockableBaseReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for DockableBaseReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class DockableBaseReference as "DockableBase"{namespace:"FerroDock.Model.ReactiveUI.Core",this:DockableBaseReference,handles:[DockableBaseReference,Option<DockableBaseReference>],base:ReactiveBaseReference,interfaces:[Rc<dyn IDockable>],notify_property_changed:DockableBaseReference,properties:[
        Id: String {get: |this: &DockableBaseReference| this.model.id(), set: |this: &DockableBaseReference, value| this.model.set_id(value)},
        Title: String {get: |this: &DockableBaseReference| this.model.title(), set: |this: &DockableBaseReference, value| this.model.set_title(value)},
        Context: Option<ferroui_base::BoxedValue> {
            get: |this: &DockableBaseReference| this.model.context().as_ref().map(ferrodock_controls_recycling::from_recycling_value),
            set: |this: &DockableBaseReference, value: Option<ferroui_base::BoxedValue>| this.model.set_context(value.map(ferrodock_controls_recycling::to_recycling_value))
        } [IgnoreDataMember],
        Owner: Option<Rc<dyn IDockable>> {get: |this: &DockableBaseReference| this.model.owner(), set: |this: &DockableBaseReference, value| this.model.set_owner(value)},
        OriginalOwner: Option<Rc<dyn IDockable>> {get: |this: &DockableBaseReference| this.model.original_owner(), set: |this: &DockableBaseReference, value| this.model.set_original_owner(value)} [IgnoreDataMember],
        Factory: Option<Rc<dyn IFactory>> {get: |this: &DockableBaseReference| this.model.factory(), set: |this: &DockableBaseReference, value| this.model.set_factory(value)} [IgnoreDataMember],
        IsEmpty: bool {get: |this: &DockableBaseReference| this.model.is_empty(), set: |this: &DockableBaseReference, value| this.model.set_is_empty(value)},
        IsCollapsable: bool {get: |this: &DockableBaseReference| this.model.is_collapsable(), set: |this: &DockableBaseReference, value| this.model.set_is_collapsable(value)},
        Proportion: f64 {get: |this: &DockableBaseReference| this.model.proportion(), set: |this: &DockableBaseReference, value| this.model.set_proportion(value)},
        Dock: DockMode {get: |this: &DockableBaseReference| this.model.dock(), set: |this: &DockableBaseReference, value| this.model.set_dock(value)},
        DockingState: DockingWindowState {get: |this: &DockableBaseReference| this.model.docking_state(), set: |this: &DockableBaseReference, value| this.model.set_docking_state(value)},
        Column: i32 {get: |this: &DockableBaseReference| this.model.column(), set: |this: &DockableBaseReference, value| this.model.set_column(value)},
        Row: i32 {get: |this: &DockableBaseReference| this.model.row(), set: |this: &DockableBaseReference, value| this.model.set_row(value)},
        ColumnSpan: i32 {get: |this: &DockableBaseReference| this.model.column_span(), set: |this: &DockableBaseReference, value| this.model.set_column_span(value)},
        RowSpan: i32 {get: |this: &DockableBaseReference| this.model.row_span(), set: |this: &DockableBaseReference, value| this.model.set_row_span(value)},
        IsSharedSizeScope: bool {get: |this: &DockableBaseReference| this.model.is_shared_size_scope(), set: |this: &DockableBaseReference, value| this.model.set_is_shared_size_scope(value)},
        CollapsedProportion: f64 {get: |this: &DockableBaseReference| this.model.collapsed_proportion(), set: |this: &DockableBaseReference, value| this.model.set_collapsed_proportion(value)},
        MinWidth: f64 {get: |this: &DockableBaseReference| this.model.min_width(), set: |this: &DockableBaseReference, value| this.model.set_min_width(value)},
        MaxWidth: f64 {get: |this: &DockableBaseReference| this.model.max_width(), set: |this: &DockableBaseReference, value| this.model.set_max_width(value)},
        MinHeight: f64 {get: |this: &DockableBaseReference| this.model.min_height(), set: |this: &DockableBaseReference, value| this.model.set_min_height(value)},
        MaxHeight: f64 {get: |this: &DockableBaseReference| this.model.max_height(), set: |this: &DockableBaseReference, value| this.model.set_max_height(value)},
        CanClose: bool {get: |this: &DockableBaseReference| this.model.can_close(), set: |this: &DockableBaseReference, value| this.model.set_can_close(value)},
        CanPin: bool {get: |this: &DockableBaseReference| this.model.can_pin(), set: |this: &DockableBaseReference, value| this.model.set_can_pin(value)},
        KeepPinnedDockableVisible: bool {get: |this: &DockableBaseReference| this.model.keep_pinned_dockable_visible(), set: |this: &DockableBaseReference, value| this.model.set_keep_pinned_dockable_visible(value)},
        PinnedDockDisplayModeOverride: Option<PinnedDockDisplayMode> {get: |this: &DockableBaseReference| this.model.pinned_dock_display_mode_override(), set: |this: &DockableBaseReference, value| this.model.set_pinned_dock_display_mode_override(value)},
        PinnedBounds: Option<DockRect> {
            get: |this: &DockableBaseReference| this.model.pinned_bounds(),
            set: |this: &DockableBaseReference, value| this.model.set_pinned_bounds_rect(value)
        },
        CanFloat: bool {get: |this: &DockableBaseReference| this.model.can_float(), set: |this: &DockableBaseReference, value| this.model.set_can_float(value)},
        CanDrag: bool {get: |this: &DockableBaseReference| this.model.can_drag(), set: |this: &DockableBaseReference, value| this.model.set_can_drag(value)},
        CanDrop: bool {get: |this: &DockableBaseReference| this.model.can_drop(), set: |this: &DockableBaseReference, value| this.model.set_can_drop(value)},
        CanDockAsDocument: bool {get: |this: &DockableBaseReference| this.model.can_dock_as_document(), set: |this: &DockableBaseReference, value| this.model.set_can_dock_as_document(value)},
        DockCapabilityOverrides: Option<Rc<DockCapabilityOverrides>> {get: |this: &DockableBaseReference| this.model.dock_capability_overrides(), set: |this: &DockableBaseReference, value| this.model.set_dock_capability_overrides(value)},
        IsModified: bool {get: |this: &DockableBaseReference| this.model.is_modified(), set: |this: &DockableBaseReference, value| this.model.set_is_modified(value)},
        DockGroup: Option<String> {get: |this: &DockableBaseReference| this.model.dock_group(), set: |this: &DockableBaseReference, value| this.model.set_dock_group(value)},
        AllowedDockOperations: DockOperationMask {try_get: |this: &DockableBaseReference| this.restrictions().map(|model| model.allowed_dock_operations()), try_set: |this: &DockableBaseReference, value| { this.restrictions()?.set_allowed_dock_operations(value); Ok::<(), MarkupInvokeError>(()) }},
        AllowedDropOperations: DockOperationMask {try_get: |this: &DockableBaseReference| this.restrictions().map(|model| model.allowed_drop_operations()), try_set: |this: &DockableBaseReference, value| { this.restrictions()?.set_allowed_drop_operations(value); Ok::<(), MarkupInvokeError>(()) }},
        ShowInSelector: bool {try_get: |this: &DockableBaseReference| this.selector().map(|model| model.show_in_selector()), try_set: |this: &DockableBaseReference, value| { this.selector()?.set_show_in_selector(value); Ok::<(), MarkupInvokeError>(()) }},
        SelectorTitle: Option<String> {try_get: |this: &DockableBaseReference| this.selector().map(|model| model.selector_title()), try_set: |this: &DockableBaseReference, value| { this.selector()?.set_selector_title(value); Ok::<(), MarkupInvokeError>(()) }},
    ],methods:[

        fn GetControlRecyclingId() -> Option<String> => (|this: &DockableBaseReference| this.model.get_control_recycling_id()),
        fn OnClose() -> bool => (|this: &DockableBaseReference| this.model.on_close()),
        fn OnSelected() => (|this: &DockableBaseReference| this.model.on_selected()),
        fn GetVisibleBounds() -> (f64, f64, f64, f64) => (|this: &DockableBaseReference| this.model.get_visible_bounds()),
        fn SetVisibleBounds(f64, f64, f64, f64) => (|this: &DockableBaseReference, x, y, width, height| this.model.set_visible_bounds(x, y, width, height)),
        fn OnVisibleBoundsChanged(f64, f64, f64, f64) => (|this: &DockableBaseReference, x, y, width, height| this.model.on_visible_bounds_changed(x, y, width, height)),
        fn GetPinnedBounds() -> (f64, f64, f64, f64) => (|this: &DockableBaseReference| this.model.get_pinned_bounds()),
        fn SetPinnedBounds(f64, f64, f64, f64) => (|this: &DockableBaseReference, x, y, width, height| this.model.set_pinned_bounds(x, y, width, height)),
        fn OnPinnedBoundsChanged(f64, f64, f64, f64) => (|this: &DockableBaseReference, x, y, width, height| this.model.on_pinned_bounds_changed(x, y, width, height)),
        fn GetTabBounds() -> (f64, f64, f64, f64) => (|this: &DockableBaseReference| this.model.get_tab_bounds()),
        fn SetTabBounds(f64, f64, f64, f64) => (|this: &DockableBaseReference, x, y, width, height| this.model.set_tab_bounds(x, y, width, height)),
        fn OnTabBoundsChanged(f64, f64, f64, f64) => (|this: &DockableBaseReference, x, y, width, height| this.model.on_tab_bounds_changed(x, y, width, height)),
        fn GetPointerPosition() -> (f64, f64) => (|this: &DockableBaseReference| this.model.get_pointer_position()),
        fn SetPointerPosition(f64, f64) => (|this: &DockableBaseReference, x, y| this.model.set_pointer_position(x, y)),
        fn OnPointerPositionChanged(f64, f64) => (|this: &DockableBaseReference, x, y| this.model.on_pointer_position_changed(x, y)),
        fn GetPointerScreenPosition() -> (f64, f64) => (|this: &DockableBaseReference| this.model.get_pointer_screen_position()),
        fn SetPointerScreenPosition(f64, f64) => (|this: &DockableBaseReference, x, y| this.model.set_pointer_screen_position(x, y)),
        fn OnPointerScreenPositionChanged(f64, f64) => (|this: &DockableBaseReference, x, y| this.model.on_pointer_screen_position_changed(x, y)),
    ],});
pub(crate) fn register() {
    ValueTypes::register_cast::<DockableBaseReference, super::ReactiveObjectReference>(|value| {
        value.base().base()
    });
    MarkupType::register(<DockableBaseReference as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockableBaseReference>();
    ValueTypes::register_cast::<DockableBaseReference, ReactiveBaseReference>(|value| value.base());
    ValueTypes::register_cast::<DockableBaseReference, Rc<dyn IDockable>>(|value| value.model());
}
