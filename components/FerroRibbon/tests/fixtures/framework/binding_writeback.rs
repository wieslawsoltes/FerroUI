//! SDK-only nested notification/writeback contract; no FerroRibbon imports.
use ferroui_base::data::core::{TypedClrPropertyInfo, Value};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{
    BindingMode, CompiledBinding, CompiledBindingPathBuilder, ReflectionBinding,
};
use ferroui_base::{
    FerroObjectImpl, FerroProperty, Ref, StyledElement, StyledElementImpl, StyledProperty,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_model, ferro_properties, instantiate,
};
use std::{cell::Cell, rc::Rc};
#[repr(C)]
struct Target {
    base: StyledElement,
}
ferro_class!(Target:StyledElement);
ferro_class_info!(Target { new: Target::new });
ferro_impl_classes!(Target:FerroObjectImpl,StyledElementImpl);
ferro_properties! {impl Target{fn value_property()->StyledProperty<f64>{FerroProperty::register_with::<Self,_>("Value",ferroui_base::StyledPropertyOptions::new(0.).binding_writeback_uses_change_value(true))}}}
impl Target {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: StyledElement::construct(),
        })
    }
    fn static_constructor() {
        Self::value_property()
            .changed()
            .add_class_handler::<Self>(|c, e| {
                let v = e.get_new_value::<f64>();
                if v > 10. {
                    c.set_value(Self::value_property(), 10.);
                }
            })
            .detach();
    }
}
struct Vm {
    value: Cell<f64>,
    changed: Event<str>,
}
impl INotifyPropertyChanged for Vm {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}
impl Vm {
    fn set(&self, v: f64) {
        if self.value.replace(v) != v {
            self.changed.raise("Value");
        }
    }
}
ferro_model!(Vm, |b| b.notify_property_changed().property::<Value<f64>>(
    "Value",
    |m| m.value.get(),
    |m, v| m.set(v)
));
#[test]
fn nested_change_can_preserve_property_notification_value_in_both_binding_modes() {
    for compiled in [true, false] {
        let vm = Model::new_model(Vm {
            value: Cell::new(4.),
            changed: Event::new(),
        });
        let t = Target::new();
        t.set_data_context(Some(vm.clone()));
        let _binding = if compiled {
            let info = TypedClrPropertyInfo::<Vm, f64>::new(
                "Value",
                Some(Rc::new(|m: &Vm| m.value.get())),
                Some(Rc::new(|m: &Vm, v| m.set(v))),
            );
            let b = CompiledBinding::new(
                CompiledBindingPathBuilder::new()
                    .typed_property_info(info)
                    .build(),
            )
            .with_mode(BindingMode::TwoWay);
            t.bind_binding(Target::value_property(), &*b)
        } else {
            let b = ReflectionBinding::new("Value");
            b.set_mode(BindingMode::TwoWay);
            t.bind_binding(Target::value_property(), &*b)
        };
        t.set_value(Target::value_property(), 25.);
        assert_eq!(t.get_value(Target::value_property()), 10.);
        assert_eq!(
            vm.value.get(),
            25.,
            "compiled={compiled}: source event snapshot must survive nested correction"
        );
    }
}
