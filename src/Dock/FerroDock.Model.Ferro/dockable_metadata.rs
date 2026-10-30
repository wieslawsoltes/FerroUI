//! Core-owned declarative metadata for the original domain dockable contract.
use ferrodock_model::core::{
    DockCapabilityOverrides, DockMode, DockRect, DockingWindowState, IDockable, IFactory,
    PinnedDockDisplayMode,
};
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{data::core::ValueTypes, AnyValue};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Descriptor carrier only; all values retain the original IDockable model.
#[doc(hidden)]
pub trait IDockableMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDockableMetadata as "IDockable" {
    namespace: "FerroDock.Model.Core",
    this: Rc<dyn IDockable>,
    handles: [Rc<dyn IDockable>, Option<Rc<dyn IDockable>>],
    interfaces: [Rc<dyn ferrodock_controls_recycling_model_ferro::IControlRecyclingIdProvider>],
    properties: [
        Id: String {get: |this: &Rc<dyn IDockable>| this.id(), set: |this: &Rc<dyn IDockable>, value| this.set_id(value)},
        Title: String {get: |this: &Rc<dyn IDockable>| this.title(), set: |this: &Rc<dyn IDockable>, value| this.set_title(value)},
        Context: Option<ferroui_base::BoxedValue> {
            get: |this: &Rc<dyn IDockable>| this.context().as_ref().map(ferrodock_runtime::from_source_value),
            set: |this: &Rc<dyn IDockable>, value: Option<ferroui_base::BoxedValue>| this.set_context(value.map(ferrodock_runtime::to_source_value))
        },
        Owner: Option<Rc<dyn IDockable>> {get: |this: &Rc<dyn IDockable>| this.owner(), set: |this: &Rc<dyn IDockable>, value| this.set_owner(value)},
        OriginalOwner: Option<Rc<dyn IDockable>> {get: |this: &Rc<dyn IDockable>| this.original_owner(), set: |this: &Rc<dyn IDockable>, value| this.set_original_owner(value)},
        Factory: Option<Rc<dyn IFactory>> {get: |this: &Rc<dyn IDockable>| this.factory(), set: |this: &Rc<dyn IDockable>, value| this.set_factory(value)},
        IsEmpty: bool {get: |this: &Rc<dyn IDockable>| this.is_empty(), set: |this: &Rc<dyn IDockable>, value| this.set_is_empty(value)},
        IsCollapsable: bool {get: |this: &Rc<dyn IDockable>| this.is_collapsable(), set: |this: &Rc<dyn IDockable>, value| this.set_is_collapsable(value)},
        Proportion: f64 {get: |this: &Rc<dyn IDockable>| this.proportion(), set: |this: &Rc<dyn IDockable>, value| this.set_proportion(value)},
        Dock: DockMode {get: |this: &Rc<dyn IDockable>| this.dock(), set: |this: &Rc<dyn IDockable>, value| this.set_dock(value)},
        DockingState: DockingWindowState {get: |this: &Rc<dyn IDockable>| this.docking_state(), set: |this: &Rc<dyn IDockable>, value| this.set_docking_state(value)},
        Column: i32 {get: |this: &Rc<dyn IDockable>| this.column(), set: |this: &Rc<dyn IDockable>, value| this.set_column(value)},
        Row: i32 {get: |this: &Rc<dyn IDockable>| this.row(), set: |this: &Rc<dyn IDockable>, value| this.set_row(value)},
        ColumnSpan: i32 {get: |this: &Rc<dyn IDockable>| this.column_span(), set: |this: &Rc<dyn IDockable>, value| this.set_column_span(value)},
        RowSpan: i32 {get: |this: &Rc<dyn IDockable>| this.row_span(), set: |this: &Rc<dyn IDockable>, value| this.set_row_span(value)},
        IsSharedSizeScope: bool {get: |this: &Rc<dyn IDockable>| this.is_shared_size_scope(), set: |this: &Rc<dyn IDockable>, value| this.set_is_shared_size_scope(value)},
        CollapsedProportion: f64 {get: |this: &Rc<dyn IDockable>| this.collapsed_proportion(), set: |this: &Rc<dyn IDockable>, value| this.set_collapsed_proportion(value)},
        MinWidth: f64 {get: |this: &Rc<dyn IDockable>| this.min_width(), set: |this: &Rc<dyn IDockable>, value| this.set_min_width(value)},
        MaxWidth: f64 {get: |this: &Rc<dyn IDockable>| this.max_width(), set: |this: &Rc<dyn IDockable>, value| this.set_max_width(value)},
        MinHeight: f64 {get: |this: &Rc<dyn IDockable>| this.min_height(), set: |this: &Rc<dyn IDockable>, value| this.set_min_height(value)},
        MaxHeight: f64 {get: |this: &Rc<dyn IDockable>| this.max_height(), set: |this: &Rc<dyn IDockable>, value| this.set_max_height(value)},
        CanClose: bool {get: |this: &Rc<dyn IDockable>| this.can_close(), set: |this: &Rc<dyn IDockable>, value| this.set_can_close(value)},
        CanPin: bool {get: |this: &Rc<dyn IDockable>| this.can_pin(), set: |this: &Rc<dyn IDockable>, value| this.set_can_pin(value)},
        KeepPinnedDockableVisible: bool {get: |this: &Rc<dyn IDockable>| this.keep_pinned_dockable_visible(), set: |this: &Rc<dyn IDockable>, value| this.set_keep_pinned_dockable_visible(value)},
        PinnedDockDisplayModeOverride: Option<PinnedDockDisplayMode> {get: |this: &Rc<dyn IDockable>| this.pinned_dock_display_mode_override(), set: |this: &Rc<dyn IDockable>, value| this.set_pinned_dock_display_mode_override(value)},
        PinnedBounds: Option<DockRect> {
            get: |this: &Rc<dyn IDockable>| this.pinned_bounds(),
            set: |this: &Rc<dyn IDockable>, value| this.set_pinned_bounds_rect(value)
        },
        CanFloat: bool {get: |this: &Rc<dyn IDockable>| this.can_float(), set: |this: &Rc<dyn IDockable>, value| this.set_can_float(value)},
        CanDrag: bool {get: |this: &Rc<dyn IDockable>| this.can_drag(), set: |this: &Rc<dyn IDockable>, value| this.set_can_drag(value)},
        CanDrop: bool {get: |this: &Rc<dyn IDockable>| this.can_drop(), set: |this: &Rc<dyn IDockable>, value| this.set_can_drop(value)},
        CanDockAsDocument: bool {get: |this: &Rc<dyn IDockable>| this.can_dock_as_document(), set: |this: &Rc<dyn IDockable>, value| this.set_can_dock_as_document(value)},
        DockCapabilityOverrides: Option<Rc<DockCapabilityOverrides>> {get: |this: &Rc<dyn IDockable>| this.dock_capability_overrides(), set: |this: &Rc<dyn IDockable>, value| this.set_dock_capability_overrides(value)},
        IsModified: bool {get: |this: &Rc<dyn IDockable>| this.is_modified(), set: |this: &Rc<dyn IDockable>, value| this.set_is_modified(value)},
        DockGroup: Option<String> {get: |this: &Rc<dyn IDockable>| this.dock_group(), set: |this: &Rc<dyn IDockable>, value| this.set_dock_group(value)},
    ],
    methods: [

        fn GetControlRecyclingId() -> Option<String> => (|this: &Rc<dyn IDockable>| this.get_control_recycling_id()),
        fn OnClose() -> bool => (|this: &Rc<dyn IDockable>| this.on_close()),
        fn OnSelected() => (|this: &Rc<dyn IDockable>| this.on_selected()),
        fn GetVisibleBounds() -> (f64, f64, f64, f64) => (|this: &Rc<dyn IDockable>| this.get_visible_bounds()),
        fn SetVisibleBounds(f64, f64, f64, f64) => (|this: &Rc<dyn IDockable>, x, y, width, height| this.set_visible_bounds(x, y, width, height)),
        fn OnVisibleBoundsChanged(f64, f64, f64, f64) => (|this: &Rc<dyn IDockable>, x, y, width, height| this.on_visible_bounds_changed(x, y, width, height)),
        fn GetPinnedBounds() -> (f64, f64, f64, f64) => (|this: &Rc<dyn IDockable>| this.get_pinned_bounds()),
        fn SetPinnedBounds(f64, f64, f64, f64) => (|this: &Rc<dyn IDockable>, x, y, width, height| this.set_pinned_bounds(x, y, width, height)),
        fn OnPinnedBoundsChanged(f64, f64, f64, f64) => (|this: &Rc<dyn IDockable>, x, y, width, height| this.on_pinned_bounds_changed(x, y, width, height)),
        fn GetTabBounds() -> (f64, f64, f64, f64) => (|this: &Rc<dyn IDockable>| this.get_tab_bounds()),
        fn SetTabBounds(f64, f64, f64, f64) => (|this: &Rc<dyn IDockable>, x, y, width, height| this.set_tab_bounds(x, y, width, height)),
        fn OnTabBoundsChanged(f64, f64, f64, f64) => (|this: &Rc<dyn IDockable>, x, y, width, height| this.on_tab_bounds_changed(x, y, width, height)),
        fn GetPointerPosition() -> (f64, f64) => (|this: &Rc<dyn IDockable>| this.get_pointer_position()),
        fn SetPointerPosition(f64, f64) => (|this: &Rc<dyn IDockable>, x, y| this.set_pointer_position(x, y)),
        fn OnPointerPositionChanged(f64, f64) => (|this: &Rc<dyn IDockable>, x, y| this.on_pointer_position_changed(x, y)),
        fn GetPointerScreenPosition() -> (f64, f64) => (|this: &Rc<dyn IDockable>| this.get_pointer_screen_position()),
        fn SetPointerScreenPosition(f64, f64) => (|this: &Rc<dyn IDockable>, x, y| this.set_pointer_screen_position(x, y)),
        fn OnPointerScreenPositionChanged(f64, f64) => (|this: &Rc<dyn IDockable>, x, y| this.on_pointer_screen_position_changed(x, y)),
    ],
});

// The foreign trait handle cannot implement a foreign System notification trait.
// The published metadata hook borrows that optional contract directly instead.
// Property/method invokers and compiler declarations remain the macro's originals.
static DOCKABLE_METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IDockableMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IDockable>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IDockable>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    // Register before consumer metadata: the shared registry preserves first identity.
    MarkupType::register(&DOCKABLE_METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&DOCKABLE_METADATA, "::ferrodock_model_ferro::IDockableMetadata", true)]);
    }
    ValueTypes::register_nullable::<Rc<dyn IDockable>>();
    ValueTypes::register_nullable::<Rc<dyn IFactory>>();
    ValueTypes::register_nullable::<Rc<DockCapabilityOverrides>>();
    ValueTypes::register_nullable::<PinnedDockDisplayMode>();
    ValueTypes::register_nullable::<DockRect>();
    ValueTypes::register_cast::<
        Rc<dyn IDockable>,
        Rc<dyn ferrodock_controls_recycling_model_ferro::IControlRecyclingIdProvider>,
    >(|model| model.clone());
}
