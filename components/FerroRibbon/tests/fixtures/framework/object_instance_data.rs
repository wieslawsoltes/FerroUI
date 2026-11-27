//! Framework-only object-owned instance data. No FerroRibbon imports.
use ferroui_base::FerroObject;
use std::{cell::Cell, rc::Rc};
#[test]
fn instance_data_has_object_and_type_identity_without_property_notifications() {
    let a = FerroObject::new();
    let b = FerroObject::new();
    let changes = Rc::new(Cell::new(0));
    let c = changes.clone();
    a.add_property_changed_handler(Rc::new(move |_| c.set(c.get() + 1)));
    let first = Rc::new(String::from("same"));
    let second = Rc::new(String::from("same"));
    assert!(a.instance_data::<String>().is_none());
    assert!(a.set_instance_data(Some(first.clone())).is_none());
    assert!(b.instance_data::<String>().is_none());
    assert!(Rc::ptr_eq(&a.instance_data::<String>().unwrap(), &first));
    assert!(a.set_instance_data(Some(Rc::new(17_i32))).is_none());
    let old = a.set_instance_data(Some(second.clone())).unwrap();
    assert!(Rc::ptr_eq(&old, &first));
    assert!(Rc::ptr_eq(&a.instance_data::<String>().unwrap(), &second));
    assert_eq!(*a.instance_data::<i32>().unwrap(), 17);
    assert!(Rc::ptr_eq(
        &a.set_instance_data::<String>(None).unwrap(),
        &second
    ));
    assert!(a.instance_data::<String>().is_none());
    assert_eq!(changes.get(), 0);
}
#[test]
fn instance_data_retains_values_until_replacement_clear_or_owner_drop() {
    let a = FerroObject::new();
    let v = Rc::new(String::from("value"));
    let weak = Rc::downgrade(&v);
    a.set_instance_data(Some(v));
    assert!(weak.upgrade().is_some());
    a.set_instance_data::<String>(None);
    assert!(weak.upgrade().is_none());
    let v = Rc::new(String::from("next"));
    let weak = Rc::downgrade(&v);
    a.set_instance_data(Some(v));
    a.set_instance_data(Some(Rc::new(String::from("last"))));
    assert!(weak.upgrade().is_none());
    let v = Rc::new(99_i32);
    let weak = Rc::downgrade(&v);
    a.set_instance_data(Some(v));
    drop(a);
    assert!(weak.upgrade().is_none());
}
