//! Listening to the changes of nine styled properties of an object: through
//! the property changed event of the object against one observable per
//! property.

use crate::harness::Registry;
use crate::test_types::{Struct1, Struct2, Struct3, Struct4, Struct5, Struct6, Struct7, Struct8};
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObject, FerroObjectExtensions,
    FerroObjectImpl, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty,
};
use std::cell::Cell;

thread_local! {
    /// The static field of the upstream class that the handlers add to. A
    /// benchmark runs on a thread of its own, so the field of the thread is
    /// the field of the benchmark.
    static RESULT: Cell<i32> = const { Cell::new(0) };
}

/// `result += value`, with the wrapping of the unchecked addition of
/// upstream: the field is never reset.
fn add_to_result(value: i32) {
    RESULT.set(RESULT.get().wrapping_add(value));
}

pub struct FerroObjectGetObservable {
    target: Ref<TestClass>,
}

impl FerroObjectGetObservable {
    pub fn new() -> Self {
        TestClass::TYPE.ensure_class_init();

        Self { target: TestClass::new() }
    }

    /// What the handlers have added up so far.
    pub fn result() -> i32 {
        RESULT.get()
    }

    pub fn property_changed_subscription(&self) {
        let target = &self.target;

        fn change_handler(e: &FerroPropertyChangedEventArgs<'_>) {
            if e.property() == TestClass::string_property().as_property() {
                let value = e.new_value().downcast_ref::<Option<String>>().expect("the value of the String property");
                add_to_result(value.as_ref().map_or(0, |s| s.len() as i32));
            } else if e.property() == TestClass::struct1_property().as_property() {
                add_to_result(e.get_new_value::<Struct1>().int1);
            } else if e.property() == TestClass::struct2_property().as_property() {
                add_to_result(e.get_new_value::<Struct2>().int1);
            } else if e.property() == TestClass::struct3_property().as_property() {
                add_to_result(e.get_new_value::<Struct3>().int1);
            } else if e.property() == TestClass::struct4_property().as_property() {
                add_to_result(e.get_new_value::<Struct4>().int1);
            } else if e.property() == TestClass::struct5_property().as_property() {
                add_to_result(e.get_new_value::<Struct5>().int1);
            } else if e.property() == TestClass::struct6_property().as_property() {
                add_to_result(e.get_new_value::<Struct6>().int1);
            } else if e.property() == TestClass::struct7_property().as_property() {
                add_to_result(e.get_new_value::<Struct7>().int1);
            } else if e.property() == TestClass::struct8_property().as_property() {
                add_to_result(e.get_new_value::<Struct8>().int1);
            }
        }

        let subscription = target.property_changed(change_handler);

        // An observable of a property fires with the initial value, so to
        // compare like for like the initial values are read here too.
        add_to_result(target.get_value(TestClass::string_property()).map_or(0, |s| s.len() as i32));
        add_to_result(target.get_value(TestClass::struct1_property()).int1);
        add_to_result(target.get_value(TestClass::struct2_property()).int1);
        add_to_result(target.get_value(TestClass::struct3_property()).int1);
        add_to_result(target.get_value(TestClass::struct4_property()).int1);
        add_to_result(target.get_value(TestClass::struct5_property()).int1);
        add_to_result(target.get_value(TestClass::struct6_property()).int1);
        add_to_result(target.get_value(TestClass::struct7_property()).int1);
        add_to_result(target.get_value(TestClass::struct8_property()).int1);

        for i in 0..100 {
            target.set_value(TestClass::string_property(), Some(format!("foo{i}")));
            target.set_value(TestClass::struct1_property(), Struct1::new(i + 1));
            target.set_value(TestClass::struct2_property(), Struct2::new(i + 1));
            target.set_value(TestClass::struct3_property(), Struct3::new(i + 1));
            target.set_value(TestClass::struct4_property(), Struct4::new(i + 1));
            target.set_value(TestClass::struct5_property(), Struct5::new(i + 1));
            target.set_value(TestClass::struct6_property(), Struct6::new(i + 1));
            target.set_value(TestClass::struct7_property(), Struct7::new(i + 1));
            target.set_value(TestClass::struct8_property(), Struct8::new(i + 1));
        }

        subscription.dispose();
    }

    pub fn get_observables(&self) {
        let target = &self.target;

        let sub1 = target
            .get_observable(TestClass::string_property())
            .subscribe_fn(|x| add_to_result(x.map_or(0, |s| s.len() as i32)));
        let sub2 = target.get_observable(TestClass::struct1_property()).subscribe_fn(|x| add_to_result(x.int1));
        let sub3 = target.get_observable(TestClass::struct2_property()).subscribe_fn(|x| add_to_result(x.int1));
        let sub4 = target.get_observable(TestClass::struct3_property()).subscribe_fn(|x| add_to_result(x.int1));
        let sub5 = target.get_observable(TestClass::struct4_property()).subscribe_fn(|x| add_to_result(x.int1));
        let sub6 = target.get_observable(TestClass::struct5_property()).subscribe_fn(|x| add_to_result(x.int1));
        let sub7 = target.get_observable(TestClass::struct6_property()).subscribe_fn(|x| add_to_result(x.int1));
        let sub8 = target.get_observable(TestClass::struct7_property()).subscribe_fn(|x| add_to_result(x.int1));
        let sub9 = target.get_observable(TestClass::struct8_property()).subscribe_fn(|x| add_to_result(x.int1));

        for i in 0..100 {
            target.set_value(TestClass::string_property(), Some(format!("foo{i}")));
            target.set_value(TestClass::struct1_property(), Struct1::new(i + 1));
            target.set_value(TestClass::struct2_property(), Struct2::new(i + 1));
            target.set_value(TestClass::struct3_property(), Struct3::new(i + 1));
            target.set_value(TestClass::struct4_property(), Struct4::new(i + 1));
            target.set_value(TestClass::struct5_property(), Struct5::new(i + 1));
            target.set_value(TestClass::struct6_property(), Struct6::new(i + 1));
            target.set_value(TestClass::struct7_property(), Struct7::new(i + 1));
            target.set_value(TestClass::struct8_property(), Struct8::new(i + 1));
        }

        sub1.dispose();
        sub2.dispose();
        sub3.dispose();
        sub4.dispose();
        sub5.dispose();
        sub6.dispose();
        sub7.dispose();
        sub8.dispose();
        sub9.dispose();
    }
}

/// The object whose properties are listened to.
#[repr(C)]
pub struct TestClass {
    base: FerroObject,
}

ferro_class!(TestClass: FerroObject);
ferro_impl_classes!(TestClass: FerroObjectImpl);

ferro_properties! {
    impl TestClass {
        pub fn string_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<TestClass, _>("String", None)
        }

        pub fn struct1_property() -> StyledProperty<Struct1> {
            FerroProperty::register::<TestClass, _>("Struct1", Struct1::default())
        }

        pub fn struct2_property() -> StyledProperty<Struct2> {
            FerroProperty::register::<TestClass, _>("Struct2", Struct2::default())
        }

        pub fn struct3_property() -> StyledProperty<Struct3> {
            FerroProperty::register::<TestClass, _>("Struct3", Struct3::default())
        }

        pub fn struct4_property() -> StyledProperty<Struct4> {
            FerroProperty::register::<TestClass, _>("Struct4", Struct4::default())
        }

        pub fn struct5_property() -> StyledProperty<Struct5> {
            FerroProperty::register::<TestClass, _>("Struct5", Struct5::default())
        }

        pub fn struct6_property() -> StyledProperty<Struct6> {
            FerroProperty::register::<TestClass, _>("Struct6", Struct6::default())
        }

        pub fn struct7_property() -> StyledProperty<Struct7> {
            FerroProperty::register::<TestClass, _>("Struct7", Struct7::default())
        }

        pub fn struct8_property() -> StyledProperty<Struct8> {
            FerroProperty::register::<TestClass, _>("Struct8", Struct8::default())
        }
    }
}

impl TestClass {
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("base", "FerroObjectGetObservable");
    class
        .benchmark("property_changed_subscription", "", FerroObjectGetObservable::new, |b| {
            b.property_changed_subscription()
        })
        .baseline();
    class.benchmark("get_observables", "", FerroObjectGetObservable::new, |b| b.get_observables());
}

#[cfg(test)]
mod tests {
    #[test]
    fn ferro_object_get_observable() {
        crate::harness::smoke_class(super::register, "FerroObjectGetObservable");
    }
}
