//! Clearing and setting an integer styled property, and binding it to a
//! subject that then produces a hundred values.

use crate::harness::Registry;
use ferroui_base::data::BindingPriority;
use ferroui_base::reactive::{IDisposable, IObserver, LightweightSubject};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObject, FerroObjectImpl, FerroProperty, Ref,
    StyledProperty,
};
use std::rc::Rc;

pub struct FerroObjectBenchmark {
    target: Ref<Class1>,
    int_binding: Rc<LightweightSubject<i32>>,
}

impl FerroObjectBenchmark {
    pub fn new() -> Self {
        let target = Class1::new();
        let int_binding = Rc::new(LightweightSubject::<i32>::new());

        target.set_value(Class1::int_property(), 123);

        Self { target, int_binding }
    }

    pub fn clear_and_set_int_property(&self) {
        self.target.clear_value(Class1::int_property());
        self.target.set_value(Class1::int_property(), 123);
    }

    pub fn bind_int_property(&self) {
        let binding = self.target.bind(Class1::int_property(), self.int_binding.clone(), BindingPriority::LocalValue);

        for i in 0..100 {
            self.int_binding.on_next(i);
        }

        binding.dispose();
    }
}

/// The object whose property is set and bound.
#[repr(C)]
pub struct Class1 {
    base: FerroObject,
}

ferro_class!(Class1: FerroObject);
ferro_impl_classes!(Class1: FerroObjectImpl);

ferro_properties! {
    impl Class1 {
        pub fn int_property() -> StyledProperty<i32> {
            FerroProperty::register::<Class1, _>("Int", 0)
        }
    }
}

impl Class1 {
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("base", "FerroObjectBenchmark");
    class.benchmark("clear_and_set_int_property", "", FerroObjectBenchmark::new, |b| b.clear_and_set_int_property());
    class.benchmark("bind_int_property", "", FerroObjectBenchmark::new, |b| b.bind_int_property());
}

#[cfg(test)]
mod tests {
    #[test]
    fn ferro_object_benchmark() {
        crate::harness::smoke_class(super::register, "FerroObjectBenchmark");
    }
}
