//! A counterpart of .NET's `System.Globalization.NumberFormatInfo`: the
//! symbols, separators, digit counts and patterns used to format and parse
//! numbers.
//!
//! A new instance carries the values of the invariant culture, the only
//! culture whose data is built in. The conventions of every other culture
//! come from the [`ICultureDataProvider`](super::ICultureDataProvider)
//! registered with the locator; a
//! culture the provider has nothing for (or any culture, without a
//! provider) has the conventions of its nearest parent culture that has
//! some, in the end those of the invariant culture.

use super::i_culture_data_provider::find_culture_data;
use super::CultureInfo;
use std::rc::Rc;

thread_local! {
    static INVARIANT: Rc<NumberFormatInfo> = Rc::new(NumberFormatInfo::new());
}

/// Culture-specific information for formatting and parsing numbers.
///
/// Used as `Rc<NumberFormatInfo>` where the managed runtime passes the
/// object around; like every reference type without value equality it
/// compares by identity.
#[derive(Clone, Debug)]
pub struct NumberFormatInfo {
    currency_decimal_digits: i32,
    currency_decimal_separator: String,
    currency_group_separator: String,
    currency_group_sizes: Vec<i32>,
    currency_negative_pattern: i32,
    currency_positive_pattern: i32,
    currency_symbol: String,
    nan_symbol: String,
    negative_infinity_symbol: String,
    negative_sign: String,
    number_decimal_digits: i32,
    number_decimal_separator: String,
    number_group_separator: String,
    number_group_sizes: Vec<i32>,
    number_negative_pattern: i32,
    percent_decimal_digits: i32,
    percent_decimal_separator: String,
    percent_group_separator: String,
    percent_group_sizes: Vec<i32>,
    percent_negative_pattern: i32,
    percent_positive_pattern: i32,
    percent_symbol: String,
    per_mille_symbol: String,
    positive_infinity_symbol: String,
    positive_sign: String,
}

impl PartialEq for NumberFormatInfo {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Default for NumberFormatInfo {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! text_properties {
    ($($(#[$doc:meta])* $name:ident, $set:ident, $with:ident;)*) => {
        impl NumberFormatInfo {
            $(
                $(#[$doc])*
                #[inline]
                pub fn $name(&self) -> &str {
                    &self.$name
                }

                $(#[$doc])*
                #[inline]
                pub fn $set(&mut self, value: impl Into<String>) {
                    self.$name = value.into();
                }

                $(#[$doc])*
                #[inline]
                pub fn $with(mut self, value: impl Into<String>) -> Self {
                    self.$name = value.into();
                    self
                }
            )*
        }
    };
}

macro_rules! number_properties {
    ($($(#[$doc:meta])* $name:ident, $set:ident, $with:ident, $max:literal, $label:literal;)*) => {
        impl NumberFormatInfo {
            $(
                $(#[$doc])*
                #[inline]
                pub fn $name(&self) -> i32 {
                    self.$name
                }

                $(#[$doc])*
                ///
                /// Panics when the value is out of range.
                pub fn $set(&mut self, value: i32) {
                    assert!(
                        (0..=$max).contains(&value),
                        concat!($label, " must be between 0 and ", stringify!($max), ", inclusive.")
                    );
                    self.$name = value;
                }

                $(#[$doc])*
                ///
                /// Panics when the value is out of range.
                #[inline]
                pub fn $with(mut self, value: i32) -> Self {
                    self.$set(value);
                    self
                }
            )*
        }
    };
}

macro_rules! group_properties {
    ($($(#[$doc:meta])* $name:ident, $set:ident, $with:ident;)*) => {
        impl NumberFormatInfo {
            $(
                $(#[$doc])*
                #[inline]
                pub fn $name(&self) -> &[i32] {
                    &self.$name
                }

                $(#[$doc])*
                ///
                /// Panics when a size is not between 1 and 9, except the
                /// last, which may be 0 (the remaining digits are not
                /// grouped).
                pub fn $set(&mut self, value: &[i32]) {
                    Self::check_group_sizes(value);
                    self.$name = value.to_vec();
                }

                $(#[$doc])*
                ///
                /// Panics when a size is not valid.
                #[inline]
                pub fn $with(mut self, value: &[i32]) -> Self {
                    self.$set(value);
                    self
                }
            )*
        }
    };
}

text_properties! {
    /// The decimal separator of currency values.
    currency_decimal_separator, set_currency_decimal_separator, with_currency_decimal_separator;
    /// The separator of the digit groups of currency values.
    currency_group_separator, set_currency_group_separator, with_currency_group_separator;
    /// The currency symbol.
    currency_symbol, set_currency_symbol, with_currency_symbol;
    /// The text of not-a-number.
    nan_symbol, set_nan_symbol, with_nan_symbol;
    /// The text of negative infinity.
    negative_infinity_symbol, set_negative_infinity_symbol, with_negative_infinity_symbol;
    /// The sign of negative numbers.
    negative_sign, set_negative_sign, with_negative_sign;
    /// The decimal separator of numbers.
    number_decimal_separator, set_number_decimal_separator, with_number_decimal_separator;
    /// The separator of the digit groups of numbers.
    number_group_separator, set_number_group_separator, with_number_group_separator;
    /// The decimal separator of percent values.
    percent_decimal_separator, set_percent_decimal_separator, with_percent_decimal_separator;
    /// The separator of the digit groups of percent values.
    percent_group_separator, set_percent_group_separator, with_percent_group_separator;
    /// The percent symbol.
    percent_symbol, set_percent_symbol, with_percent_symbol;
    /// The per mille symbol.
    per_mille_symbol, set_per_mille_symbol, with_per_mille_symbol;
    /// The text of positive infinity.
    positive_infinity_symbol, set_positive_infinity_symbol, with_positive_infinity_symbol;
    /// The sign of positive numbers.
    positive_sign, set_positive_sign, with_positive_sign;
}

number_properties! {
    /// The number of decimal places of currency values.
    currency_decimal_digits, set_currency_decimal_digits, with_currency_decimal_digits, 99, "CurrencyDecimalDigits";
    /// The pattern of negative currency values (0 is `($n)`, 1 is `-$n`).
    currency_negative_pattern, set_currency_negative_pattern, with_currency_negative_pattern, 16,
        "CurrencyNegativePattern";
    /// The pattern of positive currency values (0 is `$n`, 1 is `n$`, 2 is
    /// `$ n`, 3 is `n $`).
    currency_positive_pattern, set_currency_positive_pattern, with_currency_positive_pattern, 3,
        "CurrencyPositivePattern";
    /// The number of decimal places of numbers.
    number_decimal_digits, set_number_decimal_digits, with_number_decimal_digits, 99, "NumberDecimalDigits";
    /// The pattern of negative numbers (0 is `(n)`, 1 is `-n`, 2 is `- n`,
    /// 3 is `n-`, 4 is `n -`).
    number_negative_pattern, set_number_negative_pattern, with_number_negative_pattern, 4, "NumberNegativePattern";
    /// The number of decimal places of percent values.
    percent_decimal_digits, set_percent_decimal_digits, with_percent_decimal_digits, 99, "PercentDecimalDigits";
    /// The pattern of negative percent values (0 is `-n %`, 1 is `-n%`).
    percent_negative_pattern, set_percent_negative_pattern, with_percent_negative_pattern, 11,
        "PercentNegativePattern";
    /// The pattern of positive percent values (0 is `n %`, 1 is `n%`, 2 is
    /// `%n`, 3 is `% n`).
    percent_positive_pattern, set_percent_positive_pattern, with_percent_positive_pattern, 3,
        "PercentPositivePattern";
}

group_properties! {
    /// The number of digits in each group of currency values, from the
    /// decimal separator leftwards; the last size repeats.
    currency_group_sizes, set_currency_group_sizes, with_currency_group_sizes;
    /// The number of digits in each group of numbers, from the decimal
    /// separator leftwards; the last size repeats.
    number_group_sizes, set_number_group_sizes, with_number_group_sizes;
    /// The number of digits in each group of percent values, from the
    /// decimal separator leftwards; the last size repeats.
    percent_group_sizes, set_percent_group_sizes, with_percent_group_sizes;
}

impl NumberFormatInfo {
    /// Format information with the values of the invariant culture (C#
    /// `new NumberFormatInfo()`).
    pub fn new() -> NumberFormatInfo {
        NumberFormatInfo {
            currency_decimal_digits: 2,
            currency_decimal_separator: ".".to_string(),
            currency_group_separator: ",".to_string(),
            currency_group_sizes: vec![3],
            currency_negative_pattern: 0,
            currency_positive_pattern: 0,
            currency_symbol: "\u{00A4}".to_string(),
            nan_symbol: "NaN".to_string(),
            negative_infinity_symbol: "-Infinity".to_string(),
            negative_sign: "-".to_string(),
            number_decimal_digits: 2,
            number_decimal_separator: ".".to_string(),
            number_group_separator: ",".to_string(),
            number_group_sizes: vec![3],
            number_negative_pattern: 1,
            percent_decimal_digits: 2,
            percent_decimal_separator: ".".to_string(),
            percent_group_separator: ",".to_string(),
            percent_group_sizes: vec![3],
            percent_negative_pattern: 0,
            percent_positive_pattern: 0,
            percent_symbol: "%".to_string(),
            per_mille_symbol: "\u{2030}".to_string(),
            positive_infinity_symbol: "Infinity".to_string(),
            positive_sign: "+".to_string(),
        }
    }

    /// The shared format information of a culture: that of the registered
    /// provider for the culture or for its nearest parent culture the
    /// provider has data for, else the invariant one.
    pub(crate) fn for_culture(culture: &CultureInfo) -> Rc<NumberFormatInfo> {
        find_culture_data(culture, |provider, name| provider.get_number_format(name))
            .unwrap_or_else(Self::invariant_info)
    }

    /// The format information of the invariant culture.
    pub fn invariant_info() -> Rc<NumberFormatInfo> {
        INVARIANT.with(Clone::clone)
    }

    /// The format information of the current culture.
    pub fn current_info() -> Rc<NumberFormatInfo> {
        Self::for_culture(&CultureInfo::current_culture())
    }

    /// The given format information, or that of the current culture when
    /// there is none (C# `GetInstance(IFormatProvider)`).
    pub fn get_instance(provider: Option<&Rc<NumberFormatInfo>>) -> Rc<NumberFormatInfo> {
        match provider {
            Some(info) => info.clone(),
            None => Self::current_info(),
        }
    }

    fn check_group_sizes(sizes: &[i32]) {
        for (index, size) in sizes.iter().enumerate() {
            let valid = (1..=9).contains(size) || (*size == 0 && index + 1 == sizes.len());
            assert!(
                valid,
                "Every element in the value array should be between one and nine, except for the last element, which can be zero."
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utilities::TestCultureDataProvider;
    use crate::FerroLocator;

    #[test]
    fn a_new_instance_has_the_invariant_values() {
        let info = NumberFormatInfo::new();
        assert_eq!(info.currency_decimal_digits(), 2);
        assert_eq!(info.currency_decimal_separator(), ".");
        assert_eq!(info.currency_group_separator(), ",");
        assert_eq!(info.currency_group_sizes(), [3]);
        assert_eq!(info.currency_negative_pattern(), 0);
        assert_eq!(info.currency_positive_pattern(), 0);
        assert_eq!(info.currency_symbol(), "\u{00A4}");
        assert_eq!(info.nan_symbol(), "NaN");
        assert_eq!(info.negative_infinity_symbol(), "-Infinity");
        assert_eq!(info.negative_sign(), "-");
        assert_eq!(info.number_decimal_digits(), 2);
        assert_eq!(info.number_decimal_separator(), ".");
        assert_eq!(info.number_group_separator(), ",");
        assert_eq!(info.number_group_sizes(), [3]);
        assert_eq!(info.number_negative_pattern(), 1);
        assert_eq!(info.percent_decimal_digits(), 2);
        assert_eq!(info.percent_decimal_separator(), ".");
        assert_eq!(info.percent_group_separator(), ",");
        assert_eq!(info.percent_group_sizes(), [3]);
        assert_eq!(info.percent_negative_pattern(), 0);
        assert_eq!(info.percent_positive_pattern(), 0);
        assert_eq!(info.percent_symbol(), "%");
        assert_eq!(info.per_mille_symbol(), "\u{2030}");
        assert_eq!(info.positive_infinity_symbol(), "Infinity");
        assert_eq!(info.positive_sign(), "+");
    }

    #[test]
    fn properties_can_be_changed() {
        let mut info = NumberFormatInfo::new().with_number_decimal_separator(";").with_number_group_sizes(&[3, 2, 0]);
        info.set_currency_symbol("kr");
        info.set_number_negative_pattern(0);
        assert_eq!(info.number_decimal_separator(), ";");
        assert_eq!(info.number_group_sizes(), [3, 2, 0]);
        assert_eq!(info.currency_symbol(), "kr");
        assert_eq!(info.number_negative_pattern(), 0);
    }

    #[test]
    #[should_panic(expected = "NumberNegativePattern must be between 0 and 4, inclusive.")]
    fn a_pattern_out_of_range_panics() {
        NumberFormatInfo::new().set_number_negative_pattern(5);
    }

    #[test]
    #[should_panic(expected = "between one and nine")]
    fn a_group_size_out_of_range_panics() {
        NumberFormatInfo::new().set_number_group_sizes(&[0, 3]);
    }

    #[test]
    fn instances_compare_by_identity() {
        let a = Rc::new(NumberFormatInfo::new());
        let b = Rc::new(NumberFormatInfo::new());
        assert!(a == a.clone());
        assert!(a != b);
        assert!(NumberFormatInfo::invariant_info() == NumberFormatInfo::invariant_info());
    }

    #[test]
    fn cultures() {
        let _scope = FerroLocator::enter_scope();

        let invariant = CultureInfo::invariant_culture().number_format();
        assert!(invariant == NumberFormatInfo::invariant_info());
        assert_eq!(invariant.currency_symbol(), "\u{00A4}");

        // Without a provider every culture has the invariant conventions.
        assert!(CultureInfo::get_culture_info("en-US").number_format() == invariant);

        TestCultureDataProvider::register();

        let english = CultureInfo::get_culture_info("en-US").number_format();
        assert_eq!(english.currency_symbol(), "$");
        assert_eq!(english.currency_negative_pattern(), 1);
        assert_eq!(english.percent_positive_pattern(), 1);
        assert_eq!(english.percent_negative_pattern(), 1);
        // Shared per culture.
        assert!(english == CultureInfo::get_culture_info("EN-us").number_format());

        // A culture the provider has no data for has the data of its parent.
        assert!(CultureInfo::get_culture_info("en-AU").number_format() == CultureInfo::get_culture_info("en").number_format());
        assert!(CultureInfo::get_culture_info("ja-JP").number_format() == invariant);
        assert!(CultureInfo::invariant_culture().number_format() == invariant);

        // The current culture is the invariant culture until it is set.
        assert_eq!(NumberFormatInfo::current_info().currency_symbol(), "\u{00A4}");
        let given = Rc::new(NumberFormatInfo::new());
        assert!(NumberFormatInfo::get_instance(Some(&given)) == given);
        assert!(NumberFormatInfo::get_instance(None) == NumberFormatInfo::current_info());
    }
}
