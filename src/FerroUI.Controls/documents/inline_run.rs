use crate::{Control, RichTextBlock, TextBlock};
use ferroui_base::media::text_formatting::{
    DrawableTextRun, ITextDrawingSink, TextRun, TextRunProperties,
};
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{Point, Ref, Size};
use std::any::Any;
use std::rc::Rc;

/// The text run of a control embedded in text: it reserves the desired size
/// of the control in the line and draws nothing.
pub struct EmbeddedControlRun {
    control: Ref<Control>,
    properties: Rc<dyn TextRunProperties>,
    measured_metrics: Option<(Size, f64)>,
}

impl EmbeddedControlRun {
    pub fn new(control: Ref<Control>, properties: Rc<dyn TextRunProperties>) -> Self {
        Self { control, properties, measured_metrics: None }
    }

    /// Captures a rich paragraph's measured object without formatting a child
    /// during the shared formatter's metric/drawing passes.
    pub(crate) fn with_measured_metrics(&self) -> Self {
        Self {
            control: self.control.clone(),
            properties: self.properties.clone(),
            measured_metrics: Some((self.control.desired_size(), Self::measured_baseline(&self.control))),
        }
    }

    /// Reads only completed layout. An absent or stale cache uses the child's
    /// desired height until its next measure; this never forces nested layout.
    pub(crate) fn measured_baseline(control: &Ref<Control>) -> f64 {
        let explicit = control.get_value(TextBlock::baseline_offset_property());
        if !MathUtilities::is_zero(explicit) {
            // Retain the existing attached override, including inheritance
            // and its convention that zero means no override.
            return explicit;
        }
        if control.is_measure_valid() {
            if let Some(rich) = control.clone().cast::<RichTextBlock>() {
                if let Some(baseline) = rich.cached_baseline_offset() {
                    return baseline;
                }
            } else if let Some(text) = control.clone().cast::<TextBlock>() {
                if let Some(baseline) = text.cached_text_baseline() {
                    return baseline;
                }
            }
        }
        control.desired_size().height
    }

    /// The embedded control.
    pub fn control(&self) -> &Ref<Control> {
        &self.control
    }
}

impl TextRun for EmbeddedControlRun {
    fn properties(&self) -> Option<&Rc<dyn TextRunProperties>> {
        Some(&self.properties)
    }

    fn as_drawable(&self) -> Option<&dyn DrawableTextRun> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl DrawableTextRun for EmbeddedControlRun {
    fn size(&self) -> Size {
        self.measured_metrics.map_or_else(|| self.control.desired_size(), |metrics| metrics.0)
    }

    fn baseline(&self) -> f64 {
        if let Some((_, baseline)) = self.measured_metrics {
            return baseline;
        }
        let mut baseline = self.size().height;
        let baseline_offset_value = self.control.get_value(TextBlock::baseline_offset_property());

        if !MathUtilities::is_zero(baseline_offset_value) {
            baseline = baseline_offset_value;
        }

        baseline
    }

    fn draw(&self, _drawing_context: &mut dyn ITextDrawingSink, _origin: Point) {
        // noop
    }
}
