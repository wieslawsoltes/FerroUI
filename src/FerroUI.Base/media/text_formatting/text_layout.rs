use std::rc::Rc;

use crate::media::text_formatting::{
    FormattedTextSource, FormattingObjectPool, GenericTextParagraphProperties, GenericTextRunProperties,
    ITextDrawingSink, ITextSource, InterWordJustification, TextCollapsingProperties, TextEndOfParagraph,
    TextFormatter, TextFormatterImpl, TextLine, TextLineImpl, TextParagraphProperties, TextRun, TextRunCache,
    TextRunProperties, TextParagraphStyle, TextParagraphStyles,
};
use crate::media::{
    BaselineAlignment, CharacterHit, FlowDirection, FontFeatureCollection, IBrush, TextAlignment,
    TextCollapsingCreateInfo, TextDecorationCollection, TextHitTestResult, TextTrimming, TextWrapping, Typeface,
};
use crate::utilities::{MathUtilities, ReadOnlyMemory, ValueSpan};
use crate::{Point, Rect};

/// The optional parameters of [`TextLayout::new`]. `Default` gives upstream's
/// parameter defaults.
#[derive(Clone)]
pub struct TextLayoutOptions {
    /// Size of the font.
    pub font_size: f64,
    /// The foreground.
    pub foreground: Option<Rc<dyn IBrush>>,
    /// The text alignment.
    pub text_alignment: TextAlignment,
    /// The text wrapping.
    pub text_wrapping: TextWrapping,
    /// The text trimming (`None` means no trimming).
    pub text_trimming: Option<Rc<dyn TextTrimming>>,
    /// The text decorations.
    pub text_decorations: Option<TextDecorationCollection>,
    /// The text flow direction.
    pub flow_direction: FlowDirection,
    /// The maximum width.
    pub max_width: f64,
    /// The maximum height.
    pub max_height: f64,
    /// The height of each line of text.
    pub line_height: f64,
    /// The letter spacing that is applied to rendered glyphs.
    pub letter_spacing: f64,
    /// The maximum number of text lines.
    pub max_lines: i32,
    /// Optional list of turned on/off features.
    pub font_features: Option<FontFeatureCollection>,
    /// The text style overrides.
    pub text_style_overrides: Option<Rc<[ValueSpan<Rc<dyn TextRunProperties>>]>>,
    /// An optional cache for shaped text runs to avoid redundant shaping.
    pub text_run_cache: Option<Rc<TextRunCache>>,
}

impl Default for TextLayoutOptions {
    fn default() -> Self {
        Self {
            font_size: GenericTextRunProperties::DEFAULT_FONT_RENDERING_EM_SIZE,
            foreground: None,
            text_alignment: TextAlignment::Left,
            text_wrapping: TextWrapping::NoWrap,
            text_trimming: None,
            text_decorations: None,
            flow_direction: FlowDirection::LeftToRight,
            max_width: f64::INFINITY,
            max_height: f64::INFINITY,
            line_height: f64::NAN,
            letter_spacing: 0.0,
            max_lines: 0,
            font_features: None,
            text_style_overrides: None,
            text_run_cache: None,
        }
    }
}

/// Represents a multi line text layout.
pub struct TextLayout {
    text_source: Rc<dyn ITextSource>,
    paragraph_properties: Rc<dyn TextParagraphProperties>,
    text_trimming: Rc<dyn TextTrimming>,
    text_lines: Vec<Rc<dyn TextLine>>,
    paragraph_styles: Option<TextParagraphStyles>,
    line_placements: Vec<LinePlacement>,
    metrics: CachedMetrics,
    text_run_cache: Option<Rc<TextRunCache>>,

    text_source_length: i32,

    max_width: f64,
    max_height: f64,
    max_lines: i32,
}

impl TextLayout {
    /// Initializes a new instance of the `TextLayout` class.
    ///
    /// * `text` — the text.
    /// * `typeface` — the typeface.
    /// * `options` — the remaining (optional) parameters.
    pub fn new(text: &str, typeface: Typeface, options: TextLayoutOptions) -> Self {
        Self::from_utf16(ReadOnlyMemory::<u16>::from_str(text), typeface, options)
    }

    /// Formats independent paragraphs without changing the existing options or
    /// constructor contract. Style ranges use UTF-16 positions, including a
    /// paragraph's terminator. Caret, range geometry and drawing share the same
    /// line origins, including first-line indentation and paragraph spacing.
    pub fn new_with_paragraph_styles(
        text: &str, typeface: Typeface, options: TextLayoutOptions, styles: Option<TextParagraphStyles>,
    ) -> Self {
        Self::from_utf16_with_paragraph_styles(ReadOnlyMemory::<u16>::from_str(text), typeface, options, styles)
    }

    /// Initializes a new instance of the `TextLayout` class from UTF-16 text
    /// (no transcoding).
    pub fn from_utf16(text: ReadOnlyMemory<u16>, typeface: Typeface, options: TextLayoutOptions) -> Self {
        Self::from_utf16_with_paragraph_styles(text, typeface, options, None)
    }

    /// UTF-16 variant of [`Self::new_with_paragraph_styles`].
    pub fn from_utf16_with_paragraph_styles(
        text: ReadOnlyMemory<u16>, typeface: Typeface, options: TextLayoutOptions,
        styles: Option<TextParagraphStyles>,
    ) -> Self {
        let paragraph_properties = Self::create_text_paragraph_properties(
            typeface,
            options.font_size,
            options.foreground,
            options.text_alignment,
            options.text_wrapping,
            options.text_decorations,
            options.flow_direction,
            options.line_height,
            options.letter_spacing,
            options.font_features,
        );

        let text_source: Rc<dyn ITextSource> = Rc::new(FormattedTextSource::new(
            text,
            paragraph_properties.default_text_run_properties().clone(),
            options.text_style_overrides,
        ));

        Self::from_text_source_with_paragraph_styles(
            text_source,
            paragraph_properties,
            options.text_trimming,
            options.max_width,
            options.max_height,
            options.max_lines,
            options.text_run_cache,
            styles,
        )
    }

    /// Initializes a new instance of the `TextLayout` class.
    ///
    /// * `text_source` — the text source.
    /// * `paragraph_properties` — the default text paragraph properties.
    /// * `text_trimming` — the text trimming (upstream default: `None`).
    /// * `max_width` — the maximum width (upstream default: positive infinity).
    /// * `max_height` — the maximum height (upstream default: positive infinity).
    /// * `max_lines` — the maximum number of text lines (upstream default: 0).
    /// * `text_run_cache` — an optional cache for shaped text runs to avoid redundant shaping.
    pub fn from_text_source(
        text_source: Rc<dyn ITextSource>,
        paragraph_properties: Rc<dyn TextParagraphProperties>,
        text_trimming: Option<Rc<dyn TextTrimming>>,
        max_width: f64,
        max_height: f64,
        max_lines: i32,
        text_run_cache: Option<Rc<TextRunCache>>,
    ) -> Self {
        Self::from_text_source_with_paragraph_styles(text_source, paragraph_properties, text_trimming,
            max_width, max_height, max_lines, text_run_cache, None)
    }

    /// Formats a custom text source with paragraph overrides. A style is chosen
    /// at each required line break, never at a soft wrap.
    #[allow(clippy::too_many_arguments)]
    pub fn from_text_source_with_paragraph_styles(
        text_source: Rc<dyn ITextSource>,
        paragraph_properties: Rc<dyn TextParagraphProperties>,
        text_trimming: Option<Rc<dyn TextTrimming>>,
        max_width: f64, max_height: f64, max_lines: i32,
        text_run_cache: Option<Rc<TextRunCache>>,
        paragraph_styles: Option<TextParagraphStyles>,
    ) -> Self {
        crate::perf_count!(TextLayoutsCreated);
        let mut text_layout = Self {
            text_source,
            paragraph_properties,
            text_trimming: text_trimming.unwrap_or_else(<dyn TextTrimming>::none),
            text_lines: Vec::new(),
            paragraph_styles: TextParagraphStyle::normalize(paragraph_styles),
            line_placements: Vec::new(),
            metrics: CachedMetrics::default(),
            text_run_cache,
            text_source_length: 0,
            max_width,
            max_height,
            max_lines,
        };

        text_layout.text_lines = text_layout.create_text_lines();

        text_layout
    }

    /// Gets the height of each line of text.
    ///
    /// A value of NaN (equivalent to an attribute value of "Auto") indicates
    /// that the line height is determined automatically from the current font
    /// characteristics. The default is NaN.
    pub fn line_height(&self) -> f64 {
        self.paragraph_properties.line_height()
    }

    /// Gets the maximum width.
    pub fn max_width(&self) -> f64 {
        self.max_width
    }

    /// Gets the maximum height.
    pub fn max_height(&self) -> f64 {
        self.max_height
    }

    /// Gets the maximum number of text lines.
    pub fn max_lines(&self) -> i32 {
        self.max_lines
    }

    /// Gets the text spacing.
    pub fn letter_spacing(&self) -> f64 {
        self.paragraph_properties.letter_spacing()
    }

    /// Gets the text lines.
    pub fn text_lines(&self) -> &[Rc<dyn TextLine>] {
        &self.text_lines
    }

    /// The actual line origin used for drawing, hit testing and native carets.
    pub fn line_origin(&self, index: usize) -> Point {
        self.line_placements.get(index).map_or(Point::default(), |line| line.origin)
    }

    /// The base flow direction of the paragraph containing this line.
    pub fn line_flow_direction(&self, index: usize) -> FlowDirection {
        self.line_placements.get(index).map_or_else(|| self.paragraph_properties.flow_direction(),
            |line| line.properties.flow_direction())
    }

    /// The distance from the top of the first line to the bottom of the last line.
    pub fn height(&self) -> f64 {
        self.metrics.height
    }

    /// The distance from the topmost black pixel of the first line to the
    /// bottommost black pixel of the last line.
    pub fn extent(&self) -> f64 {
        self.metrics.extent
    }

    /// The distance from the top of the first line to the baseline of the first line.
    pub fn baseline(&self) -> f64 {
        self.metrics.baseline
    }

    /// The distance from the bottom of the last line to the extent bottom.
    pub fn overhang_after(&self) -> f64 {
        self.metrics.overhang_after
    }

    /// The maximum distance from the leading black pixel to the leading alignment point of a line.
    pub fn overhang_leading(&self) -> f64 {
        self.metrics.overhang_leading
    }

    /// The maximum distance from the trailing black pixel to the trailing alignment point of a line.
    pub fn overhang_trailing(&self) -> f64 {
        self.metrics.overhang_trailing
    }

    /// The maximum advance width between the leading and trailing alignment
    /// points of a line, excluding the width of whitespace characters at the
    /// end of the line.
    pub fn width(&self) -> f64 {
        self.metrics.width
    }

    /// The maximum advance width between the leading and trailing alignment
    /// points of a line, including the width of whitespace characters at the
    /// end of the line.
    pub fn width_including_trailing_whitespace(&self) -> f64 {
        self.metrics.width_including_trailing_whitespace
    }

    /// Draws the text layout.
    ///
    /// * `context` — the drawing context.
    /// * `origin` — the origin.
    pub fn draw(&self, context: &mut dyn ITextDrawingSink, origin: Point) {
        if self.text_lines.is_empty() {
            return;
        }

        for (index, text_line) in self.text_lines.iter().enumerate() {
            let offset = self.line_origin(index);
            text_line.draw(context, Point::new(origin.x + offset.x, origin.y + offset.y));
        }
    }

    /// Obtains the scalable contours of this already shaped layout at `origin`.
    ///
    /// The geometry follows the same line positions, bidi ordering, style runs,
    /// letter spacing and decoration strokes as [`Self::draw`], without shaping
    /// or measuring the text again. An empty layout produces an empty geometry.
    /// Glyph paints are not retained; callers can fill or stroke the returned
    /// geometry. Bitmap-only glyphs contribute no contours, and colour glyphs
    /// contribute only outlines exposed by the platform glyph-run contract.
    pub fn build_geometry(&self, origin: Point) -> crate::Ref<crate::media::Geometry> {
        let mut sink = super::text_geometry_sink::TextGeometrySink::new();
        self.draw(&mut sink, origin);
        sink.into_geometry()
    }

    /// Obtains all contours at `origin` without accepting a partial glyph
    /// outline. Positions and decorations are the same as `build_geometry`.
    ///
    /// Returns `None` when the platform cannot verify completeness or any run
    /// contains bitmap or colour-font glyphs. Glyph paints are not represented
    /// by geometry; callers must preserve or validate those separately before
    /// replacing text with a path. An empty layout succeeds with empty geometry.
    pub fn try_build_geometry(&self, origin: Point) -> Option<crate::Ref<crate::media::Geometry>> {
        let mut sink = super::text_geometry_sink::TextGeometrySink::complete();
        self.draw(&mut sink, origin);
        sink.into_complete_geometry()
    }

    /// Get the pixel location relative to the top-left of the layout box given the text position.
    ///
    /// * `text_position` — the text position.
    pub fn hit_test_text_position(&self, mut text_position: i32) -> Rect {
        if self.text_lines.is_empty() {
            return Rect::default();
        }

        if text_position < 0 {
            text_position = self.text_source_length;
        }

        for (i, text_line) in self.text_lines.iter().enumerate() {
            let end = text_line.first_text_source_index() + text_line.length();

            if end <= text_position && i + 1 < self.text_lines.len() {
                continue;
            }

            let character_hit = CharacterHit::new(text_position);

            let start_x = text_line.get_distance_from_character_hit(character_hit);

            let next_character_hit = text_line.get_next_caret_character_hit(character_hit);

            let end_x = text_line.get_distance_from_character_hit(next_character_hit);

            let origin = self.line_origin(i);
            return Rect::new(origin.x + start_x, origin.y, end_x - start_x, text_line.height());
        }

        Rect::default()
    }

    /// Gets the bounding rectangles of a range of text, relative to the
    /// top-left of the layout box.
    pub fn hit_test_text_range(&self, mut start: i32, mut length: i32) -> Vec<Rect> {
        if start + length <= 0 {
            return Vec::new();
        }

        let mut result: Vec<Rect> = Vec::with_capacity(self.text_lines.len());

        for (index, text_line) in self.text_lines.iter().enumerate() {
            let origin = self.line_origin(index);
            // Current line isn't covered.
            if text_line.first_text_source_index() + text_line.length() <= start {
                continue;
            }

            let text_bounds = text_line.get_text_bounds(start, length);

            for bounds in &text_bounds {
                let rectangle = bounds.rectangle();
                let rectangle = Rect::new(rectangle.x + origin.x, rectangle.y + origin.y,
                    rectangle.width, rectangle.height);

                match result.last_mut() {
                    Some(last)
                        if MathUtilities::are_close(last.right(), rectangle.left())
                            && MathUtilities::are_close(last.top(), rectangle.top()) =>
                    {
                        *last = last.with_width(last.width + rectangle.width);
                    }
                    _ => {
                        result.push(rectangle);
                    }
                }

                for run_bounds in bounds.text_run_bounds() {
                    start += run_bounds.length();
                    length -= run_bounds.length();
                }
            }

            if text_line.first_text_source_index() + text_line.length() >= start + length {
                break;
            }
        }

        result
    }

    /// Gets the character hit of a point relative to the top-left of the layout box.
    pub fn hit_test_point(&self, point: Point) -> TextHitTestResult {
        if self.text_lines.is_empty() {
            return TextHitTestResult::default();
        }
        let index = self.text_lines.iter().enumerate()
            .find(|(index, line)| self.line_origin(*index).y + line.height() > point.y)
            .map_or(self.text_lines.len() - 1, |(index, _)| index);
        let line = &self.text_lines[index];
        let origin = self.line_origin(index);
        let relative = Point::new(point.x - origin.x, point.y - origin.y);
        let hit = line.get_character_hit_from_distance(relative.x);
        self.get_hit_test_result(&**line, hit, relative, self.line_flow_direction(index), point.y > self.height())
    }

    /// Gets the index of the line that contains a character index.
    pub fn get_line_index_from_character_index(&self, char_index: i32, trailing_edge: bool) -> i32 {
        if char_index < 0 {
            return 0;
        }

        if char_index > self.text_source_length {
            return self.text_lines.len() as i32 - 1;
        }

        for (index, text_line) in self.text_lines.iter().enumerate() {
            if text_line.first_text_source_index() + text_line.length() < char_index {
                continue;
            }

            if char_index >= text_line.first_text_source_index()
                && char_index
                    <= text_line.first_text_source_index() + text_line.length() - if trailing_edge { 0 } else { 1 }
            {
                return index as i32;
            }
        }

        self.text_lines.len() as i32 - 1
    }

    fn get_hit_test_result(
        &self,
        text_line: &dyn TextLine,
        mut character_hit: CharacterHit,
        point: Point,
        flow_direction: FlowDirection,
        below_layout: bool,
    ) -> TextHitTestResult {
        let (x, y) = (point.x - text_line.start(), point.y);

        let is_inside = x >= 0.0 && x <= text_line.width() && y >= 0.0 && y <= text_line.height();

        let mut last_trailing_index = 0;

        let text_end_of_line_length = text_line.text_line_break().and_then(|text_line_break| {
            text_line_break
                .text_end_of_line()
                .and_then(|text_end_of_line| text_end_of_line.as_text_end_of_line())
                .map(|text_end_of_line| text_end_of_line.length())
        });

        if flow_direction == FlowDirection::LeftToRight {
            last_trailing_index = text_line.first_text_source_index() + text_line.length();

            if x >= text_line.width() && text_line.length() > 0 && text_line.new_line_length() > 0 {
                last_trailing_index -= text_line.new_line_length();
            }

            if let Some(text_end_of_line_length) = text_end_of_line_length {
                last_trailing_index -= text_end_of_line_length;
            }
        } else {
            if x <= text_line.width_including_trailing_whitespace() - text_line.width()
                && text_line.length() > 0
                && text_line.new_line_length() > 0
            {
                last_trailing_index += text_line.new_line_length();
            }

            if let Some(text_end_of_line_length) = text_end_of_line_length {
                last_trailing_index += text_end_of_line_length;
            }
        }

        let mut text_position = character_hit.first_character_index() + character_hit.trailing_length();

        let is_trailing =
            last_trailing_index == text_position && character_hit.trailing_length() > 0 || below_layout;

        if text_position == text_line.first_text_source_index() + text_line.length() {
            text_position -= text_line.new_line_length();
        }

        if text_line.new_line_length() > 0
            && text_position + text_line.new_line_length()
                == character_hit.first_character_index() + character_hit.trailing_length()
        {
            character_hit = CharacterHit::new(character_hit.first_character_index());
        }

        TextHitTestResult::new(character_hit, text_position, is_inside, is_trailing)
    }

    /// Creates the default `TextParagraphProperties` that are used by the `TextFormatter`.
    ///
    /// * `typeface` — the typeface.
    /// * `font_size` — the font size.
    /// * `foreground` — the foreground.
    /// * `text_alignment` — the text alignment.
    /// * `text_wrapping` — the text wrapping.
    /// * `text_decorations` — the text decorations.
    /// * `flow_direction` — the text flow direction.
    /// * `line_height` — the height of each line of text.
    /// * `letter_spacing` — the letter spacing that is applied to rendered glyphs.
    /// * `features` — optional list of turned on/off features.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_text_paragraph_properties(
        typeface: Typeface,
        font_size: f64,
        foreground: Option<Rc<dyn IBrush>>,
        text_alignment: TextAlignment,
        text_wrapping: TextWrapping,
        text_decorations: Option<TextDecorationCollection>,
        flow_direction: FlowDirection,
        line_height: f64,
        letter_spacing: f64,
        features: Option<FontFeatureCollection>,
    ) -> Rc<dyn TextParagraphProperties> {
        let text_run_style: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_all(
            typeface,
            font_size,
            text_decorations,
            foreground,
            None,
            BaselineAlignment::Baseline,
            None,
            features,
        ));

        Rc::new(GenericTextParagraphProperties::with_all(
            flow_direction,
            text_alignment,
            true,
            false,
            text_run_style,
            text_wrapping,
            line_height,
            0.0,
            letter_spacing,
        ))
    }

    fn paragraph_style_at(&self, index: i32) -> Option<&TextParagraphStyle> {
        let styles = self.paragraph_styles.as_ref()?;
        let after = styles.partition_point(|style| style.start() <= index);
        let span = styles.get(after.checked_sub(1)?)?;
        (span.start() == index || index < span.start().saturating_add(span.length()))
            .then(|| span.value())
    }

    fn required_break(line: &dyn TextLine) -> bool {
        line.new_line_length() > 0 || line.text_line_break()
            .is_some_and(|value| value.text_end_of_line().is_some())
    }

    fn create_text_lines(&mut self) -> Vec<Rc<dyn TextLine>> {
        let object_pool = FormattingObjectPool::instance();
        let mut first = true;
        let mut text_lines = object_pool.text_lines.rent();
        self.text_source_length = 0;
        self.line_placements.clear();
        let mut previous_line: Option<Rc<dyn TextLine>> = None;
        let text_formatter = <dyn TextFormatter>::current();
        let mut style = self.paragraph_style_at(0).cloned();
        let mut closed_paragraph = false;

        // Preserve the existing zero-constraint contract: an empty line is
        // measurable even though no text can be displayed.
        let zero_constraint = MathUtilities::is_zero(self.max_width) || MathUtilities::is_zero(self.max_height);

        loop {
            let first_in_paragraph = previous_line.as_ref().is_none_or(|line| Self::required_break(&**line));
            if first_in_paragraph { style = self.paragraph_style_at(self.text_source_length).cloned(); }
            let properties = style.as_ref().map_or_else(|| self.paragraph_properties.clone(),
                |style| style.properties(&self.paragraph_properties, first_in_paragraph));
            let before = if first_in_paragraph {
                style.as_ref().map_or(0.0, |style| TextParagraphStyle::nonnegative(style.space_before))
            } else { 0.0 };
            let indent = if first_in_paragraph {
                style.as_ref().map_or(0.0, |style| TextParagraphStyle::nonnegative(style.indent))
            } else { 0.0 };
            // Do not permit an indentation larger than the column to create a
            // negative available width or a line that cannot consume text.
            let indent = if self.max_width.is_finite() { indent.min(self.max_width.max(0.0)) } else { indent };
            let width = if zero_constraint { f64::INFINITY } else { (self.max_width - indent).max(0.0) };
            let origin = Point::new(if properties.flow_direction() == FlowDirection::LeftToRight { indent } else { 0.0 },
                self.metrics.height + before);
            let after = style.as_ref().map_or(0.0, |style| TextParagraphStyle::nonnegative(style.space_after));
            let previous_line_break = previous_line.as_ref().and_then(|line| line.text_line_break());
            let candidate: Option<Rc<dyn TextLine>> = if zero_constraint {
                Some(TextFormatterImpl::create_empty_text_line(0, width, &properties))
            } else {
                text_formatter.format_line_with_cache(&*self.text_source, self.text_source_length,
                    width, &properties, previous_line_break.as_ref(), self.text_run_cache.as_deref())
                    .filter(|line| line.as_any().is::<TextLineImpl>())
            };

            let Some(mut line) = candidate else {
                if previous_line.as_ref().is_none_or(|line| line.new_line_length() > 0) {
                    let line = TextFormatterImpl::create_empty_text_line(self.text_source_length, width, &properties);
                    self.line_placements.push(LinePlacement { origin, width, properties });
                    self.update_metrics(&*line, origin, indent, &mut first);
                    self.metrics.height += after;
                    text_lines.push(line);
                } else if !closed_paragraph {
                    self.metrics.height += after;
                }
                break;
            };
            self.text_source_length += line.length();
            let has_real_content = line.length() > line.new_line_length();

            if !text_lines.is_empty() && self.max_height != f64::INFINITY
                && MathUtilities::greater_than(origin.y + line.height(), self.max_height)
            {
                let last = text_lines.len() - 1;
                if !self.is_trimming_none() && has_real_content
                    && text_lines[last].text_line_break().is_some_and(|value| value.is_split())
                {
                    text_lines[last] = self.collapse_for_truncation(text_lines[last].clone(),
                        &self.line_placements[last].properties);
                }
                break;
            }
            if self.max_lines > 0 && text_lines.len() as i32 >= self.max_lines {
                if !self.is_trimming_none() && has_real_content {
                    let last = text_lines.len() - 1;
                    text_lines[last] = self.collapse_for_truncation(text_lines[last].clone(),
                        &self.line_placements[last].properties);
                }
                break;
            }
            if line.has_overflowed() && !self.is_trimming_none() {
                line = line.collapse(&[self.get_collapsing_properties(width, &properties)]);
            }

            self.line_placements.push(LinePlacement { origin, width, properties });
            self.update_metrics(&*line, origin, indent, &mut first);
            closed_paragraph = Self::required_break(&*line);
            if closed_paragraph { self.metrics.height += after; }
            text_lines.push(line.clone());
            previous_line = Some(line.clone());
            let end_of_source = line.text_line_break().is_some_and(|value| value.text_end_of_line()
                .is_some_and(|value| value.is::<TextEndOfParagraph>()));
            if end_of_source || zero_constraint {
                if !closed_paragraph { self.metrics.height += after; }
                break;
            }
        }

        // Each paragraph may choose its own alignment and its first line may
        // have a different available width. Hard-break and final lines retain
        // their start alignment, as with the existing whole-layout formatter.
        for (index, line) in text_lines.iter().enumerate() {
            let placement = &self.line_placements[index];
            if placement.properties.text_alignment() != TextAlignment::Justify
                || !placement.width.is_finite() || placement.width <= 0.0
                || index + 1 == text_lines.len() || Self::required_break(&**line) { continue; }
            line.justify(&InterWordJustification::new(placement.width));
        }
        let result = text_lines.to_vec();
        object_pool.text_lines.return_list(text_lines);
        result
    }

    fn update_metrics(&mut self, current_line: &dyn TextLine, origin: Point, indent: f64, first: &mut bool) {
        let metrics = &mut self.metrics;

        // 1) Accumulate total layout height by adding the line's Height.
        metrics.height = origin.y + current_line.height();

        // 2) For the layout's Width and WidthIncludingTrailingWhitespace,
        //    use the maximum of the line widths rather than the bounding box.
        metrics.width = metrics.width.max(indent + current_line.width());

        // 3) Extent is the max black-pixel extent among lines.
        metrics.extent = metrics.extent.max(current_line.extent());

        // 4) TextWidth is the max of the text width among lines.
        // We choose to update all related metrics at once (OverhangLeading,
        // WidthIncludingTrailingWhitespace, OverhangTrailing) if the current line has a
        // larger text width.
        let previous_text_width = metrics.width_including_trailing_whitespace;
        let text_width = indent + current_line.width_including_trailing_whitespace();

        if previous_text_width < text_width {
            metrics.width_including_trailing_whitespace = text_width;
            metrics.overhang_leading = current_line.overhang_leading();
            metrics.overhang_trailing = current_line.overhang_trailing();
        }

        // 5) OverhangAfter is the last line's OverhangAfter.
        metrics.overhang_after = current_line.overhang_after();

        // 6) Capture the baseline from the first line.
        if *first {
            metrics.baseline = origin.y + current_line.baseline();
            *first = false;
        }
    }

    /// `_textTrimming == TextTrimming.None`.
    fn is_trimming_none(&self) -> bool {
        Rc::ptr_eq(&self.text_trimming, &<dyn TextTrimming>::none())
    }

    /// Gets the `TextCollapsingProperties` for current text trimming mode.
    ///
    /// * `width` — the collapsing width.
    fn get_collapsing_properties(&self, width: f64, properties: &Rc<dyn TextParagraphProperties>) -> Option<Rc<dyn TextCollapsingProperties>> {
        if self.is_trimming_none() {
            return None;
        }

        Some(self.text_trimming.create_collapsing_properties(&TextCollapsingCreateInfo::new(
            width,
            properties.default_text_run_properties().clone(),
            properties.flow_direction(),
        )))
    }

    fn collapse_for_truncation(&self, line: Rc<dyn TextLine>, properties: &Rc<dyn TextParagraphProperties>) -> Rc<dyn TextLine> {
        // Collapse against the line's own rendered width rather than max_width, so there's
        // always room reserved for the ellipsis symbol and a trailing ellipsis is reliably
        // produced, even if the line already fits within max_width on its own.
        let collapsing_properties = self.get_collapsing_properties(line.width_including_trailing_whitespace(), properties);

        if collapsing_properties.is_none() {
            return line;
        }

        line.collapse(&[collapsing_properties])
    }

    /// Releases the text lines.
    pub fn dispose(&self) {
        for line in &self.text_lines {
            line.dispose();
        }
    }
}

struct LinePlacement {
    origin: Point,
    width: f64,
    properties: Rc<dyn TextParagraphProperties>,
}

#[derive(Default)]
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
