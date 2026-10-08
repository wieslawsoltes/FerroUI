use super::func_value_converter::{box_result, cast_value};
use super::IMultiValueConverter;
use crate::utilities::CultureInfo;
use crate::data::core::ValueType;
use crate::data::BindingError;
use crate::{BoxedValue, FerroProperty, PropertyValue};
use std::rc::Rc;

/// A general purpose [`IMultiValueConverter`] that uses a function to
/// provide the converter logic.
///
/// `TIn` is the type of the inputs and `TOut` the output type. Use
/// `Option<T>` for inputs that may be null and `Option<BoxedValue>` for "any
/// value". If any input is not a `TIn` the converter returns the unset value
/// marker.
pub struct FuncMultiValueConverter<TIn, TOut> {
    convert: Rc<ConvertFn<TIn, TOut>>,
}

type ConvertFn<TIn, TOut> = dyn Fn(&[TIn]) -> TOut;

impl<TIn: PropertyValue, TOut: PropertyValue> FuncMultiValueConverter<TIn, TOut> {
    /// Creates a converter from the convert function.
    pub fn new(convert: impl Fn(&[TIn]) -> TOut + 'static) -> Self {
        Self { convert: Rc::new(convert) }
    }
}

impl<TIn: PropertyValue, TOut: PropertyValue> IMultiValueConverter for FuncMultiValueConverter<TIn, TOut> {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        // Null values are kept when they are valid for the input type.
        let converted: Vec<TIn> = values.iter().filter_map(|value| cast_value::<TIn>(value.as_ref())).collect();

        if converted.len() == values.len() {
            Ok(box_result((self.convert)(&converted)))
        } else {
            Ok(Some(FerroProperty::unset_value()))
        }
    }
}

#[cfg(test)]
mod tests {
    //! The converter tests of the upstream multi-binding tests of the object
    //! class, plus tests specific to this port.

    use super::*;
    use crate::data::BindingOperations;

    fn boxed<T: PartialEq + 'static>(value: T) -> Option<BoxedValue> {
        Some(Rc::new(value))
    }

    fn string(result: Result<Option<BoxedValue>, BindingError>) -> Option<String> {
        result.expect("no error").map(|v| v.downcast_ref::<String>().cloned().expect("a string"))
    }

    fn join(values: &[Option<String>]) -> String {
        values.iter().map(|v| v.as_deref().unwrap_or("")).collect::<Vec<_>>().join(",")
    }

    #[test]
    fn multi_value_converter_should_not_skip_valid_null_reference_type_value() {
        let target = FuncMultiValueConverter::<Option<String>, String>::new(join);

        let value = target.convert(&[boxed("Foo"), boxed("Bar"), boxed("Baz")], ValueType::of::<String>(), None, &crate::utilities::CultureInfo::invariant_culture());

        assert_eq!(string(value).as_deref(), Some("Foo,Bar,Baz"));

        let value = target.convert(&[None, boxed("Bar"), boxed("Baz")], ValueType::of::<String>(), None, &crate::utilities::CultureInfo::invariant_culture());

        assert_eq!(string(value).as_deref(), Some(",Bar,Baz"));
    }

    #[derive(Clone, Default, PartialEq)]
    struct StringValueTypeWrapper {
        value: String,
    }

    #[test]
    fn multi_value_converter_should_not_skip_valid_default_value_type_value() {
        let target = FuncMultiValueConverter::<StringValueTypeWrapper, String>::new(|v| {
            v.iter().map(|v| v.value.as_str()).collect::<Vec<_>>().join(",")
        });

        let create = |values: &[Option<&str>]| -> Vec<Option<BoxedValue>> {
            values
                .iter()
                .map(|v| match v {
                    Some(v) => boxed(StringValueTypeWrapper { value: (*v).to_string() }),
                    None => boxed(StringValueTypeWrapper::default()),
                })
                .collect()
        };

        let value = target.convert(&create(&[Some("Foo"), Some("Bar"), Some("Baz")]), ValueType::of::<String>(), None, &crate::utilities::CultureInfo::invariant_culture());

        assert_eq!(string(value).as_deref(), Some("Foo,Bar,Baz"));

        let value = target.convert(&create(&[None, Some("Bar"), Some("Baz")]), ValueType::of::<String>(), None, &crate::utilities::CultureInfo::invariant_culture());

        assert_eq!(string(value).as_deref(), Some(",Bar,Baz"));
    }

    #[test]
    fn multi_value_converter_supports_indexing_the_parameters() {
        let target = FuncMultiValueConverter::<Option<String>, Option<String>>::new(|v| v[0].clone());

        let value = target.convert(&[boxed("Foo"), boxed("Bar"), boxed("Baz")], ValueType::of::<String>(), None, &crate::utilities::CultureInfo::invariant_culture());

        assert_eq!(string(value).as_deref(), Some("Foo"));

        let value = target.convert(&[None, boxed("Bar"), boxed("Baz")], ValueType::of::<String>(), None, &crate::utilities::CultureInfo::invariant_culture());

        assert!(value.expect("no error").is_none());
    }

    #[test]
    fn returns_unset_if_any_value_is_not_of_input_type() {
        let target = FuncMultiValueConverter::<Option<String>, String>::new(join);

        let value = target.convert(&[boxed("Foo"), boxed(1i32)], ValueType::of::<String>(), None, &crate::utilities::CultureInfo::invariant_culture());

        assert!(BindingOperations::is_unset(value.expect("no error").as_ref()));
    }

    #[test]
    fn returns_unset_for_null_value_of_non_nullable_input_type() {
        let target = FuncMultiValueConverter::<bool, bool>::new(|v| v.iter().all(|v| *v));

        let value = target.convert(&[boxed(true), None], ValueType::of::<bool>(), None, &crate::utilities::CultureInfo::invariant_culture());

        assert!(BindingOperations::is_unset(value.expect("no error").as_ref()));
    }

    #[test]
    fn untyped_inputs_accept_every_value() {
        let target = FuncMultiValueConverter::<Option<BoxedValue>, usize>::new(|v| v.len());

        let value = target.convert(&[boxed("Foo"), boxed(1i32), None], ValueType::of::<usize>(), None, &crate::utilities::CultureInfo::invariant_culture());

        assert_eq!(value.expect("no error").and_then(|v| v.downcast_ref::<usize>().copied()), Some(3));
    }
}
