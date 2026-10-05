use std::rc::Rc;

use crate::media::text_formatting::{
    FormattedTextSource, FormattingObjectPool, GenericTextParagraphProperties, GenericTextRunProperties,
    ITextDrawingSink, ITextSource, InterWordJustification, TextCollapsingProperties, TextEndOfParagraph,
    TextFormatter, TextFormatterImpl, TextLine, TextLineImpl, TextParagraphProperties, TextRun, TextRunCache,
    TextRunProperties,
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

    /// Initializes a new instance of the `TextLayout` class from UTF-16 text
    /// (no transcoding).
    pub fn from_utf16(text: ReadOnlyMemory<u16>, typeface: Typeface, options: TextLayoutOptions) -> Self {
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

        Self::from_text_source(
            text_source,
            paragraph_properties,
            options.text_trimming,
            options.max_width,
            options.max_height,
            options.max_lines,
            options.text_run_cache,
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
        let mut text_layout = Self {
            text_source,
            paragraph_properties,
            text_trimming: text_trimming.unwrap_or_else(<dyn TextTrimming>::none),
            text_lines: Vec::new(),
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

        let (current_x, mut current_y) = (origin.x, origin.y);

        for text_line in &self.text_lines {
            text_line.draw(context, Point::new(current_x, current_y));

            current_y += text_line.height();
        }
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

        let mut current_y = 0.0;

        for (i, text_line) in self.text_lines.iter().enumerate() {
            let end = text_line.first_text_source_index() + text_line.length();

            if end <= text_position && i + 1 < self.text_lines.len() {
                current_y += text_line.height();

                continue;
            }

            let character_hit = CharacterHit::new(text_position);

            let start_x = text_line.get_distance_from_character_hit(character_hit);

            let next_character_hit = text_line.get_next_caret_character_hit(character_hit);

            let end_x = text_line.get_distance_from_character_hit(next_character_hit);

            return Rect::new(start_x, current_y, end_x - start_x, text_line.height());
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

        let mut current_y = 0.0;

        for text_line in &self.text_lines {
            // Current line isn't covered.
            if text_line.first_text_source_index() + text_line.length() <= start {
                current_y += text_line.height();

                continue;
            }

            let text_bounds = text_line.get_text_bounds(start, length);

            for bounds in &text_bounds {
                let rectangle = bounds.rectangle();

                match result.last_mut() {
                    Some(last)
                        if MathUtilities::are_close(last.right(), rectangle.left())
                            && MathUtilities::are_close(last.top(), current_y) =>
                    {
                        *last = last.with_width(last.width + rectangle.width);
                    }
                    _ => {
                        result.push(rectangle.with_y(current_y));
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

            current_y += text_line.height();
        }

        result
    }

    /// Gets the character hit of a point relative to the top-left of the layout box.
    pub fn hit_test_point(&self, point: Point) -> TextHitTestResult {
        let mut current_y = 0.0;

        let mut current_line: Option<&Rc<dyn TextLine>> = None;

        for text_line in &self.text_lines {
            current_line = Some(text_line);

            if current_y + text_line.height() > point.y {
                let character_hit = text_line.get_character_hit_from_distance(point.x);

                return self.get_hit_test_result(&**text_line, character_hit, point);
            }

            current_y += text_line.height();
        }

        let Some(current_line) = current_line else {
            return TextHitTestResult::default();
        };

        let character_hit = current_line.get_character_hit_from_distance(point.x);

        self.get_hit_test_result(&**current_line, character_hit, point)
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
    ) -> TextHitTestResult {
        let (x, y) = (point.x, point.y);

        let is_inside = x >= 0.0 && x <= text_line.width() && y >= 0.0 && y <= text_line.height();

        let mut last_trailing_index = 0;

        let text_end_of_line_length = text_line.text_line_break().and_then(|text_line_break| {
            text_line_break
                .text_end_of_line()
                .and_then(|text_end_of_line| text_end_of_line.as_text_end_of_line())
                .map(|text_end_of_line| text_end_of_line.length())
        });

        if self.paragraph_properties.flow_direction() == FlowDirection::LeftToRight {
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
            last_trailing_index == text_position && character_hit.trailing_length() > 0 || y > self.height();

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

    fn create_text_lines(&mut self) -> Vec<Rc<dyn TextLine>> {
        let object_pool = FormattingObjectPool::instance();

        let mut first = true;

        if MathUtilities::is_zero(self.max_width) || MathUtilities::is_zero(self.max_height) {
            let text_line: Rc<dyn TextLine> =
                TextFormatterImpl::create_empty_text_line(0, f64::INFINITY, &self.paragraph_properties);

            self.update_metrics(&*text_line, &mut first);

            return vec![text_line];
        }

        let mut text_lines = object_pool.text_lines.rent();

        self.text_source_length = 0;

        let mut previous_line: Option<Rc<dyn TextLine>> = None;

        let text_formatter = <dyn TextFormatter>::current();

        loop {
            let previous_line_break = previous_line.as_ref().and_then(|line| line.text_line_break());

            // Upstream: `FormatLine(..) as TextLineImpl`.
            let text_line = text_formatter
                .format_line_with_cache(
                    &*self.text_source,
                    self.text_source_length,
                    self.max_width,
                    &self.paragraph_properties,
                    previous_line_break.as_ref(),
                    self.text_run_cache.as_deref(),
                )
                .filter(|text_line| text_line.as_any().is::<TextLineImpl>());

            let Some(mut text_line) = text_line else {
                if previous_line.as_ref().is_some_and(|previous_line| previous_line.new_line_length() > 0) {
                    let empty_text_line: Rc<dyn TextLine> = TextFormatterImpl::create_empty_text_line(
                        self.text_source_length,
                        self.max_width,
                        &self.paragraph_properties,
                    );

                    text_lines.push(empty_text_line.clone());

                    self.update_metrics(&*empty_text_line, &mut first);
                }

                break;
            };

            self.text_source_length += text_line.length();

            // Fulfill max height constraint
            if !text_lines.is_empty()
                && self.max_height != f64::INFINITY
                && MathUtilities::greater_than(self.height() + text_line.height(), self.max_height)
            {
                if let Some(previous_line) = &previous_line {
                    if previous_line.text_line_break().is_some() && !self.is_trimming_none() {
                        let collapsed_line =
                            previous_line.clone().collapse(&[self.get_collapsing_properties(self.max_width)]);

                        let last = text_lines.len() - 1;

                        text_lines[last] = collapsed_line;
                    }
                }

                break;
            }

            let has_overflowed = text_line.has_overflowed();

            if has_overflowed && !self.is_trimming_none() {
                text_line = text_line.collapse(&[self.get_collapsing_properties(self.max_width)]);
            }

            text_lines.push(text_line.clone());

            self.update_metrics(&*text_line, &mut first);

            previous_line = Some(text_line.clone());

            // Fulfill max lines constraint
            if self.max_lines > 0 && text_lines.len() as i32 >= self.max_lines {
                if text_line.text_line_break().is_some_and(|text_line_break| text_line_break.is_split()) {
                    let collapsed_line = text_line.clone().collapse(&[
                        self.get_collapsing_properties(text_line.width_including_trailing_whitespace())
                    ]);

                    let last = text_lines.len() - 1;

                    text_lines[last] = collapsed_line;
                }

                break;
            }

            let is_end_of_paragraph = text_line.text_line_break().is_some_and(|text_line_break| {
                text_line_break
                    .text_end_of_line()
                    .is_some_and(|text_end_of_line| text_end_of_line.is::<TextEndOfParagraph>())
            });

            if is_end_of_paragraph {
                break;
            }
        }

        if text_lines.is_empty() {
            let text_line: Rc<dyn TextLine> =
                TextFormatterImpl::create_empty_text_line(0, self.max_width, &self.paragraph_properties);

            text_lines.push(text_line.clone());

            self.update_metrics(&*text_line, &mut first);
        }

        if self.paragraph_properties.text_alignment() == TextAlignment::Justify {
            // Justify fills each line to the column width, which is MaxWidth for both
            // wrapped and non-wrapped text. Targeting the widest produced line instead
            // (the previous behaviour for wrapping) leaves wrapped text short of the
            // margin, since the full non-last lines already equal that width. When
            // MaxWidth is infinite there is no column to fill, so skip.
            let justification_width = self.max_width;

            if !justification_width.is_infinite() && justification_width > 0.0 {
                let justification_properties = InterWordJustification::new(justification_width);

                for (i, line) in text_lines.iter().enumerate() {
                    // Only width-driven wrapped lines are stretched to the column. Skip
                    // the last line of the layout, any line ended by a newline
                    // (NewLineLength > 0), and any line ended by a source-provided required
                    // break (TextEndOfLine) - these stay start-aligned per standard
                    // typographic convention.
                    if i == text_lines.len() - 1
                        || line.new_line_length() > 0
                        || line
                            .text_line_break()
                            .is_some_and(|text_line_break| text_line_break.text_end_of_line().is_some())
                    {
                        continue;
                    }

                    line.justify(&justification_properties);
                }
            }
        }

        let result = text_lines.to_vec();

        object_pool.text_lines.return_list(text_lines);

        result
    }

    fn update_metrics(&mut self, current_line: &dyn TextLine, first: &mut bool) {
        let metrics = &mut self.metrics;

        // 1) Accumulate total layout height by adding the line's Height.
        metrics.height += current_line.height();

        // 2) For the layout's Width and WidthIncludingTrailingWhitespace,
        //    use the maximum of the line widths rather than the bounding box.
        metrics.width = metrics.width.max(current_line.width());

        // 3) Extent is the max black-pixel extent among lines.
        metrics.extent = metrics.extent.max(current_line.extent());

        // 4) TextWidth is the max of the text width among lines.
        // We choose to update all related metrics at once (OverhangLeading,
        // WidthIncludingTrailingWhitespace, OverhangTrailing) if the current line has a
        // larger text width.
        let previous_text_width = metrics.width_including_trailing_whitespace;
        let text_width = current_line.width_including_trailing_whitespace();

        if previous_text_width < text_width {
            metrics.width_including_trailing_whitespace = current_line.width_including_trailing_whitespace();
            metrics.overhang_leading = current_line.overhang_leading();
            metrics.overhang_trailing = current_line.overhang_trailing();
        }

        // 5) OverhangAfter is the last line's OverhangAfter.
        metrics.overhang_after = current_line.overhang_after();

        // 6) Capture the baseline from the first line.
        if *first {
            metrics.baseline = current_line.baseline();
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
    fn get_collapsing_properties(&self, width: f64) -> Option<Rc<dyn TextCollapsingProperties>> {
        if self.is_trimming_none() {
            return None;
        }

        Some(self.text_trimming.create_collapsing_properties(&TextCollapsingCreateInfo::new(
            width,
            self.paragraph_properties.default_text_run_properties().clone(),
            self.paragraph_properties.flow_direction(),
        )))
    }

    /// Releases the text lines.
    pub fn dispose(&self) {
        for line in &self.text_lines {
            line.dispose();
        }
    }
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
