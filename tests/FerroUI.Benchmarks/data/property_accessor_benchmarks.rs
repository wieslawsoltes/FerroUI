//! Matching a member and starting its accessor with the plugin of plain
//! properties and with the plugin of methods.

use super::accessor_test_object::AccessorTestObject;
use crate::harness::Registry;
use ferroui_base::data::core::plugins::{IPropertyAccessorPlugin, InpcPropertyAccessorPlugin, MethodAccessorPlugin};
use ferroui_base::data::core::WeakValue;
use ferroui_base::BoxedValue;
use std::rc::Rc;

pub struct PropertyAccessorBenchmarks {
    inpc_plugin: InpcPropertyAccessorPlugin,
    method_plugin: MethodAccessorPlugin,
    /// Keeps the object alive: the reference below does not.
    _target_strong_ref: Rc<AccessorTestObject>,
    target_weak_ref: WeakValue,
}

impl PropertyAccessorBenchmarks {
    pub fn new() -> Self {
        let target_strong_ref = AccessorTestObject::new();
        let target: BoxedValue = target_strong_ref.clone();
        let target_weak_ref = WeakValue::new(&target);

        Self {
            inpc_plugin: InpcPropertyAccessorPlugin,
            method_plugin: MethodAccessorPlugin,
            _target_strong_ref: target_strong_ref,
            target_weak_ref,
        }
    }

    // The two match benchmarks give the plugin the weak reference itself as
    // the object to match, as upstream does: what is measured is the lookup
    // of a member the object does not have.

    pub fn inpc_accessor_match(&self) {
        self.inpc_plugin.match_(&self.target_weak_ref, "Test");
    }

    pub fn inpc_accessor_start(&self) {
        self.inpc_plugin.start(&self.target_weak_ref, "Test");
    }

    pub fn method_accessor_match(&self) {
        self.method_plugin.match_(&self.target_weak_ref, "Execute");
    }

    pub fn method_accessor_start(&self) {
        self.method_plugin.start(&self.target_weak_ref, "Execute");
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("data", "PropertyAccessorBenchmarks");
    class.benchmark("inpc_accessor_match", "", PropertyAccessorBenchmarks::new, |b| b.inpc_accessor_match());
    class.benchmark("inpc_accessor_start", "", PropertyAccessorBenchmarks::new, |b| b.inpc_accessor_start());
    class.benchmark("method_accessor_match", "", PropertyAccessorBenchmarks::new, |b| b.method_accessor_match());
    class.benchmark("method_accessor_start", "", PropertyAccessorBenchmarks::new, |b| b.method_accessor_start());
}

#[cfg(test)]
mod tests {
    #[test]
    fn property_accessor_benchmarks() {
        crate::harness::smoke_class(super::register, "PropertyAccessorBenchmarks");
    }
}
