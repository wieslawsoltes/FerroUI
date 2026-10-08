use super::IValueFrameDiagnostic;
use std::rc::Rc;

/// The frames of values that are applied to an object.
pub struct ValueStoreDiagnostic {
    applied_frames: Vec<Rc<dyn IValueFrameDiagnostic>>,
}

impl ValueStoreDiagnostic {
    pub(crate) fn new(applied_frames: Vec<Rc<dyn IValueFrameDiagnostic>>) -> Self {
        Self { applied_frames }
    }

    /// Currently applied frames.
    pub fn applied_frames(&self) -> &[Rc<dyn IValueFrameDiagnostic>] {
        &self.applied_frames
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of these types in the
    // tests of the base assembly.
    use crate::data::BindingPriority;
    use crate::diagnostics::{FerroObjectDiagnosticExtensions, IValueFrameDiagnosticFrameType};
    use crate::styling::{Style, ThemeVariant};

    #[test]
    fn an_object_without_values_has_no_frames() {
        let target = Style::new();

        assert!(target.get_value_store_diagnostic().applied_frames().is_empty());
    }

    #[test]
    fn local_values_are_the_first_frame() {
        let property = ThemeVariant::actual_theme_variant_property();
        let target = Style::new();
        target.set_value(property, Some(ThemeVariant::light()));

        let diagnostic = target.get_value_store_diagnostic();

        assert_eq!(1, diagnostic.applied_frames().len());
        let frame = &diagnostic.applied_frames()[0];
        assert!(frame.source().is_none());
        assert_eq!(IValueFrameDiagnosticFrameType::Local, frame.type_());
        assert!(frame.is_active());
        assert_eq!(BindingPriority::LocalValue, frame.priority());
        assert_eq!(1, frame.values().len());
        assert!(std::ptr::eq(property.as_property(), frame.values()[0].property));
    }
}
