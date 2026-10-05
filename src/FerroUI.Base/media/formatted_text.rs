use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::media::text_formatting::{
    GenericTextParagraphProperties, GenericTextRunProperties, ITextDrawingSink, ITextSource, TextCharacters,
    TextEndOfParagraph, TextFormatter, TextLine, TextLineBreak, TextParagraphProperties, TextRun, TextRunProperties,
};
use crate::media::{
    BaselineAlignment, FlowDirection, FontFamily, FontFeatureCollection, FontStretch, FontStyle, FontWeight,
    Geometry, GeometryCombineMode, IBrush, RectangleGeometry, TextAlignment, TextCollapsingCreateInfo,
    TextDecorationCollection, TextTrimming, TextWrapping, Transform, Typeface,
};
use crate::utilities::span::{SpanPosition, SpanRider, SpanVector};
use crate::utilities::{CultureInfo, ReadOnlyMemory};
use crate::{Point, Rect, Ref};

/// The FormattedText class is targeted at programmers needing to add some
/// simple text to a visual.
///
/// All character indices and counts are UTF-16 code units of the text.
pub struct FormattedText {
    // properties and format runs
    text: ReadOnlyMemory<u16>,
    format_runs: RefCell<SpanVector<Rc<dyn TextRunProperties>>>,
    latest_position: Cell<SpanPosition>,

    default_para_props: Rc<GenericTextParagraphProperties>,

    max_text_width: Cell<f64>,
    max_text_widths: RefCell<Option<Vec<f64>>>,
    max_text_height: Cell<f64>,
    max_line_count: Cell<i32>,
    trimming: RefCell<Rc<dyn TextTrimming>>,

    // cached metrics
    metrics: Cell<Option<CachedMetrics>>,
}

impl FormattedText {
    pub const DEFAULT_REAL_TO_IDEAL: f64 = 28800.0 / 96.0;
    pub const DEFAULT_IDEAL_TO_REAL: f64 = 1.0 / Self::DEFAULT_REAL_TO_IDEAL;
    pub const IDEAL_INFINITE_WIDTH: i32 = 0x3FFFFFFE;
    pub const REAL_INFINITE_WIDTH: f64 = Self::IDEAL_INFINITE_WIDTH as f64 * Self::DEFAULT_IDEAL_TO_REAL;

    pub const GREATEST_MULTIPLIER_OF_EM: f64 = 100.0;

    const MAX_FONT_EM_SIZE: f64 = Self::REAL_INFINITE_WIDTH / Self::GREATEST_MULTIPLIER_OF_EM;

    /// Construct a FormattedText object.
    ///
    /// * `text_to_format` — string of text to be displayed.
    /// * `culture` — culture of text.
    /// * `flow_direction` — flow direction of text.
    /// * `typeface` — type face used to display text.
    /// * `em_size` — font em size in visual units (1/96 of an inch).
    /// * `foreground` — foreground brush used to render text.
    ///
    /// Panics when the em size is not a positive number or is too large.
    pub fn new(
        text_to_format: &str,
        culture: CultureInfo,
        flow_direction: FlowDirection,
        typeface: Typeface,
        em_size: f64,
        foreground: Option<Rc<dyn IBrush>>,
    ) -> Self {
        Self::validate_font_size(em_size);

        let text = ReadOnlyMemory::<u16>::from_str(text_to_format);

        let run_props: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_all(
            typeface,
            em_size,
            None, // decorations
            foreground,
            None, // highlight background
            BaselineAlignment::Baseline,
            Some(culture),
            None,
        ));

        let mut format_runs = SpanVector::new(None);

        let latest_position =
            format_runs.set_value_at(0, text.len() as i32, run_props.clone(), SpanPosition::default());

        let default_para_props = Rc::new(GenericTextParagraphProperties::with_all(
            flow_direction,
            TextAlignment::Left,
            false,
            false,
            run_props,
            TextWrapping::WrapWithOverflow,
            0.0, // line height not specified
            0.0, // indentation not specified
            0.0,
        ));

        let formatted_text = Self {
            text,
            format_runs: RefCell::new(format_runs),
            latest_position: Cell::new(latest_position),
            default_para_props,
            max_text_width: Cell::new(f64::INFINITY),
            max_text_widths: RefCell::new(None),
            max_text_height: Cell::new(f64::INFINITY),
            max_line_count: Cell::new(i32::MAX),
            trimming: RefCell::new(<dyn TextTrimming>::word_ellipsis()),
            metrics: Cell::new(None),
        };

        formatted_text.invalidate_metrics();

        formatted_text
    }

    fn validate_font_size(em_size: f64) {
        if em_size <= 0.0 {
            panic!("em_size: The parameter value must be greater than zero.");
        }

        if em_size > Self::MAX_FONT_EM_SIZE {
            panic!("em_size: The parameter value cannot be greater than '{}'", Self::MAX_FONT_EM_SIZE);
        }

        if em_size.is_nan() {
            panic!("em_size: The parameter value must be a number.");
        }
    }

    fn text_length(&self) -> i32 {
        self.text.len() as i32
    }

    fn validate_range(&self, start_index: i32, count: i32) -> i32 {
        if start_index < 0 || start_index > self.text_length() {
            panic!("start_index is out of range.");
        }

        let limit = start_index.wrapping_add(count);

        if count < 0 || limit < start_index || limit > self.text_length() {
            panic!("count is out of range.");
        }

        limit
    }

    fn invalidate_metrics(&self) {
        self.metrics.set(None);
    }

    /// The loop every range setter of upstream consists of: walks the format
    /// runs of the range and replaces the properties of each run for which
    /// `create_new_props` returns new ones (`None` means the run already has
    /// the value).
    fn set_run_properties(
        &self,
        start_index: i32,
        count: i32,
        invalidate_metrics: bool,
        create_new_props: impl Fn(&dyn TextRunProperties) -> Option<GenericTextRunProperties>,
    ) {
        let limit = self.validate_range(start_index, count);

        let mut i = start_index;

        while i < limit {
            let (rider_length, current_position, span_position, run_props) = {
                let format_runs = self.format_runs.borrow();
                let format_rider = SpanRider::new(&format_runs, self.latest_position.get(), i);

                (
                    format_rider.length(),
                    format_rider.current_position(),
                    format_rider.span_position(),
                    format_rider.current_element().cloned(),
                )
            };

            i = limit.min(i.saturating_add(rider_length));

            // run_props can never be null because the rider is already checked to be in range
            let Some(run_props) = run_props else {
                panic!("run_props can not be null.");
            };

            let Some(new_props) = create_new_props(&*run_props) else {
                continue;
            };

            let new_props: Rc<dyn TextRunProperties> = Rc::new(new_props);

            let latest_position = self.format_runs.borrow_mut().set_value_at(
                current_position,
                i - current_position,
                new_props,
                span_position,
            );

            self.latest_position.set(latest_position);

            if invalidate_metrics {
                self.invalidate_metrics();
            }
        }
    }

    /// Sets foreground brush used for drawing text.
    ///
    /// * `foreground_brush` — foreground brush
    pub fn set_foreground_brush(&self, foreground_brush: Rc<dyn IBrush>) {
        self.set_foreground_brush_range(Some(foreground_brush), 0, self.text_length());
    }

    /// Sets foreground brush used for drawing text.
    ///
    /// * `foreground_brush` — foreground brush
    /// * `start_index` — the start index of initial character to apply the change to.
    /// * `count` — the number of characters the change should be applied to.
    pub fn set_foreground_brush_range(&self, foreground_brush: Option<Rc<dyn IBrush>>, start_index: i32, count: i32) {
        self.set_run_properties(start_index, count, false, |run_props| {
            let is_same = match (run_props.foreground_brush(), &foreground_brush) {
                (None, None) => true,
                (Some(current), Some(new)) => Rc::ptr_eq(current, new),
                _ => false,
            };

            if is_same {
                return None;
            }

            Some(GenericTextRunProperties::with_all(
                run_props.typeface().clone(),
                run_props.font_rendering_em_size(),
                run_props.text_decorations().cloned(),
                foreground_brush.clone(),
                run_props.background_brush().cloned(),
                run_props.baseline_alignment(),
                run_props.culture_info().cloned(),
                run_props.font_features().cloned(),
            ))
        });
    }

    /// Sets or changes the font features for the text object.
    ///
    /// * `font_features` — feature collection
    pub fn set_font_features(&self, font_features: Option<FontFeatureCollection>) {
        self.set_font_features_range(font_features, 0, self.text_length());
    }

    /// Sets or changes the font features for the text object.
    ///
    /// * `font_features` — feature collection
    /// * `start_index` — the start index of initial character to apply the change to.
    /// * `count` — the number of characters the change should be applied to.
    pub fn set_font_features_range(&self, font_features: Option<FontFeatureCollection>, start_index: i32, count: i32) {
        self.set_run_properties(start_index, count, false, |run_props| {
            let is_same = match (&font_features, run_props.font_features()) {
                (None, None) => true,
                (Some(new), Some(current)) => new.to_vec() == current.to_vec(),
                _ => false,
            };

            if is_same {
                return None;
            }

            Some(GenericTextRunProperties::with_all(
                run_props.typeface().clone(),
                run_props.font_rendering_em_size(),
                run_props.text_decorations().cloned(),
                run_props.foreground_brush().cloned(),
                run_props.background_brush().cloned(),
                run_props.baseline_alignment(),
                run_props.culture_info().cloned(),
                font_features.clone(),
            ))
        });
    }

    /// Sets or changes the font family for the text object.
    ///
    /// * `font_family` — font family name
    pub fn set_font_family_name(&self, font_family: &str) {
        self.set_font_family_name_range(font_family, 0, self.text_length());
    }

    /// Sets or changes the font family for the text object.
    ///
    /// * `font_family` — font family name
    /// * `start_index` — the start index of initial character to apply the change to.
    /// * `count` — the number of characters the change should be applied to.
    pub fn set_font_family_name_range(&self, font_family: &str, start_index: i32, count: i32) {
        self.set_font_family_range(FontFamily::new(font_family), start_index, count);
    }

    /// Sets or changes the font family for the text object.
    ///
    /// * `font_family` — font family
    pub fn set_font_family(&self, font_family: FontFamily) {
        self.set_font_family_range(font_family, 0, self.text_length());
    }

    /// Sets or changes the font family for the text object.
    ///
    /// * `font_family` — font family
    /// * `start_index` — the start index of initial character to apply the change to.
    /// * `count` — the number of characters the change should be applied to.
    pub fn set_font_family_range(&self, font_family: FontFamily, start_index: i32, count: i32) {
        self.set_run_properties(start_index, count, true, |run_props| {
            let old_typeface = run_props.typeface();

            if font_family == *old_typeface.font_family() {
                return None;
            }

            Some(GenericTextRunProperties::with_all(
                Typeface::with_style(
                    font_family.clone(),
                    old_typeface.style(),
                    old_typeface.weight(),
                    FontStretch::Normal,
                ),
                run_props.font_rendering_em_size(),
                run_props.text_decorations().cloned(),
                run_props.foreground_brush().cloned(),
                run_props.background_brush().cloned(),
                run_props.baseline_alignment(),
                run_props.culture_info().cloned(),
                run_props.font_features().cloned(),
            ))
        });
    }

    /// Sets or changes the font em size measured in device independent units.
    ///
    /// * `em_size` — font em size
    pub fn set_font_size(&self, em_size: f64) {
        self.set_font_size_range(em_size, 0, self.text_length());
    }

    /// Sets or changes the font em size measured in device independent units.
    ///
    /// * `em_size` — font em size
    /// * `start_index` — the start index of initial character to apply the change to.
    /// * `count` — the number of characters the change should be applied to.
    pub fn set_font_size_range(&self, em_size: f64, start_index: i32, count: i32) {
        Self::validate_font_size(em_size);

        self.set_run_properties(start_index, count, true, |run_props| {
            if run_props.font_rendering_em_size() == em_size {
                return None;
            }

            Some(GenericTextRunProperties::with_all(
                run_props.typeface().clone(),
                em_size,
                run_props.text_decorations().cloned(),
                run_props.foreground_brush().cloned(),
                run_props.background_brush().cloned(),
                run_props.baseline_alignment(),
                run_props.culture_info().cloned(),
                run_props.font_features().cloned(),
            ))
        });
    }

    /// Sets or changes the culture for the text object.
    ///
    /// * `culture` — the new culture for the text object.
    pub fn set_culture(&self, culture: CultureInfo) {
        self.set_culture_range(culture, 0, self.text_length());
    }

    /// Sets or changes the culture for the text object.
    ///
    /// * `culture` — the new culture for the text object.
    /// * `start_index` — the start index of initial character to apply the change to.
    /// * `count` — the number of characters the change should be applied to.
    pub fn set_culture_range(&self, culture: CultureInfo, start_index: i32, count: i32) {
        self.set_run_properties(start_index, count, true, |run_props| {
            if run_props.culture_info() == Some(&culture) {
                return None;
            }

            Some(GenericTextRunProperties::with_all(
                run_props.typeface().clone(),
                run_props.font_rendering_em_size(),
                run_props.text_decorations().cloned(),
                run_props.foreground_brush().cloned(),
                run_props.background_brush().cloned(),
                run_props.baseline_alignment(),
                Some(culture.clone()),
                run_props.font_features().cloned(),
            ))
        });
    }

    /// Sets or changes the font weight.
    ///
    /// * `weight` — font weight
    pub fn set_font_weight(&self, weight: FontWeight) {
        self.set_font_weight_range(weight, 0, self.text_length());
    }

    /// Sets or changes the font weight.
    ///
    /// * `weight` — font weight
    /// * `start_index` — the start index of initial character to apply the change to.
    /// * `count` — the number of characters the change should be applied to.
    pub fn set_font_weight_range(&self, weight: FontWeight, start_index: i32, count: i32) {
        self.set_run_properties(start_index, count, true, |run_props| {
            let old_typeface = run_props.typeface();

            if old_typeface.weight() == weight {
                return None;
            }

            Some(GenericTextRunProperties::with_all(
                Typeface::with_style(
                    old_typeface.font_family().clone(),
                    old_typeface.style(),
                    weight,
                    FontStretch::Normal,
                ),
                run_props.font_rendering_em_size(),
                run_props.text_decorations().cloned(),
                run_props.foreground_brush().cloned(),
                run_props.background_brush().cloned(),
                run_props.baseline_alignment(),
                run_props.culture_info().cloned(),
                run_props.font_features().cloned(),
            ))
        });
    }

    /// Sets or changes the font style.
    ///
    /// * `style` — font style
    pub fn set_font_style(&self, style: FontStyle) {
        self.set_font_style_range(style, 0, self.text_length());
    }

    /// Sets or changes the font style.
    ///
    /// * `style` — font style
    /// * `start_index` — the start index of initial character to apply the change to.
    /// * `count` — the number of characters the change should be applied to.
    pub fn set_font_style_range(&self, style: FontStyle, start_index: i32, count: i32) {
        self.set_run_properties(start_index, count, true, |run_props| {
            let old_typeface = run_props.typeface();

            if old_typeface.style() == style {
                return None;
            }

            Some(GenericTextRunProperties::with_all(
                Typeface::with_style(
                    old_typeface.font_family().clone(),
                    style,
                    old_typeface.weight(),
                    FontStretch::Normal,
                ),
                run_props.font_rendering_em_size(),
                run_props.text_decorations().cloned(),
                run_props.foreground_brush().cloned(),
                run_props.background_brush().cloned(),
                run_props.baseline_alignment(),
                run_props.culture_info().cloned(),
                run_props.font_features().cloned(),
            ))
        });
    }

    /// Sets or changes the type face.
    ///
    /// * `typeface` — typeface
    pub fn set_font_typeface(&self, typeface: Typeface) {
        self.set_font_typeface_range(typeface, 0, self.text_length());
    }

    /// Sets or changes the type face.
    ///
    /// * `typeface` — typeface
    /// * `start_index` — the start index of initial character to apply the change to.
    /// * `count` — the number of characters the change should be applied to.
    pub fn set_font_typeface_range(&self, typeface: Typeface, start_index: i32, count: i32) {
        self.set_run_properties(start_index, count, true, |run_props| {
            if *run_props.typeface() == typeface {
                return None;
            }

            Some(GenericTextRunProperties::with_all(
                typeface.clone(),
                run_props.font_rendering_em_size(),
                run_props.text_decorations().cloned(),
                run_props.foreground_brush().cloned(),
                run_props.background_brush().cloned(),
                run_props.baseline_alignment(),
                run_props.culture_info().cloned(),
                run_props.font_features().cloned(),
            ))
        });
    }

    /// Sets or changes the text decorations.
    ///
    /// * `text_decorations` — text decorations
    pub fn set_text_decorations(&self, text_decorations: TextDecorationCollection) {
        self.set_text_decorations_range(text_decorations, 0, self.text_length());
    }

    /// Sets or changes the text decorations.
    ///
    /// * `text_decorations` — text decorations
    /// * `start_index` — the start index of initial character to apply the change to.
    /// * `count` — the number of characters the change should be applied to.
    pub fn set_text_decorations_range(&self, text_decorations: TextDecorationCollection, start_index: i32, count: i32) {
        self.set_run_properties(start_index, count, false, |run_props| {
            if run_props.text_decorations() == Some(&text_decorations) {
                return None;
            }

            Some(GenericTextRunProperties::with_all(
                run_props.typeface().clone(),
                run_props.font_rendering_em_size(),
                Some(text_decorations.clone()),
                run_props.foreground_brush().cloned(),
                run_props.background_brush().cloned(),
                run_props.baseline_alignment(),
                run_props.culture_info().cloned(),
                run_props.font_features().cloned(),
            ))
        });
    }

    /// Returns an enumerator that can iterate through the text line collection.
    fn get_enumerator(&self) -> LineEnumerator<'_> {
        LineEnumerator::new(self)
    }

    fn advance_line_origin(&self, line_origin: &mut Point, current_line: &dyn TextLine) {
        let height = current_line.height();

        // advance line origin according to the flow direction
        match self.default_para_props.flow_direction() {
            FlowDirection::LeftToRight | FlowDirection::RightToLeft => {
                *line_origin = line_origin.with_y(line_origin.y + height);
            }
        }
    }

    /// Defines the flow direction.
    pub fn flow_direction(&self) -> FlowDirection {
        self.default_para_props.flow_direction()
    }

    /// Defines the flow direction.
    pub fn set_flow_direction(&self, value: FlowDirection) {
        self.default_para_props.set_flow_direction(value);
        self.invalidate_metrics();
    }

    /// Defines the alignment of text within the column.
    pub fn text_alignment(&self) -> TextAlignment {
        self.default_para_props.text_alignment()
    }

    /// Defines the alignment of text within the column.
    pub fn set_text_alignment(&self, value: TextAlignment) {
        self.default_para_props.set_text_alignment(value);
        self.invalidate_metrics();
    }

    /// Gets the height of, or the spacing between, each line where zero
    /// represents the default line height.
    pub fn line_height(&self) -> f64 {
        self.default_para_props.line_height()
    }

    /// Sets the height of, or the spacing between, each line where zero
    /// represents the default line height.
    ///
    /// Panics when the value is negative.
    pub fn set_line_height(&self, value: f64) {
        if value < 0.0 {
            panic!("value: Parameter must be greater than or equal to zero.");
        }

        self.default_para_props.set_line_height(value);

        self.invalidate_metrics();
    }

    /// The MaxTextWidth property defines the alignment edges for the FormattedText.
    /// For example, left aligned text is wrapped such that the leftmost glyph alignment point
    /// on each line falls exactly on the left edge of the rectangle.
    /// Note that for many fonts, especially in italic style, some glyph strokes may extend beyond
    /// the edges of the alignment rectangle. For this reason, it is recommended that clients draw
    /// text with at least 1/6 em (i.e of the font size) unused margin space either side.
    /// Zero value of MaxTextWidth is equivalent to the maximum possible paragraph width.
    pub fn max_text_width(&self) -> f64 {
        self.max_text_width.get()
    }

    /// Sets the maximum text width. Panics when the value is negative.
    pub fn set_max_text_width(&self, value: f64) {
        if value < 0.0 {
            panic!("value: Parameter must be greater than or equal to zero.");
        }

        self.max_text_width.set(value);

        self.invalidate_metrics();
    }

    /// Sets the array of lengths, which will be applied to each line of text in turn.
    /// If the text covers more lines than there are entries in the length array,
    /// the last entry is reused as many times as required.
    /// The max text widths array overrides the MaxTextWidth property.
    ///
    /// Panics when the array is empty.
    ///
    /// * `max_text_widths` — the max text width array
    pub fn set_max_text_widths(&self, max_text_widths: &[f64]) {
        if max_text_widths.is_empty() {
            panic!("max_text_widths must not be empty.");
        }

        *self.max_text_widths.borrow_mut() = Some(max_text_widths.to_vec());

        self.invalidate_metrics();
    }

    /// Obtains a copy of the array of lengths, which will be applied to each line of text in turn.
    /// If the text covers more lines than there are entries in the length array,
    /// the last entry is reused as many times as required.
    /// The max text widths array overrides the MaxTextWidth property.
    pub fn get_max_text_widths(&self) -> Vec<f64> {
        self.max_text_widths.borrow().clone().unwrap_or_default()
    }

    /// Gets the maximum length of a column of text.
    /// The last line of text displayed is the last whole line that will fit within this limit,
    /// or the nth line as specified by MaxLineCount, whichever occurs first.
    /// Use the Trimming property to control how the omission of text is indicated.
    pub fn max_text_height(&self) -> f64 {
        self.max_text_height.get()
    }

    /// Sets the maximum length of a column of text. Panics when the value is
    /// not greater than zero or is NaN.
    pub fn set_max_text_height(&self, value: f64) {
        if value <= 0.0 {
            panic!("value: 'MaxTextHeight' property value must be greater than zero.");
        }

        if value.is_nan() {
            panic!("value: 'MaxTextHeight' property value cannot be NaN.");
        }

        self.max_text_height.set(value);

        self.invalidate_metrics();
    }

    /// Defines the maximum number of lines to display.
    /// The last line of text displayed is the lineCount-1'th line,
    /// or the last whole line that will fit within the count set by MaxTextHeight,
    /// whichever occurs first.
    /// Use the Trimming property to control how the omission of text is indicated.
    pub fn max_line_count(&self) -> i32 {
        self.max_line_count.get()
    }

    /// Sets the maximum number of lines to display. Panics when the value is
    /// not greater than zero.
    pub fn set_max_line_count(&self, value: i32) {
        if value <= 0 {
            panic!("value: The parameter value must be greater than zero.");
        }

        self.max_line_count.set(value);

        self.invalidate_metrics();
    }

    /// Defines how omission of text is indicated.
    /// CharacterEllipsis trimming allows partial words to be displayed,
    /// while WordEllipsis removes whole words to fit.
    /// Both guarantee to include an ellipsis ('...') at the end of the lines
    /// where text has been trimmed as a result of line and column limits.
    pub fn trimming(&self) -> Rc<dyn TextTrimming> {
        self.trimming.borrow().clone()
    }

    /// Defines how omission of text is indicated.
    pub fn set_trimming(&self, value: Rc<dyn TextTrimming>) {
        *self.trimming.borrow_mut() = value;

        self.default_para_props.set_text_wrapping(if self.is_trimming_none() {
            TextWrapping::Wrap
        } else {
            TextWrapping::WrapWithOverflow
        });

        self.invalidate_metrics();
    }

    /// `_trimming == TextTrimming.None`.
    fn is_trimming_none(&self) -> bool {
        Rc::ptr_eq(&self.trimming.borrow(), &<dyn TextTrimming>::none())
    }

    /// Lazily initializes the cached metrics EXCEPT for black box metrics and
    /// returns the CachedMetrics structure.
    fn metrics(&self) -> CachedMetrics {
        if let Some(metrics) = self.metrics.get() {
            return metrics;
        }

        let metrics = self.draw_and_calculate_metrics(
            None,            // drawing context
            Point::default(), // drawing offset
            false,
        );

        self.metrics.set(Some(metrics));

        metrics
    }

    /// Lazily initializes the cached metrics INCLUDING black box metrics and
    /// returns the CachedMetrics structure.
    fn black_box_metrics(&self) -> CachedMetrics {
        match self.metrics.get() {
            Some(metrics) if !metrics.extent.is_nan() => metrics,
            _ => {
                // We need to obtain the metrics, including black box metrics.

                let metrics = self.draw_and_calculate_metrics(
                    None,             // drawing context
                    Point::default(), // drawing offset
                    true,             // calculate black box metrics
                );

                self.metrics.set(Some(metrics));

                metrics
            }
        }
    }

    /// The distance from the top of the first line to the bottom of the last line.
    pub fn height(&self) -> f64 {
        self.metrics().height
    }

    /// The distance from the topmost black pixel of the first line to the
    /// bottommost black pixel of the last line.
    pub fn extent(&self) -> f64 {
        self.black_box_metrics().extent
    }

    /// The distance from the top of the first line to the baseline of the first line.
    pub fn baseline(&self) -> f64 {
        self.metrics().baseline
    }

    /// The distance from the bottom of the last line to the extent bottom.
    pub fn overhang_after(&self) -> f64 {
        self.black_box_metrics().overhang_after
    }

    /// The maximum distance from the leading black pixel to the leading alignment point of a line.
    pub fn overhang_leading(&self) -> f64 {
        self.black_box_metrics().overhang_leading
    }

    /// The maximum distance from the trailing black pixel to the trailing alignment point of a line.
    pub fn overhang_trailing(&self) -> f64 {
        self.black_box_metrics().overhang_trailing
    }

    /// The maximum advance width between the leading and trailing alignment points of a line,
    /// excluding the width of whitespace characters at the end of the line.
    pub fn width(&self) -> f64 {
        self.metrics().width
    }

    /// The maximum advance width between the leading and trailing alignment points of a line,
    /// including the width of whitespace characters at the end of the line.
    pub fn width_including_trailing_whitespace(&self) -> f64 {
        self.metrics().width_including_trailing_whitespace
    }

    /// Builds a highlight geometry object.
    ///
    /// * `origin` — the origin of the highlight region
    ///
    /// Returns geometry that surrounds the text.
    pub fn build_highlight_geometry(&self, origin: Point) -> Option<Ref<Geometry>> {
        self.build_highlight_geometry_range(origin, 0, self.text_length())
    }

    /// Builds a highlight geometry object for a given character range.
    ///
    /// * `origin` — the origin of the highlight region.
    /// * `start_index` — the start index of initial character the bounds should be obtained for.
    /// * `count` — the number of characters the bounds should be obtained for.
    ///
    /// Returns geometry that surrounds the specified character range.
    pub fn build_highlight_geometry_range(&self, origin: Point, start_index: i32, count: i32) -> Option<Ref<Geometry>> {
        self.validate_range(start_index, count);

        let mut accumulated_bounds: Option<Ref<Geometry>> = None;

        {
            let mut enumerator = self.get_enumerator();

            let mut line_origin = origin;

            while enumerator.move_next() {
                let Some(current_line) = enumerator.current() else {
                    continue;
                };

                let x0 = enumerator.position().max(start_index);
                let x1 = (enumerator.position() + enumerator.length()).min(start_index + count);

                // check if this line is intersects with the specified character range
                if x0 < x1 {
                    let highlight_bounds = current_line.get_text_bounds(x0, x1 - x0);

                    for bound in &highlight_bounds {
                        let mut rect = bound.rectangle();

                        if self.flow_direction() == FlowDirection::RightToLeft {
                            // Convert logical units (which extend leftward from the right edge
                            // of the paragraph) to physical units.
                            //
                            // Note that since rect is in logical units, rect.Right corresponds to
                            // the visual *left* edge of the rectangle in the RTL case. Specifically,
                            // is the distance leftward from the right edge of the formatting rectangle
                            // whose width is the paragraph width passed to FormatLine.
                            //
                            rect = rect.with_x(enumerator.current_paragraph_width() - rect.right());
                        }

                        rect = Rect::new(rect.x + line_origin.x, rect.y + line_origin.y, rect.width, rect.height);

                        let rectangle_geometry = RectangleGeometry::with_rect(rect);

                        accumulated_bounds = Some(match accumulated_bounds {
                            None => rectangle_geometry.upcast(),
                            Some(accumulated_bounds) => Geometry::combine(
                                accumulated_bounds,
                                &rectangle_geometry,
                                GeometryCombineMode::Union,
                                None::<Ref<Transform>>,
                            ),
                        });
                    }
                }

                self.advance_line_origin(&mut line_origin, &*current_line);
            }

            enumerator.dispose();
        }

        let accumulated_bounds = accumulated_bounds?;

        let bounds = accumulated_bounds.platform_impl()?.bounds();

        if bounds.width == 0.0 && bounds.height == 0.0 {
            return None;
        }

        Some(accumulated_bounds)
    }

    /// Draws the text object.
    pub(crate) fn draw(&self, drawing_context: &mut dyn ITextDrawingSink, origin: Point) {
        let mut line_origin = origin;

        if self.metrics.get().is_some_and(|metrics| !metrics.extent.is_nan()) {
            let mut enumerator = self.get_enumerator();

            while enumerator.move_next() {
                let Some(current_line) = enumerator.current() else {
                    continue;
                };

                current_line.draw(drawing_context, line_origin);

                self.advance_line_origin(&mut line_origin, &*current_line);
            }

            enumerator.dispose();
        } else {
            // Calculate metrics as we draw to avoid formatting again if we need metrics later; we compute
            // black box metrics too because these are already known as a side-effect of drawing

            let metrics = self.draw_and_calculate_metrics(Some(drawing_context), origin, true);

            self.metrics.set(Some(metrics));
        }
    }

    fn draw_and_calculate_metrics(
        &self,
        mut drawing_context: Option<&mut dyn ITextDrawingSink>,
        drawing_offset: Point,
        get_black_box_metrics: bool,
    ) -> CachedMetrics {
        let mut metrics = CachedMetrics::default();

        if self.text.is_empty() {
            return metrics;
        }

        let mut enumerator = self.get_enumerator();

        let mut first = true;

        let mut acc_black_box_left = f64::MAX;
        let mut acc_black_box_top = f64::MAX;
        let mut acc_black_box_right = f64::MIN;
        let mut acc_black_box_bottom = f64::MIN;

        let mut origin = Point::new(0.0, 0.0);

        // Holds the TextLine.Start of the longest line. Thus it will hold the minimum value
        // of TextLine.Start among all the lines that forms the text. The overhangs (leading and trailing)
        // are calculated with an offset as a result of the same issue with TextLine.Start.
        // So, we compute this offset and remove it later from the values of the overhangs.
        let mut line_start_of_longest_line = f64::MAX;

        while enumerator.move_next() {
            let Some(current_line) = enumerator.current() else {
                continue;
            };

            // if we're drawing, do it first as this will compute black box metrics as a side-effect
            if let Some(drawing_context) = drawing_context.as_deref_mut() {
                current_line
                    .draw(drawing_context, Point::new(origin.x + drawing_offset.x, origin.y + drawing_offset.y));
            }

            if get_black_box_metrics {
                let black_box_left = origin.x + current_line.start() + current_line.overhang_leading();
                let black_box_right =
                    origin.x + current_line.start() + current_line.width() - current_line.overhang_trailing();
                let black_box_bottom = origin.y + current_line.height() + current_line.overhang_after();
                let black_box_top = black_box_bottom - current_line.extent();

                acc_black_box_left = acc_black_box_left.min(black_box_left);
                acc_black_box_right = acc_black_box_right.max(black_box_right);
                acc_black_box_bottom = acc_black_box_bottom.max(black_box_bottom);
                acc_black_box_top = acc_black_box_top.min(black_box_top);

                metrics.overhang_after = current_line.overhang_after();
            }

            metrics.height += current_line.height();
            metrics.width = metrics.width.max(current_line.width());
            metrics.width_including_trailing_whitespace = metrics
                .width_including_trailing_whitespace
                .max(current_line.width_including_trailing_whitespace());
            line_start_of_longest_line = line_start_of_longest_line.min(current_line.start());

            if first {
                metrics.baseline = current_line.baseline();
                first = false;
            }

            self.advance_line_origin(&mut origin, &*current_line);
        }

        if get_black_box_metrics {
            metrics.extent = acc_black_box_bottom - acc_black_box_top;
            metrics.overhang_leading = acc_black_box_left - line_start_of_longest_line;
            metrics.overhang_trailing = metrics.width - (acc_black_box_right - line_start_of_longest_line);
        } else {
            // indicate that black box metrics are not known
            metrics.extent = f64::NAN;
        }

        enumerator.dispose();

        metrics
    }
}

/// Enumerator used for enumerating text lines.
struct LineEnumerator<'a> {
    line_count: i32,
    total_height: f64,
    next_line: Option<Rc<dyn TextLine>>,
    formatter: Rc<dyn TextFormatter>,
    that: &'a FormattedText,
    text_source: TextSourceImplementation<'a>,
    para_props: Rc<dyn TextParagraphProperties>,

    // these are needed because the current line can be disposed before the next move_next() call
    previous_height: f64,

    // line break before the current line, needed in case we have to reformat it with collapsing symbol
    previous_line_break: Option<Rc<TextLineBreak>>,
    position: i32,
    length: i32,

    current: Option<Rc<dyn TextLine>>,
}

impl<'a> LineEnumerator<'a> {
    fn new(text: &'a FormattedText) -> Self {
        Self {
            previous_height: 0.0,
            length: 0,
            previous_line_break: None,
            position: 0,
            line_count: 0,
            total_height: 0.0,
            current: None,
            next_line: None,
            formatter: <dyn TextFormatter>::current(),
            that: text,
            text_source: TextSourceImplementation::new(text),
            para_props: text.default_para_props.clone(),
        }
    }

    fn dispose(&mut self) {
        self.current = None;

        self.next_line = None;
    }

    fn position(&self) -> i32 {
        self.position
    }

    fn length(&self) -> i32 {
        self.length
    }

    /// Gets the current text line in the collection.
    fn current(&self) -> Option<Rc<dyn TextLine>> {
        self.current.clone()
    }

    /// Gets the paragraph width used to format the current text line.
    fn current_paragraph_width(&self) -> f64 {
        self.max_line_length(self.line_count)
    }

    fn max_line_length(&self, line: i32) -> f64 {
        match &*self.that.max_text_widths.borrow() {
            None => self.that.max_text_width.get(),
            Some(max_text_widths) => max_text_widths[(line as usize).min(max_text_widths.len() - 1)],
        }
    }

    /// Advances the enumerator to the next text line of the collection.
    ///
    /// Returns true if the enumerator was successfully advanced to the next
    /// element; false if the enumerator has passed the end of the collection.
    fn move_next(&mut self) -> bool {
        let that = self.that;

        let current = match self.current.take() {
            None => {
                // this is the first line
                if that.text.is_empty() {
                    return false;
                }

                let current = self.format_line(
                    self.position,
                    self.max_line_length(self.line_count),
                    None, // no previous line break
                );

                let Some(current) = current else {
                    return false;
                };

                // check if this line fits the text height
                if self.total_height + current.height() > that.max_text_height.get() {
                    return false;
                }

                debug_assert!(self.next_line.is_none());

                current
            }
            Some(previous) => {
                // there is no next line or it didn't fit
                // either way we're finished
                let Some(next_line) = self.next_line.take() else {
                    self.current = Some(previous);

                    return false;
                };

                self.total_height += self.previous_height;
                self.position += self.length;
                self.line_count += 1;

                next_line
            }
        };

        let mut current = Some(current);

        let mut current_line_break = current.as_ref().and_then(|current| current.text_line_break());

        if let Some(current_line) = current.clone() {
            // this line is guaranteed to fit the text height
            debug_assert!(self.total_height + current_line.height() <= that.max_text_height.get());

            // now, check if the next line fits, we need to do this on this iteration
            // because we might need to add ellipsis to the current line
            // as a result of the next line measurement

            // maybe there is no next line at all
            if self.position + current_line.length() < that.text_length() {
                let mut next_line_fits = false;

                if self.line_count + 1 >= that.max_line_count.get() {
                    next_line_fits = false;
                } else {
                    self.next_line = self.format_line(
                        self.position + current_line.length(),
                        self.max_line_length(self.line_count + 1),
                        current_line_break.as_ref(),
                    );

                    if let Some(next_line) = &self.next_line {
                        next_line_fits = self.total_height + current_line.height() + next_line.height()
                            <= that.max_text_height.get();
                    }
                }

                if !next_line_fits {
                    self.next_line = None;

                    if !that.is_trimming_none() && !current_line.has_collapsed() {
                        // recreate the current line with ellipsis added
                        // Note: Paragraph ellipsis is not supported today. We'll workaround
                        // it here by faking a non-wrap text on finite column width.
                        let current_wrap = that.default_para_props.text_wrapping();

                        that.default_para_props.set_text_wrapping(TextWrapping::NoWrap);

                        current = self.format_line(
                            self.position,
                            self.max_line_length(self.line_count),
                            self.previous_line_break.as_ref(),
                        );

                        if let Some(current) = &current {
                            current_line_break = current.text_line_break();
                        }

                        that.default_para_props.set_text_wrapping(current_wrap);
                    }
                }
            }
        }

        if let Some(current) = &current {
            self.previous_height = current.height();

            self.length = current.length();
        }

        self.current = current;

        self.previous_line_break = current_line_break;

        true
    }

    /// Wrapper of `TextFormatter::format_line` that auto-collapses the line if needed.
    fn format_line(
        &self,
        text_source_position: i32,
        max_line_length: f64,
        line_break: Option<&Rc<TextLineBreak>>,
    ) -> Option<Rc<dyn TextLine>> {
        let that = self.that;

        let mut line = self.formatter.format_line(
            &self.text_source,
            text_source_position,
            max_line_length,
            &self.para_props,
            line_break,
        );

        if let Some(overflowed_line) = &line {
            if !that.is_trimming_none() && overflowed_line.has_overflowed() && overflowed_line.length() > 0 {
                // what I really need here is the last displayed text run of the line
                // text_source_position + line.length - 1 works except the end of paragraph case,
                // where line length includes the fake paragraph break run
                debug_assert!(
                    that.text_length() > 0
                        && text_source_position + overflowed_line.length() <= that.text_length() + 1
                );

                let last_run_props = {
                    let format_runs = that.format_runs.borrow();

                    let that_format_rider = SpanRider::new(
                        &format_runs,
                        that.latest_position.get(),
                        (text_source_position + overflowed_line.length() - 1).min(that.text_length() - 1),
                    );

                    that_format_rider.current_element().cloned().expect("the format runs cover the text")
                };

                let collapsing_properties =
                    that.trimming.borrow().create_collapsing_properties(&TextCollapsingCreateInfo::new(
                        max_line_length,
                        last_run_props,
                        self.para_props.flow_direction(),
                    ));

                let collapsed_line = overflowed_line.clone().collapse(&[Some(collapsing_properties)]);

                line = Some(collapsed_line);
            }
        }

        line
    }

    /// Sets the enumerator to its initial position, which is before the first
    /// element in the collection.
    #[allow(dead_code)] // upstream member nothing uses
    fn reset(&mut self) {
        self.position = 0;
        self.line_count = 0;
        self.total_height = 0.0;
        self.current = None;
        self.next_line = None;
    }
}

#[derive(Clone, Copy, Default)]
struct CachedMetrics {
    // vertical
    height: f64,
    baseline: f64,

    // horizontal
    width: f64,
    width_including_trailing_whitespace: f64,

    // vertical bounding box metrics
    extent: f64,
    overhang_after: f64,

    // horizontal bounding box metrics
    overhang_leading: f64,
    overhang_trailing: f64,
}

/// The text source of a formatted text: its text with the properties of the
/// format runs.
struct TextSourceImplementation<'a> {
    that: &'a FormattedText,
}

impl<'a> TextSourceImplementation<'a> {
    fn new(text: &'a FormattedText) -> Self {
        Self { that: text }
    }
}

impl ITextSource for TextSourceImplementation<'_> {
    fn get_text_run(&self, text_source_character_index: i32) -> Option<Rc<dyn TextRun>> {
        let that = self.that;

        if text_source_character_index >= that.text_length() {
            return Some(Rc::new(TextEndOfParagraph::new()));
        }

        let format_runs = that.format_runs.borrow();

        let that_format_rider = SpanRider::new(&format_runs, that.latest_position.get(), text_source_character_index);

        let text = that.text.slice(text_source_character_index as usize, that_format_rider.length() as usize);

        let properties = that_format_rider.current_element().cloned().expect("the format runs cover the text");

        Some(Rc::new(TextCharacters::new(text, properties)))
    }
}

#[cfg(test)]
#[path = "formatted_text_tests.rs"]
mod tests;
