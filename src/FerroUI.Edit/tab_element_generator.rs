use crate::formatted_text_element::{FormattedTextContent, FormattedTextElement, FormattedTextRun};
use crate::i_text_run_construction_context::ITextRunConstructionContext;
use crate::i_text_view_connect::ITextViewConnect;
use crate::single_character_element_generator::IBuiltinElementGenerator;
use crate::visual_line_element::{atomic_caret_stop, VisualLineElement, VisualLineElementData};
use crate::visual_line_element_generator::VisualLineElementGenerator;
use crate::visual_line_element_text_run_properties::VisualLineElementTextRunProperties;
use crate::visual_line_error::VisualLineError;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::text_formatting::{TextCharacters, TextLayout, TextRun};
use ferroui_base::media::{Color, IBrush};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_base::Size;
use ferroui_edit_core::document::{CaretPositioningMode, LogicalDirection};
use ferroui_edit_core::editing::EditorOptions;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// Tabs expand from the visual line's display columns, including generated labels.
/// The tab marker occupies one space-width even when its chosen glyph is wider.
pub struct TabElementGenerator {
    pub indentation_size: usize,
    pub show_tabs: bool,
    pub glyph: String,
    pub foreground: Option<Rc<dyn IBrush>>,
    prepared: RefCell<Option<Rc<TextLayout>>>,
}
impl Default for TabElementGenerator {
    fn default() -> Self {
        Self {
            indentation_size: 4,
            show_tabs: false,
            glyph: "→".into(),
            foreground: Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(
                128, 128, 128,
            )))),
            prepared: RefCell::new(None),
        }
    }
}
impl TabElementGenerator {
    pub fn new(indentation_size: usize, show_tabs: bool) -> Self {
        Self {
            indentation_size: indentation_size.clamp(1, 256),
            show_tabs,
            ..Self::default()
        }
    }
    pub fn from_options(options: &EditorOptions) -> Self {
        Self::new(options.indentation_size, options.show_tabs)
    }
    fn glyph_layout(&self, context: &dyn ITextRunConstructionContext) -> Rc<TextLayout> {
        if let Some(layout) = &*self.prepared.borrow() {
            return layout.clone();
        }
        let mut properties =
            VisualLineElementTextRunProperties::new(&**context.global_text_run_properties());
        properties.set_foreground_brush(self.foreground.clone());
        let layout = FormattedTextElement::prepare_text(&self.glyph, properties.snapshot());
        *self.prepared.borrow_mut() = Some(layout.clone());
        layout
    }
}
impl IBuiltinElementGenerator for TabElementGenerator {
    fn fetch_options(&mut self, options: &EditorOptions) {
        self.indentation_size = options.indentation_size.clamp(1, 256);
        self.show_tabs = options.show_tabs;
        self.prepared.get_mut().take();
    }
}
impl ITextViewConnect for TabElementGenerator {}
impl VisualLineElementGenerator for TabElementGenerator {
    fn start_generation(
        &self,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<(), VisualLineError> {
        self.prepared.borrow_mut().take();
        Ok(())
    }
    fn runs_on_long_lines(&self) -> bool {
        true
    }
    fn get_first_interested_offset(
        &self,
        start: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<usize>, VisualLineError> {
        let end = context.last_document_line().end_offset();
        if start >= end {
            return Ok(None);
        }
        Ok(context
            .get_text(start, end - start)?
            .span()
            .iter()
            .position(|unit| *unit == 9)
            .map(|index| start + index))
    }
    fn construct_element(
        &self,
        offset: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<Box<dyn VisualLineElement>>, VisualLineError> {
        if offset >= context.last_document_line().end_offset()
            || context.document().get_char_at(offset)? != 9
        {
            return Ok(None);
        }
        let stop = self.indentation_size.clamp(1, 256);
        let columns = stop - context.current_display_column() % stop;
        let glyph = self.show_tabs.then(|| self.glyph_layout(context));
        let space_width = if context.wide_space_width().is_finite()
            && context.wide_space_width() > 0.0
        {
            context.wide_space_width()
        } else if glyph.is_some() {
            FormattedTextElement::prepare_text(" ", context.global_text_run_properties().clone())
                .width_including_trailing_whitespace()
                .max(1.0)
        } else {
            0.0
        };
        Ok(Some(Box::new(TabTextElement {
            data: VisualLineElementData::new(columns, 1, &**context.global_text_run_properties())?,
            padding: ReadOnlyMemory::from_vec(vec![32; columns]),
            glyph,
            space_width,
        })))
    }
}

pub struct TabTextElement {
    data: VisualLineElementData,
    padding: ReadOnlyMemory<u16>,
    glyph: Option<Rc<TextLayout>>,
    space_width: f64,
}
impl VisualLineElement for TabTextElement {
    fn data(&self) -> &VisualLineElementData {
        &self.data
    }
    fn data_mut(&mut self) -> &mut VisualLineElementData {
        &mut self.data
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn create_text_run(
        &self,
        column: usize,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<Rc<dyn TextRun>, VisualLineError> {
        let relative = self.data.checked_visual_index(column)?;
        let properties = self.data.text_run_properties().snapshot();
        if relative == 0 {
            if let Some(glyph) = &self.glyph {
                return Ok(Rc::new(
                    FormattedTextRun::new(FormattedTextContent::Layout(glyph.clone()), properties)
                        .with_size(Size::new(self.space_width, 0.0)),
                ));
            }
        }
        Ok(Rc::new(TextCharacters::new(
            self.padding.slice_from(relative),
            properties,
        )))
    }
    fn get_next_caret_position(
        &self,
        column: isize,
        direction: LogicalDirection,
        mode: CaretPositioningMode,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<usize>, VisualLineError> {
        Ok(
            if matches!(
                mode,
                CaretPositioningMode::Normal | CaretPositioningMode::EveryCodepoint
            ) {
                atomic_caret_stop(&self.data, column, direction, mode)
            } else {
                None
            },
        )
    }
    fn is_whitespace(&self, _column: usize, _context: &dyn ITextRunConstructionContext) -> bool {
        true
    }
}
