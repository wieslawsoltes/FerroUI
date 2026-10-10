//! Setting and binding an integer styled property of a new object: with
//! local value priority, with every priority, and with a validation and a
//! coercion callback.

use crate::harness::Registry;
use ferroui_base::data::{BindingPriority, BindingValue};
use ferroui_base::reactive::{IObserver, LightweightSubject};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObject, FerroObjectImpl, FerroProperty, Ref,
    StyledProperty, StyledPropertyOptions,
};
use std::rc::Rc;

/// The priorities from animation to style, in the order of their values:
/// what upstream counts through with the increment of the enumeration.
const PRIORITIES: [BindingPriority; 5] = [
    BindingPriority::Animation,
    BindingPriority::LocalValue,
    BindingPriority::StyleTrigger,
    BindingPriority::Template,
    BindingPriority::Style,
];

pub struct StyledPropertyBenchmarks;

impl StyledPropertyBenchmarks {
    pub fn new() -> Self {
        Self
    }

    pub fn set_int_property_local_value(&self) {
        let obj = StyledClass::new();

        for _ in 0..100 {
            obj.set_int_value(obj.int_value() + 1);
        }
    }

    pub fn set_int_property_multiple_priorities(&self) {
        let obj = StyledClass::new();
        let mut value = 0;

        for _ in 0..100 {
            for p in PRIORITIES {
                obj.set_value_with_priority(StyledClass::int_value_property(), value, p);
                value += 1;
            }
        }
    }

    pub fn set_int_property_templated_parent(&self) {
        let obj = StyledClass::new();

        for _ in 0..100 {
            obj.set_value_with_priority(
                StyledClass::int_value_property(),
                obj.int_value() + 1,
                BindingPriority::Template,
            );
        }
    }

    pub fn bind_int_property_local_value(&self) {
        let obj = StyledClass::new();
        let source = Rc::new(LightweightSubject::<BindingValue<i32>>::new());

        obj.bind_value(StyledClass::int_value_property(), source.clone(), BindingPriority::LocalValue);

        for i in 0..100 {
            source.on_next(BindingValue::new(i));
        }
    }

    pub fn bind_int_property_multiple_priorities(&self) {
        let obj = StyledClass::new();
        let mut sources: Vec<Rc<LightweightSubject<BindingValue<i32>>>> = Vec::new();
        let mut value = 0;

        for p in PRIORITIES {
            let source = Rc::new(LightweightSubject::<BindingValue<i32>>::new());
            sources.push(source.clone());
            obj.bind_value(StyledClass::int_value_property(), source, p);
        }

        for _ in 0..100 {
            for source in &sources {
                source.on_next(BindingValue::new(value));
                value += 1;
            }
        }
    }

    pub fn set_validated_int_property_local_value(&self) {
        let obj = StyledClass::new();

        for _ in 0..100 {
            obj.set_validated_int_value(obj.validated_int_value() + 1);
        }
    }

    pub fn set_coerced_int_property_local_value(&self) {
        let obj = StyledClass::new();

        for _ in 0..100 {
            obj.set_coerced_int_value(obj.coerced_int_value() + 1);
        }
    }
}

/// The object whose properties are set and bound.
#[repr(C)]
pub struct StyledClass {
    base: FerroObject,
}

ferro_class!(StyledClass: FerroObject);
ferro_impl_classes!(StyledClass: FerroObjectImpl);

ferro_properties! {
    impl StyledClass {
        pub fn int_value_property() -> StyledProperty<i32> {
            FerroProperty::register::<StyledClass, _>("IntValue", 0)
        }

        pub fn validated_int_value_property() -> StyledProperty<i32> {
            FerroProperty::register_with::<StyledClass, _>(
                "ValidatedIntValue",
                StyledPropertyOptions::new(0).validate(StyledClass::validate_int_value),
            )
        }

        pub fn coerced_int_value_property() -> StyledProperty<i32> {
            FerroProperty::register_with::<StyledClass, _>(
                "CoercedIntValue",
                StyledPropertyOptions::new(0).coerce(StyledClass::coerce_int_value),
            )
        }
    }
}

impl StyledClass {
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
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

    pub fn validated_int_value(&self) -> i32 {
        self.get_value(Self::validated_int_value_property())
    }

    pub fn set_validated_int_value(&self, value: i32) {
        self.set_value(Self::validated_int_value_property(), value)
    }

    pub fn coerced_int_value(&self) -> i32 {
        self.get_value(Self::coerced_int_value_property())
    }

    pub fn set_coerced_int_value(&self, value: i32) {
        self.set_value(Self::coerced_int_value_property(), value)
    }

    fn validate_int_value(arg: &i32) -> bool {
        *arg < 1000
    }

    fn coerce_int_value(_arg1: &FerroObject, arg2: i32) -> i32 {
        i32::min(1000, arg2)
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("base", "StyledPropertyBenchmarks");
    class.benchmark("set_int_property_local_value", "", StyledPropertyBenchmarks::new, |b| {
        b.set_int_property_local_value()
    });
    class.benchmark("set_int_property_multiple_priorities", "", StyledPropertyBenchmarks::new, |b| {
        b.set_int_property_multiple_priorities()
    });
    class.benchmark("set_int_property_templated_parent", "", StyledPropertyBenchmarks::new, |b| {
        b.set_int_property_templated_parent()
    });
    class.benchmark("bind_int_property_local_value", "", StyledPropertyBenchmarks::new, |b| {
        b.bind_int_property_local_value()
    });
    class.benchmark("bind_int_property_multiple_priorities", "", StyledPropertyBenchmarks::new, |b| {
        b.bind_int_property_multiple_priorities()
    });
    class.benchmark("set_validated_int_property_local_value", "", StyledPropertyBenchmarks::new, |b| {
        b.set_validated_int_property_local_value()
    });
    class.benchmark("set_coerced_int_property_local_value", "", StyledPropertyBenchmarks::new, |b| {
        b.set_coerced_int_property_local_value()
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn styled_property_benchmarks() {
        crate::harness::smoke_class(super::register, "StyledPropertyBenchmarks");
    }
}
