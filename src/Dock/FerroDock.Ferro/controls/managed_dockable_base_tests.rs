//! Original managed state and notification behavior through composed source contracts.
use super::{ManagedDockableBase, ManagedDockableBaseImpl, ManagedDockableBaseReference};
use crate::model_projection;
use ferrodock_model::{core::*, BoxedValue};
use ferrodock_model_reactiveui::{controls::Document, Factory};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Plain {
    base: ManagedDockableBase,
}
impl ManagedDockableBaseImpl for Plain {}
impl_managed_dockable!(Plain);
impl Plain {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|runtime| Self {
            base: ManagedDockableBase::construct(runtime.clone()),
        })
    }
}
struct Derived {
    base: ManagedDockableBase,
    calls: RefCell<Vec<String>>,
}
impl_managed_dockable!(Derived);
impl ManagedDockableBaseImpl for Derived {
    fn on_property_changed(this: &Self, name: &str) {
        this.base.raise_property_changed(name);
        this.calls.borrow_mut().push(format!("override:{name}"));
    }
    fn on_close(this: &Self) -> bool {
        this.calls.borrow_mut().push("close".into());
        false
    }
    fn on_selected(this: &Self) {
        this.calls.borrow_mut().push("selected".into());
    }
    fn on_visible_bounds_changed(this: &Self, x: f64, y: f64, width: f64, height: f64) {
        assert_eq!(this.get_visible_bounds(), (x, y, width, height));
        this.calls.borrow_mut().push("visible".into());
    }
    fn on_pinned_bounds_changed(this: &Self, x: f64, y: f64, width: f64, height: f64) {
        let actual = this.get_pinned_bounds();
        for (a, b) in [actual.0, actual.1, actual.2, actual.3]
            .into_iter()
            .zip([x, y, width, height])
        {
            assert!(a == b || (a.is_nan() && b.is_nan()));
        }
        this.calls.borrow_mut().push("pinned".into());
    }
    fn on_tab_bounds_changed(this: &Self, x: f64, y: f64, width: f64, height: f64) {
        assert_eq!(this.get_tab_bounds(), (x, y, width, height));
        this.calls.borrow_mut().push("tab".into());
    }
    fn on_pointer_position_changed(this: &Self, x: f64, y: f64) {
        assert_eq!(this.get_pointer_position(), (x, y));
        this.calls.borrow_mut().push("pointer".into());
    }
    fn on_pointer_screen_position_changed(this: &Self, x: f64, y: f64) {
        assert_eq!(this.get_pointer_screen_position(), (x, y));
        this.calls.borrow_mut().push("screen".into());
    }
}
impl Derived {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|runtime| Self {
            base: ManagedDockableBase::construct(runtime.clone()),
            calls: RefCell::new(Vec::new()),
        })
    }
}
fn notifications(model: &(impl INotifyPropertyChanged + ?Sized)) -> Rc<RefCell<Vec<String>>> {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = calls.clone();
    model
        .property_changed()
        .add(Rc::new(move |name| observed.borrow_mut().push(name.into())));
    calls
}
#[test]
fn all_source_defaults_and_base_virtual_callbacks_are_preserved() {
    let model = Plain::new();
    let erased: Rc<dyn IDockable> = model.clone();
    assert!(std::ptr::eq(
        erased
            .as_notify_property_changed()
            .unwrap()
            .property_changed(),
        model.property_changed()
    ));
    let native = model_projection::to_dockable_value(erased);
    assert!(std::ptr::eq(
        model_projection::notifier(&*native)
            .unwrap()
            .property_changed(),
        model.property_changed()
    ));
    assert_eq!(model.id(), "");
    assert_eq!(model.title(), "");
    assert!(
        model.context().is_none()
            && model.owner().is_none()
            && model.original_owner().is_none()
            && model.factory().is_none()
    );
    assert!(!model.is_empty());
    assert!(model.is_collapsable());
    assert!(model.proportion().is_nan() && model.collapsed_proportion().is_nan());
    assert!(
        model.min_width().is_nan()
            && model.min_height().is_nan()
            && model.max_width().is_nan()
            && model.max_height().is_nan()
    );
    assert_eq!(model.dock(), DockMode::Center);
    assert_eq!(model.docking_state(), DockingWindowState::DOCKED);
    assert_eq!(
        (
            model.column(),
            model.row(),
            model.column_span(),
            model.row_span()
        ),
        (0, 0, 1, 1)
    );
    assert!(!model.is_shared_size_scope());
    assert!(
        model.can_close()
            && model.can_pin()
            && model.can_float()
            && model.can_drag()
            && model.can_drop()
            && model.can_dock_as_document()
    );
    assert!(!model.keep_pinned_dockable_visible());
    assert!(
        model.pinned_dock_display_mode_override().is_none()
            && model.dock_capability_overrides().is_none()
    );
    assert!(!model.is_modified());
    assert!(model.dock_group().is_none() && model.selector_title().is_none());
    assert!(model.show_in_selector());
    assert_eq!(model.allowed_dock_operations(), DockOperationMask::ALL);
    assert_eq!(model.allowed_drop_operations(), DockOperationMask::ALL);
    assert_eq!(model.get_control_recycling_id(), Some(String::new()));
    assert!(model.on_close());
    model.on_selected();
    model.on_visible_bounds_changed(1.0, 2.0, 3.0, 4.0);
    model.on_pinned_bounds_changed(1.0, 2.0, 3.0, 4.0);
    model.on_tab_bounds_changed(1.0, 2.0, 3.0, 4.0);
    model.on_pointer_position_changed(1.0, 2.0);
    model.on_pointer_screen_position_changed(1.0, 2.0);
    assert!(model.base.tracking_adapter.get().is_none());
    assert!(model.pinned_bounds().is_none());
    assert!(model.get_visible_bounds().0.is_nan());
    assert!(model.get_tab_bounds().0.is_nan());
    assert!(model.get_pointer_position().0.is_nan());
    assert!(model.get_pointer_screen_position().0.is_nan());
}
#[test]
fn source_setters_raise_exact_names_once_keep_unvalidated_values_and_expose_optional_interfaces() {
    let model = Plain::new();
    let calls = notifications(&*model);
    macro_rules! change {
        ($get:ident, $set:ident, $value:expr, $name:literal) => {{
            let value = $value;
            calls.borrow_mut().clear();
            model.$set(value.clone());
            assert_eq!(model.$get(), value);
            model.$set(value);
            assert_eq!(&*calls.borrow(), &[$name]);
        }};
    }
    change!(id, set_id, String::from("id"), "Id");
    change!(title, set_title, String::from("Title"), "Title");
    change!(is_empty, set_is_empty, true, "IsEmpty");
    change!(is_collapsable, set_is_collapsable, false, "IsCollapsable");
    change!(proportion, set_proportion, -2.5f64, "Proportion");
    change!(
        collapsed_proportion,
        set_collapsed_proportion,
        f64::INFINITY,
        "CollapsedProportion"
    );
    change!(dock, set_dock, DockMode::Top, "Dock");
    change!(
        docking_state,
        set_docking_state,
        DockingWindowState::FLOATING,
        "DockingState"
    );
    change!(column, set_column, -5i32, "Column");
    change!(row, set_row, -6i32, "Row");
    change!(column_span, set_column_span, 0i32, "ColumnSpan");
    change!(row_span, set_row_span, -2i32, "RowSpan");
    change!(
        is_shared_size_scope,
        set_is_shared_size_scope,
        true,
        "IsSharedSizeScope"
    );
    change!(min_width, set_min_width, -20.0f64, "MinWidth");
    change!(min_height, set_min_height, f64::NEG_INFINITY, "MinHeight");
    change!(max_width, set_max_width, -1.0f64, "MaxWidth");
    change!(max_height, set_max_height, f64::INFINITY, "MaxHeight");
    change!(can_close, set_can_close, false, "CanClose");
    change!(can_pin, set_can_pin, false, "CanPin");
    change!(can_float, set_can_float, false, "CanFloat");
    change!(can_drag, set_can_drag, false, "CanDrag");
    change!(can_drop, set_can_drop, false, "CanDrop");
    change!(
        can_dock_as_document,
        set_can_dock_as_document,
        false,
        "CanDockAsDocument"
    );
    change!(
        keep_pinned_dockable_visible,
        set_keep_pinned_dockable_visible,
        true,
        "KeepPinnedDockableVisible"
    );
    change!(
        pinned_dock_display_mode_override,
        set_pinned_dock_display_mode_override,
        Some(PinnedDockDisplayMode::Overlay),
        "PinnedDockDisplayModeOverride"
    );
    change!(dock_group, set_dock_group, Some(String::new()), "DockGroup");
    change!(is_modified, set_is_modified, true, "IsModified");
    change!(
        allowed_dock_operations,
        set_allowed_dock_operations,
        DockOperationMask::LEFT,
        "AllowedDockOperations"
    );
    change!(
        allowed_drop_operations,
        set_allowed_drop_operations,
        DockOperationMask::NONE,
        "AllowedDropOperations"
    );
    change!(
        show_in_selector,
        set_show_in_selector,
        false,
        "ShowInSelector"
    );
    change!(
        selector_title,
        set_selector_title,
        Some(String::new()),
        "SelectorTitle"
    );
    let policy = DockCapabilityOverrides::new();
    change!(
        dock_capability_overrides,
        set_dock_capability_overrides,
        Some(policy.clone()),
        "DockCapabilityOverrides"
    );
    assert!(Rc::ptr_eq(
        &model.dock_capability_overrides().unwrap(),
        &policy
    ));
    assert_eq!(model.get_control_recycling_id(), Some(String::from("id")));
    let original = model.clone() as Rc<dyn IDockable>;
    assert!(!original.as_dock_selector_info().unwrap().show_in_selector());
    assert_eq!(
        original
            .as_docking_restrictions()
            .unwrap()
            .allowed_dock_operations(),
        DockOperationMask::LEFT
    );
    assert!(original.clone().as_dock().is_none());
    assert!(original.as_mdi_document().is_none());
}
#[test]
fn source_double_equality_suppresses_nan_payload_and_signed_zero_changes() {
    let model = Plain::new();
    let calls = notifications(&*model);
    let nan = f64::from_bits(0x7ff8_0000_0000_0123);
    model.set_proportion(nan);
    model.set_collapsed_proportion(nan);
    model.set_min_width(nan);
    model.set_min_height(nan);
    model.set_max_width(nan);
    model.set_max_height(nan);
    assert!(calls.borrow().is_empty());
    assert_eq!(model.proportion().to_bits(), f64::NAN.to_bits());
    model.set_proportion(-0.0);
    model.set_proportion(0.0);
    assert_eq!(model.proportion().to_bits(), (-0.0f64).to_bits());
    assert_eq!(&*calls.borrow(), &["Proportion"]);
    let field = RefCell::new(f32::NAN);
    assert!(!model.set_property(&field, f32::from_bits(0x7fc00123), Some("Float")));
    assert!(model.set_property(&field, 0.0f32, Some("Float")));
    assert!(!model.set_property(&field, -0.0f32, Some("Float")));
}
#[test]
fn source_context_scalar_geometry_and_reference_equality_keep_original_objects() {
    let model = Plain::new();
    let calls = notifications(&*model);
    let first = Rc::new(String::from("context")) as BoxedValue;
    model.set_context(Some(first.clone()));
    model.set_context(Some(Rc::new(String::from("context"))));
    assert!(Rc::ptr_eq(&model.context().unwrap(), &first));
    model.set_context(Some(Rc::new(f64::NAN)));
    model.set_context(Some(Rc::new(f64::from_bits(0x7ff8000000000123))));
    let rect = Rc::new(DockRect::new(f64::NAN, 0.0, -2.0, 3.0)) as BoxedValue;
    model.set_context(Some(rect.clone()));
    model.set_context(Some(Rc::new(DockRect::new(f64::NAN, -0.0, -2.0, 3.0))));
    assert!(Rc::ptr_eq(&model.context().unwrap(), &rect));
    let doc = Document::new() as Rc<dyn IDockable>;
    let original = Rc::new(doc.clone()) as BoxedValue;
    model.set_context(Some(original.clone()));
    model.set_context(Some(Rc::new(doc)));
    assert!(Rc::ptr_eq(&model.context().unwrap(), &original));
    model.set_context(None);
    model.set_context(None);
    assert_eq!(calls.borrow().len(), 5);
}
#[test]
fn owners_original_owner_and_factory_retain_source_strong_identity_until_cleared() {
    let model = Plain::new();
    let calls = notifications(&*model);
    let owner = Plain::new();
    let weak_owner = Rc::downgrade(&owner);
    model.set_owner(Some(owner.clone()));
    model.set_owner(Some(owner.clone()));
    model.set_original_owner(Some(owner));
    assert!(weak_owner.upgrade().is_some());
    assert!(Rc::ptr_eq(
        &model.owner().unwrap(),
        &model.original_owner().unwrap()
    ));
    model.set_owner(None);
    assert!(weak_owner.upgrade().is_some());
    model.set_original_owner(None);
    assert!(weak_owner.upgrade().is_none());
    let factory = Factory::new();
    let weak_factory = Rc::downgrade(&factory);
    model.set_factory(Some(factory.clone()));
    model.set_factory(Some(factory));
    assert!(weak_factory.upgrade().is_some());
    model.set_factory(None);
    assert!(weak_factory.upgrade().is_none());
    assert_eq!(
        &*calls.borrow(),
        &[
            "Owner",
            "OriginalOwner",
            "Owner",
            "OriginalOwner",
            "Factory",
            "Factory"
        ]
    );
}
#[test]
fn tracking_updates_before_every_source_callback_without_property_events_or_cross_space_changes() {
    let model = Derived::new();
    let calls = notifications(&*model);
    assert!(!model.on_close());
    model.on_selected();
    model.set_visible_bounds(1.0, 2.0, 3.0, 4.0);
    model.set_pinned_bounds(5.0, 6.0, 7.0, 8.0);
    model.set_tab_bounds(9.0, 10.0, 11.0, 12.0);
    model.set_pointer_position(13.0, 14.0);
    model.set_pointer_screen_position(15.0, 16.0);
    assert_eq!(model.get_visible_bounds(), (1.0, 2.0, 3.0, 4.0));
    assert_eq!(model.get_pinned_bounds(), (5.0, 6.0, 7.0, 8.0));
    assert_eq!(model.get_tab_bounds(), (9.0, 10.0, 11.0, 12.0));
    assert_eq!(model.get_pointer_position(), (13.0, 14.0));
    assert_eq!(model.get_pointer_screen_position(), (15.0, 16.0));
    model.set_pinned_bounds(5.0, 6.0, 7.0, 8.0);
    assert_eq!(
        &*model.calls.borrow(),
        &["close", "selected", "visible", "pinned", "tab", "pointer", "screen", "pinned"]
    );
    assert!(calls.borrow().is_empty());
}
#[test]
fn source_optional_pinned_bounds_validate_dimensions_only_and_clear_with_callback() {
    let model = Derived::new();
    for (width, height, valid) in [
        (0.0, 0.0, true),
        (-3.0, -2.0, true),
        (f64::NAN, 5.0, false),
        (5.0, f64::NAN, false),
        (f64::INFINITY, 5.0, false),
        (5.0, f64::NEG_INFINITY, false),
    ] {
        model.set_pinned_bounds(f64::NAN, f64::INFINITY, width, height);
        assert_eq!(model.pinned_bounds().is_some(), valid);
        if let Some(rect) = model.pinned_bounds() {
            assert!(rect.x.is_nan());
            assert_eq!(rect.y, f64::INFINITY);
            assert_eq!(rect.width, width);
        }
    }
    model.set_pinned_bounds_rect(Some(DockRect::new(1.0, 2.0, -3.0, 0.0)));
    assert_eq!(
        model.pinned_bounds(),
        Some(DockRect::new(1.0, 2.0, -3.0, 0.0))
    );
    model.set_pinned_bounds_rect(None);
    let (x, y, width, height) = model.get_pinned_bounds();
    assert!(x.is_nan() && y.is_nan() && width.is_nan() && height.is_nan());
    assert!(model.pinned_bounds().is_none());
    assert_eq!(model.calls.borrow().len(), 8);
}
#[test]
fn property_notification_uses_source_snapshot_order_reentrancy_and_virtual_override_order() {
    let model = Derived::new();
    let weak = Rc::downgrade(&model);
    let second_token = Rc::new(Cell::new(0));
    let captured = second_token.clone();
    model.property_changed().add(Rc::new(move |name| {
        let model = weak.upgrade().unwrap();
        model.calls.borrow_mut().push(format!("first:{name}"));
        if name == "Id" {
            assert_eq!(model.id(), "id");
            model.property_changed().remove(captured.get());
            let weak = Rc::downgrade(&model);
            model.property_changed().add(Rc::new(move |name| {
                weak.upgrade()
                    .unwrap()
                    .calls
                    .borrow_mut()
                    .push(format!("added:{name}"))
            }));
            model.set_title(String::from("reentrant"));
        }
    }));
    let weak = Rc::downgrade(&model);
    second_token.set(model.property_changed().add(Rc::new(move |name| {
        weak.upgrade()
            .unwrap()
            .calls
            .borrow_mut()
            .push(format!("second:{name}"))
    })));
    model.set_id(String::from("id"));
    assert_eq!(
        &*model.calls.borrow(),
        &[
            "first:Id",
            "first:Title",
            "added:Title",
            "override:Title",
            "second:Id",
            "override:Id"
        ]
    );
}
#[test]
fn source_changed_handler_exception_keeps_assignment_skips_later_handlers_and_override_tail() {
    let model = Derived::new();
    let token = model
        .property_changed()
        .add(Rc::new(|_| panic!("source handler failed")));
    let later = Rc::new(Cell::new(false));
    let captured = later.clone();
    model
        .property_changed()
        .add(Rc::new(move |_| captured.set(true)));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        model.set_title(String::from("assigned"))
    }));
    assert!(result.is_err());
    assert_eq!(model.title(), "assigned");
    assert!(!later.get());
    assert!(model.calls.borrow().is_empty());
    model.property_changed().remove(token);
    model.set_title(String::from("after"));
    assert!(later.get());
    assert_eq!(&*model.calls.borrow(), &["override:Title"]);
}
#[test]
fn generic_source_set_property_compares_outside_storage_borrows_then_writes_and_handles_empty_name()
{
    #[derive(Clone)]
    struct Equality {
        value: i32,
        compare: Rc<dyn Fn()>,
    }
    impl PartialEq for Equality {
        fn eq(&self, other: &Self) -> bool {
            (self.compare)();
            self.value == other.value
        }
    }
    let model = Plain::new();
    let field = Rc::new(RefCell::new(Equality {
        value: 1,
        compare: Rc::new(|| {}),
    }));
    let weak = Rc::downgrade(&field);
    field.borrow_mut().compare =
        Rc::new(move || assert_eq!(weak.upgrade().unwrap().borrow().value, 1));
    let calls = notifications(&*model);
    assert!(!model.set_property(
        &field,
        Equality {
            value: 1,
            compare: Rc::new(|| {})
        },
        Some("Custom")
    ));
    assert!(model.set_property(
        &field,
        Equality {
            value: 2,
            compare: Rc::new(|| {})
        },
        None
    ));
    assert_eq!(field.borrow().value, 2);
    assert_eq!(&*calls.borrow(), &[""]);
}
#[test]
fn native_managed_reference_keeps_original_identity_notifications_and_core_projection() {
    let model = Plain::new();
    let weak = Rc::downgrade(&model);
    let native = model_projection::to_managed_dockable_value(model.clone());
    let reference = native
        .downcast_ref::<ManagedDockableBaseReference>()
        .unwrap();
    let calls = notifications(reference);
    reference.set_title(String::from("Original"));
    assert_eq!(model.title(), "Original");
    assert_eq!(&*calls.borrow(), &["Title"]);
    assert!(Rc::ptr_eq(
        &reference.model(),
        &(model.clone() as Rc<dyn ManagedDockableBaseImpl>)
    ));
    assert!(reference == &reference.clone());
    assert!(reference != &ManagedDockableBaseReference::new(Plain::new()));
    assert!(Rc::ptr_eq(
        &model_projection::dockable(Some(&native)).unwrap(),
        &(model.clone() as Rc<dyn IDockable>)
    ));
    drop(model);
    assert!(weak.upgrade().is_some());
    drop(native);
    assert!(weak.upgrade().is_none());
}

#[test]
fn original_abstract_class_metadata_exports_properties_methods_and_original_event_identity() {
    use ferroui_base::metadata::{MarkupTypeKind, MarkupTyped};
    let model = Plain::new();
    let this = model_projection::to_managed_dockable_value(model.clone());
    crate::register_types();
    let metadata = ManagedDockableBase::MARKUP;
    assert_eq!(metadata.kind, MarkupTypeKind::Class);
    assert_eq!(metadata.namespace(), "FerroDock.Ferro.Controls");
    assert!(metadata.constructors.is_empty());
    assert_eq!(metadata.properties.len(), 37);
    assert_eq!(metadata.methods.len(), 18);
    let notify = (metadata.notify_property_changed.unwrap())(&*this).unwrap();
    assert!(std::ptr::eq(
        notify.property_changed(),
        model.property_changed()
    ));
    // The shared metadata notification hook recognizes the local native handle.
    // A raw Rc trait handle first needs the exported concrete conversion.
    let raw = Rc::new(model.clone() as Rc<dyn ManagedDockableBaseImpl>) as ferroui_base::BoxedValue;
    assert!((metadata.notify_property_changed.unwrap())(&*raw).is_none());
    let converted = ferroui_base::data::core::ValueTypes::try_cast(
        &raw,
        ferroui_base::data::core::ValueType::of::<ManagedDockableBaseReference>(),
    )
    .unwrap();
    let converted_notify = (metadata.notify_property_changed.unwrap())(&*converted).unwrap();
    assert!(std::ptr::eq(
        converted_notify.property_changed(),
        model.property_changed()
    ));
    let calls = notifications(notify);
    let set_title = metadata.find_property("Title").unwrap().set.unwrap();
    set_title(&[Some(this.clone()), Some(Rc::new(String::from("Metadata")))]).unwrap();
    assert_eq!(model.title(), "Metadata");
    let title = (metadata.find_property("Title").unwrap().get.unwrap())(&[Some(this.clone())])
        .unwrap()
        .unwrap();
    assert_eq!(title.downcast_ref::<String>().unwrap(), "Metadata");
    let setter = metadata.find_property("Context").unwrap().set.unwrap();
    for _ in 0..2 {
        setter(&[
            Some(this.clone()),
            Some(Rc::new(Some(
                Rc::new(String::from("context")) as ferroui_base::BoxedValue
            ))),
        ])
        .unwrap();
    }
    for _ in 0..2 {
        setter(&[
            Some(this.clone()),
            Some(Rc::new(Some(Rc::new(f64::NAN) as ferroui_base::BoxedValue))),
        ])
        .unwrap();
    }
    assert_eq!(&*calls.borrow(), &["Title", "Context", "Context"]);
    let get = (metadata.find_property("Context").unwrap().get.unwrap())(&[Some(this.clone())])
        .unwrap()
        .unwrap();
    assert!(get
        .downcast_ref::<Option<ferroui_base::BoxedValue>>()
        .unwrap()
        .as_ref()
        .unwrap()
        .downcast_ref::<f64>()
        .unwrap()
        .is_nan());
    let method = |name| {
        metadata
            .methods
            .iter()
            .find(|method| method.name == name)
            .unwrap()
    };
    let result = (method("OnClose").invoke)(&[Some(this.clone())])
        .unwrap()
        .unwrap();
    assert_eq!(result.downcast_ref::<bool>(), Some(&true));
    let result = (method("GetControlRecyclingId").invoke)(&[Some(this.clone())])
        .unwrap()
        .unwrap();
    // Native method invokers unwrap a nullable return into the optional boxed result.
    assert_eq!(result.downcast_ref::<String>(), Some(&String::new()));
    (method("SetVisibleBounds").invoke)(&[
        Some(this.clone()),
        Some(Rc::new(1.0f64)),
        Some(Rc::new(2.0f64)),
        Some(Rc::new(3.0f64)),
        Some(Rc::new(4.0f64)),
    ])
    .unwrap();
    let result = (method("GetVisibleBounds").invoke)(&[Some(this)])
        .unwrap()
        .unwrap();
    assert_eq!(
        result.downcast_ref::<(f64, f64, f64, f64)>(),
        Some(&(1.0, 2.0, 3.0, 4.0))
    );
}
