//! Port of `Converter/HexConverter.cs`.

use ferroui_base::utilities::CultureInfo;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingError;
use ferroui_base::utilities::Decimal;
use ferroui_base::{ferro_markup_type, BoxedValue, FerroProperty};
use std::rc::{Rc, Weak};

pub struct HexConverter {
    this: Weak<HexConverter>,
}

impl PartialEq for HexConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl HexConverter {
    pub fn new() -> Rc<HexConverter> {
        Rc::new_cyclic(|this| Self { this: this.clone() })
    }

    /// The converter as the value converter contract.
    pub fn as_value_converter(&self) -> Rc<dyn IValueConverter> {
        self.this.upgrade().expect("the converter is alive while it is used")
    }
}

impl IValueConverter for HexConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let Some(value) = value else {
            return Ok(Some(FerroProperty::unset_value()));
        };
        let str = ValueTypes::to_display_string(Some(value));

        if let Some(x) = try_parse_hex(&str) {
            return Ok(Some(Rc::new(Decimal::from(x))));
        }

        Ok(Some(FerroProperty::unset_value()))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        // A number out of the range of an integer is the overflow the original catches.
        match value.and_then(|value| value.downcast_ref::<Decimal>()).and_then(truncate_to_i32) {
            Some(x) => Ok(Some(Rc::new(format!("{x:08X}")))),
            None => Ok(Some(FerroProperty::unset_value())),
        }
    }
}

ferro_markup_type!(class HexConverter {
    this: Rc<HexConverter>,
    handles: [HexConverter, Rc<HexConverter>, Option<Rc<HexConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => HexConverter::new],
});

/// `int.TryParse(text, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out value)`:
/// hexadecimal digits between optional white space, read as the bits of the
/// integer.
fn try_parse_hex(text: &str) -> Option<i32> {
    // The white space the number styles allow: U+0009 to U+000D and U+0020.
    let is_white = |c: char| matches!(c, '\u{9}'..='\u{d}' | ' ');
    let digits = text.trim_start_matches(is_white).trim_end_matches(is_white);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    let significant = digits.trim_start_matches('0');
    if significant.len() > 8 {
        return None;
    }
    let bits = if significant.is_empty() { 0 } else { u32::from_str_radix(significant, 16).ok()? };
    Some(bits as i32)
}

/// `(int)value`: the number without its fraction, or `None` when it is out
/// of the range of an integer.
fn truncate_to_i32(value: &Decimal) -> Option<i32> {
    let mut whole = value.mantissa();
    for _ in 0..value.scale() {
        whole /= 10;
    }
    let whole = i64::try_from(whole).ok()?;
    i32::try_from(if value.is_sign_negative() { -whole } else { whole }).ok()
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue {
        Rc::new(value)
    }

    fn convert(text: &str) -> Option<Decimal> {
        let converter = HexConverter::new();
        let value = converter.convert(Some(&boxed(text.to_string())), ValueType::of::<Decimal>(), None, &ferroui_base::utilities::CultureInfo::invariant_culture());
        value.ok().flatten().expect("a value").downcast_ref::<Decimal>().copied()
    }

    fn convert_back(value: BoxedValue) -> Option<String> {
        let converter = HexConverter::new();
        let value = converter.convert_back(Some(&value), ValueType::of::<String>(), None, &ferroui_base::utilities::CultureInfo::invariant_culture());
        value.ok().flatten().expect("a value").downcast_ref::<String>().cloned()
    }

    #[test]
    fn text_of_hexadecimal_digits_converts_to_a_number() {
        assert_eq!(Some(Decimal::from(255)), convert("FF"));
        assert_eq!(Some(Decimal::from(255)), convert(" 0000000000ff\t"));
        assert_eq!(Some(Decimal::from(-1)), convert("FFFFFFFF"));
        assert_eq!(None, convert("100000000"));
        assert_eq!(None, convert("0x10"));
        assert_eq!(None, convert("-1"));
        assert_eq!(None, convert(""));
        let converter = HexConverter::new();
        let unset = converter.convert(None, ValueType::of::<Decimal>(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).ok().flatten().expect("a value");
        assert!(Rc::ptr_eq(&unset, &FerroProperty::unset_value()));
    }

    #[test]
    fn a_number_converts_back_to_eight_digits() {
        assert_eq!(Some(String::from("000000FF")), convert_back(boxed(Decimal::from(255))));
        assert_eq!(Some(String::from("FFFFFFFF")), convert_back(boxed(Decimal::from(-1))));
        assert_eq!(Some(String::from("00000002")), convert_back(boxed(Decimal::parse("2.9").expect("a number"))));
        assert_eq!(None, convert_back(boxed(Decimal::from(4294967296_i64))));
        assert_eq!(None, convert_back(boxed(255_i32)));
    }
}
