use super::composite_format::format_values_with;
use super::IValueConverter;
use crate::data::core::ValueType;
use crate::data::BindingError;
use crate::utilities::CultureInfo;
use crate::BoxedValue;
use std::rc::Rc;

/// A value converter which formats the value with a composite format string
/// (see [`composite_format`](super::composite_format)).
pub struct StringFormatValueConverter {
    format: String,
    inner: Option<Rc<dyn IValueConverter>>,
}

impl StringFormatValueConverter {
    /// Creates a string format value converter.
    ///
    /// `format` is the format string; `inner` is an optional inner converter
    /// to be called before the format takes place.
    pub fn new(format: impl Into<String>, inner: Option<Rc<dyn IValueConverter>>) -> Self {
        Self { format: format.into(), inner }
    }

    /// Gets an inner value converter which will be called before the string
    /// format takes place.
    pub fn inner(&self) -> Option<&Rc<dyn IValueConverter>> {
        self.inner.as_ref()
    }

    /// Gets the format string.
    pub fn format(&self) -> &str {
        &self.format
    }
}

impl IValueConverter for StringFormatValueConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        parameter: Option<&BoxedValue>,
        culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let converted = match &self.inner {
            Some(inner) => inner.convert(value, target_type, parameter, culture)?,
            None => None,
        };
        let value = converted.or_else(|| value.cloned());

        let result = if self.format.contains('{') {
            format_values_with(&self.format, &[value], culture)
        } else {
            format_values_with(&format!("{{0:{}}}", self.format), &[value], culture)
        };
        match result {
            Ok(text) => Ok(Some(Rc::new(text))),
            Err(error) => Err(BindingError::new(error)),
        }
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message("Two way bindings are not supported with a string format"))
    }
}

#[cfg(test)]
mod tests {
    //! Upstream has no dedicated tests for this type; these cover the port.

    use super::*;
    use crate::data::converters::composite_format::FormatError;
    use crate::data::converters::FuncValueConverter;

    fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue {
        Rc::new(value)
    }

    fn convert(target: &StringFormatValueConverter, value: Option<&BoxedValue>) -> String {
        let result = target.convert(value, ValueType::of::<String>(), None, &crate::utilities::CultureInfo::invariant_culture()).expect("no error").expect("not null");
        result.downcast_ref::<String>().cloned().expect("a string")
    }

    #[test]
    fn exposes_format_and_inner() {
        let inner: Rc<dyn IValueConverter> = Rc::new(FuncValueConverter::<i32, i32>::new(|x| x));
        let target = StringFormatValueConverter::new("{0}", Some(inner.clone()));

        assert_eq!(target.format(), "{0}");
        assert!(target.inner().is_some_and(|i| Rc::ptr_eq(i, &inner)));
        assert!(StringFormatValueConverter::new("{0}", None).inner().is_none());
    }

    #[test]
    fn formats_value_with_composite_format() {
        let target = StringFormatValueConverter::new("Hello {0}!", None);

        assert_eq!(convert(&target, Some(&boxed(String::from("world")))), "Hello world!");
        assert_eq!(convert(&target, Some(&boxed(5i32))), "Hello 5!");
    }

    #[test]
    fn format_without_braces_is_the_format_of_the_value() {
        let target = StringFormatValueConverter::new("0.00", None);

        assert_eq!(convert(&target, Some(&boxed(1.5f64))), "1.50");
        assert_eq!(convert(&StringFormatValueConverter::new("F1", None), Some(&boxed(2.25f64))), "2.2");
        assert_eq!(convert(&StringFormatValueConverter::new("X4", None), Some(&boxed(255i32))), "00FF");
    }

    #[test]
    fn null_value_formats_as_empty_text() {
        let target = StringFormatValueConverter::new("Converted: {0}", None);

        assert_eq!(convert(&target, None), "Converted: ");
    }

    #[test]
    fn inner_converter_is_called_before_formatting() {
        let inner: Rc<dyn IValueConverter> = Rc::new(FuncValueConverter::<i32, i32>::new(|x| x * 2));
        let target = StringFormatValueConverter::new("{0:D3}", Some(inner));

        assert_eq!(convert(&target, Some(&boxed(21i32))), "042");
    }

    #[test]
    fn value_is_used_if_inner_converter_returns_null() {
        let inner: Rc<dyn IValueConverter> = Rc::new(FuncValueConverter::<i32, Option<i32>>::new(|_| None));
        let target = StringFormatValueConverter::new("{0}", Some(inner));

        assert_eq!(convert(&target, Some(&boxed(21i32))), "21");
    }

    #[test]
    fn error_of_inner_converter_is_returned() {
        struct Failing;
        impl IValueConverter for Failing {
            fn convert(
                &self,
                _value: Option<&BoxedValue>,
                _target_type: ValueType,
                _parameter: Option<&BoxedValue>,
                _culture: &CultureInfo,
            ) -> Result<Option<BoxedValue>, BindingError> {
                Err(BindingError::message("inner failed"))
            }

            fn convert_back(
                &self,
                _value: Option<&BoxedValue>,
                _target_type: ValueType,
                _parameter: Option<&BoxedValue>,
                _culture: &CultureInfo,
            ) -> Result<Option<BoxedValue>, BindingError> {
                Err(BindingError::message("inner failed"))
            }
        }
        let target = StringFormatValueConverter::new("{0}", Some(Rc::new(Failing)));

        let result = target.convert(Some(&boxed(1i32)), ValueType::of::<String>(), None, &crate::utilities::CultureInfo::invariant_culture());

        assert_eq!(result.expect_err("an error").to_string(), "inner failed");
    }

    #[test]
    fn invalid_format_is_an_error() {
        let target = StringFormatValueConverter::new("{1}", None);

        let error = target.convert(Some(&boxed(1i32)), ValueType::of::<String>(), None, &crate::utilities::CultureInfo::invariant_culture()).expect_err("an error");

        assert_eq!(error.inner().downcast_ref::<FormatError>(), Some(&FormatError::IndexOutOfRange));
    }

    #[test]
    fn convert_back_is_not_supported() {
        let target = StringFormatValueConverter::new("{0}", None);

        let result = target.convert_back(Some(&boxed(String::from("1"))), ValueType::of::<i32>(), None, &crate::utilities::CultureInfo::invariant_culture());

        assert_eq!(
            result.expect_err("an error").to_string(),
            "Two way bindings are not supported with a string format"
        );
    }
}
