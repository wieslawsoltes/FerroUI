//! The benchmarks of the object model: getting, setting, binding and
//! observing the properties of objects, and creating objects.

use crate::harness::Registry;

pub mod direct_property_benchmark;
pub mod ferro_object_binding;
pub mod ferro_object_construct;
pub mod ferro_object_get_observable;
pub mod ferro_object_get_value;
pub mod ferro_object_get_value_inherited;
pub mod ferro_object_initialization_benchmark;
pub mod ferro_object_set_value;
pub mod properties;
pub mod styled_property_benchmark;

pub fn register(registry: &mut Registry) {
    direct_property_benchmark::register(registry);
    ferro_object_binding::register(registry);
    ferro_object_construct::register(registry);
    ferro_object_get_observable::register(registry);
    ferro_object_get_value::register(registry);
    ferro_object_get_value_inherited::register(registry);
    ferro_object_initialization_benchmark::register(registry);
    ferro_object_set_value::register(registry);
    properties::register(registry);
    styled_property_benchmark::register(registry);
}
