use super::{
    impl_untyped_binding_expression, IBindingExpressionSink, Publish, SinkRef, UntypedBindingExpression,
    UntypedBindingExpressionBase, ValueTypes,
};
use crate::data::converters::IMultiValueConverter;
use crate::data::{BindingBase, BindingExpressionBase, BindingNotification, BindingOperations, BindingPriority};
use crate::utilities::CultureInfo;
use crate::{BoxedValue, FerroProperty};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The binding expression of a multi-binding: runs the child bindings and
/// combines their values through a converter.
pub struct MultiBindingExpression {
    this: Weak<MultiBindingExpression>,
    base: UntypedBindingExpressionBase,
    bindings: Vec<Rc<dyn BindingBase>>,
    converter: Option<Rc<dyn IMultiValueConverter>>,
    converter_parameter: Option<BoxedValue>,
    expressions: RefCell<Vec<Option<Rc<dyn BindingExpressionBase>>>>,
    fallback_value: Option<BoxedValue>,
    target_null_value: Option<BoxedValue>,
    /// The current value of each child; `None` until the child has produced
    /// one.
    values: RefCell<Vec<Option<Option<BoxedValue>>>>,
}

impl MultiBindingExpression {
    pub fn new(
        priority: BindingPriority,
        bindings: Vec<Rc<dyn BindingBase>>,
        converter: Option<Rc<dyn IMultiValueConverter>>,
        converter_parameter: Option<BoxedValue>,
        fallback_value: Option<BoxedValue>,
        target_null_value: Option<BoxedValue>,
    ) -> Rc<Self> {
        let count = bindings.len();
        Rc::new_cyclic(|this: &Weak<MultiBindingExpression>| Self {
            this: this.clone(),
            base: UntypedBindingExpressionBase::new(this.clone(), priority, None, false),
            bindings,
            converter,
            converter_parameter,
            expressions: RefCell::new(vec![None; count]),
            fallback_value,
            target_null_value,
            values: RefCell::new(vec![None; count]),
        })
    }

    pub fn converter(&self) -> Option<&Rc<dyn IMultiValueConverter>> {
        self.converter.as_ref()
    }

    fn publish(&self) {
        let values: Vec<Option<BoxedValue>> = {
            let values = self.values.borrow();
            if values.iter().any(Option::is_none) {
                return;
            }
            values.iter().map(|v| v.clone().expect("checked")).collect()
        };

        let target_type = self.base.target_type();
        let has_target = self.base.target_property().is_some();

        match &self.converter {
            Some(converter) => {
                let culture = CultureInfo::current_culture();
                let converted = match converter.convert(&values, target_type, self.converter_parameter.as_ref(), &culture)
                {
                    Ok(v) => v,
                    Err(_) => Some(FerroProperty::unset_value()),
                };
                let mut converted = BindingNotification::extract_value(converted.as_ref());
                if BindingOperations::is_do_nothing(converted.as_ref()) {
                    return;
                }
                if converted.is_none() {
                    converted = self.target_null_value.clone();
                }
                if BindingOperations::is_unset(converted.as_ref()) {
                    converted = self.fallback_value.clone();
                }
                if has_target && !BindingOperations::is_unset(converted.as_ref()) {
                    converted = ValueTypes::try_convert(converted.as_ref(), target_type)
                        .unwrap_or_else(|| Some(FerroProperty::unset_value()));
                }
                self.base.publish_value(Publish::Value(converted), None, false);
            }
            None => {
                let list: BoxedValue = Rc::new(values);
                let value = if has_target {
                    ValueTypes::try_convert(Some(&list), target_type)
                        .unwrap_or_else(|| Some(FerroProperty::unset_value()))
                } else {
                    Some(list)
                };
                self.base.publish_value(Publish::Value(value), None, false);
            }
        }
    }
}

impl IBindingExpressionSink for MultiBindingExpression {
    fn on_changed(&self, instance: &Rc<dyn BindingExpressionBase>, _has_value_changed: bool, _has_error_changed: bool) {
        let index = self
            .expressions
            .borrow()
            .iter()
            .position(|e| e.as_ref().is_some_and(|e| std::ptr::addr_eq(Rc::as_ptr(e), Rc::as_ptr(instance))));
        let Some(index) = index else { return };
        let value = if instance.has_value() {
            BindingNotification::extract_value(instance.get_value_boxed().as_ref())
        } else {
            Some(FerroProperty::unset_value())
        };
        self.values.borrow_mut()[index] = Some(value);
        self.publish();
    }

    fn on_completed(&self, _instance: &Rc<dyn BindingExpressionBase>) {
        // Nothing to do here.
    }
}

impl UntypedBindingExpression for MultiBindingExpression {
    fn base(&self) -> &UntypedBindingExpressionBase {
        &self.base
    }

    fn description(&self) -> String {
        "MultiBinding".to_string()
    }

    fn start_core(&self) {
        let target = self.base.try_get_target().expect("MultiBindingExpression has no target.");
        let sink: Weak<dyn IBindingExpressionSink> = self.this.clone();
        for (i, binding) in self.bindings.iter().enumerate() {
            let expression = binding.create_instance(&target, None, None);
            let Some(untyped) = expression.as_untyped() else {
                panic!("Unsupported BindingExpressionBase implementation.");
            };
            self.expressions.borrow_mut()[i] = Some(expression.clone());
            untyped.base().attach(SinkRef::Other(sink.clone()), None, &target, None, self.base.priority());
            untyped.start(true);
        }
    }

    fn stop_core(&self) {
        let expressions: Vec<_> = self.expressions.borrow_mut().iter_mut().map(Option::take).collect();
        for expression in expressions.into_iter().flatten() {
            expression.dispose();
        }
        for value in self.values.borrow_mut().iter_mut() {
            *value = None;
        }
    }
}

impl_untyped_binding_expression!(MultiBindingExpression);
