//! Port of `Converter/MathSubtractConverter.cs`.

use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::rc::{Rc, Weak};

pub struct MathSubtractConverter {
    this: Weak<MathSubtractConverter>,
}

impl PartialEq for MathSubtractConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl MathSubtractConverter {
    pub fn new() -> Rc<MathSubtractConverter> {
        Rc::new_cyclic(|this| Self { this: this.clone() })
    }

    /// The converter as the value converter contract.
    pub fn as_value_converter(&self) -> Rc<dyn IValueConverter> {
        self.this.upgrade().expect("the converter is alive while it is used")
    }
}

impl IValueConverter for MathSubtractConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let dv = value.and_then(|value| value.downcast_ref::<f64>());
        let dp = parameter.and_then(|parameter| parameter.downcast_ref::<f64>());
        if let (Some(dv), Some(dp)) = (dv, dp) {
            return Ok(Some(Rc::new(dv - dp)));
        }

        Ok(Some(Rc::new(f64::NAN)))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message("Specified method is not supported."))
    }
}

ferro_markup_type!(class MathSubtractConverter {
    this: Rc<MathSubtractConverter>,
    handles: [MathSubtractConverter, Rc<MathSubtractConverter>, Option<Rc<MathSubtractConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => MathSubtractConverter::new],
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue {
        Rc::new(value)
    }

    fn number(value: Result<Option<BoxedValue>, BindingError>) -> f64 {
        *value.ok().flatten().expect("a value").downcast_ref::<f64>().expect("a number")
    }

    #[test]
    fn subtracts_the_parameter_from_the_value() {
        let converter = MathSubtractConverter::new();
        let target = ValueType::of::<f64>();
        assert_eq!(7.5, number(converter.convert(Some(&boxed(10.0_f64)), target, Some(&boxed(2.5_f64)))));
        assert!(number(converter.convert(Some(&boxed(10.0_f64)), target, Some(&boxed(String::from("2.5"))))).is_nan());
        assert!(number(converter.convert(Some(&boxed(10_i32)), target, Some(&boxed(2.5_f64)))).is_nan());
        assert!(number(converter.convert(Some(&boxed(10.0_f64)), target, None)).is_nan());
        assert!(converter.convert_back(Some(&boxed(1.0_f64)), target, None).is_err());
    }
}
