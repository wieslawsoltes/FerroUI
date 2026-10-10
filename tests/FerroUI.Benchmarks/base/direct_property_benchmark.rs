//! Setting a direct property: through the member that sets the field and
//! raises the change, against doing the same steps by hand.

use crate::harness::Registry;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, DirectProperty, FerroObject, FerroObjectImpl,
    FerroProperty, Ref,
};
use std::cell::Cell;

pub struct DirectPropertyBenchmark {
    target: Ref<DirectClass>,
}

impl DirectPropertyBenchmark {
    pub fn new() -> Self {
        let target = DirectClass::new();

        DirectClass::TYPE.ensure_class_init();

        Self { target }
    }

    pub fn set_and_raise_original(&self) {
        let obj = &self.target;

        for _ in 0..100 {
            // The addition wraps, as the unchecked addition of upstream: the
            // object is the same for every invocation.
            obj.set_int_value(obj.int_value().wrapping_add(1));
        }
    }

    pub fn set_and_raise_simple(&self) {
        let obj = &self.target;

        for _ in 0..100 {
            obj.set_int_value_simple(obj.int_value_simple().wrapping_add(1));
        }
    }
}

/// The object whose direct property is set.
#[repr(C)]
pub struct DirectClass {
    base: FerroObject,
    int_value: Cell<i32>,
}

ferro_class!(DirectClass: FerroObject);
ferro_impl_classes!(DirectClass: FerroObjectImpl);

ferro_properties! {
    impl DirectClass {
        pub fn int_value_property() -> DirectProperty<DirectClass, i32> {
            FerroProperty::register_direct::<DirectClass, _>(
                "IntValue",
                |o| o.int_value(),
                Some(|o, v| o.set_int_value(v)),
                0,
            )
        }
    }
}

impl DirectClass {
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), int_value: Cell::new(0) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn int_value(&self) -> i32 {
        self.int_value.get()
    }

    pub fn set_int_value(&self, value: i32) {
        self.set_and_raise_cell(Self::int_value_property(), &self.int_value, value);
    }

    pub fn int_value_simple(&self) -> i32 {
        self.int_value.get()
    }

    pub fn set_int_value_simple(&self, value: i32) {
        self.verify_access();

        if self.int_value.get() == value {
            return;
        }

        let old = self.int_value.get();
        self.int_value.set(value);

        self.raise_direct_property_changed(Self::int_value_property(), &old, &self.int_value.get());
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("base", "DirectPropertyBenchmark");
    class
        .benchmark("set_and_raise_original", "", DirectPropertyBenchmark::new, |b| b.set_and_raise_original())
        .baseline();
    class.benchmark("set_and_raise_simple", "", DirectPropertyBenchmark::new, |b| b.set_and_raise_simple());
}

#[cfg(test)]
mod tests {
    #[test]
    fn direct_property_benchmark() {
        crate::harness::smoke_class(super::register, "DirectPropertyBenchmark");
    }
}
