//! Binding a property to a property of the templated parent, and disposing
//! the binding, a hundred times.

use crate::harness::Registry;
use ferroui_base::data::{BindingMode, TemplateBinding};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Control, Decorator};
use std::rc::Rc;

pub struct TemplateBindingSetup {
    target: Ref<Decorator>,
    /// The target refers to its templated parent weakly.
    _templated_parent: Ref<Control>,
}

impl TemplateBindingSetup {
    pub fn new() -> Self {
        let target = Decorator::new();
        let templated_parent = Control::new();

        target.set_templated_parent(&templated_parent);
        templated_parent.set_tag(Some(Rc::new("parentTag".to_string()) as BoxedValue));

        Self { target, _templated_parent: templated_parent }
    }

    pub fn setup_template_binding_one_way(&self) {
        let target = &self.target;
        let binding = TemplateBinding::new(Control::tag_property());

        for _ in 0..100 {
            let d = target.bind_binding(Control::tag_property(), &binding);
            d.dispose();
        }
    }

    pub fn setup_template_binding_two_way(&self) {
        let target = &self.target;
        let binding = TemplateBinding::new(Control::tag_property()).with_mode(BindingMode::TwoWay);

        for _ in 0..100 {
            let d = target.bind_binding(Control::tag_property(), &binding);
            d.dispose();
        }
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("data", "TemplateBindingSetup");
    class.benchmark("setup_template_binding_one_way", "", TemplateBindingSetup::new, |b| {
        b.setup_template_binding_one_way()
    });
    class.benchmark("setup_template_binding_two_way", "", TemplateBindingSetup::new, |b| {
        b.setup_template_binding_two_way()
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn template_binding_setup() {
        crate::harness::smoke_class(super::register, "TemplateBindingSetup");
    }
}
