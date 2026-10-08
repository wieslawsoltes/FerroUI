use crate::data::core::{ValueType, ValueTypes};
use crate::data::{AggregateError, BindingErrorType, BindingNotification, BindingOperations};
use crate::{BoxedValue, DoNothingType, FerroProperty, PropertyValue, UnsetValueType};
use std::any::Any;
use std::fmt;
use std::rc::Rc;

/// An error produced by a binding or by data validation.
#[derive(Clone)]
pub struct BindingError(Rc<dyn std::error::Error>);

impl BindingError {
    pub fn new(error: impl std::error::Error + 'static) -> Self {
        Self(Rc::new(error))
    }

    pub fn message(message: impl Into<String>) -> Self {
        #[derive(Debug)]
        struct Message(String);
        impl fmt::Display for Message {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
        impl std::error::Error for Message {}
        Self(Rc::new(Message(message.into())))
    }

    pub fn inner(&self) -> &(dyn std::error::Error + 'static) {
        &*self.0
    }
}

impl fmt::Debug for BindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for BindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl PartialEq for BindingError {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

bitflags::bitflags! {
    /// Describes the state of a [`BindingValue`]: a kind (the bits of
    /// [`TYPE_MASK`](Self::TYPE_MASK)) combined with the
    /// [`HAS_VALUE`](Self::HAS_VALUE) and [`HAS_ERROR`](Self::HAS_ERROR)
    /// flags.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct BindingValueType: i32 {
        /// An unset value: the target property will revert to its unbound
        /// state until a new binding value is produced.
        const UNSET_VALUE = 0;
        /// Do nothing: the binding value will be ignored.
        const DO_NOTHING = 1;
        /// A simple value.
        const VALUE = 2 | Self::HAS_VALUE.bits();
        /// A binding error, such as a missing source property.
        const BINDING_ERROR = 3 | Self::HAS_ERROR.bits();
        /// A data validation error.
        const DATA_VALIDATION_ERROR = 4 | Self::HAS_ERROR.bits();
        /// A binding error with a fallback value.
        const BINDING_ERROR_WITH_FALLBACK = Self::BINDING_ERROR.bits() | Self::HAS_VALUE.bits();
        /// A data validation error with a fallback value.
        const DATA_VALIDATION_ERROR_WITH_FALLBACK = Self::DATA_VALIDATION_ERROR.bits() | Self::HAS_VALUE.bits();
        /// The bits that hold the kind of the value.
        const TYPE_MASK = 0x00ff;
        /// The binding value has a value.
        const HAS_VALUE = 0x0100;
        /// The binding value has an error.
        const HAS_ERROR = 0x0200;
    }
}

/// A value passed into a binding: a value, an "unset"/"do nothing" marker, or
/// an error with an optional fallback value.
#[derive(Clone, Debug, PartialEq)]
pub struct BindingValue<T> {
    type_: BindingValueType,
    value: Option<T>,
    error: Option<BindingError>,
}

impl<T> BindingValue<T> {
    /// Creates a binding value holding a simple value.
    #[inline]
    pub fn new(value: T) -> Self {
        Self { type_: BindingValueType::VALUE, value: Some(value), error: None }
    }

    /// A binding value representing the unset state.
    #[inline]
    pub fn unset() -> Self {
        Self { type_: BindingValueType::UNSET_VALUE, value: None, error: None }
    }

    /// A binding value that is ignored by its target.
    #[inline]
    pub fn do_nothing() -> Self {
        Self { type_: BindingValueType::DO_NOTHING, value: None, error: None }
    }

    /// A binding error without a fallback value.
    pub fn binding_error(error: BindingError) -> Self {
        Self { type_: BindingValueType::BINDING_ERROR, value: None, error: Some(error) }
    }

    /// A binding error with an optional fallback value.
    pub fn binding_error_with_fallback(error: BindingError, fallback: Option<T>) -> Self {
        let type_ = if fallback.is_some() {
            BindingValueType::BINDING_ERROR_WITH_FALLBACK
        } else {
            BindingValueType::BINDING_ERROR
        };
        Self { type_, value: fallback, error: Some(error) }
    }

    /// A data validation error without a fallback value.
    pub fn data_validation_error(error: BindingError) -> Self {
        Self { type_: BindingValueType::DATA_VALIDATION_ERROR, value: None, error: Some(error) }
    }

    /// A data validation error with an optional fallback value.
    pub fn data_validation_error_with_fallback(error: BindingError, fallback: Option<T>) -> Self {
        let type_ = if fallback.is_some() {
            BindingValueType::DATA_VALIDATION_ERROR_WITH_FALLBACK
        } else {
            BindingValueType::DATA_VALIDATION_ERROR
        };
        Self { type_, value: fallback, error: Some(error) }
    }

    /// Whether the binding value carries a value (or fallback value).
    #[inline]
    pub fn has_value(&self) -> bool {
        self.value.is_some()
    }

    /// Whether the binding value carries an error.
    #[inline]
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    #[inline]
    pub fn value_type(&self) -> BindingValueType {
        self.type_
    }

    /// The value. Panics if there is none, see [`has_value`](Self::has_value).
    #[inline]
    pub fn value(&self) -> &T {
        self.value.as_ref().expect("BindingValue has no value")
    }

    #[inline]
    pub fn error(&self) -> Option<&BindingError> {
        self.error.as_ref()
    }

    /// Converts into the contained value, if any.
    #[inline]
    pub fn into_option(self) -> Option<T> {
        self.value
    }

    /// Returns the value, or `default` if there is none.
    #[inline]
    pub fn get_value_or_default(self, default: T) -> T {
        self.value.unwrap_or(default)
    }

    /// Returns a binding value with the same error state and a new value. For
    /// error states this turns them into their "with fallback" variants.
    pub fn with_value(self, value: T) -> Self {
        let type_ = match self.type_ {
            BindingValueType::DO_NOTHING => panic!("Cannot add value to DoNothing binding value."),
            BindingValueType::BINDING_ERROR => BindingValueType::BINDING_ERROR_WITH_FALLBACK,
            BindingValueType::DATA_VALIDATION_ERROR => {
                BindingValueType::DATA_VALIDATION_ERROR_WITH_FALLBACK
            }
            BindingValueType::UNSET_VALUE => BindingValueType::VALUE,
            other => other,
        };
        Self { type_, value: Some(value), error: self.error }
    }

    /// Maps the contained value, keeping the state.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> BindingValue<U> {
        BindingValue { type_: self.type_, value: self.value.map(f), error: self.error }
    }
}

impl<T: Clone> BindingValue<T> {
    /// Converts the binding value to an optional value: the value (or fallback value), if
    /// there is one.
    pub fn to_optional(&self) -> Option<T> {
        self.value.clone()
    }

    /// Gets the value of the binding value if present, otherwise the default value: none.
    pub fn get_value_or_default_option(&self) -> Option<T> {
        self.value.clone()
    }
}

impl<T: Clone + 'static> BindingValue<T> {
    /// Gets the value of the binding value if it is present and of type `TResult`, otherwise
    /// the default value: none.
    pub fn get_value_or_default_as<TResult: Clone + 'static>(&self) -> Option<TResult> {
        self.value.as_ref().and_then(|value| (value as &dyn Any).downcast_ref::<TResult>().cloned())
    }

    /// Gets the value of the binding value if it is present and of type `TResult`; none if it
    /// is present and of another type; `default_value` if there is no value.
    pub fn get_value_or_default_as_or<TResult: Clone + 'static>(&self, default_value: TResult) -> Option<TResult> {
        match &self.value {
            Some(value) => (value as &dyn Any).downcast_ref::<TResult>().cloned(),
            None => Some(default_value),
        }
    }
}

impl<T: PropertyValue> BindingValue<T> {
    /// A value as an untyped value: null for the null of a nullable type, the contained value
    /// for a value of one (the boxing of the managed original).
    fn box_value(value: &T) -> Option<BoxedValue> {
        ValueTypes::normalize(Rc::new(value.clone()))
    }

    /// Converts the value to untyped representation, using the unset and do-nothing markers
    /// and [`BindingNotification`] where appropriate.
    ///
    /// # Panics
    /// Panics if the type of the binding value is none of the kinds [`BindingValueType`]
    /// declares.
    pub fn to_untyped(&self) -> Option<BoxedValue> {
        let error = || self.error.clone().expect("a binding value with an error state has an error");
        if self.type_ == BindingValueType::UNSET_VALUE {
            Some(FerroProperty::unset_value())
        } else if self.type_ == BindingValueType::DO_NOTHING {
            Some(BindingOperations::do_nothing())
        } else if self.type_ == BindingValueType::VALUE {
            Self::box_value(self.value())
        } else if self.type_ == BindingValueType::BINDING_ERROR {
            Some(Rc::new(BindingNotification::with_error(error(), BindingErrorType::Error)))
        } else if self.type_ == BindingValueType::BINDING_ERROR_WITH_FALLBACK {
            Some(Rc::new(BindingNotification::with_error_and_fallback(
                error(),
                BindingErrorType::Error,
                Self::box_value(self.value()),
            )))
        } else if self.type_ == BindingValueType::DATA_VALIDATION_ERROR {
            Some(Rc::new(BindingNotification::with_error(error(), BindingErrorType::DataValidationError)))
        } else if self.type_ == BindingValueType::DATA_VALIDATION_ERROR_WITH_FALLBACK {
            Some(Rc::new(BindingNotification::with_error_and_fallback(
                error(),
                BindingErrorType::DataValidationError,
                Self::box_value(self.value()),
            )))
        } else {
            panic!("Invalid BindingValueType.")
        }
    }

    /// Creates a binding value from an untyped value: the unset or do-nothing marker, a
    /// [`BindingNotification`], or a value that converts implicitly to `T`.
    pub fn from_untyped(value: Option<&BoxedValue>) -> Self {
        Self::from_untyped_with_type(value, ValueType::of::<T>())
    }

    /// Creates a binding value from an untyped value, converting the value to `target_type`.
    ///
    /// A value that does not convert to `target_type`, or whose conversion is not a `T`,
    /// gives a binding error. Null converts only to a target type that has a null value of
    /// its own (a nullable form): `T` itself is never null here.
    // Deviation (DEVIATIONS.md, Bindings): upstream converts null to a `T` that is a reference
    // type; here null converts only to a nullable form with a registered null value.
    pub fn from_untyped_with_type(value: Option<&BoxedValue>, target_type: ValueType) -> Self {
        if value.is_some_and(|value| value.is::<UnsetValueType>()) {
            return Self::unset();
        } else if value.is_some_and(|value| value.is::<DoNothingType>()) {
            return Self::do_nothing();
        }

        let mut type_ = BindingValueType::VALUE;
        let mut v: Option<T> = None;
        let mut error: Option<BindingError> = None;
        let mut errors: Option<Vec<BindingError>> = None;
        let mut value: Option<BoxedValue> = value.cloned();

        let notification = value.as_ref().and_then(|value| {
            value.downcast_ref::<BindingNotification>().map(|n| (n.error(), n.error_type(), n.has_value(), n.value()))
        });
        if let Some((notification_error, error_type, has_value, notification_value)) = notification {
            error = notification_error;
            type_ = match error_type {
                BindingErrorType::Error => BindingValueType::BINDING_ERROR,
                BindingErrorType::DataValidationError => BindingValueType::DATA_VALIDATION_ERROR,
                BindingErrorType::None => BindingValueType::VALUE,
            };

            if has_value {
                type_ |= BindingValueType::HAS_VALUE;
            }
            value = notification_value;
        }

        if type_.contains(BindingValueType::HAS_VALUE) {
            let typed = ValueTypes::try_convert(value.as_ref(), target_type)
                .flatten()
                .and_then(|typed| typed.downcast_ref::<T>().cloned());
            match typed {
                Some(typed) => v = Some(typed),
                None => {
                    let e = BindingError::message(format!(
                        "Unable to convert object '{}' of type '{}' to type '{}'.",
                        ValueTypes::to_display_string(value.as_ref()),
                        value
                            .as_ref()
                            .map(|value| ValueTypes::type_full_name(ValueType::of_value(&**value)))
                            .unwrap_or_default(),
                        ValueTypes::type_full_name(target_type)
                    ));

                    match error.take() {
                        None => error = Some(e),
                        Some(first) => errors.get_or_insert_with(|| vec![first]).push(e),
                    }

                    type_ = BindingValueType::BINDING_ERROR;
                }
            }
        }

        if let Some(errors) = errors {
            error = Some(BindingError::new(AggregateError::new(errors)));
        }

        Self { type_, value: v, error }
    }
}

/// The text of the binding value: `Error: <message>` for an error, the text of the value, or
/// `(null)` without a value.
// Deviation (DEVIATIONS.md, Bindings): without a value upstream prints the default of `T`
// (`0` for a number); there is no default here, so the text is `(null)`.
impl<T: fmt::Display> fmt::Display for BindingValue<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.error, &self.value) {
            (Some(error), _) => write!(f, "Error: {error}"),
            (None, Some(value)) => fmt::Display::fmt(value, f),
            (None, None) => f.write_str("(null)"),
        }
    }
}

impl<T> From<T> for BindingValue<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> From<Option<T>> for BindingValue<T> {
    /// `None` maps to the unset state.
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => Self::new(v),
            None => Self::unset(),
        }
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream project has no tests of this type.
    use super::*;

    fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue {
        Rc::new(value)
    }

    fn notification(value: &Option<BoxedValue>) -> &BindingNotification {
        value.as_ref().and_then(|value| value.downcast_ref::<BindingNotification>()).expect("a binding notification")
    }

    #[test]
    fn to_untyped_gives_the_value_or_the_marker_of_the_state() {
        let value = BindingValue::new(5).to_untyped();
        assert_eq!(Some(&5), value.as_ref().and_then(|value| value.downcast_ref::<i32>()));
        assert!(BindingValue::<i32>::unset().to_untyped().is_some_and(|value| value.is::<UnsetValueType>()));
        assert!(BindingValue::<i32>::do_nothing().to_untyped().is_some_and(|value| value.is::<DoNothingType>()));
    }

    #[test]
    fn to_untyped_gives_a_notification_for_an_error() {
        let untyped = BindingValue::<i32>::binding_error(BindingError::message("broken")).to_untyped();
        let n = notification(&untyped);
        assert_eq!(BindingErrorType::Error, n.error_type());
        assert!(!n.has_value());
        assert_eq!(Some(String::from("broken")), n.error().map(|error| error.to_string()));

        let untyped =
            BindingValue::data_validation_error_with_fallback(BindingError::message("invalid"), Some(7)).to_untyped();
        let n = notification(&untyped);
        assert_eq!(BindingErrorType::DataValidationError, n.error_type());
        assert!(n.has_value());
        assert_eq!(Some(7), n.value().and_then(|value| value.downcast_ref::<i32>().copied()));
    }

    #[test]
    fn from_untyped_reads_values_and_markers() {
        assert_eq!(BindingValue::new(5), BindingValue::<i32>::from_untyped(Some(&boxed(5))));
        assert_eq!(BindingValue::<i32>::unset(), BindingValue::from_untyped(Some(&FerroProperty::unset_value())));
        assert_eq!(BindingValue::<i32>::do_nothing(), BindingValue::from_untyped(Some(&BindingOperations::do_nothing())));
    }

    #[test]
    fn from_untyped_reads_a_notification() {
        let error = BindingError::message("invalid");
        let n: BoxedValue = Rc::new(BindingNotification::with_error_and_fallback(
            error.clone(),
            BindingErrorType::DataValidationError,
            Some(boxed(7)),
        ));
        let value = BindingValue::<i32>::from_untyped(Some(&n));
        assert_eq!(BindingValueType::DATA_VALIDATION_ERROR_WITH_FALLBACK, value.value_type());
        assert_eq!(7, *value.value());
        assert_eq!(Some(&error), value.error());

        let n: BoxedValue = Rc::new(BindingNotification::with_error(error.clone(), BindingErrorType::Error));
        let value = BindingValue::<i32>::from_untyped(Some(&n));
        assert_eq!(BindingValueType::BINDING_ERROR, value.value_type());
        assert!(!value.has_value());
        assert_eq!(Some(&error), value.error());
    }

    #[test]
    fn from_untyped_gives_a_binding_error_for_a_value_of_another_type() {
        #[derive(Clone, PartialEq)]
        struct Unrelated;

        let value = BindingValue::<i32>::from_untyped(Some(&boxed(Unrelated)));
        assert_eq!(BindingValueType::BINDING_ERROR, value.value_type());
        assert!(!value.has_value());
        let message = value.error().map(|error| error.to_string()).unwrap_or_default();
        assert!(message.starts_with("Unable to convert object '"), "{message}");

        // The error of a notification is kept next to the conversion error.
        let n: BoxedValue = Rc::new(BindingNotification::with_error_and_fallback(
            BindingError::message("invalid"),
            BindingErrorType::DataValidationError,
            Some(boxed(Unrelated)),
        ));
        let value = BindingValue::<i32>::from_untyped(Some(&n));
        assert_eq!(BindingValueType::BINDING_ERROR, value.value_type());
        let count = value
            .error()
            .and_then(|error| error.inner().downcast_ref::<AggregateError>())
            .map(|error| error.inner_errors().len());
        assert_eq!(Some(2), count);
    }

    #[test]
    fn to_string_formats_the_error_the_value_or_null() {
        assert_eq!("5", BindingValue::new(5).to_string());
        assert_eq!("(null)", BindingValue::<i32>::unset().to_string());
        assert_eq!("Error: broken", BindingValue::<i32>::binding_error(BindingError::message("broken")).to_string());
        assert_eq!(
            "Error: broken",
            BindingValue::binding_error_with_fallback(BindingError::message("broken"), Some(1)).to_string()
        );
    }

    #[test]
    fn the_value_or_a_default_can_be_read() {
        assert_eq!(Some(5), BindingValue::new(5).to_optional());
        assert_eq!(None, BindingValue::<i32>::unset().to_optional());
        assert_eq!(Some(5), BindingValue::new(5).get_value_or_default_option());
        assert_eq!(None, BindingValue::<i32>::do_nothing().get_value_or_default_option());
        assert_eq!(9, BindingValue::<i32>::unset().get_value_or_default(9));

        assert_eq!(Some(5), BindingValue::new(5).get_value_or_default_as::<i32>());
        assert_eq!(None, BindingValue::new(5).get_value_or_default_as::<String>());
        assert_eq!(None, BindingValue::<i32>::unset().get_value_or_default_as::<i32>());
        assert_eq!(Some(5), BindingValue::new(5).get_value_or_default_as_or(9));
        assert_eq!(None, BindingValue::new(5).get_value_or_default_as_or(String::from("x")));
        assert_eq!(Some(9), BindingValue::<i32>::unset().get_value_or_default_as_or(9));
    }
}
