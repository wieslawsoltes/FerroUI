//! Native metadata forwards to the same domain objects, methods and notifier.
use crate::IDockableMetadata;
use ferrodock_model::core::*;
use ferrodock_model_reactiveui::{controls::Document, Factory};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::{MarkupType, MarkupTyped},
    BoxedValue,
};
use std::{any::TypeId, rc::Rc};
fn get(this: &BoxedValue, name: &str) -> Option<BoxedValue> {
    let property = <dyn IDockableMetadata>::MARKUP.find_property(name).unwrap();
    (property.get.unwrap())(&[Some(this.clone())]).unwrap()
}
fn set(this: &BoxedValue, name: &str, value: Option<BoxedValue>) {
    let property = <dyn IDockableMetadata>::MARKUP.find_property(name).unwrap();
    (property.set.unwrap())(&[Some(this.clone()), value]).unwrap();
}
#[test]
fn interface_properties_preserve_original_model_context_and_reference_identity() {
    crate::register_types();
    let model: Rc<dyn IDockable> = Document::new();
    let this: BoxedValue = Rc::new(model.clone());
    assert_eq!(<dyn IDockableMetadata>::MARKUP.properties.len(), 33);
    assert_eq!(<dyn IDockableMetadata>::MARKUP.methods.len(), 18);
    set(&this, "Title", Some(Rc::new("From markup".to_owned())));
    assert_eq!(model.title(), "From markup");
    model.set_title("From model".into());
    assert_eq!(
        get(&this, "Title")
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "From model"
    );
    let owner: Rc<dyn IDockable> = Document::new();
    for name in ["Owner", "OriginalOwner"] {
        set(&this, name, Some(Rc::new(owner.clone())));
        let actual =
            ferroui_base::metadata::from_markup_value::<Rc<dyn IDockable>>(&get(&this, name))
                .unwrap();
        assert!(Rc::ptr_eq(&actual, &owner));
        set(&this, name, None);
        assert!(get(&this, name).is_none());
    }
    let factory: Rc<dyn IFactory> = Factory::new();
    set(&this, "Factory", Some(Rc::new(factory.clone())));
    assert!(Rc::ptr_eq(&model.factory().unwrap(), &factory));
    set(&this, "Factory", None);
    assert!(get(&this, "Factory").is_none());
    let overrides = DockCapabilityOverrides::new();
    set(
        &this,
        "DockCapabilityOverrides",
        Some(Rc::new(overrides.clone())),
    );
    assert!(Rc::ptr_eq(
        &model.dock_capability_overrides().unwrap(),
        &overrides
    ));
    set(&this, "DockCapabilityOverrides", None);
    assert!(get(&this, "DockCapabilityOverrides").is_none());
    set(
        &this,
        "PinnedDockDisplayModeOverride",
        Some(Rc::new(PinnedDockDisplayMode::default())),
    );
    assert_eq!(
        model.pinned_dock_display_mode_override(),
        Some(PinnedDockDisplayMode::default())
    );
    set(&this, "PinnedDockDisplayModeOverride", None);
    assert!(get(&this, "PinnedDockDisplayModeOverride").is_none());
    let context: BoxedValue = Rc::new("Shared context".to_owned());
    set(&this, "Context", Some(context.clone()));
    let roundtrip = get(&this, "Context").unwrap();
    assert!(Rc::ptr_eq(&roundtrip, &context));
    set(&this, "Context", None);
    assert!(model.context().is_none());
    let bounds = DockRect::new(-10.0, 20.0, 30.0, 40.0);
    set(&this, "PinnedBounds", Some(Rc::new(bounds)));
    assert_eq!(model.pinned_bounds(), Some(bounds));
    set(&this, "PinnedBounds", None);
    assert!(model.pinned_bounds().is_none());
}
#[test]
fn native_notifier_and_inherited_recycling_cast_borrow_the_original_contracts() {
    crate::register_types();
    let model: Rc<dyn IDockable> = Document::new();
    model.set_id("stable".into());
    let value: BoxedValue = Rc::new(model.clone());
    let metadata = MarkupType::find_by_handle(TypeId::of::<Rc<dyn IDockable>>()).unwrap();
    let find = metadata.notify_property_changed.unwrap();
    assert!(std::ptr::eq(
        find(value.as_ref()).unwrap(),
        model.as_notify_property_changed().unwrap()
    ));
    let nullable: BoxedValue = Rc::new(Some(model.clone()));
    assert!(std::ptr::eq(
        find(nullable.as_ref()).unwrap(),
        model.as_notify_property_changed().unwrap()
    ));
    assert!(find(&None::<Rc<dyn IDockable>>).is_none());
    assert!(find(&42_i32).is_none());
    type Recycling = Rc<dyn ferrodock_controls_recycling_model_ferro::IControlRecyclingIdProvider>;
    let cast = ValueTypes::try_cast(&value, ValueType::of::<Recycling>()).unwrap();
    let expected: Recycling = model;
    assert!(Rc::ptr_eq(
        cast.downcast_ref::<Recycling>().unwrap(),
        &expected
    ));
}
#[test]
fn metadata_methods_update_original_bounds_and_positions() {
    crate::register_types();
    let model: Rc<dyn IDockable> = Document::new();
    let value: BoxedValue = Rc::new(model.clone());
    for (setter, getter) in [
        ("SetVisibleBounds", "GetVisibleBounds"),
        ("SetPinnedBounds", "GetPinnedBounds"),
        ("SetTabBounds", "GetTabBounds"),
    ] {
        let set = <dyn IDockableMetadata>::MARKUP
            .find_methods(setter)
            .next()
            .unwrap();
        (set.invoke)(&[
            Some(value.clone()),
            Some(Rc::new(-1.0_f64)),
            Some(Rc::new(2.0_f64)),
            Some(Rc::new(30.0_f64)),
            Some(Rc::new(40.0_f64)),
        ])
        .unwrap();
        let get = <dyn IDockableMetadata>::MARKUP
            .find_methods(getter)
            .next()
            .unwrap();
        let result = (get.invoke)(&[Some(value.clone())]).unwrap().unwrap();
        assert_eq!(
            result.downcast_ref::<(f64, f64, f64, f64)>(),
            Some(&(-1.0, 2.0, 30.0, 40.0))
        );
    }
    for (setter, getter) in [
        ("SetPointerPosition", "GetPointerPosition"),
        ("SetPointerScreenPosition", "GetPointerScreenPosition"),
    ] {
        let set = <dyn IDockableMetadata>::MARKUP
            .find_methods(setter)
            .next()
            .unwrap();
        (set.invoke)(&[
            Some(value.clone()),
            Some(Rc::new(12.0_f64)),
            Some(Rc::new(-34.0_f64)),
        ])
        .unwrap();
        let get = <dyn IDockableMetadata>::MARKUP
            .find_methods(getter)
            .next()
            .unwrap();
        let result = (get.invoke)(&[Some(value.clone())]).unwrap().unwrap();
        assert_eq!(result.downcast_ref::<(f64, f64)>(), Some(&(12.0, -34.0)));
    }
}
