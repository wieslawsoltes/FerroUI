use crate::{
    dummy_control_recycling::DummyControlRecycling, IControlRecycling, IControlRecyclingIdProvider,
};
use std::any::Any;
use std::rc::Rc;

fn recycling() -> Rc<dyn IControlRecycling> {
    Rc::new(DummyControlRecycling::default())
}
fn object() -> Rc<dyn Any> {
    Rc::new(())
}

#[test]
fn try_to_use_id_as_key_property_works() {
    let recycling = recycling();
    assert!(!recycling.try_to_use_id_as_key());
    recycling.set_try_to_use_id_as_key(true);
    assert!(recycling.try_to_use_id_as_key());
    recycling.set_try_to_use_id_as_key(false);
    assert!(!recycling.try_to_use_id_as_key());
}

#[test]
fn add_and_try_get_value_returns_added_control() {
    let recycling = recycling();
    let data = object();
    let control = object();
    recycling.add(data.clone(), control.clone());
    let retrieved = recycling.try_get_value(Some(data)).unwrap();
    assert!(Rc::ptr_eq(&control, &retrieved));
    assert!(recycling.try_get_value(None).is_none());
    assert!(recycling.try_get_value(Some(object())).is_none());
}

#[test]
fn build_with_existing_returns_existing() {
    let recycling = recycling();
    let data = object();
    let existing = object();
    let result = recycling
        .build(Some(data.clone()), Some(existing.clone()), None)
        .unwrap();
    assert!(Rc::ptr_eq(&existing, &result));
    assert!(Rc::ptr_eq(
        &existing,
        &recycling.try_get_value(Some(data)).unwrap()
    ));
}

#[test]
fn build_with_null_existing_creates_new_control() {
    let recycling = recycling();
    let data = object();
    let result = recycling.build(Some(data.clone()), None, None).unwrap();
    assert!(Rc::ptr_eq(
        &result,
        &recycling.try_get_value(Some(data)).unwrap()
    ));
}

#[test]
fn clear_removes_all_controls() {
    let recycling = recycling();
    let data = object();
    let control = object();
    let retained = Rc::downgrade(&control);
    recycling.add(data.clone(), control.clone());
    drop(control);
    assert!(retained.upgrade().is_some());
    recycling.clear();
    assert!(recycling.try_get_value(Some(data)).is_none());
    assert!(retained.upgrade().is_none());
}

#[test]
fn recycling_handles_compare_the_underlying_reference_identity() {
    let first = recycling();
    assert!(first.as_any().is_none());
    assert!(first == first.clone());
    assert!(first != recycling());
}

#[test]
fn id_provider_preserves_absent_and_empty_identifiers_for_callers_to_interpret() {
    struct Provider(Option<String>);
    impl IControlRecyclingIdProvider for Provider {
        fn get_control_recycling_id(&self) -> Option<String> {
            self.0.clone()
        }
    }
    for value in [None, Some(""), Some("document-1")] {
        let provider: Rc<dyn IControlRecyclingIdProvider> =
            Rc::new(Provider(value.map(String::from)));
        assert_eq!(provider.get_control_recycling_id().as_deref(), value);
        assert!(provider == provider.clone());
        let other: Rc<dyn IControlRecyclingIdProvider> = Rc::new(Provider(value.map(String::from)));
        assert!(provider != other);
    }
}
