use crate::{
    ITextRunConstructionContext, VisualLineElement, VisualLineElementData, VisualLineError,
};
use ferroui_base::Ref;
use ferroui_base::media::text_formatting::{TextRun, TextRunProperties};
use ferroui_controls::Control;
use ferroui_controls::documents::EmbeddedControlRun;
use std::any::Any;
use std::rc::Rc;

/// An atomic span of formatter columns displaying a native control.
///
/// A generator can supply a zero document length to insert a control between
/// characters. Reusing the same control on rebuild preserves its state while
/// it remains visible. A control can occupy only one position in the viewport.
pub struct InlineObjectElement {
    data: VisualLineElementData,
    element: Ref<Control>,
}

impl InlineObjectElement {
    pub fn new(
        document_length: usize,
        element: Ref<Control>,
        properties: &dyn TextRunProperties,
    ) -> Result<Self, VisualLineError> {
        Self::with_visual_length(1, document_length, element, properties)
    }

    /// Creates an atomic inline control whose formatter and document lengths
    /// can differ. A zero document length inserts it between characters.
    pub fn with_visual_length(
        visual_length: usize,
        document_length: usize,
        element: Ref<Control>,
        properties: &dyn TextRunProperties,
    ) -> Result<Self, VisualLineError> {
        Ok(Self {
            data: VisualLineElementData::new(visual_length, document_length, properties)?,
            element,
        })
    }

    pub fn element(&self) -> &Ref<Control> {
        &self.element
    }
}

impl VisualLineElement for InlineObjectElement {
    fn data(&self) -> &VisualLineElementData {
        &self.data
    }

    fn data_mut(&mut self) -> &mut VisualLineElementData {
        &mut self.data
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn inline_control(&self) -> Option<Ref<Control>> {
        Some(self.element.clone())
    }

    fn create_text_run(
        &self,
        start_visual_column: usize,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<Rc<dyn TextRun>, VisualLineError> {
        let index = self.data.checked_visual_index(start_visual_column)?;
        let length = (self.data.visual_length() - index) as i32;
        let run = EmbeddedControlRun::from_measurement(
            self.element.clone(),
            self.data.text_run_properties().snapshot(),
        )
        .with_length(length)
        .map_err(|_| VisualLineError::InvalidVisualLength)?;
        Ok(Rc::new(run))
    }
}
