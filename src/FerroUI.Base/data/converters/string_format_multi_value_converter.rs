use super::composite_format::format_values_with;
use super::IMultiValueConverter;
use crate::data::core::ValueType;
use crate::data::BindingError;
use crate::utilities::CultureInfo;
use crate::BoxedValue;
use std::rc::Rc;

/// A multi-value converter which formats the values with a composite format
/// string (see [`composite_format`](super::composite_format)).
pub struct StringFormatMultiValueConverter {
    format: String,
    inner: Option<Rc<dyn IMultiValueConverter>>,
}

impl StringFormatMultiValueConverter {
    /// Creates a string format multi-value converter.
    ///
    /// `format` is the format string; `inner` is an optional inner converter
    /// to be called before the format takes place.
    pub fn new(format: impl Into<String>, inner: Option<Rc<dyn IMultiValueConverter>>) -> Self {
        Self { format: format.into(), inner }
    }

    /// Gets an inner value converter which will be called before the string
    /// format takes place.
    pub fn inner(&self) -> Option<&Rc<dyn IMultiValueConverter>> {
        self.inner.as_ref()
    }

    /// Gets the format string.
    pub fn format(&self) -> &str {
        &self.format
    }
}

impl IMultiValueConverter for StringFormatMultiValueConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        target_type: ValueType,
        parameter: Option<&BoxedValue>,
        culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let result = match &self.inner {
            None => format_values_with(&self.format, values, culture),
            Some(inner) => {
                format_values_with(&self.format, &[inner.convert(values, target_type, parameter, culture)?], culture)
            }
        };
        match result {
            Ok(text) => Ok(Some(Rc::new(text))),
            Err(error) => Err(BindingError::new(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Upstream tests this type through multi-bindings only; these cover the
    //! port.

    use super::*;
    use crate::data::converters::composite_format::FormatError;
    use crate::data::converters::FuncMultiValueConverter;

    fn boxed<T: PartialEq + 'static>(value: T) -> Option<BoxedValue> {
        Some(Rc::new(value))
    }

    fn convert(target: &StringFormatMultiValueConverter, values: &[Option<BoxedValue>]) -> String {
        let result = target.convert(values, ValueType::of::<String>(), None, &crate::utilities::CultureInfo::invariant_culture()).expect("no error").expect("not null");
        result.downcast_ref::<String>().cloned().expect("a string")
    }

    #[test]
    fn exposes_format_and_inner() {
        let inner: Rc<dyn IMultiValueConverter> = Rc::new(FuncMultiValueConverter::<i32, i32>::new(|x| x[0]));
        let target = StringFormatMultiValueConverter::new("{0}", Some(inner.clone()));

        assert_eq!(target.format(), "{0}");
        assert!(target.inner().is_some_and(|i| Rc::ptr_eq(i, &inner)));
        assert!(StringFormatMultiValueConverter::new("{0}", None).inner().is_none());
    }

    #[test]
    fn formats_values_with_composite_format() {
        let target = StringFormatMultiValueConverter::new("{0}+{1}={2:F1}", None);

        assert_eq!(convert(&target, &[boxed(1i32), boxed(String::from("b")), boxed(2.5f64)]), "1+b=2.5");
    }

    /// The format part of the upstream multi-binding test
    /// `Should_Update_When_Null_Value_In_Bindings_With_StringFormat`.
    #[test]
    fn null_value_formats_as_empty_text() {
        let target = StringFormatMultiValueConverter::new("Converted: {0}", None);

        assert_eq!(convert(&target, &[boxed(String::from("Foo"))]), "Converted: Foo");
        assert_eq!(convert(&target, &[None]), "Converted: ");
    }

    #[test]
    fn format_without_braces_is_literal_text() {
        let target = StringFormatMultiValueConverter::new("0.00", None);

        assert_eq!(convert(&target, &[boxed(1.5f64)]), "0.00");
    }

    #[test]
    fn result_of_inner_converter_is_the_only_argument() {
        let inner: Rc<dyn IMultiValueConverter> =
            Rc::new(FuncMultiValueConverter::<i32, i32>::new(|x| x.iter().sum()));
        let target = StringFormatMultiValueConverter::new("Sum: {0:D3}", Some(inner.clone()));

        assert_eq!(convert(&target, &[boxed(1i32), boxed(2i32), boxed(3i32)]), "Sum: 006");

        let target = StringFormatMultiValueConverter::new("{1}", Some(inner));
        let error = target.convert(&[boxed(1i32), boxed(2i32)], ValueType::of::<String>(), None, &crate::utilities::CultureInfo::invariant_culture()).expect_err("an error");
        assert_eq!(error.inner().downcast_ref::<FormatError>(), Some(&FormatError::IndexOutOfRange));
    }

    #[test]
    fn missing_value_is_an_error() {
        let target = StringFormatMultiValueConverter::new("{0} {1}", None);

        let error = target.convert(&[boxed(1i32)], ValueType::of::<String>(), None, &crate::utilities::CultureInfo::invariant_culture()).expect_err("an error");

        assert_eq!(error.inner().downcast_ref::<FormatError>(), Some(&FormatError::IndexOutOfRange));
    }
}
