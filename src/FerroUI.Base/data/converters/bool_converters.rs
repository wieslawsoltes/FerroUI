use super::func_value_converter::cast_value;
use super::{FuncMultiValueConverter, IMultiValueConverter, IValueConverter};
use crate::data::core::ValueType;
use crate::data::BindingError;
use crate::{BoxedValue, FerroProperty};
use std::rc::Rc;

/// Provides a set of useful [`IValueConverter`]s for working with bool
/// values.
pub struct BoolConverters;

impl BoolConverters {
    /// A multi-value converter that returns true if all inputs are true.
    pub fn and() -> Rc<dyn IMultiValueConverter> {
        thread_local! {
            static AND: Rc<dyn IMultiValueConverter> =
                Rc::new(FuncMultiValueConverter::<bool, bool>::new(|x| x.iter().all(|y| *y)));
        }
        AND.with(Rc::clone)
    }

    /// A multi-value converter that returns true if any of the inputs is
    /// true.
    pub fn or() -> Rc<dyn IMultiValueConverter> {
        thread_local! {
            static OR: Rc<dyn IMultiValueConverter> =
                Rc::new(FuncMultiValueConverter::<bool, bool>::new(|x| x.iter().any(|y| *y)));
        }
        OR.with(Rc::clone)
    }

    /// A value converter that returns true when input is false and false
    /// when input is true.
    pub fn not() -> Rc<dyn IValueConverter> {
        thread_local! {
            static NOT: Rc<dyn IValueConverter> = Rc::new(NotConverter);
        }
        NOT.with(Rc::clone)
    }
}

struct NotConverter;

impl NotConverter {
    fn negate(value: Option<&BoxedValue>) -> Option<BoxedValue> {
        match cast_value::<bool>(value) {
            Some(val) => Some(Rc::new(!val)),
            None => Some(FerroProperty::unset_value()),
        }
    }
}

impl IValueConverter for NotConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Self::negate(value))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Self::negate(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::BindingOperations;

    fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue {
        Rc::new(value)
    }

    fn as_bool(result: Result<Option<BoxedValue>, BindingError>) -> bool {
        let value = result.expect("no error").expect("not null");
        *value.downcast_ref::<bool>().expect("a bool")
    }

    #[test]
    fn bool_converters_not_works_two_way() {
        let converter = BoolConverters::not();
        let result = converter.convert(Some(&boxed(true)), ValueType::of::<bool>(), None);
        assert!(!as_bool(result));

        let result = converter.convert_back(Some(&boxed(false)), ValueType::of::<bool>(), None);
        assert!(as_bool(result));
    }

    #[test]
    fn bool_converters_not_returns_unset_on_invalid_input() {
        let converter = BoolConverters::not();
        let result = converter.convert(Some(&boxed(1234i32)), ValueType::of::<bool>(), None);
        assert!(BindingOperations::is_unset(result.expect("no error").as_ref()));
    }

    #[test]
    fn bool_converters_and_works() {
        for (a, b, y) in [(false, false, false), (false, true, false), (true, false, false), (true, true, true)] {
            let converter = BoolConverters::and();
            let result = converter.convert(&[Some(boxed(a)), Some(boxed(b))], ValueType::of::<bool>(), None);
            assert_eq!(as_bool(result), y, "{a} and {b}");
        }
    }

    #[test]
    fn bool_converters_or_works() {
        for (a, b, y) in [(false, false, false), (false, true, true), (true, false, true), (true, true, true)] {
            let converter = BoolConverters::or();
            let result = converter.convert(&[Some(boxed(a)), Some(boxed(b))], ValueType::of::<bool>(), None);
            assert_eq!(as_bool(result), y, "{a} or {b}");
        }
    }

    // Specific to this port.

    #[test]
    fn not_returns_unset_on_null_input() {
        let converter = BoolConverters::not();
        let result = converter.convert(None, ValueType::of::<bool>(), None);
        assert!(BindingOperations::is_unset(result.expect("no error").as_ref()));
    }

    #[test]
    fn and_returns_unset_on_invalid_input() {
        let converter = BoolConverters::and();
        let result = converter.convert(&[Some(boxed(true)), Some(boxed(1234i32))], ValueType::of::<bool>(), None);
        assert!(BindingOperations::is_unset(result.expect("no error").as_ref()));
    }

    #[test]
    fn converters_are_shared_instances() {
        assert!(Rc::ptr_eq(&BoolConverters::not(), &BoolConverters::not()));
        assert!(Rc::ptr_eq(&BoolConverters::and(), &BoolConverters::and()));
        assert!(Rc::ptr_eq(&BoolConverters::or(), &BoolConverters::or()));
    }
}
