//! Reading the values of nine styled properties of an object: the default
//! values, local values, and local values over style values, against
//! reading the fields of a plain object.

use crate::harness::Registry;
use crate::test_types::{Struct1, Struct2, Struct3, Struct4, Struct5, Struct6, Struct7, Struct8};
use ferroui_base::data::BindingPriority;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObject, FerroObjectImpl, FerroProperty, Ref,
    StyledProperty,
};
use std::hint::black_box;

pub struct FerroObjectGetValue {
    baseline: BaselineTestClass,
    target: Ref<TestClass>,
}

impl FerroObjectGetValue {
    pub fn new() -> Self {
        let baseline = BaselineTestClass { string_property: Some("foo"), ..BaselineTestClass::default() };
        let target = TestClass::new();

        TestClass::TYPE.ensure_class_init();

        Self { baseline, target }
    }

    pub fn get_clr_property_values(&self) -> i32 {
        // The fields are read through a reference the optimiser knows
        // nothing about: upstream reads them through property accessors of
        // an object on the heap.
        let target = black_box(&self.baseline);
        let mut result = 0;

        for _ in 0..100 {
            result += target.string_property.map_or(0, |s| s.len() as i32);
            result += target.struct1_property.int1;
            result += target.struct2_property.int1;
            result += target.struct3_property.int1;
            result += target.struct4_property.int1;
            result += target.struct5_property.int1;
            result += target.struct6_property.int1;
            result += target.struct7_property.int1;
            result += target.struct8_property.int1;
        }

        result
    }

    pub fn get_default_values(&self) -> i32 {
        let target = &self.target;
        let mut result = 0;

        for _ in 0..100 {
            result += target.get_value(TestClass::string_property()).map_or(0, |s| s.len() as i32);
            result += target.get_value(TestClass::struct1_property()).int1;
            result += target.get_value(TestClass::struct2_property()).int1;
            result += target.get_value(TestClass::struct3_property()).int1;
            result += target.get_value(TestClass::struct4_property()).int1;
            result += target.get_value(TestClass::struct5_property()).int1;
            result += target.get_value(TestClass::struct6_property()).int1;
            result += target.get_value(TestClass::struct7_property()).int1;
            result += target.get_value(TestClass::struct8_property()).int1;
        }

        result
    }

    /// The global setup of [`get_local_values`](Self::get_local_values).
    pub fn setup_local_values() -> Self {
        let this = Self::new();
        this.target.set_value(TestClass::string_property(), Some("foo".to_string()));
        this.target.set_value(TestClass::struct1_property(), Struct1::new(1));
        this.target.set_value(TestClass::struct2_property(), Struct2::new(1));
        this.target.set_value(TestClass::struct3_property(), Struct3::new(1));
        this.target.set_value(TestClass::struct4_property(), Struct4::new(1));
        this.target.set_value(TestClass::struct5_property(), Struct5::new(1));
        this.target.set_value(TestClass::struct6_property(), Struct6::new(1));
        this.target.set_value(TestClass::struct7_property(), Struct7::new(1));
        this.target.set_value(TestClass::struct8_property(), Struct8::new(1));
        this
    }

    pub fn get_local_values(&self) -> i32 {
        let target = &self.target;
        let mut result = 0;

        for _ in 0..100 {
            result += target.get_value(TestClass::string_property()).map_or(0, |s| s.len() as i32);
            result += target.get_value(TestClass::struct1_property()).int1;
            result += target.get_value(TestClass::struct2_property()).int1;
            result += target.get_value(TestClass::struct3_property()).int1;
            result += target.get_value(TestClass::struct4_property()).int1;
            result += target.get_value(TestClass::struct5_property()).int1;
            result += target.get_value(TestClass::struct6_property()).int1;
            result += target.get_value(TestClass::struct7_property()).int1;
            result += target.get_value(TestClass::struct8_property()).int1;
        }

        result
    }

    /// The global setup of
    /// [`get_local_values_with_style_values`](Self::get_local_values_with_style_values).
    pub fn setup_local_values_and_style_values() -> Self {
        let this = Self::new();
        let target = &this.target;
        target.set_value(TestClass::string_property(), Some("foo".to_string()));
        target.set_value(TestClass::struct1_property(), Struct1::new(1));
        target.set_value(TestClass::struct2_property(), Struct2::new(1));
        target.set_value(TestClass::struct3_property(), Struct3::new(1));
        target.set_value(TestClass::struct4_property(), Struct4::new(1));
        target.set_value(TestClass::struct5_property(), Struct5::new(1));
        target.set_value(TestClass::struct6_property(), Struct6::new(1));
        target.set_value(TestClass::struct7_property(), Struct7::new(1));
        target.set_value(TestClass::struct8_property(), Struct8::new(1));
        target.set_value_with_priority(TestClass::string_property(), Some("bar".to_string()), BindingPriority::Style);
        target.set_value_with_priority(TestClass::struct1_property(), Struct1::default(), BindingPriority::Style);
        target.set_value_with_priority(TestClass::struct2_property(), Struct2::default(), BindingPriority::Style);
        target.set_value_with_priority(TestClass::struct3_property(), Struct3::default(), BindingPriority::Style);
        target.set_value_with_priority(TestClass::struct4_property(), Struct4::default(), BindingPriority::Style);
        target.set_value_with_priority(TestClass::struct5_property(), Struct5::default(), BindingPriority::Style);
        target.set_value_with_priority(TestClass::struct6_property(), Struct6::default(), BindingPriority::Style);
        target.set_value_with_priority(TestClass::struct7_property(), Struct7::default(), BindingPriority::Style);
        target.set_value_with_priority(TestClass::struct8_property(), Struct8::default(), BindingPriority::Style);
        this
    }

    pub fn get_local_values_with_style_values(&self) -> i32 {
        let target = &self.target;
        let mut result = 0;

        for _ in 0..100 {
            result += target.get_value(TestClass::string_property()).map_or(0, |s| s.len() as i32);
            result += target.get_value(TestClass::struct1_property()).int1;
            result += target.get_value(TestClass::struct2_property()).int1;
            result += target.get_value(TestClass::struct3_property()).int1;
            result += target.get_value(TestClass::struct4_property()).int1;
            result += target.get_value(TestClass::struct5_property()).int1;
            result += target.get_value(TestClass::struct6_property()).int1;
            result += target.get_value(TestClass::struct7_property()).int1;
            result += target.get_value(TestClass::struct8_property()).int1;
        }

        result
    }
}

/// The object whose properties are read.
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

/// The plain object of the baseline: the same nine values as fields.
#[derive(Default)]
pub struct BaselineTestClass {
    pub string_property: Option<&'static str>,
    pub struct1_property: Struct1,
    pub struct2_property: Struct2,
    pub struct3_property: Struct3,
    pub struct4_property: Struct4,
    pub struct5_property: Struct5,
    pub struct6_property: Struct6,
    pub struct7_property: Struct7,
    pub struct8_property: Struct8,
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("base", "FerroObjectGetValue");
    class
        .benchmark("get_clr_property_values", "", FerroObjectGetValue::new, |b| b.get_clr_property_values())
        .baseline();
    class.benchmark("get_default_values", "", FerroObjectGetValue::new, |b| b.get_default_values());
    class.benchmark("get_local_values", "", FerroObjectGetValue::setup_local_values, |b| b.get_local_values());
    class.benchmark(
        "get_local_values_with_style_values",
        "",
        FerroObjectGetValue::setup_local_values_and_style_values,
        |b| b.get_local_values_with_style_values(),
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn ferro_object_get_value() {
        crate::harness::smoke_class(super::register, "FerroObjectGetValue");
    }
}
