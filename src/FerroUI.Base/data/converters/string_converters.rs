use super::{FuncValueConverter, IValueConverter};
use std::rc::Rc;

/// Provides a set of useful [`IValueConverter`]s for working with string
/// values.
pub struct StringConverters;

impl StringConverters {
    /// A value converter that returns true if the input string is null or an
    /// empty string.
    pub fn is_null_or_empty() -> Rc<dyn IValueConverter> {
        thread_local! {
            static IS_NULL_OR_EMPTY: Rc<dyn IValueConverter> =
                Rc::new(FuncValueConverter::<Option<String>, bool>::new(|x| x.is_none_or(|x| x.is_empty())));
        }
        IS_NULL_OR_EMPTY.with(Rc::clone)
    }

    /// A value converter that returns true if the input string is not null
    /// or empty.
    pub fn is_not_null_or_empty() -> Rc<dyn IValueConverter> {
        thread_local! {
            static IS_NOT_NULL_OR_EMPTY: Rc<dyn IValueConverter> =
                Rc::new(FuncValueConverter::<Option<String>, bool>::new(|x| !x.is_none_or(|x| x.is_empty())));
        }
        IS_NOT_NULL_OR_EMPTY.with(Rc::clone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::core::ValueType;
    use crate::data::{BindingError, BindingOperations};
    use crate::BoxedValue;

    fn boxed(input: Option<&str>) -> Option<BoxedValue> {
        input.map(|s| Rc::new(s.to_string()) as BoxedValue)
    }

    fn as_bool(result: Result<Option<BoxedValue>, BindingError>) -> bool {
        let value = result.expect("no error").expect("not null");
        *value.downcast_ref::<bool>().expect("a bool")
    }

    #[test]
    fn string_converters_is_null_or_empty_works() {
        for (input, expected) in [(Some("hello"), false), (Some(""), true), (None, true)] {
            let converter = StringConverters::is_null_or_empty();
            let result = converter.convert(boxed(input).as_ref(), ValueType::of::<bool>(), None, &crate::utilities::CultureInfo::current_culture());
            assert_eq!(as_bool(result), expected, "{input:?}");
        }
    }

    #[test]
    fn string_converters_is_not_null_or_empty_works() {
        for (input, expected) in [(Some("hello"), true), (Some(""), false), (None, false)] {
            let converter = StringConverters::is_not_null_or_empty();
            let result = converter.convert(boxed(input).as_ref(), ValueType::of::<bool>(), None, &crate::utilities::CultureInfo::current_culture());
            assert_eq!(as_bool(result), expected, "{input:?}");
        }
    }

    // Specific to this port.

    #[test]
    fn returns_unset_for_input_that_is_not_a_string() {
        let converter = StringConverters::is_null_or_empty();
        let input: BoxedValue = Rc::new(5i32);
        let result = converter.convert(Some(&input), ValueType::of::<bool>(), None, &crate::utilities::CultureInfo::current_culture());
        assert!(BindingOperations::is_unset(result.expect("no error").as_ref()));
    }

    #[test]
    fn accepts_string_literals_and_nullable_strings() {
        let converter = StringConverters::is_null_or_empty();
        let literal: BoxedValue = Rc::new("hello");
        let nullable: BoxedValue = Rc::new(Option::<String>::None);
        assert!(!as_bool(converter.convert(Some(&literal), ValueType::of::<bool>(), None, &crate::utilities::CultureInfo::current_culture())));
        assert!(as_bool(converter.convert(Some(&nullable), ValueType::of::<bool>(), None, &crate::utilities::CultureInfo::current_culture())));
    }
}
