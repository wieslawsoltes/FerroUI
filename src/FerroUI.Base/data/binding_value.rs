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
