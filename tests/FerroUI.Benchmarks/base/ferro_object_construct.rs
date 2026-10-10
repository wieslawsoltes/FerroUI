//! Creating an object and setting the local values of its nine styled
//! properties, against creating a plain object and assigning its fields.

use crate::harness::Registry;
use crate::test_types::{Struct1, Struct2, Struct3, Struct4, Struct5, Struct6, Struct7, Struct8};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObject, FerroObjectImpl, FerroProperty, Ref,
    StyledProperty,
};
use std::hint::black_box;

pub struct FerroObjectConstruct;

impl FerroObjectConstruct {
    pub fn new() -> Self {
        TestClass::TYPE.ensure_class_init();

        Self
    }

    pub fn construct_clr_object_and_set_values(&self) {
        let mut target = Box::new(BaselineTestClass::default());
        target.string_property = Some("foo");
        target.struct1_property = Struct1::new(1);
        target.struct2_property = Struct2::new(1);
        target.struct3_property = Struct3::new(1);
        target.struct4_property = Struct4::new(1);
        target.struct5_property = Struct5::new(1);
        target.struct6_property = Struct6::new(1);
        target.struct7_property = Struct7::new(1);
        target.struct8_property = Struct8::new(1);

        // Upstream allocates the object on the heap and assigns its
        // properties; nothing reads them, so the object is kept from the
        // optimiser here.
        black_box(target);
    }

    pub fn construct_and_set_values(&self) {
        let target = TestClass::new();
        target.set_value(TestClass::string_property(), Some("foo".to_string()));
        target.set_value(TestClass::struct1_property(), Struct1::new(1));
        target.set_value(TestClass::struct2_property(), Struct2::new(1));
        target.set_value(TestClass::struct3_property(), Struct3::new(1));
        target.set_value(TestClass::struct4_property(), Struct4::new(1));
        target.set_value(TestClass::struct5_property(), Struct5::new(1));
        target.set_value(TestClass::struct6_property(), Struct6::new(1));
        target.set_value(TestClass::struct7_property(), Struct7::new(1));
        target.set_value(TestClass::struct8_property(), Struct8::new(1));
    }
}

/// The object that is created.
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
    let mut class = registry.class("base", "FerroObjectConstruct");
    class
        .benchmark("construct_clr_object_and_set_values", "", FerroObjectConstruct::new, |b| {
            b.construct_clr_object_and_set_values()
        })
        .baseline();
    class.benchmark("construct_and_set_values", "", FerroObjectConstruct::new, |b| b.construct_and_set_values());
}

#[cfg(test)]
mod tests {
    #[test]
    fn ferro_object_construct() {
        crate::harness::smoke_class(super::register, "FerroObjectConstruct");
    }
}
