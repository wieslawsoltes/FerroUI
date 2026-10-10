//! Compares the setup (create instance + attach + dispose) cost of the
//! three kinds of binding to a property of the data context:
//!
//!  - Typed:           the typed binding expression, from a compiled path of
//!                     one property with typed accessors. Strongly typed,
//!                     does not box.
//!  - CompiledBinding: the untyped compiled binding, from a compiled path of
//!                     one property with untyped accessors (an accessor that
//!                     follows change notifications + boxed values).
//!  - Reflection:      the binding with a string path.
//!
//! An `i32` property is used as boxing avoidance is the main selling point
//! of the typed expression. The binding instances are created once and
//! reused across the loop (as they are in real markup) so that only the
//! per-bind cost is measured, not path/expression construction.

use crate::harness::Registry;
use ferroui_base::data::core::plugins::PropertyInfoAccessorFactory;
use ferroui_base::data::core::{
    ClrPropertyInfo, PropertyGetter, PropertySetter, TypedClrPropertyInfo, ValueType, ValueTypes,
};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::data::{BindingError, BindingMode, CompiledBinding, CompiledBindingPathBuilder};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_markup_type, ferro_properties, instantiate, AnyValue, BoxedValue,
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_markup::data::Binding;
use std::cell::Cell;
use std::rc::Rc;

pub struct TypedBindingSetup {
    _data: Rc<TestData>,
    target: Ref<TestControl>,
    typed_one_way: Rc<CompiledBinding>,
    typed_two_way: Rc<CompiledBinding>,
    compiled_one_way: Rc<CompiledBinding>,
    compiled_two_way: Rc<CompiledBinding>,
    reflection_one_way: Rc<Binding>,
    reflection_two_way: Rc<Binding>,
}

impl TypedBindingSetup {
    pub fn new() -> Self {
        let data = TestData::new();
        let target = TestControl::new();
        let typed_one_way = Self::create_typed_binding(BindingMode::OneWay);
        let typed_two_way = Self::create_typed_binding(BindingMode::TwoWay);
        let compiled_one_way = Self::create_compiled_binding(BindingMode::OneWay);
        let compiled_two_way = Self::create_compiled_binding(BindingMode::TwoWay);
        let reflection_one_way = Binding::with_path("IntValue").with_mode(BindingMode::OneWay);
        let reflection_two_way = Binding::with_path("IntValue").with_mode(BindingMode::TwoWay);

        target.set_data_context(Some(data.clone() as BoxedValue));

        Self {
            _data: data,
            target,
            typed_one_way,
            typed_two_way,
            compiled_one_way,
            compiled_two_way,
            reflection_one_way,
            reflection_two_way,
        }
    }

    pub fn setup_typed_one_way(&self) {
        for _ in 0..100 {
            self.target.bind_binding(TestControl::int_value_property(), &self.typed_one_way).dispose();
        }
    }

    pub fn setup_compiled_binding_one_way(&self) {
        for _ in 0..100 {
            self.target.bind_binding(TestControl::int_value_property(), &self.compiled_one_way).dispose();
        }
    }

    pub fn setup_reflection_one_way(&self) {
        for _ in 0..100 {
            self.target.bind_binding(TestControl::int_value_property(), &self.reflection_one_way).dispose();
        }
    }

    pub fn setup_typed_two_way(&self) {
        for _ in 0..100 {
            self.target.bind_binding(TestControl::int_value_property(), &self.typed_two_way).dispose();
        }
    }

    pub fn setup_compiled_binding_two_way(&self) {
        for _ in 0..100 {
            self.target.bind_binding(TestControl::int_value_property(), &self.compiled_two_way).dispose();
        }
    }

    pub fn setup_reflection_two_way(&self) {
        for _ in 0..100 {
            self.target.bind_binding(TestControl::int_value_property(), &self.reflection_two_way).dispose();
        }
    }

    fn create_typed_binding(mode: BindingMode) -> Rc<CompiledBinding> {
        let getter: Rc<dyn Fn(&TestData) -> i32> = Rc::new(|v: &TestData| v.int_value());
        let setter: Rc<dyn Fn(&TestData, i32)> = Rc::new(|o: &TestData, v: i32| o.set_int_value(v));
        let property_info = TypedClrPropertyInfo::<TestData, i32>::new("IntValue", Some(getter), Some(setter));
        let path = CompiledBindingPathBuilder::new().typed_property_info_with(property_info, false).build();
        CompiledBinding::new(path).with_mode(mode)
    }

    /// Builds the untyped compiled binding: a compiled path with a property
    /// description of untyped accessors, read through an accessor that
    /// follows the change notifications of its owner. Differs from the typed
    /// binding only in the path element used.
    fn create_compiled_binding(mode: BindingMode) -> Rc<CompiledBinding> {
        let getter: PropertyGetter =
            Rc::new(|o: &dyn AnyValue| Some(Rc::new(TestData::cast(o).int_value()) as BoxedValue));
        let setter: PropertySetter = Rc::new(|o: &dyn AnyValue, v: Option<&BoxedValue>| {
            match v.and_then(|v| v.downcast_ref::<i32>()) {
                Some(v) => {
                    TestData::cast(o).set_int_value(*v);
                    Ok(())
                }
                None => Err(BindingError::message("Unable to cast the value to the type of the property 'IntValue'.")),
            }
        });
        let property_info = ClrPropertyInfo::new("IntValue", Some(getter), Some(setter), ValueType::of::<i32>());
        let path = CompiledBindingPathBuilder::new()
            .property(Rc::new(property_info), PropertyInfoAccessorFactory::create_inpc_property_accessor::<TestData>())
            .build();
        CompiledBinding::new(path).with_mode(mode)
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
    property_changed: Event<str>,
}

/// A reference type: two objects are equal when they are the same object.
impl PartialEq for TestData {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for TestData {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl TestData {
    /// Creates the object. The binding with a string path finds its property
    /// through its metadata, which is made known here (upstream finds it by
    /// reflection, which needs no registration).
    pub fn new() -> Rc<Self> {
        MarkupType::register(<TestData as MarkupTyped>::MARKUP);
        ValueTypes::register_reference::<TestData>();

        Rc::new(Self { int_value: Cell::new(0), property_changed: Event::new() })
    }

    /// The owner of a property as the type that declares it (the cast of
    /// the untyped accessors).
    fn cast(o: &dyn AnyValue) -> &TestData {
        o.downcast_ref::<TestData>().expect("the owner of the property is a TestData")
    }

    pub fn int_value(&self) -> i32 {
        self.int_value.get()
    }

    pub fn set_int_value(&self, value: i32) {
        if self.int_value.get() == value {
            return;
        }
        self.int_value.set(value);
        self.property_changed.raise("IntValue");
    }
}

ferro_markup_type!(class TestData {
    this: Rc<TestData>,
    handles: [TestData, Rc<TestData>, Option<Rc<TestData>>],
    properties: [
        IntValue: i32 { get: TestData::int_value, set: TestData::set_int_value },
    ],
    notify_property_changed: TestData,
});

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("data", "TypedBindingSetup");
    class.benchmark("setup_typed_one_way", "", TypedBindingSetup::new, |b| b.setup_typed_one_way());
    class.benchmark("setup_compiled_binding_one_way", "", TypedBindingSetup::new, |b| {
        b.setup_compiled_binding_one_way()
    });
    class.benchmark("setup_reflection_one_way", "", TypedBindingSetup::new, |b| b.setup_reflection_one_way());
    class.benchmark("setup_typed_two_way", "", TypedBindingSetup::new, |b| b.setup_typed_two_way());
    class.benchmark("setup_compiled_binding_two_way", "", TypedBindingSetup::new, |b| {
        b.setup_compiled_binding_two_way()
    });
    class.benchmark("setup_reflection_two_way", "", TypedBindingSetup::new, |b| b.setup_reflection_two_way());
}

#[cfg(test)]
mod tests {
    #[test]
    fn typed_binding_setup() {
        crate::harness::smoke_class(super::register, "TypedBindingSetup");
    }
}
