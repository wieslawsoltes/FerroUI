use crate::{Control, TextBlock};
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
}

impl EmbeddedControlRun {
    pub fn new(control: Ref<Control>, properties: Rc<dyn TextRunProperties>) -> Self {
        Self { control, properties }
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
        self.control.desired_size()
    }

    fn baseline(&self) -> f64 {
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
