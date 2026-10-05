use super::IValueConverter;
use crate::data::core::{ValueType, ValueTypes};
use crate::data::BindingError;
use crate::{BoxedValue, FerroObject, FerroProperty, PropertyValue, Ref};
use std::any::Any;
use std::rc::Rc;

/// Casts an untyped value to `T`: the equivalent of a type test followed by
/// a cast, where null is accepted by types that can be null.
///
/// A value casts to `T` if it is a `T`, if `T` is the "any value" type
/// (`Option<BoxedValue>` or [`BoxedValue`]), if `T` is the nullable form
/// (`Option<U>`) of the value's type, or if it is an object handle that
/// casts to the handle type `T`. Null casts to `Option<U>` and to
/// `Option<BoxedValue>`. A `&'static str` casts to `String`. No other
/// conversion (numeric, parsing, to text) is attempted.
pub fn cast_value<T: PropertyValue>(value: Option<&BoxedValue>) -> Option<T> {
    fn downcast<T: PropertyValue>(value: &BoxedValue) -> Option<T> {
        value.downcast_ref::<T>().cloned()
    }

    let target = ValueType::of::<T>();
    if let Some(v) = value.and_then(downcast::<T>) {
        return Some(v);
    }

    // A nullable value is null or its contents.
    let Some(value) = value.cloned().and_then(ValueTypes::normalize) else {
        return ValueTypes::null_value(target).and_then(|null| downcast::<T>(&null));
    };
    if let Some(v) = downcast::<T>(&value) {
        return Some(v);
    }
    if target.is::<Option<BoxedValue>>() {
        let wrapped: BoxedValue = Rc::new(Some(value));
        return downcast::<T>(&wrapped);
    }
    if target.is::<BoxedValue>() {
        let wrapped: BoxedValue = Rc::new(value);
        return downcast::<T>(&wrapped);
    }

    if let Some(object) = ValueTypes::as_object(&*value) {
        // A handle casts to the handles of its class and base classes.
        let root: BoxedValue = Rc::new(object);
        if target.is::<Ref<FerroObject>>() {
            return downcast::<T>(&root);
        }
        if target.is_string() {
            return None;
        }
        return ValueTypes::try_convert_registered(&value, target)
            .or_else(|| ValueTypes::try_convert_registered(&root, target))
            .and_then(|converted| downcast::<T>(&converted));
    }

    // Of the registered conversions only those that keep the value as it is
    // are casts: wrapping into the nullable form of its type.
    let converted = ValueTypes::try_convert_registered(&value, target)?;
    let contents = ValueTypes::normalize(converted.clone())?;
    let source_id = (*value).as_any().type_id();
    let same_type = (*contents).as_any().type_id() == source_id;
    let string_literal = value.is::<&'static str>() && contents.is::<String>();
    if same_type || string_literal {
        downcast::<T>(&converted)
    } else {
        None
    }
}

/// Boxes the result of a typed converter function: a nullable result becomes
/// null or its contents, an untyped result is passed through.
pub(crate) fn box_result<T: PropertyValue>(value: T) -> Option<BoxedValue> {
    if let Some(untyped) = (&value as &dyn Any).downcast_ref::<BoxedValue>() {
        return Some(untyped.clone());
    }
    ValueTypes::normalize(Rc::new(value))
}

fn not_implemented() -> BindingError {
    BindingError::message("The method or operation is not implemented.")
}

/// A general purpose [`IValueConverter`] that uses a function to provide the
/// converter logic.
///
/// `TIn` is the input type and `TOut` the output type. Use `Option<T>` for
/// inputs that may be null and `Option<BoxedValue>` for "any value". An
/// input that is not a `TIn` converts to the unset value marker.
pub struct FuncValueConverter<TIn, TOut> {
    convert: Rc<dyn Fn(TIn) -> TOut>,
    convert_back: Option<Rc<dyn Fn(TOut) -> TIn>>,
}

impl<TIn: PropertyValue, TOut: PropertyValue> FuncValueConverter<TIn, TOut> {
    /// Creates a converter from the function that converts `TIn` to `TOut`.
    pub fn new(convert: impl Fn(TIn) -> TOut + 'static) -> Self {
        Self { convert: Rc::new(convert), convert_back: None }
    }

    /// Creates a converter from the function that converts `TIn` to `TOut`
    /// and the function that converts `TOut` back to `TIn`.
    pub fn new_two_way(convert: impl Fn(TIn) -> TOut + 'static, convert_back: impl Fn(TOut) -> TIn + 'static) -> Self {
        Self { convert: Rc::new(convert), convert_back: Some(Rc::new(convert_back)) }
    }
}

impl<TIn: PropertyValue, TOut: PropertyValue> IValueConverter for FuncValueConverter<TIn, TOut> {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        match cast_value::<TIn>(value) {
            Some(value) => Ok(box_result((self.convert)(value))),
            None => Ok(Some(FerroProperty::unset_value())),
        }
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let Some(convert_back) = &self.convert_back else {
            return Err(not_implemented());
        };

        match cast_value::<TOut>(value) {
            Some(value) => Ok(box_result(convert_back(value))),
            None => Ok(Some(FerroProperty::unset_value())),
        }
    }
}

/// A general purpose [`IValueConverter`] that uses a function of the value
/// and the converter parameter to provide the converter logic.
///
/// `TIn` is the input type, `TParam` the parameter type and `TOut` the
/// output type. An input that is not a `TIn`, or a parameter that is not a
/// `TParam`, converts to the unset value marker.
pub struct FuncValueConverterWithParameter<TIn, TParam, TOut> {
    convert: Rc<dyn Fn(TIn, TParam) -> TOut>,
    convert_back: Option<Rc<dyn Fn(TOut, TParam) -> TIn>>,
}

impl<TIn: PropertyValue, TParam: PropertyValue, TOut: PropertyValue> FuncValueConverterWithParameter<TIn, TParam, TOut> {
    /// Creates a converter from the function that converts `TIn` to `TOut`.
    pub fn new(convert: impl Fn(TIn, TParam) -> TOut + 'static) -> Self {
        Self { convert: Rc::new(convert), convert_back: None }
    }

    /// Creates a converter from the function that converts `TIn` to `TOut`
    /// and the function that converts `TOut` back to `TIn`.
    pub fn new_two_way(
        convert: impl Fn(TIn, TParam) -> TOut + 'static,
        convert_back: impl Fn(TOut, TParam) -> TIn + 'static,
    ) -> Self {
        Self { convert: Rc::new(convert), convert_back: Some(Rc::new(convert_back)) }
    }
}

impl<TIn: PropertyValue, TParam: PropertyValue, TOut: PropertyValue> IValueConverter
    for FuncValueConverterWithParameter<TIn, TParam, TOut>
{
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        match (cast_value::<TIn>(value), cast_value::<TParam>(parameter)) {
            (Some(value), Some(parameter)) => Ok(box_result((self.convert)(value, parameter))),
            _ => Ok(Some(FerroProperty::unset_value())),
        }
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let Some(convert_back) = &self.convert_back else {
            return Err(not_implemented());
        };

        match (cast_value::<TOut>(value), cast_value::<TParam>(parameter)) {
            (Some(value), Some(parameter)) => Ok(box_result(convert_back(value, parameter))),
            _ => Ok(Some(FerroProperty::unset_value())),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Upstream has no dedicated tests for this type; these cover the port.

    use super::*;
    use crate::data::BindingOperations;
    use crate::StyledElement;

    fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue {
        Rc::new(value)
    }

    fn get<T: PropertyValue>(result: Result<Option<BoxedValue>, BindingError>) -> T {
        let value = result.expect("no error").expect("not null");
        value.downcast_ref::<T>().cloned().expect("a value of the expected type")
    }

    fn is_unset(result: Result<Option<BoxedValue>, BindingError>) -> bool {
        BindingOperations::is_unset(result.expect("no error").as_ref())
    }

    #[test]
    fn convert_calls_function_for_input_of_input_type() {
        let target = FuncValueConverter::<i32, String>::new(|x| format!("<{x}>"));

        let result = target.convert(Some(&boxed(5i32)), ValueType::of::<String>(), None);

        assert_eq!(get::<String>(result), "<5>");
    }

    #[test]
    fn convert_returns_unset_for_input_of_other_type() {
        let target = FuncValueConverter::<i32, String>::new(|x| format!("<{x}>"));

        assert!(is_unset(target.convert(Some(&boxed(5i64)), ValueType::of::<String>(), None)));
        assert!(is_unset(target.convert(Some(&boxed(5.0f64)), ValueType::of::<String>(), None)));
        assert!(is_unset(target.convert(Some(&boxed(String::from("5"))), ValueType::of::<String>(), None)));
    }

    #[test]
    fn convert_returns_unset_for_null_input_of_non_nullable_type() {
        let target = FuncValueConverter::<i32, String>::new(|x| format!("<{x}>"));

        assert!(is_unset(target.convert(None, ValueType::of::<String>(), None)));
    }

    #[test]
    fn convert_passes_null_to_nullable_input_type() {
        let target = FuncValueConverter::<Option<i32>, String>::new(|x| format!("{x:?}"));

        assert_eq!(get::<String>(target.convert(None, ValueType::of::<String>(), None)), "None");
        assert_eq!(get::<String>(target.convert(Some(&boxed(5i32)), ValueType::of::<String>(), None)), "Some(5)");
        assert_eq!(get::<String>(target.convert(Some(&boxed(Some(6i32))), ValueType::of::<String>(), None)), "Some(6)");
        assert_eq!(
            get::<String>(target.convert(Some(&boxed(Option::<i32>::None)), ValueType::of::<String>(), None)),
            "None"
        );
        assert!(is_unset(target.convert(Some(&boxed(5i64)), ValueType::of::<String>(), None)));
    }

    #[test]
    fn convert_accepts_nullable_value_for_non_nullable_input_type() {
        let target = FuncValueConverter::<i32, i32>::new(|x| x + 1);

        assert_eq!(get::<i32>(target.convert(Some(&boxed(Some(5i32))), ValueType::of::<i32>(), None)), 6);
        assert!(is_unset(target.convert(Some(&boxed(Option::<i32>::None)), ValueType::of::<i32>(), None)));
    }

    #[test]
    fn convert_accepts_string_literal_as_string() {
        let target = FuncValueConverter::<String, usize>::new(|x| x.len());
        let nullable = FuncValueConverter::<Option<String>, usize>::new(|x| x.map_or(0, |x| x.len()));

        assert_eq!(get::<usize>(target.convert(Some(&boxed("abc")), ValueType::of::<usize>(), None)), 3);
        assert_eq!(get::<usize>(nullable.convert(Some(&boxed("abcd")), ValueType::of::<usize>(), None)), 4);
    }

    #[test]
    fn convert_accepts_any_value_for_untyped_input_type() {
        let target = FuncValueConverter::<Option<BoxedValue>, bool>::new(|x| x.is_some());

        assert!(get::<bool>(target.convert(Some(&boxed(5i32)), ValueType::of::<bool>(), None)));
        assert!(get::<bool>(target.convert(Some(&FerroProperty::unset_value()), ValueType::of::<bool>(), None)));
        assert!(!get::<bool>(target.convert(None, ValueType::of::<bool>(), None)));
    }

    #[test]
    fn convert_accepts_handle_of_derived_class() {
        let target = FuncValueConverter::<Ref<FerroObject>, bool>::new(|_| true);
        let nullable = FuncValueConverter::<Option<Ref<FerroObject>>, bool>::new(|x| x.is_some());
        let derived = FuncValueConverter::<Ref<StyledElement>, bool>::new(|_| true);
        let element = boxed(StyledElement::new());
        let object = boxed(FerroObject::new());

        assert!(get::<bool>(target.convert(Some(&element), ValueType::of::<bool>(), None)));
        assert!(get::<bool>(nullable.convert(Some(&element), ValueType::of::<bool>(), None)));
        assert!(!get::<bool>(nullable.convert(None, ValueType::of::<bool>(), None)));
        assert!(get::<bool>(derived.convert(Some(&element), ValueType::of::<bool>(), None)));
        assert!(is_unset(derived.convert(Some(&object), ValueType::of::<bool>(), None)));
        assert!(is_unset(target.convert(None, ValueType::of::<bool>(), None)));
    }

    #[test]
    fn convert_boxes_nullable_result_as_null_or_contents() {
        let target = FuncValueConverter::<i32, Option<i32>>::new(|x| (x > 0).then_some(x));

        assert_eq!(get::<i32>(target.convert(Some(&boxed(5i32)), ValueType::of::<Option<i32>>(), None)), 5);
        assert!(target.convert(Some(&boxed(-5i32)), ValueType::of::<Option<i32>>(), None).expect("no error").is_none());
    }

    #[test]
    fn convert_passes_untyped_result_through() {
        let target = FuncValueConverter::<i32, BoxedValue>::new(|_| BindingOperations::do_nothing());

        let result = target.convert(Some(&boxed(5i32)), ValueType::object(), None).expect("no error");

        assert!(BindingOperations::is_do_nothing(result.as_ref()));
    }

    #[test]
    fn convert_back_without_function_is_an_error() {
        let target = FuncValueConverter::<i32, String>::new(|x| x.to_string());

        let result = target.convert_back(Some(&boxed(String::from("5"))), ValueType::of::<i32>(), None);

        assert_eq!(result.expect_err("an error").to_string(), "The method or operation is not implemented.");
    }

    #[test]
    fn convert_back_calls_function_for_input_of_output_type() {
        let target =
            FuncValueConverter::<i32, String>::new_two_way(|x| x.to_string(), |x| x.parse().unwrap_or_default());

        assert_eq!(get::<String>(target.convert(Some(&boxed(5i32)), ValueType::of::<String>(), None)), "5");
        assert_eq!(get::<i32>(target.convert_back(Some(&boxed(String::from("7"))), ValueType::of::<i32>(), None)), 7);
        assert!(is_unset(target.convert_back(Some(&boxed(7i32)), ValueType::of::<i32>(), None)));
        assert!(is_unset(target.convert_back(None, ValueType::of::<i32>(), None)));
    }

    #[test]
    fn parameter_converter_passes_parameter() {
        let target = FuncValueConverterWithParameter::<i32, i32, i32>::new(|x, p| x * p);

        assert_eq!(get::<i32>(target.convert(Some(&boxed(5i32)), ValueType::of::<i32>(), Some(&boxed(3i32)))), 15);
    }

    #[test]
    fn parameter_converter_returns_unset_for_value_or_parameter_of_other_type() {
        let target = FuncValueConverterWithParameter::<i32, i32, i32>::new(|x, p| x * p);

        assert!(is_unset(target.convert(Some(&boxed(5i64)), ValueType::of::<i32>(), Some(&boxed(3i32)))));
        assert!(is_unset(target.convert(Some(&boxed(5i32)), ValueType::of::<i32>(), Some(&boxed(3i64)))));
        assert!(is_unset(target.convert(Some(&boxed(5i32)), ValueType::of::<i32>(), None)));
    }

    #[test]
    fn parameter_converter_accepts_null_parameter_of_nullable_type() {
        let target = FuncValueConverterWithParameter::<i32, Option<i32>, i32>::new(|x, p| x * p.unwrap_or(1));

        assert_eq!(get::<i32>(target.convert(Some(&boxed(5i32)), ValueType::of::<i32>(), None)), 5);
        assert_eq!(get::<i32>(target.convert(Some(&boxed(5i32)), ValueType::of::<i32>(), Some(&boxed(2i32)))), 10);
    }

    #[test]
    fn parameter_converter_convert_back() {
        let one_way = FuncValueConverterWithParameter::<i32, i32, i32>::new(|x, p| x * p);
        let two_way = FuncValueConverterWithParameter::<i32, i32, i32>::new_two_way(|x, p| x * p, |x, p| x / p);

        let result = one_way.convert_back(Some(&boxed(15i32)), ValueType::of::<i32>(), Some(&boxed(3i32)));
        assert_eq!(result.expect_err("an error").to_string(), "The method or operation is not implemented.");
        assert_eq!(get::<i32>(two_way.convert_back(Some(&boxed(15i32)), ValueType::of::<i32>(), Some(&boxed(3i32)))), 5);
        assert!(is_unset(two_way.convert_back(Some(&boxed(15i32)), ValueType::of::<i32>(), Some(&boxed(String::new())))));
    }
}
