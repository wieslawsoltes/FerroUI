use crate::{
    DocumentColorizingTransformer, ITextRunConstructionContext, ITextViewConnect,
    IVisualLineTransformer, NativeHighlightingStyle, VisualLineBuilder, VisualLineError,
};
use ferroui_base::StyledElement;
use ferroui_edit_core::highlighting::{HighlightingColor, RichTextModel};
use std::cell::RefCell;
use std::rc::Rc;

/// Applies a shared rich-text model over the current visual-line formatting.
/// Add it to the text area's line transformers. After changing the model,
/// release the mutable borrow and call `TextArea::redraw` on the affected views.
pub struct RichTextColorizer(DocumentColorizingTransformer);

impl RichTextColorizer {
    pub fn new(model: Rc<RefCell<RichTextModel>>) -> Self {
        Self(DocumentColorizingTransformer::new(move |context, line| {
            let source = line.document_line();
            let sections = model
                .try_borrow()
                .map_err(|_| {
                    VisualLineError::Extension("The rich-text model is being edited".into())
                })?
                .get_highlighted_sections(source.offset(), source.length())
                .map_err(|error| VisualLineError::Extension(error.to_string()))?;
            let view = context.text_view();
            let resources = view.as_deref().map(|view| -> &StyledElement { view });
            for section in sections {
                let color = HighlightingColor {
                    name: None,
                    ..section.color
                };
                // Deviation (DEVIATIONS.md, Editor rich text): apply every native
                // visual field, including when it is the only field in the style.
                if color == HighlightingColor::default() {
                    continue;
                }
                let style = NativeHighlightingStyle::resolve(&color, resources)
                    .map_err(|error| VisualLineError::Extension(error.to_string()))?;
                line.change_line_part(
                    section.offset,
                    section.offset + section.length,
                    |element| {
                        let data = element.data_mut();
                        let properties = style.text_run_properties(data.text_run_properties());
                        data.set_text_run_properties(properties.as_ref());
                        if let Some(background) = &style.background {
                            // Draw the background before the editor's selection overlay.
                            data.set_background_brush(Some(background.clone()));
                            data.text_run_properties_mut().set_background_brush(None);
                        }
                    },
                )?;
            }
            Ok(())
        }))
    }
}

impl ITextViewConnect for RichTextColorizer {}

impl IVisualLineTransformer for RichTextColorizer {
    fn transform(
        &self,
        context: &dyn ITextRunConstructionContext,
        line: &mut VisualLineBuilder,
    ) -> Result<(), VisualLineError> {
        self.0.transform(context, line)
    }
}
