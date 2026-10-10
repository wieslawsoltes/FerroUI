//! Reading the values of nine inherited styled properties of a control
//! whose values are set on an ancestor a number of logical parents away.

use crate::harness::Registry;
use crate::test_types::{Struct1, Struct2, Struct3, Struct4, Struct5, Struct6, Struct7, Struct8};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, StyledPropertyOptions, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};

/// The values of the `Depth` parameter.
const DEPTHS: [i32; 6] = [1, 2, 10, 50, 100, 200];

pub struct FerroObjectGetValueInherited {
    target: Ref<TestClass>,
    /// The root and every control under it, the target last. Upstream a
    /// control holds its logical parent, so the target keeps the chain
    /// alive; here an element holds its parent weakly, so the chain is held
    /// by the benchmark.
    _chain: Vec<Ref<TestClass>>,
}

impl FerroObjectGetValueInherited {
    pub fn new(depth: i32) -> Self {
        TestClass::TYPE.ensure_class_init();

        let root = TestClass::new();
        root.set_value(TestClass::string_property(), Some("foo".to_string()));
        root.set_value(TestClass::struct1_property(), Struct1::new(1));
        root.set_value(TestClass::struct2_property(), Struct2::new(1));
        root.set_value(TestClass::struct3_property(), Struct3::new(1));
        root.set_value(TestClass::struct4_property(), Struct4::new(1));
        root.set_value(TestClass::struct5_property(), Struct5::new(1));
        root.set_value(TestClass::struct6_property(), Struct6::new(1));
        root.set_value(TestClass::struct7_property(), Struct7::new(1));
        root.set_value(TestClass::struct8_property(), Struct8::new(1));

        let mut chain = vec![root.clone()];
        let mut parent = root;

        for _ in 0..depth {
            let c = TestClass::new();
            c.set_parent(&parent);
            chain.push(c.clone());
            parent = c;
        }

        Self { target: parent, _chain: chain }
    }

    pub fn get_inherited_values(&self) -> i32 {
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

/// The control of the chain.
#[repr(C)]
pub struct TestClass {
    base: Control,
}

ferro_class!(TestClass: Control);
ferro_impl_classes!(
    TestClass: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferro_properties! {
    impl TestClass {
        pub fn string_property() -> StyledProperty<Option<String>> {
            FerroProperty::register_with::<TestClass, _>("String", StyledPropertyOptions::new(None).inherits(true))
        }

        pub fn struct1_property() -> StyledProperty<Struct1> {
            FerroProperty::register_with::<TestClass, _>(
                "Struct1",
                StyledPropertyOptions::new(Struct1::default()).inherits(true),
            )
        }

        pub fn struct2_property() -> StyledProperty<Struct2> {
            FerroProperty::register_with::<TestClass, _>(
                "Struct2",
                StyledPropertyOptions::new(Struct2::default()).inherits(true),
            )
        }

        pub fn struct3_property() -> StyledProperty<Struct3> {
            FerroProperty::register_with::<TestClass, _>(
                "Struct3",
                StyledPropertyOptions::new(Struct3::default()).inherits(true),
            )
        }

        pub fn struct4_property() -> StyledProperty<Struct4> {
            FerroProperty::register_with::<TestClass, _>(
                "Struct4",
                StyledPropertyOptions::new(Struct4::default()).inherits(true),
            )
        }

        pub fn struct5_property() -> StyledProperty<Struct5> {
            FerroProperty::register_with::<TestClass, _>(
                "Struct5",
                StyledPropertyOptions::new(Struct5::default()).inherits(true),
            )
        }

        pub fn struct6_property() -> StyledProperty<Struct6> {
            FerroProperty::register_with::<TestClass, _>(
                "Struct6",
                StyledPropertyOptions::new(Struct6::default()).inherits(true),
            )
        }

        pub fn struct7_property() -> StyledProperty<Struct7> {
            FerroProperty::register_with::<TestClass, _>(
                "Struct7",
                StyledPropertyOptions::new(Struct7::default()).inherits(true),
            )
        }

        pub fn struct8_property() -> StyledProperty<Struct8> {
            FerroProperty::register_with::<TestClass, _>(
                "Struct8",
                StyledPropertyOptions::new(Struct8::default()).inherits(true),
            )
        }
    }
}

impl TestClass {
    pub fn construct() -> Self {
        Self { base: Control::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("base", "FerroObjectGetValueInherited");
    for depth in DEPTHS {
        class.benchmark(
            "get_inherited_values",
            format!("Depth={depth}"),
            move || FerroObjectGetValueInherited::new(depth),
            |b| b.get_inherited_values(),
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ferro_object_get_value_inherited() {
        crate::harness::smoke_class(super::register, "FerroObjectGetValueInherited");
    }
}
