use std::cell::Cell;
use std::rc::Rc;

use crate::media::text_formatting::{TextLineMetricsMode, TextParagraphProperties, TextRunProperties, TextTabStop, TextTabStops};
use crate::media::{FlowDirection, TextAlignment, TextLineBounds, TextWrapping};

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
    tabs: Option<TextTabStops>,
    default_tab: f64,
    tab_origin: f64,
    text_line_bounds: Cell<TextLineBounds>,
    line_metrics_mode: Cell<TextLineMetricsMode>,
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
            tabs: None,
            default_tab: 0.0,
            tab_origin: 0.0,
            text_line_bounds: Cell::new(TextLineBounds::Full),
            line_metrics_mode: Cell::new(TextLineMetricsMode::GlyphTypeface),
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
            tabs: None,
            default_tab: 0.0,
            tab_origin: 0.0,
            text_line_bounds: Cell::new(TextLineBounds::Full),
            line_metrics_mode: Cell::new(TextLineMetricsMode::GlyphTypeface),
        }
    }

    /// Enables paragraph tab-stop layout while retaining the existing run shaper.
    pub fn with_tabs(mut self, tabs: TextTabStops, default_tab: f64, origin: f64) -> Self {
        self.tabs = Some(tabs);
        self.default_tab = default_tab;
        self.tab_origin = origin;
        self
    }

    /// Set line spacing: the additional space between lines.
    pub fn set_line_spacing(&self, line_spacing: f64) {
        self.line_spacing.set(line_spacing);
    }

    /// Sets how the box of each line is bounded (a FerroUI addition, docs/porting/DEVIATIONS.md,
    /// Text block).
    pub fn set_text_line_bounds(&self, text_line_bounds: TextLineBounds) {
        self.text_line_bounds.set(text_line_bounds);
    }

    /// Selects which font metrics allocate the line box. The glyph-typeface
    /// mode remains the default; family mode preserves metrics across glyph
    /// fallback and retains full embedded-object contributions.
    pub fn set_line_metrics_mode(&self, mode: TextLineMetricsMode) {
        self.line_metrics_mode.set(mode);
    }

    /// Creates a copy of other paragraph properties.
    pub fn from_properties(text_paragraph_properties: &dyn TextParagraphProperties) -> Self {
        let mut result = Self::with_all(
            text_paragraph_properties.flow_direction(),
            text_paragraph_properties.text_alignment(),
            text_paragraph_properties.first_line_in_paragraph(),
            text_paragraph_properties.always_collapsible(),
            text_paragraph_properties.default_text_run_properties().clone(),
            text_paragraph_properties.text_wrapping(),
            text_paragraph_properties.line_height(),
            text_paragraph_properties.indent(),
            text_paragraph_properties.letter_spacing(),
        );
        // Deviation (DEVIATIONS.md, Text formatting): the copy constructor of
        // `GenericTextParagraphProperties` does not copy `LineSpacing`; the copy has it.
        result.line_spacing.set(text_paragraph_properties.line_spacing());
        // Tab layout is opt-in: the copy carries the default tab width and the
        // origin only for a source with tab stops, so a copy of any other
        // paragraph keeps the upstream values.
        if let Some(tabs) = text_paragraph_properties.tab_stops() {
            // The source trait may be implemented by a caller; invalid custom
            // values cannot enter the validated immutable collection.
            if let Ok(tabs) = TextTabStops::new(tabs.iter().copied()) {
                result = result.with_tabs(
                    tabs,
                    text_paragraph_properties.default_incremental_tab(),
                    text_paragraph_properties.tab_origin(),
                );
            }
        }
        result.set_text_line_bounds(text_paragraph_properties.text_line_bounds());
        result.set_line_metrics_mode(text_paragraph_properties.line_metrics_mode());
        result
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

    fn text_line_bounds(&self) -> TextLineBounds {
        self.text_line_bounds.get()
    }

    fn line_metrics_mode(&self) -> TextLineMetricsMode {
        self.line_metrics_mode.get()
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

    fn default_incremental_tab(&self) -> f64 {
        self.default_tab
    }

    fn tab_stops(&self) -> Option<&[TextTabStop]> {
        self.tabs.as_ref().map(TextTabStops::as_slice)
    }

    fn tab_origin(&self) -> f64 {
        self.tab_origin
    }

    fn letter_spacing(&self) -> f64 {
        self.letter_spacing
    }
}
