use super::{IValueFrameDiagnostic, IValueFrameDiagnosticFrameType, ValueEntryDiagnostic};
use crate::data::BindingPriority;
use crate::property_store::ValueFrame;
use crate::styling::{ControlTheme, IStyleInstance, Setter, SetterValue, Style, StyleBase, StyleInstance};
use crate::{BoxedValue, Ref};
use std::rc::Rc;

/// The frame of a style or control theme that is applied to an object.
pub(crate) struct StyleValueFrameDiagnostic {
    /// The style instance, as the value frame it is.
    style_instance: Rc<dyn ValueFrame>,
    /// The source of the style instance.
    source: Ref<StyleBase>,
}

impl StyleValueFrameDiagnostic {
    /// The diagnostic of a frame that is a style instance with a source;
    /// `None` for any other frame (the original's test
    /// `frame is StyleInstance { Source: StyleBase }`).
    pub fn new(frame: &Rc<dyn ValueFrame>) -> Option<Self> {
        let style_instance = frame.as_any().downcast_ref::<StyleInstance>()?;
        let source = style_instance.source()?;
        Some(Self { style_instance: frame.clone(), source })
    }

    /// The value of a setter as the object it was set to. (A setter bound to
    /// a plain source of values reads as nothing.)
    fn setter_value(setter: &Setter) -> Option<BoxedValue> {
        match setter.value()? {
            SetterValue::Value(value) => Some(value),
            SetterValue::BindingBase(binding) => Some(Rc::new(binding)),
            SetterValue::Template(template) => Some(Rc::new(template)),
            SetterValue::Binding(_) => None,
        }
    }
}

impl IValueFrameDiagnostic for StyleValueFrameDiagnostic {
    fn source(&self) -> Option<BoxedValue> {
        Some(Rc::new(self.source.clone()))
    }

    fn type_(&self) -> IValueFrameDiagnosticFrameType {
        if self.source.is::<Style>() {
            IValueFrameDiagnosticFrameType::Style
        } else if self.source.is::<ControlTheme>() {
            IValueFrameDiagnosticFrameType::Theme
        } else {
            IValueFrameDiagnosticFrameType::Unknown
        }
    }

    fn is_active(&self) -> bool {
        self.style_instance.is_active()
    }

    fn priority(&self) -> BindingPriority {
        self.style_instance.base().frame_priority().to_binding_priority()
    }

    fn values(&self) -> Vec<ValueEntryDiagnostic> {
        let mut result = Vec::new();
        for setter in self.source.setters().snapshot().iter() {
            let regular_setter = setter.as_any().and_then(|setter| setter.downcast_ref::<Setter>());
            if let Some(regular_setter) = regular_setter {
                if let Some(property) = regular_setter.property() {
                    result.push(ValueEntryDiagnostic::new(property, Self::setter_value(regular_setter)));
                }
            }
        }
        result
    }
}
