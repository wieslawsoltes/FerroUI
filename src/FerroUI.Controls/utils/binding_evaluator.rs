use crate::assigned_binding::AssignedBinding;
use crate::items_source::items_equal;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::BindingExpressionBase;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElement, StyledElementImpl, StyledProperty,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Evaluates a binding against arbitrary data contexts.
///
/// The value is untyped; [`evaluate_string`](Self::evaluate_string) gives
/// its text form for the consumers that evaluate text.
#[repr(C)]
pub struct BindingEvaluator {
    base: StyledElement,
    expression: RefCell<Option<Rc<dyn BindingExpressionBase>>>,
    last_binding: RefCell<Option<AssignedBinding>>,
}

ferro_class!(BindingEvaluator: StyledElement);
ferroui_base::ferro_class_info!(BindingEvaluator { new: BindingEvaluator::new });
ferro_impl_classes!(BindingEvaluator: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_properties! { impl BindingEvaluator {
    ferro_property!(
        /// Defines the `Value` property.
        pub fn value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<BindingEvaluator, _>("Value", None)
        }
    );
} }

impl BindingEvaluator {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: StyledElement::construct(),
            expression: RefCell::new(None),
            last_binding: RefCell::new(None),
        })
    }

    /// The evaluated value.
    pub fn value(&self) -> Option<BoxedValue> {
        self.get_value(Self::value_property())
    }

    /// Sets the evaluated value (the setter of `Value`; `set_value` is the property setter
    /// of every object).
    pub fn set_evaluated_value(&self, value: Option<BoxedValue>) {
        self.set_value(Self::value_property(), value)
    }

    /// Evaluates the binding against a data context.
    pub fn evaluate(&self, data_context: &Option<BoxedValue>) -> Option<BoxedValue> {
        // Only update the data context if necessary.
        if !items_equal(data_context, &self.data_context()) {
            self.set_data_context(data_context.clone());
        }

        self.get_value(Self::value_property())
    }

    /// Evaluates the binding against a data context and returns the text
    /// form of the value, if it has one.
    pub fn evaluate_string(&self, data_context: &Option<BoxedValue>) -> Option<String> {
        let value = self.evaluate(data_context)?;
        ValueTypes::try_to_string(&*value)
    }

    /// Replaces the evaluated binding.
    pub fn update_binding(&self, binding: &AssignedBinding) {
        if self.last_binding.borrow().as_ref() == Some(binding) {
            return;
        }

        if let Some(expression) = self.expression.take() {
            expression.dispose();
        }
        let expression = self.bind_binding(Self::value_property().as_property(), &**binding);
        *self.expression.borrow_mut() = Some(expression);
        *self.last_binding.borrow_mut() = Some(binding.clone());
    }

    /// Clears the data context.
    pub fn clear_data_context(&self) {
        self.set_data_context(None);
    }

    /// Releases the binding.
    pub fn dispose(&self) {
        if let Some(expression) = self.expression.take() {
            expression.dispose();
        }
        *self.last_binding.borrow_mut() = None;
        self.set_data_context(None);
    }

    /// Creates an evaluator for a binding, if there is one.
    pub fn try_create(binding: Option<&AssignedBinding>) -> Option<Ref<BindingEvaluator>> {
        let binding = binding?;

        let evaluator = Self::new();
        evaluator.update_binding(binding);
        Some(evaluator)
    }
}
