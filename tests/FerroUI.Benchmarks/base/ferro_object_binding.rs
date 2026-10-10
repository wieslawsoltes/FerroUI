//! Binding nine styled properties of an object with local value priority:
//! setting the bindings up and disposing them, and producing values through
//! them, with and without style values under them.

use crate::harness::Registry;
use crate::test_binding_observable::TestBindingObservable;
use crate::test_types::{Struct1, Struct2, Struct3, Struct4, Struct5, Struct6, Struct7, Struct8};
use ferroui_base::data::BindingPriority;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObject, FerroObjectImpl, FerroProperty, Ref,
    StyledProperty,
};
use std::rc::Rc;

/// The target and the sources are static fields upstream; a benchmark runs
/// on a thread of its own here, so they are the fields of its state.
pub struct FerroObjectBinding {
    target: Ref<TestClass>,
    string_source: Rc<TestBindingObservable<Option<String>>>,
    struct1_source: Rc<TestBindingObservable<Struct1>>,
    struct2_source: Rc<TestBindingObservable<Struct2>>,
    struct3_source: Rc<TestBindingObservable<Struct3>>,
    struct4_source: Rc<TestBindingObservable<Struct4>>,
    struct5_source: Rc<TestBindingObservable<Struct5>>,
    struct6_source: Rc<TestBindingObservable<Struct6>>,
    struct7_source: Rc<TestBindingObservable<Struct7>>,
    struct8_source: Rc<TestBindingObservable<Struct8>>,
}

impl FerroObjectBinding {
    pub fn new() -> Self {
        TestClass::TYPE.ensure_class_init();

        Self {
            target: TestClass::new(),
            string_source: TestBindingObservable::new(None),
            struct1_source: TestBindingObservable::new(Struct1::default()),
            struct2_source: TestBindingObservable::new(Struct2::default()),
            struct3_source: TestBindingObservable::new(Struct3::default()),
            struct4_source: TestBindingObservable::new(Struct4::default()),
            struct5_source: TestBindingObservable::new(Struct5::default()),
            struct6_source: TestBindingObservable::new(Struct6::default()),
            struct7_source: TestBindingObservable::new(Struct7::default()),
            struct8_source: TestBindingObservable::new(Struct8::default()),
        }
    }

    pub fn setup_dispose_local_value_bindings(&self) {
        let target = &self.target;

        for _ in 0..100 {
            let s0 = target.bind_value(
                TestClass::string_property(),
                self.string_source.clone(),
                BindingPriority::LocalValue,
            );
            let s1 = target.bind_value(
                TestClass::struct1_property(),
                self.struct1_source.clone(),
                BindingPriority::LocalValue,
            );
            let s2 = target.bind_value(
                TestClass::struct2_property(),
                self.struct2_source.clone(),
                BindingPriority::LocalValue,
            );
            let s3 = target.bind_value(
                TestClass::struct3_property(),
                self.struct3_source.clone(),
                BindingPriority::LocalValue,
            );
            let s4 = target.bind_value(
                TestClass::struct4_property(),
                self.struct4_source.clone(),
                BindingPriority::LocalValue,
            );
            let s5 = target.bind_value(
                TestClass::struct5_property(),
                self.struct5_source.clone(),
                BindingPriority::LocalValue,
            );
            let s6 = target.bind_value(
                TestClass::struct6_property(),
                self.struct6_source.clone(),
                BindingPriority::LocalValue,
            );
            let s7 = target.bind_value(
                TestClass::struct7_property(),
                self.struct7_source.clone(),
                BindingPriority::LocalValue,
            );
            let s8 = target.bind_value(
                TestClass::struct8_property(),
                self.struct8_source.clone(),
                BindingPriority::LocalValue,
            );

            // The end of the scope upstream: the bindings are disposed in
            // the reverse order of their declaration.
            s8.dispose();
            s7.dispose();
            s6.dispose();
            s5.dispose();
            s4.dispose();
            s3.dispose();
            s2.dispose();
            s1.dispose();
            s0.dispose();
        }
    }

    pub fn fire_local_value_bindings(&self) {
        let target = &self.target;

        let s0 =
            target.bind_value(TestClass::string_property(), self.string_source.clone(), BindingPriority::LocalValue);
        let s1 =
            target.bind_value(TestClass::struct1_property(), self.struct1_source.clone(), BindingPriority::LocalValue);
        let s2 =
            target.bind_value(TestClass::struct2_property(), self.struct2_source.clone(), BindingPriority::LocalValue);
        let s3 =
            target.bind_value(TestClass::struct3_property(), self.struct3_source.clone(), BindingPriority::LocalValue);
        let s4 =
            target.bind_value(TestClass::struct4_property(), self.struct4_source.clone(), BindingPriority::LocalValue);
        let s5 =
            target.bind_value(TestClass::struct5_property(), self.struct5_source.clone(), BindingPriority::LocalValue);
        let s6 =
            target.bind_value(TestClass::struct6_property(), self.struct6_source.clone(), BindingPriority::LocalValue);
        let s7 =
            target.bind_value(TestClass::struct7_property(), self.struct7_source.clone(), BindingPriority::LocalValue);
        let s8 =
            target.bind_value(TestClass::struct8_property(), self.struct8_source.clone(), BindingPriority::LocalValue);

        for i in 0..100 {
            self.string_source.on_next(Some(i.to_string()));
            self.struct1_source.on_next(Struct1::new(i + 1));
            self.struct2_source.on_next(Struct2::new(i + 1));
            self.struct3_source.on_next(Struct3::new(i + 1));
            self.struct4_source.on_next(Struct4::new(i + 1));
            self.struct5_source.on_next(Struct5::new(i + 1));
            self.struct6_source.on_next(Struct6::new(i + 1));
            self.struct7_source.on_next(Struct7::new(i + 1));
            self.struct8_source.on_next(Struct8::new(i + 1));
        }

        // The end of the scope upstream: the bindings are disposed in the
        // reverse order of their declaration.
        s8.dispose();
        s7.dispose();
        s6.dispose();
        s5.dispose();
        s4.dispose();
        s3.dispose();
        s2.dispose();
        s1.dispose();
        s0.dispose();
    }

    /// The global setup of
    /// [`fire_local_value_bindings_with_style_values`](Self::fire_local_value_bindings_with_style_values).
    pub fn setup_style_values() -> Self {
        let this = Self::new();
        let target = &this.target;
        target.set_value_with_priority(TestClass::string_property(), Some("foo".to_string()), BindingPriority::Style);
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

    pub fn fire_local_value_bindings_with_style_values(&self) {
        let target = &self.target;

        let s0 =
            target.bind_value(TestClass::string_property(), self.string_source.clone(), BindingPriority::LocalValue);
        let s1 =
            target.bind_value(TestClass::struct1_property(), self.struct1_source.clone(), BindingPriority::LocalValue);
        let s2 =
            target.bind_value(TestClass::struct2_property(), self.struct2_source.clone(), BindingPriority::LocalValue);
        let s3 =
            target.bind_value(TestClass::struct3_property(), self.struct3_source.clone(), BindingPriority::LocalValue);
        let s4 =
            target.bind_value(TestClass::struct4_property(), self.struct4_source.clone(), BindingPriority::LocalValue);
        let s5 =
            target.bind_value(TestClass::struct5_property(), self.struct5_source.clone(), BindingPriority::LocalValue);
        let s6 =
            target.bind_value(TestClass::struct6_property(), self.struct6_source.clone(), BindingPriority::LocalValue);
        let s7 =
            target.bind_value(TestClass::struct7_property(), self.struct7_source.clone(), BindingPriority::LocalValue);
        let s8 =
            target.bind_value(TestClass::struct8_property(), self.struct8_source.clone(), BindingPriority::LocalValue);

        for i in 0..100 {
            self.string_source.on_next(Some(i.to_string()));
            self.struct1_source.on_next(Struct1::new(i + 1));
            self.struct2_source.on_next(Struct2::new(i + 1));
            self.struct3_source.on_next(Struct3::new(i + 1));
            self.struct4_source.on_next(Struct4::new(i + 1));
            self.struct5_source.on_next(Struct5::new(i + 1));
            self.struct6_source.on_next(Struct6::new(i + 1));
            self.struct7_source.on_next(Struct7::new(i + 1));
            self.struct8_source.on_next(Struct8::new(i + 1));
        }

        // The end of the scope upstream: the bindings are disposed in the
        // reverse order of their declaration.
        s8.dispose();
        s7.dispose();
        s6.dispose();
        s5.dispose();
        s4.dispose();
        s3.dispose();
        s2.dispose();
        s1.dispose();
        s0.dispose();
    }
}

/// The object whose properties are bound.
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
    let mut class = registry.class("base", "FerroObjectBinding");
    class.benchmark("setup_dispose_local_value_bindings", "", FerroObjectBinding::new, |b| {
        b.setup_dispose_local_value_bindings()
    });
    class.benchmark("fire_local_value_bindings", "", FerroObjectBinding::new, |b| b.fire_local_value_bindings());
    class.benchmark(
        "fire_local_value_bindings_with_style_values",
        "",
        FerroObjectBinding::setup_style_values,
        |b| b.fire_local_value_bindings_with_style_values(),
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn ferro_object_binding() {
        crate::harness::smoke_class(super::register, "FerroObjectBinding");
    }
}
