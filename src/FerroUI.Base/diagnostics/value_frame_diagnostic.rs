use super::{IValueFrameDiagnostic, IValueFrameDiagnosticFrameType, ValueEntryDiagnostic};
use crate::data::BindingPriority;
use crate::property_store::ValueFrame;
use crate::BoxedValue;
use std::rc::Rc;

/// A value frame that is not the frame of a style: the values a template
/// gives an object.
pub(crate) struct ValueFrameDiagnostic {
    value_frame: Rc<dyn ValueFrame>,
}

impl ValueFrameDiagnostic {
    pub fn new(value_frame: Rc<dyn ValueFrame>) -> Self {
        Self { value_frame }
    }
}

impl IValueFrameDiagnostic for ValueFrameDiagnostic {
    fn source(&self) -> Option<BoxedValue> {
        let owner = self.value_frame.base().owner()?;
        Some(Rc::new(owner))
    }

    fn type_(&self) -> IValueFrameDiagnosticFrameType {
        IValueFrameDiagnosticFrameType::Template
    }

    fn is_active(&self) -> bool {
        self.value_frame.is_active()
    }

    fn priority(&self) -> BindingPriority {
        self.value_frame.base().frame_priority().to_binding_priority()
    }

    fn values(&self) -> Vec<ValueEntryDiagnostic> {
        let base = self.value_frame.base();
        let mut result = Vec::new();
        for i in 0..base.entry_count() {
            let entry = base.get_entry(i);
            if entry.has_value() {
                result.push(ValueEntryDiagnostic::new(entry.property(), entry.get_value_boxed()));
            }
        }
        result
    }
}
