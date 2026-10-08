use crate::data::core::ValueTypes;
use crate::BoxedValue;
use std::fmt;

/// An error that describes a data validation failure reported by a model
/// object.
#[derive(Clone)]
pub struct DataValidationException {
    error_data: Option<BoxedValue>,
    message: String,
}

impl DataValidationException {
    /// Creates the error from the data received from the validation source.
    pub fn new(error_data: Option<BoxedValue>) -> Self {
        let message = match &error_data {
            Some(v) => ValueTypes::to_display_string(Some(v)),
            None => String::new(),
        };
        Self { error_data, message }
    }

    /// The data of the validation error.
    pub fn error_data(&self) -> Option<&BoxedValue> {
        self.error_data.as_ref()
    }
}

/// Two errors are equal if they carry the same error data, so that the error
/// can be held in an untyped value.
impl PartialEq for DataValidationException {
    fn eq(&self, other: &Self) -> bool {
        ValueTypes::identity_equals(self.error_data.as_ref(), other.error_data.as_ref())
    }
}

impl fmt::Debug for DataValidationException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DataValidationException({})", self.message)
    }
}

impl fmt::Display for DataValidationException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DataValidationException {}

/// Several validation errors reported for one property.
#[derive(Clone, Debug)]
pub struct AggregateException {
    inner: Vec<DataValidationException>,
}

impl AggregateException {
    pub fn new(inner: Vec<DataValidationException>) -> Self {
        Self { inner }
    }

    pub fn inner_exceptions(&self) -> &[DataValidationException] {
        &self.inner
    }
}

impl fmt::Display for AggregateException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("One or more errors occurred.")?;
        for e in &self.inner {
            write!(f, " ({e})")?;
        }
        Ok(())
    }
}

impl std::error::Error for AggregateException {}
