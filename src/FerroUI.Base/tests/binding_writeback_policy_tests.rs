//! The nested notification and writeback contract.
use crate::data::core::{TypedClrPropertyInfo, Value};
use crate::data::model::{Event, INotifyPropertyChanged, Model};
use crate::data::{BindingMode, CompiledBinding, CompiledBindingPathBuilder, ReflectionBinding};
use crate::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_model, ferro_properties, instantiate,
    FerroObjectImpl, FerroProperty, Ref, StyledElement, StyledElementImpl, StyledProperty,
};
use std::{cell::Cell, rc::Rc};
#[repr(C)]
struct Target {
    base: StyledElement,
}
ferro_class!(Target:StyledElement);
ferro_class_info!(Target { new: Target::new });
ferro_impl_classes!(Target:FerroObjectImpl,StyledElementImpl);
ferro_properties! {impl Target{fn value_property()->StyledProperty<f64>{FerroProperty::register_with::<Self,_>("Value",crate::StyledPropertyOptions::new(0.).coerce(|_,v|v).binding_writeback_uses_change_value(true))}}}
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
            });
    }
}

#[repr(C)]
struct CurrentTarget {
    base: Target,
}
ferro_class!(CurrentTarget:Target);
ferro_class_info!(CurrentTarget {
    new: CurrentTarget::new
});
ferro_impl_classes!(CurrentTarget:FerroObjectImpl,StyledElementImpl);
impl CurrentTarget {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Target {
                base: StyledElement::construct(),
            },
        })
    }
    fn static_constructor() {
        Target::value_property().override_metadata::<Self>(
            crate::StyledPropertyMetadata::new(None)
                .with_binding_writeback_uses_change_value(false),
        );
    }
}
#[repr(C)]
struct CurrentGrandchild {
    base: CurrentTarget,
}
ferro_class!(CurrentGrandchild:CurrentTarget);
ferro_class_info!(CurrentGrandchild {
    new: CurrentGrandchild::new
});
ferro_impl_classes!(CurrentGrandchild:FerroObjectImpl,StyledElementImpl);
impl CurrentGrandchild {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: CurrentTarget {
                base: Target {
                    base: StyledElement::construct(),
                },
            },
        })
    }
}

#[test]
fn explicit_effective_value_policy_merges_into_downstream_subclasses() {
    for compiled in [true, false] {
        let vm = Model::new_model(Vm {
            value: Cell::new(4.),
            changed: Event::new(),
        });
        let target = CurrentGrandchild::new();
        target.set_data_context(Some(vm.clone()));
        assert_eq!(
            Target::value_property()
                .get_metadata_for(&target)
                .binding_writeback_uses_change_value(),
            Some(false)
        );
        let _expression = if compiled {
            let info = TypedClrPropertyInfo::<Vm, f64>::new(
                "Value",
                Some(Rc::new(|m: &Vm| m.value.get())),
                Some(Rc::new(|m: &Vm, v| m.set(v))),
            );
            target.bind_binding(
                Target::value_property(),
                &*CompiledBinding::new(
                    CompiledBindingPathBuilder::new()
                        .typed_property_info(info)
                        .build(),
                )
                .with_mode(BindingMode::TwoWay),
            )
        } else {
            let b = ReflectionBinding::new("Value");
            b.set_mode(BindingMode::TwoWay);
            target.bind_binding(Target::value_property(), &*b)
        };
        target.set_value(Target::value_property(), 25.);
        assert_eq!(target.get_value(Target::value_property()), 10.);
        assert_eq!(vm.value.get(), 10., "compiled={compiled}");
    }
}

#[test]
fn snapshot_source_push_does_not_echo_nested_coercion_to_model() {
    for compiled in [true, false] {
        let vm = Model::new_model(Vm {
            value: Cell::new(4.),
            changed: Event::new(),
        });
        let target = Target::new();
        target.set_data_context(Some(vm.clone()));
        let _expression = if compiled {
            let info = TypedClrPropertyInfo::<Vm, f64>::new(
                "Value",
                Some(Rc::new(|m: &Vm| m.value.get())),
                Some(Rc::new(|m: &Vm, v| m.set(v))),
            );
            target.bind_binding(
                Target::value_property(),
                &*CompiledBinding::new(
                    CompiledBindingPathBuilder::new()
                        .typed_property_info(info)
                        .build(),
                )
                .with_mode(BindingMode::TwoWay),
            )
        } else {
            let b = ReflectionBinding::new("Value");
            b.set_mode(BindingMode::TwoWay);
            target.bind_binding(Target::value_property(), &*b)
        };
        vm.set(25.);
        assert_eq!(target.get_value(Target::value_property()), 10.);
        assert_eq!(vm.value.get(), 25., "compiled={compiled}");
        vm.set(8.);
        assert_eq!(target.get_value(Target::value_property()), 8.);
        target.set_value(Target::value_property(), 25.);
        assert_eq!(vm.value.get(), 25.);
        assert_eq!(target.get_value(Target::value_property()), 10.);
        vm.set(8.);
        assert_eq!(
            target.get_value(Target::value_property()),
            8.,
            "reset to the pre-writeback source value, compiled={compiled}"
        );
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
        if v == 5. {
            panic!("source setter failed");
        }
        if self.value.replace(v) != v {
            self.changed.raise("Value");
        }
    }
}

#[test]
fn source_setter_failure_restores_snapshot_direction_guard() {
    for compiled in [true, false] {
        let vm = Model::new_model(Vm {
            value: Cell::new(4.),
            changed: Event::new(),
        });
        let target = Target::new();
        target.set_data_context(Some(vm.clone()));
        let _expression = if compiled {
            let info = TypedClrPropertyInfo::<Vm, f64>::new(
                "Value",
                Some(Rc::new(|m: &Vm| m.value.get())),
                Some(Rc::new(|m: &Vm, v| m.set(v))),
            );
            target.bind_binding(
                Target::value_property(),
                &*CompiledBinding::new(
                    CompiledBindingPathBuilder::new()
                        .typed_property_info(info)
                        .build(),
                )
                .with_mode(BindingMode::TwoWay),
            )
        } else {
            let b = ReflectionBinding::new("Value");
            b.set_mode(BindingMode::TwoWay);
            target.bind_binding(Target::value_property(), &*b)
        };
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || target.set_value(Target::value_property(), 5.)
        ))
        .is_err());
        target.set_value(Target::value_property(), 6.);
        assert_eq!(vm.value.get(), 6.);
    }
}

#[test]
fn policy_survives_type_safe_metadata_and_direct_metadata_construction() {
    let t = Target::new();
    assert_eq!(
        Target::value_property()
            .get_metadata_for(&t)
            .binding_writeback_uses_change_value(),
        Some(true)
    );
    assert_eq!(
        Target::value_property()
            .get_metadata(StyledElement::TYPE)
            .binding_writeback_uses_change_value(),
        Some(true)
    );
    assert_eq!(
        crate::DirectPropertyMetadata::new(Some(0.))
            .with_binding_writeback_uses_change_value(false)
            .binding_writeback_uses_change_value(),
        Some(false)
    );
}

#[test]
fn reentrant_explicit_source_update_does_not_replace_snapshot_writeback() {
    let vm = Model::new_model(Vm {
        value: Cell::new(4.),
        changed: Event::new(),
    });
    let target = Target::new();
    target.set_data_context(Some(vm.clone()));
    let binding = ReflectionBinding::new("Value");
    binding.set_mode(BindingMode::TwoWay);
    let expression = target.bind_binding(Target::value_property(), &*binding);
    let weak = Rc::downgrade(&expression);
    vm.changed.add(Rc::new(move |_| {
        if let Some(expression) = weak.upgrade() {
            expression.update_source();
        }
    }));
    target.set_value(Target::value_property(), 25.);
    assert_eq!(target.get_value(Target::value_property()), 10.);
    assert_eq!(vm.value.get(), 25.);
}

#[derive(Clone, PartialEq)]
struct Scalar {
    value: f64,
}
crate::styled_value!(Scalar);
#[repr(C)]
struct CastTarget {
    base: StyledElement,
}
ferro_class!(CastTarget:StyledElement);
ferro_class_info!(CastTarget {
    new: CastTarget::new
});
ferro_impl_classes!(CastTarget:FerroObjectImpl,StyledElementImpl);
ferro_properties! {impl CastTarget{fn value_property()->StyledProperty<Scalar>{FerroProperty::register_with::<Self,_>("Value",crate::StyledPropertyOptions::new(Scalar{value:0.}).binding_writeback_uses_change_value(true))}}}
impl CastTarget {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: StyledElement::construct(),
        })
    }
    fn static_constructor() {
        Self::value_property()
            .changed()
            .add_class_handler::<Self>(|t, e| {
                if e.get_new_value::<Scalar>().value > 10. {
                    t.set_value(Self::value_property(), Scalar { value: 10. });
                }
            });
    }
}
#[test]
fn snapshot_policy_casts_registered_reference_forms_for_typed_source() {
    crate::data::core::ValueTypes::register_reference::<Scalar>();
    let vm = Model::new_model(Vm {
        value: Cell::new(4.),
        changed: Event::new(),
    });
    let target = CastTarget::new();
    target.set_data_context(Some(vm.clone()));
    let info = TypedClrPropertyInfo::<Vm, Rc<Scalar>>::new(
        "Value",
        Some(Rc::new(|m: &Vm| {
            Rc::new(Scalar {
                value: m.value.get(),
            })
        })),
        Some(Rc::new(|m: &Vm, v: Rc<Scalar>| m.set(v.value))),
    );
    let expression = target.bind_binding(
        CastTarget::value_property(),
        &*CompiledBinding::new(
            CompiledBindingPathBuilder::new()
                .typed_property_info(info)
                .build(),
        )
        .with_mode(BindingMode::TwoWay),
    );
    assert!(expression
        .as_any()
        .is::<crate::data::core::TypedBindingExpression<Vm, Rc<Scalar>>>());
    target.set_value(CastTarget::value_property(), Scalar { value: 25. });
    assert_eq!(target.get_value(CastTarget::value_property()).value, 10.);
    assert_eq!(vm.value.get(), 25.);
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
