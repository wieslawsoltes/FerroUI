//! Port of `Converter/DegToRadConverter.cs`.

use ferroui_base::utilities::CultureInfo;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::rc::{Rc, Weak};

pub struct DegToRadConverter {
    this: Weak<DegToRadConverter>,
}

impl PartialEq for DegToRadConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl DegToRadConverter {
    pub fn new() -> Rc<DegToRadConverter> {
        Rc::new_cyclic(|this| Self { this: this.clone() })
    }

    /// The converter as the value converter contract.
    pub fn as_value_converter(&self) -> Rc<dyn IValueConverter> {
        self.this.upgrade().expect("the converter is alive while it is used")
    }
}

impl IValueConverter for DegToRadConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(rad) = value.and_then(|value| value.downcast_ref::<f64>()) {
            return Ok(Some(Rc::new(rad * 180.0 / std::f64::consts::PI)));
        }

        Ok(Some(Rc::new(0.0_f64)))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(deg) = value.and_then(|value| value.downcast_ref::<f64>()) {
            return Ok(Some(Rc::new(deg / 180.0 * std::f64::consts::PI)));
        }

        Ok(Some(Rc::new(0.0_f64)))
    }
}

ferro_markup_type!(class DegToRadConverter {
    this: Rc<DegToRadConverter>,
    handles: [DegToRadConverter, Rc<DegToRadConverter>, Option<Rc<DegToRadConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => DegToRadConverter::new],
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
    fn converts_radians_to_degrees_and_back() {
        let converter = DegToRadConverter::new();
        let target = ValueType::of::<f64>();
        assert_eq!(180.0, number(converter.convert(Some(&boxed(std::f64::consts::PI)), target, None, &ferroui_base::utilities::CultureInfo::invariant_culture())));
        assert_eq!(std::f64::consts::PI, number(converter.convert_back(Some(&boxed(180.0_f64)), target, None, &ferroui_base::utilities::CultureInfo::invariant_culture())));
    }

    #[test]
    fn a_value_that_is_not_a_number_converts_to_zero() {
        let converter = DegToRadConverter::new();
        let target = ValueType::of::<f64>();
        assert_eq!(0.0, number(converter.convert(Some(&boxed(1_i32)), target, None, &ferroui_base::utilities::CultureInfo::invariant_culture())));
        assert_eq!(0.0, number(converter.convert(None, target, None, &ferroui_base::utilities::CultureInfo::invariant_culture())));
        assert_eq!(0.0, number(converter.convert_back(Some(&boxed(String::from("1"))), target, None, &ferroui_base::utilities::CultureInfo::invariant_culture())));
    }
}
