//! The values a visual state applies: WinUI's `VisualStateSetterHelper`
//! (`dxaml/xcp/components/vsm/VisualStateSetterHelper.cpp` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md), which sets them with `SetAnimatedValue` and removes them with
//! `ClearAnimatedValue`: in the animation layer, above local values, which they do not
//! overwrite. FerroUI's counterpart of that layer is `BindingPriority::Animation`.

use super::VisualState;
use crate::Control;
use ferroui_base::controls::{NameScope, NameScopeRef};
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
    /// Applies the setters of `state` that have a target and a property. A target named in markup
    /// is found in the name scope of `template_root` (the element that has the groups: the root of
    /// the template, or the content of the document), as WinUI resolves it when the setter applies.
    pub(crate) fn apply(state: &Ref<VisualState>, template_root: Option<&Ref<Control>>) -> Self {
        let names = template_root.map(Names::of);
        let mut applied = Vec::new();
        for setter in state.setters().snapshot().iter() {
            let Some(setter) = setter.as_any().and_then(|any| any.downcast_ref::<Setter>()) else {
                continue;
            };
            if let Some(handle) = apply_setter(setter, names.as_ref()) {
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

/// Where the names of the setters of a visual state are found: the elements of the template (its
/// root and the elements under it) when the groups are on the root of a template, else the name
/// scope of the document (the nearest of the element and its logical ancestors: the content of a
/// document has no scope of its own, and its document root has the scope of the document).
enum Names {
    Template(Ref<Control>, Ref<crate::primitives::TemplatedControl>),
    Scope(NameScopeRef),
    None,
}

impl Names {
    fn of(template_root: &Ref<Control>) -> Self {
        // An element created by a template has the templated control as its templated parent.
        let templated =
            template_root.templated_parent().and_then(|parent| parent.cast::<crate::primitives::TemplatedControl>());
        if let Some(templated) = templated {
            return Names::Template(template_root.clone(), templated);
        }
        // The logical ancestors: the content of a document is a logical child of the document root
        // (with or without a template that presents it).
        match template_root.get_self_and_logical_ancestors().find_map(|element| NameScope::get_name_scope(&element)) {
            Some(scope) => Names::Scope(scope),
            None => Names::None,
        }
    }

    fn find(&self, name: &str) -> Option<Ref<FerroObject>> {
        match self {
            Names::Template(root, templated) => {
                if root.name().as_deref() == Some(name) {
                    return Some(root.clone().upcast());
                }
                templated
                    .get_template_descendants()
                    .into_iter()
                    .find(|element| element.name().as_deref() == Some(name))
                    .map(|element| element.upcast())
            }
            Names::Scope(scope) => scope.find(name),
            Names::None => None,
        }
    }
}

/// Applies one setter of a visual state; the handle removes its value.
///
/// A setter whose target is not set, is not found by its name or is gone, or that has no property,
/// does nothing and logs a warning (WinUI fails the state change with an error; deviation,
/// docs/porting/winui-controls.md, VisualStateManager).
fn apply_setter(setter: &Setter, names: Option<&Names>) -> Option<Rc<dyn IDisposable>> {
    let path = setter.target()?;
    let name = path.target_name();
    let target = path.target().or_else(|| (!name.is_empty()).then(|| names?.find(&name)).flatten());
    let Some(target) = target else {
        if !name.is_empty() {
            warn(&format!("Visual state: no element named '{name}' in the name scope of the setter."));
        }
        return None;
    };
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
