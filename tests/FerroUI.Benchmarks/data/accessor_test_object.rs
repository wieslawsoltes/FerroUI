//! The object the property accessor benchmarks read: a property that
//! raises a change notification and a method with three overloads.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::BoxedValue;
use std::cell::RefCell;
use std::rc::Rc;

pub struct AccessorTestObject {
    test: RefCell<Option<String>>,
    property_changed: Event<str>,
}

/// A reference type: two objects are equal when they are the same object.
impl PartialEq for AccessorTestObject {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for AccessorTestObject {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl AccessorTestObject {
    /// Creates the object. Its members are found through its metadata, which
    /// is made known here (upstream finds them by reflection, which needs no
    /// registration).
    pub fn new() -> Rc<Self> {
        MarkupType::register(<AccessorTestObject as MarkupTyped>::MARKUP);
        ValueTypes::register_reference::<AccessorTestObject>();

        Rc::new(Self { test: RefCell::new(None), property_changed: Event::new() })
    }

    pub fn test(&self) -> Option<String> {
        self.test.borrow().clone()
    }

    pub fn set_test(&self, value: Option<String>) {
        if *self.test.borrow() == value {
            return;
        }

        *self.test.borrow_mut() = value;

        self.on_property_changed("Test");
    }

    pub fn execute(&self) {}

    pub fn execute_with_parameter(&self, _p0: Option<BoxedValue>) {}

    pub fn execute_with_parameters(&self, _p0: Option<BoxedValue>, _p1: Option<BoxedValue>) {}

    fn on_property_changed(&self, property_name: &str) {
        self.property_changed.raise(property_name);
    }
}

ferro_markup_type!(class AccessorTestObject {
    this: Rc<AccessorTestObject>,
    handles: [AccessorTestObject, Rc<AccessorTestObject>, Option<Rc<AccessorTestObject>>],
    properties: [
        Test: Option<String> { get: AccessorTestObject::test, set: AccessorTestObject::set_test },
    ],
    methods: [
        fn Execute() => AccessorTestObject::execute,
        fn Execute(Option<BoxedValue>) => AccessorTestObject::execute_with_parameter,
        fn Execute(Option<BoxedValue>, Option<BoxedValue>) => AccessorTestObject::execute_with_parameters,
    ],
    notify_property_changed: AccessorTestObject,
});
