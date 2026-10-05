//! Port of `Converter/TableViewColumnWidthConverter.cs`.

use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::utilities::Decimal;
use ferroui_base::{ferro_markup_type, BoxedValue, FerroProperty};
use ferroui_controls::{GridLength, GridUnitType};
use std::rc::{Rc, Weak};

pub struct TableViewColumnWidthConverter {
    this: Weak<TableViewColumnWidthConverter>,
}

impl PartialEq for TableViewColumnWidthConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl TableViewColumnWidthConverter {
    pub fn new() -> Rc<TableViewColumnWidthConverter> {
        Rc::new_cyclic(|this| Self { this: this.clone() })
    }

    /// The converter as the value converter contract.
    pub fn as_value_converter(&self) -> Rc<dyn IValueConverter> {
        self.this.upgrade().expect("the converter is alive while it is used")
    }
}

impl IValueConverter for TableViewColumnWidthConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let use_star_size = value.and_then(|value| value.downcast_ref::<bool>()).copied();
        let base_width = parameter.and_then(|parameter| parameter.downcast_ref::<String>()).and_then(|text| try_parse_number(text));
        if let (Some(use_star_size), Some(base_width)) = (use_star_size, base_width) {
            return Ok(Some(Rc::new(if use_star_size {
                GridLength::new(base_width, GridUnitType::Star)
            } else {
                GridLength::new(base_width * 100.0, GridUnitType::Pixel)
            })));
        }

        Ok(Some(FerroProperty::unset_value()))
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

ferro_markup_type!(class TableViewColumnWidthConverter {
    this: Rc<TableViewColumnWidthConverter>,
    handles: [TableViewColumnWidthConverter, Rc<TableViewColumnWidthConverter>, Option<Rc<TableViewColumnWidthConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => TableViewColumnWidthConverter::new],
});

/// `double.TryParse(text, NumberStyles.Number, CultureInfo.InvariantCulture, out value)`.
/// The text is read with the parser of the decimal type, which takes the
/// same styles; a number out of the range of that type is not parsed.
fn try_parse_number(text: &str) -> Option<f64> {
    Decimal::try_parse(text)?.to_string().parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue {
        Rc::new(value)
    }

    fn convert(value: BoxedValue, parameter: Option<BoxedValue>) -> Option<GridLength> {
        let converter = TableViewColumnWidthConverter::new();
        let value = converter.convert(Some(&value), ValueType::of::<GridLength>(), parameter.as_ref());
        value.ok().flatten().expect("a value").downcast_ref::<GridLength>().copied()
    }

    #[test]
    fn the_width_is_a_star_or_a_hundred_pixels_per_unit() {
        let parameter = || Some(boxed(String::from(" 1.5 ")));
        assert_eq!(Some(GridLength::new(1.5, GridUnitType::Star)), convert(boxed(true), parameter()));
        assert_eq!(Some(GridLength::new(150.0, GridUnitType::Pixel)), convert(boxed(false), parameter()));
    }

    #[test]
    fn other_values_and_parameters_are_unset_and_the_way_back_is_not_supported() {
        assert_eq!(None, convert(boxed(1_i32), Some(boxed(String::from("1")))));
        assert_eq!(None, convert(boxed(true), Some(boxed(String::from("1e3")))));
        assert_eq!(None, convert(boxed(true), Some(boxed(1.0_f64))));
        assert_eq!(None, convert(boxed(true), None));
        let converter = TableViewColumnWidthConverter::new();
        assert!(converter.convert_back(Some(&boxed(true)), ValueType::of::<bool>(), None).is_err());
    }
}
