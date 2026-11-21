//! Framework-only erased value recovery; no FerroRibbon imports or proxy values.
use ferroui_base::{
    BoxedValue, FerroObject,
    data::core::{ValueType, ValueTypes},
};
use std::{any::Any, cell::Cell, rc::Rc};
struct Model {
    value: Cell<i32>,
}
impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
#[test]
fn independent_registered_reference_recovers_same_nonclone_object_and_metadata_cast() {
    ValueTypes::register_reference::<Model>();
    let model = Rc::new(Model {
        value: Cell::new(5),
    });
    let erased: Rc<dyn Any> = model.clone();
    let boxed = ValueTypes::box_erased(erased.clone()).unwrap();
    let same: Rc<dyn Any> = boxed.clone();
    assert!(Rc::ptr_eq(&same, &erased));
    let handle = ValueTypes::try_cast(&boxed, ValueType::of::<Rc<Model>>()).unwrap();
    let handle = handle.downcast_ref::<Rc<Model>>().unwrap();
    assert!(Rc::ptr_eq(handle, &model));
    handle.value.set(8);
    assert_eq!(model.value.get(), 8);
}
#[test]
fn independent_native_object_and_scalar_recovery_preserves_identity_and_weak_lifetime() {
    let object = FerroObject::new();
    let erased: Rc<dyn Any> = Rc::new(object.clone());
    let boxed = ValueTypes::box_erased(erased.clone()).unwrap();
    assert_eq!(ValueTypes::as_object(&*boxed), Some(object));
    let same: Rc<dyn Any> = boxed;
    assert!(Rc::ptr_eq(&same, &erased));
    let text = Rc::new("unchanged".to_owned());
    let weak = Rc::downgrade(&text);
    let erased: Rc<dyn Any> = text.clone();
    let boxed: BoxedValue = ValueTypes::box_erased(erased).unwrap();
    let same: Rc<dyn Any> = boxed.clone();
    let original: Rc<dyn Any> = text.clone();
    assert!(Rc::ptr_eq(&same, &original));
    drop(text);
    drop(same);
    drop(original);
    assert!(weak.upgrade().is_some());
    drop(boxed);
    assert!(weak.upgrade().is_none());
}
#[test]
fn independent_unknown_payload_is_returned_without_loss_and_can_be_registered() {
    struct Unknown;
    impl PartialEq for Unknown {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }
    let original: Rc<dyn Any> = Rc::new(Unknown);
    let unknown = ValueTypes::box_erased(original.clone()).err().unwrap();
    assert!(Rc::ptr_eq(&unknown, &original));
    ValueTypes::register_erased_value::<Unknown>();
    let boxed = ValueTypes::box_erased(unknown).unwrap();
    let same: Rc<dyn Any> = boxed;
    assert!(Rc::ptr_eq(&same, &original));
}
