//! Allocation and ownership checks for the UI-independent erased object boundary.
use ferroribbon_core::model::{
    RibbonButtonModel, RibbonNodeModelExt, RibbonObject, RibbonObjectEquality,
};
use std::{any::Any, cell::Cell, rc::Rc};

// Deliberately neither Clone nor PartialEq: representation must not copy the value.
struct Payload(Cell<i32>);
struct View {
    payload: Rc<Payload>,
    comparisons: Rc<Cell<usize>>,
}
impl RibbonObjectEquality for View {
    fn equals(&self, other: &RibbonObject) -> bool {
        self.comparisons.set(self.comparisons.get() + 1);
        other
            .downcast_ref::<Payload>()
            .is_some_and(|p| p.0.get() == self.payload.0.get())
    }
}

#[test]
fn erased_view_keeps_actual_mutable_allocation_and_releases_it_after_last_owner() {
    let payload = Rc::new(Payload(Cell::new(7)));
    let weak = Rc::downgrade(&payload);
    let comparisons = Rc::new(Cell::new(0));
    let view = Rc::new(View {
        payload: payload.clone(),
        comparisons: comparisons.clone(),
    });
    let weak_view = Rc::downgrade(&view);
    let object = RibbonObject::with_erased_equality(payload.clone(), view);
    let model = RibbonButtonModel::new(None, None);
    model.set_tag(Some(object.clone()));
    drop(payload);
    drop(object);
    let tag = model.tag().unwrap();
    assert!(std::ptr::eq(
        tag.downcast_ref::<Payload>().unwrap(),
        weak.upgrade().unwrap().as_ref()
    ));
    tag.downcast_ref::<Payload>().unwrap().0.set(19);
    let equivalent = RibbonObject::reference(Rc::new(Payload(Cell::new(19))));
    model.set_tag(Some(equivalent.clone()));
    assert_eq!(comparisons.get(), 1);
    assert!(!model.tag().unwrap().ptr_eq(&equivalent));
    assert!((tag.equality() as &dyn Any).is::<View>());
    model.set_tag(None);
    assert!(weak.upgrade().is_some());
    drop(tag);
    assert!(weak.upgrade().is_none());
    assert!(weak_view.upgrade().is_none());
}

#[test]
fn typed_operation_receives_original_other_payload_and_never_shortcuts_same_reference() {
    let payload = Rc::new(Payload(Cell::new(1)));
    let weak = Rc::downgrade(&payload);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let object = RibbonObject::with_equality(payload.clone(), move |old, other| {
        assert!(std::ptr::eq(old, other.downcast_ref::<Payload>().unwrap()));
        observed.set(observed.get() + 1);
        false
    });
    let model = RibbonButtonModel::new(None, None);
    model.set_tag(Some(object.clone()));
    model.set_tag(Some(object.clone()));
    assert_eq!(calls.get(), 1);
    assert!(model.tag().unwrap().ptr_eq(&object));
    model.set_tag(None);
    drop(object);
    drop(payload);
    assert!(weak.upgrade().is_none());
}

#[test]
fn scalar_nan_and_signed_zero_keep_source_default_equality() {
    assert!(RibbonObject::value(f64::NAN) == RibbonObject::value(f64::NAN));
    assert!(RibbonObject::value(-0.0_f64) == RibbonObject::value(0.0_f64));
    assert!(RibbonObject::value("e\u{301}".to_owned()) != RibbonObject::value("é".to_owned()));
}
