use std::cell::Cell;
use std::rc::Rc;

use crate::media::text_formatting::{TextParagraphProperties, TextRunProperties};
use crate::media::{FlowDirection, TextAlignment, TextWrapping};

/// Generic implementation of `TextParagraphProperties`.
pub struct GenericTextParagraphProperties {
    flow_direction: Cell<FlowDirection>,
    text_alignment: Cell<TextAlignment>,
    text_wrap: Cell<TextWrapping>,
    line_height: Cell<f64>,
    first_line_in_paragraph: bool,
    always_collapsible: bool,
    default_text_run_properties: Rc<dyn TextRunProperties>,
    indent: f64,
    letter_spacing: f64,
    line_spacing: Cell<f64>,
}

impl GenericTextParagraphProperties {
    /// Creates left aligned, non wrapping paragraph properties with automatic
    /// line height and no letter spacing.
    pub fn new(default_text_run_properties: Rc<dyn TextRunProperties>) -> Self {
        Self::with_options(default_text_run_properties, TextAlignment::Left, TextWrapping::NoWrap, 0.0, 0.0)
    }

    /// Creates paragraph properties.
    ///
    /// * `default_text_run_properties` — default paragraph's default run properties
    /// * `text_alignment` — logical horizontal alignment
    /// * `text_wrapping` — text wrap option
    /// * `line_height` — paragraph line height
    /// * `letter_spacing` — letter spacing
    pub fn with_options(
        default_text_run_properties: Rc<dyn TextRunProperties>,
        text_alignment: TextAlignment,
        text_wrapping: TextWrapping,
        line_height: f64,
        letter_spacing: f64,
    ) -> Self {
        Self {
            flow_direction: Cell::new(FlowDirection::LeftToRight),
            text_alignment: Cell::new(text_alignment),
            text_wrap: Cell::new(text_wrapping),
            line_height: Cell::new(line_height),
            first_line_in_paragraph: false,
            always_collapsible: false,
            default_text_run_properties,
            indent: 0.0,
            letter_spacing,
            line_spacing: Cell::new(0.0),
        }
    }

    /// Creates paragraph properties with every value specified.
    #[allow(clippy::too_many_arguments)]
    pub fn with_all(
        flow_direction: FlowDirection,
        text_alignment: TextAlignment,
        first_line_in_paragraph: bool,
        always_collapsible: bool,
        default_text_run_properties: Rc<dyn TextRunProperties>,
        text_wrapping: TextWrapping,
        line_height: f64,
        indent: f64,
        letter_spacing: f64,
    ) -> Self {
        Self {
            flow_direction: Cell::new(flow_direction),
            text_alignment: Cell::new(text_alignment),
            text_wrap: Cell::new(text_wrapping),
            line_height: Cell::new(line_height),
            first_line_in_paragraph,
            always_collapsible,
            default_text_run_properties,
            indent,
            letter_spacing,
            line_spacing: Cell::new(0.0),
        }
    }

    /// Set line spacing: the additional space between lines.
    pub fn set_line_spacing(&self, line_spacing: f64) {
        self.line_spacing.set(line_spacing);
    }

    /// Creates a copy of other paragraph properties.
    pub fn from_properties(text_paragraph_properties: &dyn TextParagraphProperties) -> Self {
        Self::with_all(
            text_paragraph_properties.flow_direction(),
            text_paragraph_properties.text_alignment(),
            text_paragraph_properties.first_line_in_paragraph(),
            text_paragraph_properties.always_collapsible(),
            text_paragraph_properties.default_text_run_properties().clone(),
            text_paragraph_properties.text_wrapping(),
            text_paragraph_properties.line_height(),
            text_paragraph_properties.indent(),
            text_paragraph_properties.letter_spacing(),
        )
    }

    /// Set flow direction.
    pub(crate) fn set_flow_direction(&self, flow_direction: FlowDirection) {
        self.flow_direction.set(flow_direction);
    }

    /// Set text alignment.
    pub(crate) fn set_text_alignment(&self, text_alignment: TextAlignment) {
        self.text_alignment.set(text_alignment);
    }

    /// Set line height.
    pub(crate) fn set_line_height(&self, line_height: f64) {
        self.line_height.set(line_height);
    }

    /// Set text wrap.
    pub(crate) fn set_text_wrapping(&self, text_wrap: TextWrapping) {
        self.text_wrap.set(text_wrap);
    }
}

impl TextParagraphProperties for GenericTextParagraphProperties {
    fn flow_direction(&self) -> FlowDirection {
        self.flow_direction.get()
    }

    fn text_alignment(&self) -> TextAlignment {
        self.text_alignment.get()
    }

    fn line_height(&self) -> f64 {
        self.line_height.get()
    }

    fn line_spacing(&self) -> f64 {
        self.line_spacing.get()
    }

    fn first_line_in_paragraph(&self) -> bool {
        self.first_line_in_paragraph
    }

    fn always_collapsible(&self) -> bool {
        self.always_collapsible
    }

    fn default_text_run_properties(&self) -> &Rc<dyn TextRunProperties> {
        &self.default_text_run_properties
    }

    fn text_wrapping(&self) -> TextWrapping {
        self.text_wrap.get()
    }

    fn indent(&self) -> f64 {
        self.indent
    }

    fn letter_spacing(&self) -> f64 {
        self.letter_spacing
    }
}
