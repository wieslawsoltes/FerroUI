use super::activators::PropertyEqualsActivator;
use super::{Selector, SelectorMatch, SelectorNode, Style, StyleBase};
use crate::{AnyValue, BoxedValue, FerroProperty, StyledElement, TypeInfo};
use std::any::Any;
use std::cell::OnceCell;

/// A selector that matches the common case of a type and/or name followed by
/// a collection of style classes and pseudoclasses.
pub(crate) struct PropertyEqualsSelector {
    previous: Option<Selector>,
    property: &'static FerroProperty,
    value: BoxedValue,
    selector_string: OnceCell<String>,
}

impl PropertyEqualsSelector {
    pub fn new(previous: Option<Selector>, property: &'static FerroProperty, value: BoxedValue) -> Self {
        Self { previous, property, value, selector_string: OnceCell::new() }
    }

    /// Compares a property value with the value of a selector.
    ///
    /// The selector value must hold exactly the property's value type. A
    /// property of an untyped value type (`Option<BoxedValue>`) is compared
    /// by its content.
    pub(crate) fn compare(property_value: &BoxedValue, value: &BoxedValue) -> bool {
        let property_value: &dyn AnyValue = &**property_value;
        let value: &dyn AnyValue = &**value;

        if property_value.value_eq(value) {
            return true;
        }

        if let Some(Some(inner)) = property_value.downcast_ref::<Option<BoxedValue>>() {
            let inner: &dyn AnyValue = &**inner;
            return inner.value_eq(value);
        }

        false
    }

    fn value_string(&self) -> String {
        let value: &dyn AnyValue = &*self.value;
        if let Some(v) = value.downcast_ref::<String>() {
            v.clone()
        } else if let Some(v) = value.downcast_ref::<&'static str>() {
            v.to_string()
        } else if let Some(v) = value.downcast_ref::<bool>() {
            if *v { "True".to_string() } else { "False".to_string() }
        } else if let Some(v) = value.downcast_ref::<i32>() {
            v.to_string()
        } else if let Some(v) = value.downcast_ref::<f64>() {
            v.to_string()
        } else {
            value.type_name().to_string()
        }
    }
}

impl SelectorNode for PropertyEqualsSelector {
    fn in_template(&self) -> bool {
        self.previous.as_ref().is_some_and(Selector::in_template)
    }

    fn is_combinator(&self) -> bool {
        false
    }

    fn target_type(&self) -> Option<&'static TypeInfo> {
        self.previous.as_ref().and_then(Selector::target_type)
    }

    fn to_string(&self, owner: Option<&Style>) -> String {
        self.selector_string
            .get_or_init(|| {
                let mut builder = String::new();

                if let Some(previous) = &self.previous {
                    builder.push_str(&previous.to_string_with_next(owner, true));
                }

                builder.push('[');

                if self.property.is_attached() {
                    builder.push('(');
                    builder.push_str(self.property.owner_type().name());
                    builder.push('.');
                }

                builder.push_str(self.property.name());
                if self.property.is_attached() {
                    builder.push(')');
                }
                builder.push('=');
                builder.push_str(&self.value_string());
                builder.push(']');

                builder
            })
            .clone()
    }

    fn evaluate(&self, control: &StyledElement, _parent: Option<&StyleBase>, subscribe: bool) -> SelectorMatch {
        if subscribe {
            SelectorMatch::sometimes(PropertyEqualsActivator::new(control, self.property, self.value.clone()))
        } else if Self::compare(&control.get_value_untyped(self.property), &self.value) {
            SelectorMatch::ALWAYS_THIS_INSTANCE
        } else {
            SelectorMatch::NEVER_THIS_INSTANCE
        }
    }

    fn move_previous(&self) -> Option<&Selector> {
        self.previous.as_ref()
    }

    fn move_previous_or_parent(&self) -> Option<&Selector> {
        self.previous.as_ref()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
