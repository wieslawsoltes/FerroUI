//! Changing a property of the templated parent a hundred times under a
//! binding to it.

use crate::harness::Registry;
use ferroui_base::data::{BindingMode, TemplateBinding};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Control, Decorator};
use std::rc::Rc;

pub struct TemplateBindingValues {
    target: Ref<Decorator>,
    templated_parent: Ref<Control>,
}

impl TemplateBindingValues {
    pub fn new() -> Self {
        let target = Decorator::new();
        let templated_parent = Control::new();

        target.set_templated_parent(&templated_parent);

        Self { target, templated_parent }
    }

    pub fn produce_template_binding_value_one_way(&self) {
        let target = &self.target;
        let binding = TemplateBinding::new(Control::tag_property());

        let d = target.bind_binding(Control::tag_property(), &binding);

        for i in 0..100_i32 {
            self.templated_parent.set_tag(Some(Rc::new(i) as BoxedValue));
        }

        d.dispose();
    }

    pub fn produce_template_binding_value_two_way(&self) {
        let target = &self.target;
        let binding = TemplateBinding::new(Control::tag_property()).with_mode(BindingMode::TwoWay);

        let d = target.bind_binding(Control::tag_property(), &binding);

        for i in 0..100_i32 {
            self.templated_parent.set_tag(Some(Rc::new(i * 2) as BoxedValue));
            target.set_current_value(Control::tag_property(), Some(Rc::new((i * 2) + 1) as BoxedValue));
        }

        d.dispose();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("data", "TemplateBindingValues");
    class.benchmark("produce_template_binding_value_one_way", "", TemplateBindingValues::new, |b| {
        b.produce_template_binding_value_one_way()
    });
    class.benchmark("produce_template_binding_value_two_way", "", TemplateBindingValues::new, |b| {
        b.produce_template_binding_value_two_way()
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn template_binding_values() {
        crate::harness::smoke_class(super::register, "TemplateBindingValues");
    }
}
