use crate::data::BindingPriority;
use crate::{BoxedValue, FerroProperty};

/// A property and the value a value frame holds for it.
#[derive(Clone)]
pub struct ValueEntryDiagnostic {
    pub property: &'static FerroProperty,
    pub value: Option<BoxedValue>,
}

impl ValueEntryDiagnostic {
    pub fn new(property: &'static FerroProperty, value: Option<BoxedValue>) -> Self {
        Self { property, value }
    }
}

/// The kind of a value frame (the original's nested enumeration
/// `IValueFrameDiagnostic.FrameType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum IValueFrameDiagnosticFrameType {
    Unknown = 0,
    Local,
    Theme,
    Style,
    Template,
}

/// What the developer tools read about one frame of the values of an
/// object. Implemented by the framework only.
pub trait IValueFrameDiagnostic {
    /// The object the values come from: a style or control theme, the
    /// templated parent of a template frame, nothing for local values.
    fn source(&self) -> Option<BoxedValue>;

    fn type_(&self) -> IValueFrameDiagnosticFrameType;

    fn is_active(&self) -> bool;

    fn priority(&self) -> BindingPriority;

    fn values(&self) -> Vec<ValueEntryDiagnostic>;
}
