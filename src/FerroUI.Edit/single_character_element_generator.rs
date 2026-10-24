use crate::formatted_text_element::{FormattedTextElement, FormattedTextRun};
use crate::i_text_run_construction_context::ITextRunConstructionContext;
use crate::i_text_view_connect::ITextViewConnect;
use crate::visual_line_element::{VisualLineElement, VisualLineElementData, atomic_caret_stop};
use crate::visual_line_element_generator::VisualLineElementGenerator;
use crate::visual_line_element_text_run_properties::VisualLineElementTextRunProperties;
use crate::visual_line_error::VisualLineError;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::text_formatting::{TextLayout, TextRun};
use ferroui_base::media::{Color, IBrush};
use ferroui_edit_core::document::{CaretPositioningMode, LogicalDirection, TextUtilities};
use ferroui_edit_core::editing::EditorOptions;
use std::any::Any;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// Updates a built-in generator from the editor's document-independent options.
pub trait IBuiltinElementGenerator {
    fn fetch_options(&mut self, options: &EditorOptions);
}

/// Spaces and control-character boxes use native drawable runs, so their width,
/// baseline, hit testing, wrapping and selection participate in the formatter.
pub struct SingleCharacterElementGenerator {
    pub show_spaces: bool,
    pub show_box_for_control_characters: bool,
    pub space_glyph: String,
    pub foreground: Option<Rc<dyn IBrush>>,
    prepared: RefCell<BTreeMap<(String, bool), Rc<TextLayout>>>,
}
impl Default for SingleCharacterElementGenerator {
    fn default() -> Self {
        Self {
            show_spaces: true,
            show_box_for_control_characters: true,
            space_glyph: "·".into(),
            foreground: Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(
                128, 128, 128,
            )))),
            prepared: RefCell::new(BTreeMap::new()),
        }
    }
}
impl SingleCharacterElementGenerator {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn from_options(options: &EditorOptions) -> Self {
        let mut generator = Self::default();
        generator.fetch_options(options);
        generator
    }
    fn shows_box_for(&self, unit: u16) -> bool {
        self.show_box_for_control_characters
            && unit != 9
            && char::from_u32(u32::from(unit)).is_some_and(char::is_control)
    }
    fn prepare(
        &self,
        text: &str,
        boxed: bool,
        context: &dyn ITextRunConstructionContext,
    ) -> Rc<TextLayout> {
        let key = (text.to_owned(), boxed);
        if let Some(layout) = self.prepared.borrow().get(&key) {
            return layout.clone();
        }
        let mut properties =
            VisualLineElementTextRunProperties::new(&**context.global_text_run_properties());
        properties.set_foreground_brush(if boxed {
            Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(
                255, 255, 255,
            ))))
        } else {
            self.foreground.clone()
        });
        let layout = FormattedTextElement::prepare_text(text, properties.snapshot());
        self.prepared.borrow_mut().insert(key, layout.clone());
        layout
    }
}
impl IBuiltinElementGenerator for SingleCharacterElementGenerator {
    fn fetch_options(&mut self, options: &EditorOptions) {
        self.show_spaces = options.show_spaces;
        self.show_box_for_control_characters = options.show_box_for_control_characters;
        self.prepared.get_mut().clear();
    }
}
impl ITextViewConnect for SingleCharacterElementGenerator {}
impl VisualLineElementGenerator for SingleCharacterElementGenerator {
    fn start_generation(
        &self,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<(), VisualLineError> {
        self.prepared.borrow_mut().clear();
        Ok(())
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
        for (index, unit) in context
            .get_text(start, end - start)?
            .span()
            .iter()
            .copied()
            .enumerate()
        {
            if unit == 32 && self.show_spaces || self.shows_box_for(unit) {
                return Ok(Some(start + index));
            }
        }
        Ok(None)
    }
    fn construct_element(
        &self,
        offset: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<Box<dyn VisualLineElement>>, VisualLineError> {
        if offset >= context.last_document_line().end_offset() {
            return Ok(None);
        }
        let unit = context.document().get_char_at(offset)?;
        if unit == 32 && self.show_spaces {
            let inner = FormattedTextElement::from_text_layout(
                self.prepare(&self.space_glyph, false, context),
                1,
                &**context.global_text_run_properties(),
            )?;
            return Ok(Some(Box::new(SpaceTextElement(inner))));
        }
        if self.shows_box_for(unit) {
            let label = TextUtilities::get_control_character_name(unit);
            let mut inner = FormattedTextElement::from_text_layout(
                self.prepare(&label, true, context),
                1,
                &**context.global_text_run_properties(),
            )?;
            inner.set_display_column_length(label.encode_utf16().count());
            return Ok(Some(Box::new(SpecialCharacterBoxElement(inner))));
        }
        Ok(None)
    }
}

struct SpaceTextElement(FormattedTextElement);
impl VisualLineElement for SpaceTextElement {
    fn data(&self) -> &VisualLineElementData {
        self.0.data()
    }
    fn data_mut(&mut self) -> &mut VisualLineElementData {
        self.0.data_mut()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn create_text_run(
        &self,
        column: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Rc<dyn TextRun>, VisualLineError> {
        self.0.create_text_run(column, context)
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
                atomic_caret_stop(self.data(), column, direction, mode)
            } else {
                None
            },
        )
    }
    fn is_whitespace(&self, _column: usize, _context: &dyn ITextRunConstructionContext) -> bool {
        true
    }
}
struct SpecialCharacterBoxElement(FormattedTextElement);
impl VisualLineElement for SpecialCharacterBoxElement {
    fn data(&self) -> &VisualLineElementData {
        self.0.data()
    }
    fn data_mut(&mut self) -> &mut VisualLineElementData {
        self.0.data_mut()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn display_column_length(&self) -> usize {
        self.0.display_column_length()
    }
    fn create_text_run(
        &self,
        column: usize,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<Rc<dyn TextRun>, VisualLineError> {
        self.data().checked_visual_index(column)?;
        let background: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Color::from_argb(
            200, 128, 128, 128,
        )));
        let run: FormattedTextRun =
            self.0
                .create_formatted_run()
                .with_box(3.0, Some(background), None);
        Ok(Rc::new(run))
    }
}
