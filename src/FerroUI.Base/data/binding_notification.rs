use crate::data::core::ValueTypes;
use crate::data::{BindingError, BindingOperations, BindingValueType};
use crate::{BoxedValue, DoNothingType, FerroProperty, UnsetValueType};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// Defines the types of binding errors for a [`BindingNotification`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum BindingErrorType {
    /// There was no error.
    None = 0,
    /// There was a binding error.
    Error = 1,
    /// There was a data validation error.
    DataValidationError = 2,
}

impl BindingErrorType {
    /// The binding value type that represents this error type.
    pub fn to_binding_value_type(self) -> BindingValueType {
        match self {
            BindingErrorType::Error => BindingValueType::BINDING_ERROR,
            BindingErrorType::DataValidationError => BindingValueType::DATA_VALIDATION_ERROR,
            BindingErrorType::None => BindingValueType::VALUE,
        }
    }
}

/// Several binding errors that occurred on the same binding notification.
#[derive(Clone, Debug)]
pub struct AggregateError {
    inner_errors: Vec<BindingError>,
}

impl AggregateError {
    /// Creates an error that aggregates `inner_errors`.
    pub fn new(inner_errors: Vec<BindingError>) -> Self {
        Self { inner_errors }
    }

    /// The aggregated errors.
    pub fn inner_errors(&self) -> &[BindingError] {
        &self.inner_errors
    }
}

impl fmt::Display for AggregateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("One or more errors occurred.")?;
        for error in &self.inner_errors {
            write!(f, " ({error})")?;
        }
        Ok(())
    }
}

impl std::error::Error for AggregateError {}

/// Represents a binding notification that can be a valid binding value, or a
/// binding or data validation error.
///
/// This type is very similar to [`BindingValue`](crate::data::BindingValue),
/// but where `BindingValue` is used by typed bindings, this type is used to
/// hold binding and data validation errors in untyped bindings.
///
/// A notification has reference semantics: it is shared as
/// `Rc<BindingNotification>` (which converts to a [`BoxedValue`] holding the
/// notification) and is modified in place.
pub struct BindingNotification {
    value: RefCell<Option<BoxedValue>>,
    error: RefCell<Option<BindingError>>,
    error_type: Cell<BindingErrorType>,
}

impl BindingNotification {
    /// A binding notification representing the null value.
    pub fn null() -> Rc<BindingNotification> {
        thread_local! {
            static NULL: Rc<BindingNotification> = Rc::new(BindingNotification::new(None));
        }
        NULL.with(Rc::clone)
    }

    /// A binding notification representing the unset value marker.
    pub fn unset_value() -> Rc<BindingNotification> {
        thread_local! {
            static UNSET_VALUE: Rc<BindingNotification> =
                Rc::new(BindingNotification::new(Some(FerroProperty::unset_value())));
        }
        UNSET_VALUE.with(Rc::clone)
    }

    /// Creates a binding notification for a binding value.
    pub fn new(value: Option<BoxedValue>) -> Self {
        Self { value: RefCell::new(value), error: RefCell::new(None), error_type: Cell::new(BindingErrorType::None) }
    }

    /// Creates a binding notification for a binding error.
    ///
    /// Panics if `error_type` is [`BindingErrorType::None`].
    pub fn with_error(error: BindingError, error_type: BindingErrorType) -> Self {
        Self::with_error_and_fallback(error, error_type, Some(FerroProperty::unset_value()))
    }

    /// Creates a binding notification for a binding error with a fallback
    /// value.
    ///
    /// Panics if `error_type` is [`BindingErrorType::None`].
    pub fn with_error_and_fallback(
        error: BindingError,
        error_type: BindingErrorType,
        fallback_value: Option<BoxedValue>,
    ) -> Self {
        assert!(error_type != BindingErrorType::None, "'error_type' may not be None");
        Self {
            value: RefCell::new(fallback_value),
            error: RefCell::new(Some(error)),
            error_type: Cell::new(error_type),
        }
    }

    /// Gets the value that should be passed to the target when
    /// [`has_value`](Self::has_value) is true.
    ///
    /// If this is read when `has_value` is false then it returns the unset
    /// value marker.
    pub fn value(&self) -> Option<BoxedValue> {
        self.value.borrow().clone()
    }

    /// Gets a value indicating whether [`value`](Self::value) should be
    /// pushed to the target.
    pub fn has_value(&self) -> bool {
        !BindingOperations::is_unset(self.value.borrow().as_ref())
    }

    /// Gets the error that occurred on the source, if any.
    pub fn error(&self) -> Option<BindingError> {
        self.error.borrow().clone()
    }

    /// Sets the error that occurred on the source.
    pub fn set_error(&self, error: Option<BindingError>) {
        *self.error.borrow_mut() = error;
    }

    /// Gets the type of error that [`error`](Self::error) represents, if any.
    pub fn error_type(&self) -> BindingErrorType {
        self.error_type.get()
    }

    /// Sets the type of error that [`error`](Self::error) represents.
    pub fn set_error_type(&self, error_type: BindingErrorType) {
        self.error_type.set(error_type);
    }

    /// Gets a value from an untyped value that may be a binding notification.
    ///
    /// If `o` is a binding notification then returns the binding
    /// notification's [`value`](Self::value). If not, returns the value
    /// unchanged.
    pub fn extract_value(o: Option<&BoxedValue>) -> Option<BoxedValue> {
        match o.and_then(|o| o.downcast_ref::<BindingNotification>()) {
            Some(notification) => notification.value(),
            None => o.cloned(),
        }
    }

    /// Updates the value of an untyped value that may be a binding
    /// notification.
    ///
    /// If `o` is a binding notification then sets its value to `value` and
    /// returns the notification; otherwise returns `value`. If `value` is a
    /// binding notification then its value is extracted first.
    pub fn update_value(o: Option<&BoxedValue>, value: Option<&BoxedValue>) -> Option<BoxedValue> {
        if let Some(o) = o {
            if let Some(notification) = o.downcast_ref::<BindingNotification>() {
                notification.set_value(Self::extract_value(value));
                return Some(o.clone());
            }
        }
        value.cloned()
    }

    /// Gets an error from an untyped value that may be a binding
    /// notification.
    ///
    /// If `o` is a binding notification then returns the binding
    /// notification's [`error`](Self::error) (boxed as a [`BindingError`]).
    /// If not, returns the value unchanged.
    pub fn extract_error(o: Option<&BoxedValue>) -> Option<BoxedValue> {
        match o.and_then(|o| o.downcast_ref::<BindingNotification>()) {
            Some(notification) => notification.error().map(|error| Rc::new(error) as BoxedValue),
            None => o.cloned(),
        }
    }

    /// Converts the notification to a typed binding value (the notification
    /// part of the reference's untyped-to-typed binding value conversion).
    ///
    /// The notification's value must hold exactly `T`; a value that cannot be
    /// converted produces a binding error, aggregated with the notification's
    /// own error if it has one. `target_type` names `T` in that error.
    pub(crate) fn to_binding_value<T: Clone + 'static>(&self, target_type: &str) -> crate::data::BindingValue<T> {
        use crate::data::BindingValue;

        let error = self.error();
        let error_type = self.error_type();

        // As in the reference, a notification without an error always counts
        // as carrying a value, even when its value is the unset marker.
        if !self.has_value() && error_type != BindingErrorType::None {
            let error = error.unwrap_or_else(|| BindingError::message("Binding notification has no error."));
            return match error_type {
                BindingErrorType::DataValidationError => BindingValue::data_validation_error(error),
                _ => BindingValue::binding_error(error),
            };
        }

        let value = self.value();
        let typed = value.as_ref().and_then(|value| value.downcast_ref::<T>().cloned());

        match typed {
            Some(typed) => match (error_type, error) {
                (BindingErrorType::Error, Some(error)) => BindingValue::binding_error_with_fallback(error, Some(typed)),
                (BindingErrorType::DataValidationError, Some(error)) => {
                    BindingValue::data_validation_error_with_fallback(error, Some(typed))
                }
                _ => BindingValue::new(typed),
            },
            None => {
                let e = BindingError::message(format!(
                    "Unable to convert object '{}' to type '{target_type}'.",
                    if value.is_some() { "(object)" } else { "(null)" }
                ));
                let error = match error {
                    Some(error) => BindingError::new(AggregateError::new(vec![error, e])),
                    None => e,
                };
                BindingValue::binding_error(error)
            }
        }
    }

    /// Adds an error to the binding notification.
    ///
    /// Panics if `error_type` is [`BindingErrorType::None`].
    pub fn add_error(&self, e: BindingError, error_type: BindingErrorType) {
        assert!(error_type != BindingErrorType::None, "BindingErrorType may not be None");

        let current = self.error.borrow_mut().take();
        let error = match current {
            Some(current) => BindingError::new(AggregateError::new(vec![current, e])),
            None => e,
        };
        *self.error.borrow_mut() = Some(error);

        if error_type == BindingErrorType::Error || self.error_type.get() == BindingErrorType::Error {
            self.error_type.set(BindingErrorType::Error);
        }
    }

    /// Removes the [`value`](Self::value) and makes
    /// [`has_value`](Self::has_value) return false.
    pub fn clear_value(&self) {
        *self.value.borrow_mut() = Some(FerroProperty::unset_value());
    }

    /// Sets the [`value`](Self::value).
    pub fn set_value(&self, value: Option<BoxedValue>) {
        *self.value.borrow_mut() = value;
    }

    /// Errors are equal if they are the same error, or have the same kind and
    /// message.
    fn error_equals(a: Option<&BindingError>, b: Option<&BindingError>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => a == b || (format!("{a:?}") == format!("{b:?}") && a.to_string() == b.to_string()),
            _ => false,
        }
    }
}

/// The text of an untyped value: empty for null.
fn value_text(value: Option<&BoxedValue>) -> String {
    let Some(value) = value else {
        return String::new();
    };
    if let Some(marker) = value.downcast_ref::<UnsetValueType>() {
        return marker.to_string();
    }
    if let Some(marker) = value.downcast_ref::<DoNothingType>() {
        return marker.to_string();
    }
    if let Some(notification) = value.downcast_ref::<BindingNotification>() {
        return notification.to_string();
    }
    ValueTypes::to_display_string(Some(value))
}

impl PartialEq for BindingNotification {
    /// Compares two binding notifications for equality.
    fn eq(&self, other: &Self) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }

        let a_value = self.value();
        let b_value = other.value();
        let a_has_value = self.has_value();
        a_has_value == other.has_value()
            && self.error_type() == other.error_type()
            && (!a_has_value || ValueTypes::identity_equals(a_value.as_ref(), b_value.as_ref()))
            && (self.error_type() == BindingErrorType::None
                || Self::error_equals(self.error().as_ref(), other.error().as_ref()))
    }
}

impl fmt::Display for BindingNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = self.value();
        let value = value_text(value.as_ref());
        let error = self.error().map(|e| e.to_string()).unwrap_or_default();
        match self.error_type() {
            BindingErrorType::None => write!(f, "{{Value: {value}}}"),
            error_type if self.has_value() => write!(f, "{{{error_type:?}: {error}, Fallback: {value}}}"),
            error_type => write!(f, "{{{error_type:?}: {error}}}"),
        }
    }
}

impl fmt::Debug for BindingNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    //! Upstream has no dedicated tests for this type; these cover the port.

    use super::*;

    fn boxed<T: PartialEq + 'static>(value: T) -> Option<BoxedValue> {
        Some(Rc::new(value))
    }

    fn error(message: &str) -> BindingError {
        BindingError::message(message)
    }

    #[test]
    fn null_and_unset_value_statics() {
        assert!(BindingNotification::null().has_value());
        assert!(BindingNotification::null().value().is_none());
        assert_eq!(BindingNotification::null().error_type(), BindingErrorType::None);
        assert!(Rc::ptr_eq(&BindingNotification::null(), &BindingNotification::null()));

        assert!(!BindingNotification::unset_value().has_value());
        assert!(BindingOperations::is_unset(BindingNotification::unset_value().value().as_ref()));
        assert!(Rc::ptr_eq(&BindingNotification::unset_value(), &BindingNotification::unset_value()));
    }

    #[test]
    fn value_notification_has_value_and_no_error() {
        let target = BindingNotification::new(boxed(42i32));

        assert!(target.has_value());
        assert_eq!(target.value().and_then(|v| v.downcast_ref::<i32>().copied()), Some(42));
        assert!(target.error().is_none());
        assert_eq!(target.error_type(), BindingErrorType::None);
    }

    #[test]
    fn error_notification_has_no_value() {
        let e = error("foo");
        let target = BindingNotification::with_error(e.clone(), BindingErrorType::Error);

        assert!(!target.has_value());
        assert!(BindingOperations::is_unset(target.value().as_ref()));
        assert_eq!(target.error(), Some(e));
        assert_eq!(target.error_type(), BindingErrorType::Error);
    }

    #[test]
    fn error_notification_with_fallback_has_value() {
        let target =
            BindingNotification::with_error_and_fallback(error("foo"), BindingErrorType::DataValidationError, boxed(1i32));

        assert!(target.has_value());
        assert_eq!(target.error_type(), BindingErrorType::DataValidationError);

        let null_fallback =
            BindingNotification::with_error_and_fallback(error("foo"), BindingErrorType::Error, None);
        assert!(null_fallback.has_value());
        assert!(null_fallback.value().is_none());
    }

    #[test]
    #[should_panic(expected = "may not be None")]
    fn error_notification_rejects_error_type_none() {
        let _ = BindingNotification::with_error(error("foo"), BindingErrorType::None);
    }

    #[test]
    #[should_panic(expected = "may not be None")]
    fn add_error_rejects_error_type_none() {
        BindingNotification::new(None).add_error(error("foo"), BindingErrorType::None);
    }

    #[test]
    fn equality_compares_values() {
        assert_eq!(BindingNotification::new(boxed(1i32)), BindingNotification::new(boxed(1i32)));
        assert_ne!(BindingNotification::new(boxed(1i32)), BindingNotification::new(boxed(2i32)));
        assert_ne!(BindingNotification::new(boxed(1i32)), BindingNotification::new(boxed(1i64)));
        assert_ne!(BindingNotification::new(boxed(1i32)), BindingNotification::new(None));
        assert_eq!(BindingNotification::new(None), BindingNotification::new(None));
        assert_eq!(*BindingNotification::null(), BindingNotification::new(None));
        assert_ne!(*BindingNotification::null(), *BindingNotification::unset_value());
        assert_eq!(*BindingNotification::unset_value(), BindingNotification::new(Some(FerroProperty::unset_value())));
    }

    #[test]
    fn equality_compares_errors_by_kind_and_message() {
        let a = BindingNotification::with_error(error("foo"), BindingErrorType::Error);
        let b = BindingNotification::with_error(error("foo"), BindingErrorType::Error);
        let other_message = BindingNotification::with_error(error("bar"), BindingErrorType::Error);
        let other_type = BindingNotification::with_error(error("foo"), BindingErrorType::DataValidationError);
        let other_kind =
            BindingNotification::with_error(BindingError::new(AggregateError::new(Vec::new())), BindingErrorType::Error);
        let with_fallback =
            BindingNotification::with_error_and_fallback(error("foo"), BindingErrorType::Error, boxed(1i32));

        assert_eq!(a, b);
        assert_ne!(a, other_message);
        assert_ne!(a, other_type);
        assert_ne!(a, other_kind);
        assert_ne!(a, with_fallback);
    }

    #[test]
    fn equality_ignores_error_when_error_type_is_none() {
        let a = BindingNotification::new(boxed(1i32));
        let b = BindingNotification::new(boxed(1i32));
        b.set_error(Some(error("ignored")));

        assert_eq!(a, b);
    }

    #[test]
    fn can_be_carried_and_compared_as_untyped_value() {
        let a: BoxedValue = Rc::new(BindingNotification::new(boxed(1i32)));
        let b: BoxedValue = Rc::new(BindingNotification::new(boxed(1i32)));
        let c: BoxedValue = Rc::new(1i32);

        assert!(ValueTypes::identity_equals(Some(&a), Some(&b)));
        assert!(!ValueTypes::identity_equals(Some(&a), Some(&c)));
    }

    #[test]
    fn extract_value_returns_value_of_notification_or_the_value_itself() {
        let notification: BoxedValue = Rc::new(BindingNotification::new(boxed(5i32)));
        let plain: BoxedValue = Rc::new(6i32);

        let extracted = BindingNotification::extract_value(Some(&notification));
        assert_eq!(extracted.and_then(|v| v.downcast_ref::<i32>().copied()), Some(5));
        let extracted = BindingNotification::extract_value(Some(&plain));
        assert!(extracted.is_some_and(|v| Rc::ptr_eq(&v, &plain)));
        assert!(BindingNotification::extract_value(None).is_none());

        let null: BoxedValue = BindingNotification::null();
        assert!(BindingNotification::extract_value(Some(&null)).is_none());
    }

    #[test]
    fn extract_error_returns_error_of_notification_or_the_value_itself() {
        let e = error("foo");
        let notification: BoxedValue = Rc::new(BindingNotification::with_error(e.clone(), BindingErrorType::Error));
        let plain: BoxedValue = Rc::new(6i32);

        let extracted = BindingNotification::extract_error(Some(&notification));
        assert_eq!(extracted.and_then(|v| v.downcast_ref::<BindingError>().cloned()), Some(e));
        let extracted = BindingNotification::extract_error(Some(&plain));
        assert!(extracted.is_some_and(|v| Rc::ptr_eq(&v, &plain)));
        assert!(BindingNotification::extract_error(None).is_none());

        let no_error: BoxedValue = Rc::new(BindingNotification::new(boxed(1i32)));
        assert!(BindingNotification::extract_error(Some(&no_error)).is_none());
    }

    #[test]
    fn update_value_updates_notification_in_place() {
        let notification: BoxedValue = Rc::new(BindingNotification::with_error(error("foo"), BindingErrorType::Error));
        let new_value: BoxedValue = Rc::new(BindingNotification::new(boxed(7i32)));

        let result = BindingNotification::update_value(Some(&notification), Some(&new_value)).expect("a value");

        assert!(Rc::ptr_eq(&result, &notification));
        let updated = notification.downcast_ref::<BindingNotification>().expect("a notification");
        assert!(updated.has_value());
        assert_eq!(updated.value().and_then(|v| v.downcast_ref::<i32>().copied()), Some(7));
        assert_eq!(updated.error_type(), BindingErrorType::Error);
    }

    #[test]
    fn update_value_returns_value_when_not_a_notification() {
        let plain: BoxedValue = Rc::new(6i32);
        let value: BoxedValue = Rc::new(7i32);

        let result = BindingNotification::update_value(Some(&plain), Some(&value)).expect("a value");
        assert!(Rc::ptr_eq(&result, &value));
        let result = BindingNotification::update_value(None, Some(&value)).expect("a value");
        assert!(Rc::ptr_eq(&result, &value));
    }

    #[test]
    fn add_error_sets_error() {
        let target = BindingNotification::new(boxed(1i32));
        let e = error("foo");

        target.add_error(e.clone(), BindingErrorType::Error);

        assert_eq!(target.error(), Some(e));
        assert_eq!(target.error_type(), BindingErrorType::Error);
        assert!(target.has_value());
    }

    #[test]
    fn add_error_aggregates_errors() {
        let target = BindingNotification::with_error(error("first"), BindingErrorType::DataValidationError);

        target.add_error(error("second"), BindingErrorType::Error);

        assert_eq!(target.error_type(), BindingErrorType::Error);
        let e = target.error().expect("an error");
        assert_eq!(e.to_string(), "One or more errors occurred. (first) (second)");
        let aggregate = e.inner().downcast_ref::<AggregateError>().expect("an aggregate error");
        assert_eq!(aggregate.inner_errors().len(), 2);
    }

    #[test]
    fn add_error_keeps_error_type_unless_either_is_error() {
        let target = BindingNotification::with_error(error("first"), BindingErrorType::Error);
        target.add_error(error("second"), BindingErrorType::DataValidationError);
        assert_eq!(target.error_type(), BindingErrorType::Error);

        let target = BindingNotification::with_error(error("first"), BindingErrorType::DataValidationError);
        target.add_error(error("second"), BindingErrorType::DataValidationError);
        assert_eq!(target.error_type(), BindingErrorType::DataValidationError);

        // As upstream: a data validation error added to a notification
        // without an error leaves the error type unchanged.
        let target = BindingNotification::new(boxed(1i32));
        target.add_error(error("first"), BindingErrorType::DataValidationError);
        assert_eq!(target.error_type(), BindingErrorType::None);
        assert!(target.error().is_some());
    }

    #[test]
    fn clear_value_and_set_value() {
        let target = BindingNotification::new(boxed(1i32));

        target.clear_value();
        assert!(!target.has_value());

        target.set_value(None);
        assert!(target.has_value());
        assert!(target.value().is_none());
    }

    #[test]
    fn to_string_formats_value_and_error() {
        assert_eq!(BindingNotification::new(boxed(42i32)).to_string(), "{Value: 42}");
        assert_eq!(BindingNotification::new(None).to_string(), "{Value: }");
        assert_eq!(BindingNotification::unset_value().to_string(), "{Value: (unset)}");
        assert_eq!(
            BindingNotification::with_error(error("foo"), BindingErrorType::Error).to_string(),
            "{Error: foo}"
        );
        assert_eq!(
            BindingNotification::with_error_and_fallback(
                error("foo"),
                BindingErrorType::DataValidationError,
                boxed(String::from("fallback"))
            )
            .to_string(),
            "{DataValidationError: foo, Fallback: fallback}"
        );
    }

    #[test]
    fn error_type_converts_to_binding_value_type() {
        assert_eq!(BindingErrorType::Error.to_binding_value_type(), BindingValueType::BINDING_ERROR);
        assert_eq!(
            BindingErrorType::DataValidationError.to_binding_value_type(),
            BindingValueType::DATA_VALIDATION_ERROR
        );
        assert_eq!(BindingErrorType::None.to_binding_value_type(), BindingValueType::VALUE);
    }
}
