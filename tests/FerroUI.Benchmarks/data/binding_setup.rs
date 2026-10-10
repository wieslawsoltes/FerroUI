//! Binding a property to a property of the data context with a string
//! path, and disposing the binding, a hundred times.

use crate::harness::Registry;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_markup_type, ferro_properties, instantiate, BoxedValue, FerroObjectImpl,
    FerroProperty, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_markup::data::Binding;
use std::cell::Cell;
use std::rc::Rc;

pub struct BindingSetup {
    _data: Rc<TestData>,
    target: Ref<TestControl>,
}

impl BindingSetup {
    pub fn new() -> Self {
        let data = TestData::new();
        let target = TestControl::new();

        target.set_data_context(Some(data.clone() as BoxedValue));

        Self { _data: data, target }
    }

    pub fn setup_data_context_property_binding_one_way(&self) {
        let target = &self.target;
        let binding = Binding::with_path("IntValue");

        for _ in 0..100 {
            let d = target.bind_binding(TestControl::int_value_property(), &binding);
            d.dispose();
        }
    }

    pub fn setup_data_context_property_binding_two_way(&self) {
        let target = &self.target;
        let binding = Binding::with_path("IntValue").with_mode(BindingMode::TwoWay);

        for _ in 0..100 {
            let d = target.bind_binding(TestControl::int_value_property(), &binding);
            d.dispose();
        }
    }
}

#[repr(C)]
pub struct TestControl {
    base: Control,
}

ferro_class!(TestControl: Control);
ferro_impl_classes!(
    TestControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferro_properties! {
    impl TestControl {
        pub fn int_value_property() -> StyledProperty<i32> {
            FerroProperty::register::<TestControl, _>("IntValue", 0)
        }
    }
}

impl TestControl {
    pub fn construct() -> Self {
        Self { base: Control::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn int_value(&self) -> i32 {
        self.get_value(Self::int_value_property())
    }

    pub fn set_int_value(&self, value: i32) {
        self.set_value(Self::int_value_property(), value)
    }
}

pub struct TestData {
    int_value: Cell<i32>,
}

/// A reference type: two objects are equal when they are the same object.
impl PartialEq for TestData {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl TestData {
    /// Creates the object. Its property is found through its metadata, which
    /// is made known here (upstream finds it by reflection, which needs no
    /// registration).
    pub fn new() -> Rc<Self> {
        MarkupType::register(<TestData as MarkupTyped>::MARKUP);
        ValueTypes::register_reference::<TestData>();

        Rc::new(Self { int_value: Cell::new(0) })
    }

    pub fn int_value(&self) -> i32 {
        self.int_value.get()
    }

    pub fn set_int_value(&self, value: i32) {
        self.int_value.set(value);
    }
}

ferro_markup_type!(class TestData {
    this: Rc<TestData>,
    handles: [TestData, Rc<TestData>, Option<Rc<TestData>>],
    properties: [
        IntValue: i32 { get: TestData::int_value, set: TestData::set_int_value },
    ],
});

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("data", "BindingSetup");
    class.benchmark("setup_data_context_property_binding_one_way", "", BindingSetup::new, |b| {
        b.setup_data_context_property_binding_one_way()
    });
    class.benchmark("setup_data_context_property_binding_two_way", "", BindingSetup::new, |b| {
        b.setup_data_context_property_binding_two_way()
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn binding_setup() {
        crate::harness::smoke_class(super::register, "BindingSetup");
    }
}
