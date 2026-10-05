//! Port of `Converter/FlexDemoNumberToThicknessConverter.cs`.

use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingError;
use ferroui_base::{ferro_markup_type, BoxedValue, Thickness};
use std::rc::{Rc, Weak};

pub struct FlexDemoNumberToThicknessConverter {
    this: Weak<FlexDemoNumberToThicknessConverter>,
}

impl PartialEq for FlexDemoNumberToThicknessConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl FlexDemoNumberToThicknessConverter {
    pub fn new() -> Rc<FlexDemoNumberToThicknessConverter> {
        Rc::new_cyclic(|this| Self { this: this.clone() })
    }

    /// The converter as the value converter contract.
    pub fn as_value_converter(&self) -> Rc<dyn IValueConverter> {
        self.this.upgrade().expect("the converter is alive while it is used")
    }
}

impl IValueConverter for FlexDemoNumberToThicknessConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(x) = value.and_then(|value| value.downcast_ref::<i32>()) {
            if ValueTypes::is_assignable(ValueType::of::<Thickness>(), target_type) {
                // The arithmetic wraps, as the unchecked arithmetic of the original.
                let y = 16_i32.wrapping_add(2_i32.wrapping_mul(x.wrapping_mul(5) % 9));
                return Ok(Some(Rc::new(Thickness::symmetric(f64::from(y.wrapping_mul(2)), f64::from(y)))));
            }
        }

        Err(BindingError::message("Specified method is not supported."))
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

ferro_markup_type!(class FlexDemoNumberToThicknessConverter {
    this: Rc<FlexDemoNumberToThicknessConverter>,
    handles: [FlexDemoNumberToThicknessConverter, Rc<FlexDemoNumberToThicknessConverter>, Option<Rc<FlexDemoNumberToThicknessConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => FlexDemoNumberToThicknessConverter::new],
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue {
        Rc::new(value)
    }

    #[test]
    fn converts_a_number_to_a_thickness() {
        let converter = FlexDemoNumberToThicknessConverter::new();
        let target = ValueType::of::<Thickness>();
        let convert = |x: i32| {
            let value = converter.convert(Some(&boxed(x)), target, None).ok().flatten().expect("a value");
            *value.downcast_ref::<Thickness>().expect("a thickness")
        };
        assert_eq!(Thickness::symmetric(32.0, 16.0), convert(0));
        // 16 + 2 * (15 % 9) = 28
        assert_eq!(Thickness::symmetric(56.0, 28.0), convert(3));
        // The remainder keeps the sign of the dividend: 16 + 2 * (-5 % 9) = 6
        assert_eq!(Thickness::symmetric(12.0, 6.0), convert(-1));
        assert!(converter.convert(Some(&boxed(1_i32)), ValueType::object(), None).is_ok());
    }

    #[test]
    fn other_values_targets_and_the_way_back_are_not_supported() {
        let converter = FlexDemoNumberToThicknessConverter::new();
        assert!(converter.convert(Some(&boxed(1.0_f64)), ValueType::of::<Thickness>(), None).is_err());
        assert!(converter.convert(Some(&boxed(1_i32)), ValueType::of::<f64>(), None).is_err());
        assert!(converter.convert_back(Some(&boxed(1_i32)), ValueType::of::<i32>(), None).is_err());
    }
}
