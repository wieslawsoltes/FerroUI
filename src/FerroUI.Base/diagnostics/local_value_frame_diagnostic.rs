use super::{IValueFrameDiagnostic, IValueFrameDiagnosticFrameType, ValueEntryDiagnostic};
use crate::data::BindingPriority;
use crate::BoxedValue;

/// The local values of an object, presented as a frame.
pub(crate) struct LocalValueFrameDiagnostic {
    values: Vec<ValueEntryDiagnostic>,
}

impl LocalValueFrameDiagnostic {
    pub fn new(values: Vec<ValueEntryDiagnostic>) -> Self {
        Self { values }
    }
}

impl IValueFrameDiagnostic for LocalValueFrameDiagnostic {
    fn source(&self) -> Option<BoxedValue> {
        None
    }

    fn type_(&self) -> IValueFrameDiagnosticFrameType {
        IValueFrameDiagnosticFrameType::Local
    }

    fn is_active(&self) -> bool {
        true
    }

    fn priority(&self) -> BindingPriority {
        BindingPriority::LocalValue
    }

    fn values(&self) -> Vec<ValueEntryDiagnostic> {
        self.values.clone()
    }
}
