use super::{FuncMultiValueConverter, FuncValueConverter, FuncValueConverterWithParameter};
use super::{IMultiValueConverter, IValueConverter};
use crate::data::core::ValueTypes;
use crate::BoxedValue;
use std::rc::Rc;

/// Provides a set of useful [`IValueConverter`]s for working with objects.
pub struct ObjectConverters;

/// An untyped value that may be null.
type Object = Option<BoxedValue>;

/// The value equality of two non-null untyped values.
fn equals(a: &BoxedValue, b: &Object) -> bool {
    ValueTypes::identity_equals(Some(a), b.as_ref())
}

impl ObjectConverters {
    /// A value converter that returns true if the input object is a null
    /// reference.
    pub fn is_null() -> Rc<dyn IValueConverter> {
        thread_local! {
            static IS_NULL: Rc<dyn IValueConverter> =
                Rc::new(FuncValueConverter::<Object, bool>::new(|x| x.is_none()));
        }
        IS_NULL.with(Rc::clone)
    }

    /// A value converter that returns true if the input object is not null.
    pub fn is_not_null() -> Rc<dyn IValueConverter> {
        thread_local! {
            static IS_NOT_NULL: Rc<dyn IValueConverter> =
                Rc::new(FuncValueConverter::<Object, bool>::new(|x| x.is_some()));
        }
        IS_NOT_NULL.with(Rc::clone)
    }

    /// A value converter that returns true if the input object is equal to a
    /// parameter object.
    pub fn equal() -> Rc<dyn IValueConverter> {
        thread_local! {
            static EQUAL: Rc<dyn IValueConverter> =
                Rc::new(FuncValueConverterWithParameter::<Object, Object, bool>::new(|a, b| match &a {
                    Some(a) => equals(a, &b),
                    None => b.is_none(),
                }));
        }
        EQUAL.with(Rc::clone)
    }

    /// A value converter that returns true if the input object is not equal
    /// to a parameter object.
    pub fn not_equal() -> Rc<dyn IValueConverter> {
        thread_local! {
            static NOT_EQUAL: Rc<dyn IValueConverter> =
                Rc::new(FuncValueConverterWithParameter::<Object, Object, bool>::new(|a, b| match &a {
                    Some(a) => !equals(a, &b),
                    None => b.is_some(),
                }));
        }
        NOT_EQUAL.with(Rc::clone)
    }

    /// A multi-value converter that returns true if all inputs are null.
    pub fn are_all_null() -> Rc<dyn IMultiValueConverter> {
        thread_local! {
            static ARE_ALL_NULL: Rc<dyn IMultiValueConverter> =
                Rc::new(FuncMultiValueConverter::<Object, bool>::new(|x| x.iter().all(|item| item.is_none())));
        }
        ARE_ALL_NULL.with(Rc::clone)
    }

    /// A multi-value converter that returns true if at least one input is
    /// null.
    pub fn are_any_null() -> Rc<dyn IMultiValueConverter> {
        thread_local! {
            static ARE_ANY_NULL: Rc<dyn IMultiValueConverter> =
                Rc::new(FuncMultiValueConverter::<Object, bool>::new(|x| x.iter().any(|item| item.is_none())));
        }
        ARE_ANY_NULL.with(Rc::clone)
    }

    /// A multi-value converter that returns true if all inputs are equal to
    /// each other. Null values are not considered equal to anything.
    pub fn are_all_equal() -> Rc<dyn IMultiValueConverter> {
        thread_local! {
            static ARE_ALL_EQUAL: Rc<dyn IMultiValueConverter> =
                Rc::new(FuncMultiValueConverter::<Object, bool>::new(ObjectConverters::equality_function));
        }
        ARE_ALL_EQUAL.with(Rc::clone)
    }

    /// Helper function for the [`are_all_equal`](Self::are_all_equal)
    /// converter. Returns true if all inputs are equal to each other.
    fn equality_function(values: &[Object]) -> bool {
        let mut values = values.iter();

        // Empty collection is considered equal.
        let Some(first) = values.next() else {
            return true;
        };

        // Null values are not considered equal to anything.
        if first.is_none() {
            return false;
        }

        for item in values {
            match item {
                Some(item) if equals(item, first) => {}
                _ => return false,
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::core::ValueType;
    use crate::data::BindingError;
    use crate::FerroObject;

    fn as_bool(result: Result<Option<BoxedValue>, BindingError>) -> bool {
        let value = result.expect("no error").expect("not null");
        *value.downcast_ref::<bool>().expect("a bool")
    }

    /// A new object with reference identity.
    fn new_object() -> BoxedValue {
        Rc::new(FerroObject::new())
    }

    // ObjectConvertersTests

    fn text(value: Option<&str>) -> Object {
        value.map(|v| Rc::new(v.to_string()) as BoxedValue)
    }

    const VALUE: Option<&str> = Some("value");

    #[test]
    fn object_converters_tests_are_all_null_works() {
        let cases = [
            (None, None, None, true),
            (None, None, VALUE, false),
            (None, VALUE, None, false),
            (VALUE, None, None, false),
            (VALUE, VALUE, VALUE, false),
        ];
        for (value1, value2, value3, valid) in cases {
            let converter = ObjectConverters::are_all_null();
            let result = converter.convert(&[text(value1), text(value2), text(value3)], ValueType::of::<bool>(), None);
            assert_eq!(as_bool(result), valid, "{value1:?} {value2:?} {value3:?}");
        }
    }

    #[test]
    fn object_converters_tests_are_any_null_works() {
        let cases = [
            (None, None, None, true),
            (None, None, VALUE, true),
            (None, VALUE, None, true),
            (VALUE, None, None, true),
            (VALUE, VALUE, VALUE, false),
        ];
        for (value1, value2, value3, valid) in cases {
            let converter = ObjectConverters::are_any_null();
            let result = converter.convert(&[text(value1), text(value2), text(value3)], ValueType::of::<bool>(), None);
            assert_eq!(as_bool(result), valid, "{value1:?} {value2:?} {value3:?}");
        }
    }

    #[test]
    fn object_converters_tests_are_equal_works() {
        let cases: [([Object; 3], bool); 6] = [
            ([text(VALUE), text(VALUE), text(VALUE)], true),
            ([None, text(VALUE), None], false),
            ([text(VALUE), None, text(VALUE)], false),
            ([text(VALUE), text(VALUE), text(Some("value1"))], false),
            ([text(Some("value1")), text(VALUE), text(Some("value1"))], false),
            ([text(VALUE), text(VALUE), Some(Rc::new(1i32))], false),
        ];
        for (values, valid) in cases {
            let converter = ObjectConverters::are_all_equal();
            let result = converter.convert(&values, ValueType::of::<bool>(), None);
            assert_eq!(as_bool(result), valid);
        }
    }

    #[test]
    fn object_converters_tests_are_equal_edge_works() {
        for (empty, unique, valid) in [(true, true, true), (false, true, false), (false, false, true)] {
            let values: Vec<Object> = if empty {
                Vec::new()
            } else if unique {
                vec![text(Some("1")), text(Some("2"))]
            } else {
                vec![text(Some("1")), text(Some("1"))]
            };

            let converter = ObjectConverters::are_all_equal();
            let result = converter.convert(&values, ValueType::of::<bool>(), None);
            assert_eq!(as_bool(result), valid, "empty: {empty}, unique: {unique}");
        }
    }

    // ObjectConvertersTests_Equal

    mod equal {
        use super::*;

        #[test]
        fn returns_true_if_value_and_parameter_are_null() {
            let result = ObjectConverters::equal().convert(None, ValueType::object(), None);

            assert!(as_bool(result));
        }

        #[test]
        fn returns_false_if_value_is_null_and_parameter_is_not_null() {
            let result = ObjectConverters::equal().convert(None, ValueType::object(), Some(&new_object()));

            assert!(!as_bool(result));
        }

        #[test]
        fn returns_false_if_value_and_parameter_are_different_objects() {
            let result = ObjectConverters::equal().convert(Some(&new_object()), ValueType::object(), Some(&new_object()));

            assert!(!as_bool(result));
        }

        #[test]
        fn returns_true_if_value_and_parameter_are_same_object() {
            let target = new_object();
            let result = ObjectConverters::equal().convert(Some(&target), ValueType::object(), Some(&target));

            assert!(as_bool(result));
        }

        // Specific to this port.

        #[test]
        fn returns_false_if_value_is_not_null_and_parameter_is_null() {
            let result = ObjectConverters::equal().convert(Some(&new_object()), ValueType::object(), None);

            assert!(!as_bool(result));
        }

        #[test]
        fn compares_values_by_value() {
            let a: BoxedValue = Rc::new(5i32);
            let b: BoxedValue = Rc::new(5i32);
            let c: BoxedValue = Rc::new(5i64);

            assert!(as_bool(ObjectConverters::equal().convert(Some(&a), ValueType::object(), Some(&b))));
            assert!(!as_bool(ObjectConverters::equal().convert(Some(&a), ValueType::object(), Some(&c))));
        }

        #[test]
        fn same_handle_in_different_boxes_is_equal() {
            let object = FerroObject::new();
            let a: BoxedValue = Rc::new(object.clone());
            let b: BoxedValue = Rc::new(object);

            assert!(as_bool(ObjectConverters::equal().convert(Some(&a), ValueType::object(), Some(&b))));
        }
    }

    // ObjectConvertersTests_IsNull

    mod is_null {
        use super::*;

        #[test]
        fn returns_true_if_value_is_null() {
            let result = ObjectConverters::is_null().convert(None, ValueType::object(), None);

            assert!(as_bool(result));
        }

        #[test]
        fn returns_false_if_value_is_not_null() {
            let result = ObjectConverters::is_null().convert(Some(&new_object()), ValueType::object(), None);

            assert!(!as_bool(result));
        }

        // Specific to this port.

        #[test]
        fn returns_true_if_value_is_an_empty_nullable() {
            let value: BoxedValue = Rc::new(Option::<i32>::None);
            let result = ObjectConverters::is_null().convert(Some(&value), ValueType::object(), None);

            assert!(as_bool(result));
        }
    }

    // ObjectConvertersTests_IsNotNull

    mod is_not_null {
        use super::*;

        #[test]
        fn returns_true_if_value_is_not_null() {
            let result = ObjectConverters::is_not_null().convert(Some(&new_object()), ValueType::object(), None);

            assert!(as_bool(result));
        }

        #[test]
        fn returns_false_if_value_is_null() {
            let result = ObjectConverters::is_not_null().convert(None, ValueType::object(), None);

            assert!(!as_bool(result));
        }
    }

    // ObjectConvertersTests_NotEqual

    mod not_equal {
        use super::*;

        #[test]
        fn returns_false_if_value_and_parameter_are_null() {
            let result = ObjectConverters::not_equal().convert(None, ValueType::object(), None);

            assert!(!as_bool(result));
        }

        #[test]
        fn returns_true_if_value_is_null_and_parameter_is_not_null() {
            let result = ObjectConverters::not_equal().convert(None, ValueType::object(), Some(&new_object()));

            assert!(as_bool(result));
        }

        #[test]
        fn returns_true_if_value_and_parameter_are_different_objects() {
            let result =
                ObjectConverters::not_equal().convert(Some(&new_object()), ValueType::object(), Some(&new_object()));

            assert!(as_bool(result));
        }

        #[test]
        fn returns_false_if_value_and_parameter_are_same_object() {
            let target = new_object();
            let result = ObjectConverters::not_equal().convert(Some(&target), ValueType::object(), Some(&target));

            assert!(!as_bool(result));
        }
    }
}
