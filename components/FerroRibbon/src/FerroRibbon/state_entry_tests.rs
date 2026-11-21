use ferroribbon_core::{RibbonString, state::*};
use ferroui_base::{
    BoxedValue,
    data::core::{ValueType, ValueTypes},
};
use std::rc::Rc;
#[test]
fn state_entry_payload_transport_keeps_the_actual_rc_without_view_or_object_proxies() {
    crate::group_tests::run(|| {
        let owner = RibbonCustomization::new();
        let labels = owner.labels().unwrap();
        let weak = Rc::downgrade(&labels);
        let boxed: BoxedValue = Rc::new(labels.clone());
        let actual: Rc<dyn std::any::Any> = labels.clone();
        let object = ValueTypes::reference_object(&boxed).unwrap();
        let actual_value: BoxedValue = labels.clone();
        assert!(Rc::ptr_eq(&object, &actual_value));
        let payload = crate::metadata::model_objects::from_native(Some(boxed)).unwrap();
        assert!(Rc::ptr_eq(&payload.payload(), &actual));
        let native = crate::metadata::model_objects::to_native(Some(payload))
            .unwrap()
            .unwrap();
        let cast = ValueTypes::try_cast(&native, ValueType::of::<RibbonStateLabels>()).unwrap();
        let alias = cast.downcast_ref::<RibbonStateLabels>().unwrap();
        assert!(Rc::ptr_eq(alias, &labels));
        alias.insert("home".into(), Some(RibbonString::from("Start")));
        assert_eq!(
            owner.labels().unwrap().get(&"home".into()),
            Some(Some("Start".into()))
        );
        drop(owner);
        drop(labels);
        assert!(weak.upgrade().is_some());
        drop(actual);
        drop(actual_value);
        drop(object);
        drop(native);
        drop(cast);
        assert!(weak.upgrade().is_none());
    });
}
