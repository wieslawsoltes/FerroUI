use std::borrow::Cow;

/// The argument of the `ErrorsChanged` event of an [`INotifyDataErrorInfo`](super::INotifyDataErrorInfo)
/// (`System.ComponentModel.DataErrorsChangedEventArgs`): the name of the property whose
/// validation errors changed.
///
/// A name that is `None` or empty means that the errors of the whole object changed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DataErrorsChangedEventArgs {
    property_name: Option<Cow<'static, str>>,
}

impl DataErrorsChangedEventArgs {
    /// The argument for the property named `property_name`
    /// (`DataErrorsChangedEventArgs(string?)`).
    pub fn new(property_name: Option<&str>) -> Self {
        Self { property_name: property_name.map(|name| Cow::Owned(name.to_owned())) }
    }

    /// The argument for a property whose name is known at compile time, without an allocation;
    /// for the shared arguments that a type raises again and again.
    pub const fn from_static(property_name: &'static str) -> Self {
        Self { property_name: Some(Cow::Borrowed(property_name)) }
    }

    /// The name of the property whose errors changed, or `None` for the whole object
    /// (`PropertyName`).
    pub fn property_name(&self) -> Option<&str> {
        self.property_name.as_deref()
    }
}
