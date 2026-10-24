//! The values a visual state applies: WinUI's `VisualStateSetterHelper`
//! (`dxaml/xcp/components/vsm/VisualStateSetterHelper.cpp` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md), which sets them with `SetAnimatedValue` and removes them with
//! `ClearAnimatedValue`: in the animation layer, above local values, which they do not
//! overwrite. FerroUI's counterpart of that layer is `BindingPriority::Animation`.

use super::VisualState;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingPriority;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::{Setter, SetterValue};
use ferroui_base::{AnyValue, BoxedValue, FerroObject, FerroProperty, Ref};
use std::rc::Rc;

/// The values a visual state set, removed together.
#[derive(Default)]
pub(crate) struct AppliedSetters(Vec<Rc<dyn IDisposable>>);

impl AppliedSetters {
    /// Applies the setters of `state` that have a live target and a property.
    pub(crate) fn apply(state: &Ref<VisualState>) -> Self {
        let mut applied = Vec::new();
        for setter in state.setters().snapshot().iter() {
            let Some(setter) = setter.as_any().and_then(|any| any.downcast_ref::<Setter>()) else {
                continue;
            };
            if let Some(handle) = apply_setter(setter) {
                applied.push(handle);
            }
        }
        Self(applied)
    }

    /// Removes the values, in the reverse order of their application.
    pub(crate) fn remove(self) {
        for handle in self.0.into_iter().rev() {
            handle.dispose();
        }
    }
}

/// Applies one setter of a visual state; the handle removes its value.
///
/// A setter whose target is not set or is gone, or that has no property, does nothing (WinUI
/// fails the state change with an error; deviation, docs/porting/winui-controls.md,
/// VisualStateManager).
fn apply_setter(setter: &Setter) -> Option<Rc<dyn IDisposable>> {
    let target = setter.target()?.target()?;
    let property = setter.property()?;
    match setter.value() {
        Some(SetterValue::Value(value)) => set_value(&target, property, &value),
        Some(SetterValue::BindingBase(binding)) => {
            let expression = target.bind_binding_with_priority(property, &*binding, BindingPriority::Animation, None);
            Some(expression)
        }
        Some(SetterValue::Binding(source)) => {
            Some(target.bind_property_untyped(property, source, BindingPriority::Animation))
        }
        Some(SetterValue::Template(template)) => {
            let value = template.build();
            set_value(&target, property, &value)
        }
        // A setter without a value sets null where the property takes it.
        None => {
            let null = ValueTypes::null_value(ValueType::new(property.property_type(), property.property_type_name()))?;
            set_value(&target, property, &null)
        }
    }
}

/// Sets a value at animation priority. A direct property has no priorities: its value is set
/// and the value it had is restored when the state is left.
fn set_value(target: &Ref<FerroObject>, property: &'static FerroProperty, value: &BoxedValue) -> Option<Rc<dyn IDisposable>> {
    let inner: &dyn AnyValue = &**value;
    if !property.is_valid_value(inner.as_any()) {
        warn(&format!("Visual state: the value {inner:?} is not a valid value for property '{property}'."));
        return None;
    }
    if property.is_direct() {
        let old = target.get_value_untyped(property);
        target.set_value_untyped(property, inner.as_any(), BindingPriority::LocalValue);
        let weak = target.downgrade();
        return Some(Disposable::create(move || {
            if let Some(target) = weak.upgrade() {
                let old: &dyn AnyValue = &*old;
                target.set_value_untyped(property, old.as_any(), BindingPriority::LocalValue);
            }
        }));
    }
    target.set_value_untyped(property, inner.as_any(), BindingPriority::Animation)
}

/// Logs a warning of the visual state manager (area `Control`).
pub(crate) fn warn(message: &str) {
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
        logger.log(None, message);
    }
}
