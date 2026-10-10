//! Finding the plugin that reads a plain property, with the plugins in the
//! old order (methods before plain properties) and in the new one.

use super::accessor_test_object::AccessorTestObject;
use crate::harness::Registry;
use ferroui_base::data::core::plugins::{
    FerroPropertyAccessorPlugin, IPropertyAccessorPlugin, InpcPropertyAccessorPlugin, MethodAccessorPlugin,
};
use std::rc::Rc;

pub struct PropertyAccessorPluginBenchmarks {
    target_strong_ref: Rc<AccessorTestObject>,

    old_plugins: Vec<Rc<dyn IPropertyAccessorPlugin>>,
    new_plugins: Vec<Rc<dyn IPropertyAccessorPlugin>>,
}

impl PropertyAccessorPluginBenchmarks {
    pub fn new() -> Self {
        let target_strong_ref = AccessorTestObject::new();

        let old_plugins: Vec<Rc<dyn IPropertyAccessorPlugin>> = vec![
            Rc::new(FerroPropertyAccessorPlugin),
            Rc::new(MethodAccessorPlugin),
            Rc::new(InpcPropertyAccessorPlugin),
        ];

        let new_plugins: Vec<Rc<dyn IPropertyAccessorPlugin>> = vec![
            Rc::new(FerroPropertyAccessorPlugin),
            Rc::new(InpcPropertyAccessorPlugin),
            Rc::new(MethodAccessorPlugin),
        ];

        Self { target_strong_ref, old_plugins, new_plugins }
    }

    pub fn match_accessor_old(&self) {
        let property_name = "Test";

        for x in &self.old_plugins {
            if x.match_(&*self.target_strong_ref, property_name) {
                break;
            }
        }
    }

    pub fn match_accessor_new(&self) {
        let property_name = "Test";

        for x in &self.new_plugins {
            if x.match_(&*self.target_strong_ref, property_name) {
                break;
            }
        }
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("data", "PropertyAccessorPluginBenchmarks");
    class.benchmark("match_accessor_old", "", PropertyAccessorPluginBenchmarks::new, |b| b.match_accessor_old());
    class.benchmark("match_accessor_new", "", PropertyAccessorPluginBenchmarks::new, |b| b.match_accessor_new());
}

#[cfg(test)]
mod tests {
    #[test]
    fn property_accessor_plugin_benchmarks() {
        crate::harness::smoke_class(super::register, "PropertyAccessorPluginBenchmarks");
    }
}
