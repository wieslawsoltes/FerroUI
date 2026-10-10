//! The binding benchmarks.

use crate::harness::Registry;

pub mod accessor_test_object;
pub mod binding_setup;
pub mod binding_values;
pub mod inherited_properties;
pub mod property_accessor_benchmarks;
pub mod property_accessor_plugin_benchmarks;
pub mod template_binding_setup;
pub mod template_binding_values;
pub mod typed_binding_setup;
pub mod typed_binding_values;

pub fn register(registry: &mut Registry) {
    binding_setup::register(registry);
    binding_values::register(registry);
    inherited_properties::register(registry);
    property_accessor_benchmarks::register(registry);
    property_accessor_plugin_benchmarks::register(registry);
    template_binding_setup::register(registry);
    template_binding_values::register(registry);
    typed_binding_setup::register(registry);
    typed_binding_values::register(registry);
}
