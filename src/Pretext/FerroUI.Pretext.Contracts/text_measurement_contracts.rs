use std::rc::Rc;

use crate::{IPretextTextShaperFactory, PretextError};

/// Measures an entire string in one fixed font, including contextual advances.
pub trait IPretextTextMeasurer {
    fn measure_text(&self, text: &str) -> f64;
}

/// An explicitly registered, platform-independent measurement backend.
pub trait IPretextTextMeasurerFactory {
    fn name(&self) -> &str;

    fn is_supported(&self) -> bool {
        true
    }

    fn priority(&self) -> i32 {
        0
    }

    fn create(&self, font: &str) -> Result<Rc<dyn IPretextTextMeasurer>, PretextError>;

    /// A combined backend can supply shaping without a second registration.
    fn shaper_factory(&self) -> Option<Rc<dyn IPretextTextShaperFactory>> {
        None
    }
}

/// Adapter for renderers that already expose a string/font measurement function.
pub struct DelegateTextMeasurerFactory<F> {
    measure: Rc<F>,
}

impl<F: Fn(&str, &str) -> f64 + 'static> DelegateTextMeasurerFactory<F> {
    pub fn new(measure: F) -> Self {
        Self {
            measure: Rc::new(measure),
        }
    }
}

struct DelegateTextMeasurer<F> {
    font: String,
    measure: Rc<F>,
}

impl<F: Fn(&str, &str) -> f64 + 'static> IPretextTextMeasurerFactory
    for DelegateTextMeasurerFactory<F>
{
    fn name(&self) -> &str {
        "Delegate"
    }

    fn create(&self, font: &str) -> Result<Rc<dyn IPretextTextMeasurer>, PretextError> {
        Ok(Rc::new(DelegateTextMeasurer {
            font: font.into(),
            measure: self.measure.clone(),
        }))
    }
}

impl<F: Fn(&str, &str) -> f64> IPretextTextMeasurer for DelegateTextMeasurer<F> {
    fn measure_text(&self, text: &str) -> f64 {
        (self.measure)(text, &self.font)
    }
}
