use super::{
    Block, Inline, LineSpacingRule, LineStackingStrategy, Paragraph, TabAlignment, TabLeader,
    TextElement, TextTabStop, TextTabStops, TypographyFormat,
};
use ferroui_base::media::{
    BaselineAlignment, FlowDirection, FontFamily, FontFeatureCollection, FontStretch, FontStyle,
    FontWeight, IBrush, MediaCollection, TextAlignment, TextDecoration, TextDecorationCollection,
};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{Ref, Thickness};
use std::rc::Rc;

/// A character-format patch, also used to capture the assigned formatting of an
/// element. `None` leaves a property unchanged. For nullable properties,
/// `Some(None)` explicitly clears it. Snapshots copy mutable formatting collections.
#[derive(Default)]
pub struct CharacterFormat {
    pub language: Option<CultureInfo>,
    pub font_family: Option<FontFamily>,
    pub font_size: Option<f64>,
    pub font_style: Option<FontStyle>,
    pub font_weight: Option<FontWeight>,
    pub font_stretch: Option<FontStretch>,
    pub font_features: Option<Option<FontFeatureCollection>>,
    /// Each assigned typography field is patched independently. Effective
    /// formats contain all resolved values, including explicit defaults.
    pub typography: TypographyFormat,
    pub foreground: Option<Option<Rc<dyn IBrush>>>,
    pub background: Option<Option<Rc<dyn IBrush>>>,
    pub text_decorations: Option<Option<TextDecorationCollection>>,
    pub baseline_alignment: Option<BaselineAlignment>,
    pub letter_spacing: Option<f64>,
}

impl std::fmt::Debug for CharacterFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CharacterFormat")
            .field("language", &self.language)
            .field("font_family", &self.font_family)
            .field("font_size", &self.font_size)
            .field("font_style", &self.font_style)
            .field("font_weight", &self.font_weight)
            .field("font_stretch", &self.font_stretch)
            .field("typography", &self.typography)
            .field("foreground", &self.foreground)
            .field("background", &self.background)
            .field("baseline_alignment", &self.baseline_alignment)
            .field("letter_spacing", &self.letter_spacing)
            .field(
                "font_features",
                &self
                    .font_features
                    .as_ref()
                    .map(|v| v.as_ref().map(|v| v.list().snapshot())),
            )
            .field(
                "text_decorations",
                &self
                    .text_decorations
                    .as_ref()
                    .map(|v| v.as_ref().map(|v| v.list().snapshot())),
            )
            .finish()
    }
}

impl Clone for CharacterFormat {
    fn clone(&self) -> Self {
        Self {
            language: self.language.clone(),
            font_family: self.font_family.clone(),
            font_size: self.font_size,
            font_style: self.font_style,
            font_weight: self.font_weight,
            font_stretch: self.font_stretch,
            typography: self.typography.clone(),
            font_features: self
                .font_features
                .as_ref()
                .map(|v| v.as_ref().map(copy_features)),
            foreground: self
                .foreground
                .as_ref()
                .map(|v| v.as_ref().map(freeze_brush)),
            background: self
                .background
                .as_ref()
                .map(|v| v.as_ref().map(freeze_brush)),
            text_decorations: self
                .text_decorations
                .as_ref()
                .map(|v| v.as_ref().map(copy_decorations)),
            baseline_alignment: self.baseline_alignment,
            letter_spacing: self.letter_spacing,
        }
    }
}

impl PartialEq for CharacterFormat {
    fn eq(&self, other: &Self) -> bool {
        self.language == other.language
            && self.font_family == other.font_family
            && self.font_size == other.font_size
            && self.font_style == other.font_style
            && self.font_weight == other.font_weight
            && self.font_stretch == other.font_stretch
            && self.typography == other.typography
            && self.foreground == other.foreground
            && self.background == other.background
            && self.baseline_alignment == other.baseline_alignment
            && self.letter_spacing == other.letter_spacing
            && nested_eq(&self.font_features, &other.font_features, |a, b| {
                a.list().snapshot() == b.list().snapshot()
            })
            && nested_eq(
                &self.text_decorations,
                &other.text_decorations,
                decorations_equal,
            )
    }
}

fn nested_eq<T>(
    a: &Option<Option<T>>,
    b: &Option<Option<T>>,
    compare: impl Fn(&T, &T) -> bool,
) -> bool {
    match (a, b) {
        (None, None) | (Some(None), Some(None)) => true,
        (Some(Some(a)), Some(Some(b))) => compare(a, b),
        _ => false,
    }
}

impl CharacterFormat {
    /// Captures assigned properties without flattening inherited span formatting.
    pub fn from_element(element: &TextElement) -> Self {
        Self {
            language: element.get_base_value(TextElement::language_property()),
            font_family: element.get_base_value(TextElement::font_family_property()),
            font_size: element.get_base_value(TextElement::font_size_property()),
            font_style: element.get_base_value(TextElement::font_style_property()),
            font_weight: element.get_base_value(TextElement::font_weight_property()),
            font_stretch: element.get_base_value(TextElement::font_stretch_property()),
            typography: TypographyFormat::from_object(element),
            font_features: element
                .get_base_value(TextElement::font_features_property())
                .map(|v| v.as_ref().map(copy_features)),
            foreground: element
                .get_base_value(TextElement::foreground_property())
                .map(|v| v.as_ref().map(freeze_brush)),
            background: element
                .get_base_value(TextElement::background_property())
                .map(|v| v.as_ref().map(freeze_brush)),
            text_decorations: element
                .get_base_value(Inline::text_decorations_property())
                .map(|v| v.as_ref().map(copy_decorations)),
            baseline_alignment: element.get_base_value(Inline::baseline_alignment_property()),
            letter_spacing: element.get_base_value(TextElement::letter_spacing_property()),
        }
    }

    /// Captures computed values, including formatting inherited through logical parents.
    pub fn effective(element: &TextElement) -> Self {
        Self {
            language: Some(element.language()),
            font_family: Some(element.font_family()),
            font_size: Some(element.font_size()),
            font_style: Some(element.font_style()),
            font_weight: Some(element.font_weight()),
            font_stretch: Some(element.font_stretch()),
            typography: TypographyFormat::effective(element),
            font_features: Some(element.font_features().as_ref().map(copy_features)),
            foreground: Some(element.foreground().as_ref().map(freeze_brush)),
            background: Some(element.background().as_ref().map(freeze_brush)),
            text_decorations: Some(
                element
                    .get_value(Inline::text_decorations_property())
                    .as_ref()
                    .map(copy_decorations),
            ),
            baseline_alignment: Some(element.get_value(Inline::baseline_alignment_property())),
            letter_spacing: Some(element.letter_spacing()),
        }
    }

    /// Returns this format with every assigned field of `patch` replaced.
    pub fn overlay(&self, patch: &Self) -> Self {
        let mut result = self.clone();
        macro_rules! overlay {
            ($($field:ident),* $(,)?) => { $(if patch.$field.is_some() { result.$field = patch.$field.clone(); })* };
        }
        overlay!(
            language,
            font_family,
            font_size,
            font_style,
            font_weight,
            font_stretch,
            font_features,
            foreground,
            background,
            text_decorations,
            baseline_alignment,
            letter_spacing
        );
        result.typography = self.typography.overlay(&patch.typography);
        result
    }

    pub fn validate(&self) -> Result<(), super::DocumentError> {
        if self
            .language
            .as_ref()
            .is_some_and(|language| !super::text_language::is_valid_language_tag(language.name()))
        {
            return Err(super::DocumentError::InvalidFormat(
                "Language must be a valid XML language tag",
            ));
        }
        if self.font_size.is_some_and(|v| !v.is_finite() || v <= 0.0) {
            return Err(super::DocumentError::InvalidFormat(
                "FontSize must be finite and positive",
            ));
        }
        if self.letter_spacing.is_some_and(|v| !v.is_finite()) {
            return Err(super::DocumentError::InvalidFormat(
                "LetterSpacing must be finite",
            ));
        }
        Ok(())
    }

    pub fn try_apply_to(&self, element: &TextElement) -> Result<(), super::DocumentError> {
        self.validate()?;
        self.apply_to(element);
        Ok(())
    }

    pub(crate) fn apply_to(&self, element: &TextElement) {
        self.typography.apply_to(element);
        if let Some(language) = &self.language {
            element.set_language(language.clone());
        }
        if let Some(value) = &self.font_family {
            element.set_font_family(value.clone());
        }
        if let Some(value) = self.font_size {
            element.set_font_size(value);
        }
        if let Some(value) = self.font_style {
            element.set_font_style(value);
        }
        if let Some(value) = self.font_weight {
            element.set_font_weight(value);
        }
        if let Some(value) = self.font_stretch {
            element.set_font_stretch(value);
        }
        if let Some(value) = &self.font_features {
            element.set_font_features(value.as_ref().map(copy_features));
        }
        if let Some(value) = &self.foreground {
            element.set_foreground(value.as_ref().map(freeze_brush));
        }
        if let Some(value) = &self.background {
            element.set_background(value.as_ref().map(freeze_brush));
        }
        if let Some(value) = &self.text_decorations {
            element.set_value(
                Inline::text_decorations_property(),
                value.as_ref().map(copy_decorations),
            );
        }
        if let Some(value) = self.baseline_alignment {
            element.set_value(Inline::baseline_alignment_property(), value);
        }
        if let Some(value) = self.letter_spacing {
            element.set_letter_spacing(value);
        }
    }

    pub(crate) fn replace_on(&self, element: &TextElement) {
        TypographyFormat::clear_on(element);
        element.clear_value(TextElement::language_property());
        element.clear_value(TextElement::font_family_property());
        element.clear_value(TextElement::font_size_property());
        element.clear_value(TextElement::font_style_property());
        element.clear_value(TextElement::font_weight_property());
        element.clear_value(TextElement::font_stretch_property());
        element.clear_value(TextElement::font_features_property());
        element.clear_value(TextElement::foreground_property());
        element.clear_value(TextElement::background_property());
        element.clear_value(Inline::text_decorations_property());
        element.clear_value(Inline::baseline_alignment_property());
        element.clear_value(TextElement::letter_spacing_property());
        self.apply_to(element);
    }

    /// The assigned native LanguageTag value. None means that this patch leaves
    /// language unchanged; mixed range values use TextFormatValue as usual.
    pub fn language_tag(&self) -> Option<&str> {
        self.language.as_ref().map(CultureInfo::name)
    }

    /// Assigns the language without losing the original tag or modifying any
    /// other character formatting. Validation completes before this patch changes.
    pub fn set_language_tag(&mut self, tag: &str) -> Result<(), super::DocumentError> {
        if !super::text_language::is_valid_language_tag(tag) {
            return Err(super::DocumentError::InvalidFormat(
                "Language must be a valid XML language tag",
            ));
        }
        self.language = Some(CultureInfo::get_culture_info(tag));
        Ok(())
    }
}

/// Paragraph formatting and inherited character defaults, represented as a patch.
#[derive(Clone, Debug, Default)]
pub struct ParagraphFormat {
    pub character: CharacterFormat,
    pub text_alignment: Option<TextAlignment>,
    pub line_height: Option<f64>,
    pub line_stacking_strategy: Option<LineStackingStrategy>,
    pub text_indent: Option<f64>,
    pub margin: Option<Thickness>,
    pub flow_direction: Option<FlowDirection>,
    pub line_spacing_rule: Option<LineSpacingRule>,
    pub line_spacing: Option<f64>,
    pub left_indent: Option<f64>,
    pub right_indent: Option<f64>,
    pub tab_stops: Option<TextTabStops>,
    pub default_tab_stop: Option<f64>,
}

impl PartialEq for ParagraphFormat {
    fn eq(&self, other: &Self) -> bool {
        self.character == other.character
            && self.text_alignment == other.text_alignment
            && option_float_equal(self.line_height, other.line_height)
            && self.line_stacking_strategy == other.line_stacking_strategy
            && self.text_indent == other.text_indent
            && self.margin == other.margin
            && self.flow_direction == other.flow_direction
            && self.line_spacing_rule == other.line_spacing_rule
            && self.line_spacing == other.line_spacing
            && self.left_indent == other.left_indent
            && self.right_indent == other.right_indent
            && self.tab_stops == other.tab_stops
            && self.default_tab_stop == other.default_tab_stop
    }
}

impl ParagraphFormat {
    pub fn from_paragraph(paragraph: &Paragraph) -> Self {
        Self {
            character: CharacterFormat::from_element(paragraph),
            text_alignment: paragraph.get_base_value(Block::text_alignment_property()),
            line_height: paragraph.get_base_value(Block::line_height_property()),
            line_stacking_strategy: paragraph
                .get_base_value(Block::line_stacking_strategy_property()),
            text_indent: paragraph.get_base_value(Paragraph::text_indent_property()),
            margin: paragraph.get_base_value(Block::margin_property()),
            flow_direction: paragraph.get_base_value(Block::flow_direction_property()),
            line_spacing_rule: paragraph.get_base_value(Paragraph::line_spacing_rule_property()),
            line_spacing: paragraph.get_base_value(Paragraph::line_spacing_property()),
            left_indent: paragraph.get_base_value(Paragraph::left_indent_property()),
            right_indent: paragraph.get_base_value(Paragraph::right_indent_property()),
            tab_stops: paragraph.get_base_value(Paragraph::tab_stops_property()),
            default_tab_stop: paragraph.get_base_value(Paragraph::default_tab_stop_property()),
        }
    }

    /// Captures resolved paragraph and inherited character values for format queries.
    pub fn effective(paragraph: &Paragraph) -> Self {
        Self {
            character: CharacterFormat::effective(paragraph),
            text_alignment: Some(paragraph.text_alignment()),
            line_height: Some(paragraph.line_height()),
            line_stacking_strategy: Some(paragraph.line_stacking_strategy()),
            text_indent: Some(paragraph.text_indent()),
            margin: Some(paragraph.margin()),
            flow_direction: Some(paragraph.flow_direction()),
            line_spacing_rule: Some(paragraph.line_spacing_rule()),
            line_spacing: Some(paragraph.line_spacing()),
            left_indent: Some(paragraph.left_indent()),
            right_indent: Some(paragraph.right_indent()),
            tab_stops: Some(paragraph.tab_stops()),
            default_tab_stop: Some(paragraph.default_tab_stop()),
        }
    }

    /// Adds or replaces an explicit tab. TOM positions are points; paragraph
    /// properties, snapshots and the formatter retain layout units (96 per inch).
    pub fn add_tab(
        &mut self,
        position: f64,
        alignment: TabAlignment,
        leader: TabLeader,
    ) -> Result<(), super::DocumentError> {
        let stop = TextTabStop::new(position * (96.0 / 72.0), alignment, leader);
        let tabs = self
            .tab_stops
            .clone()
            .unwrap_or_default()
            .with_tab(stop)
            .map_err(|error| super::DocumentError::InvalidFormat(error.message()))?;
        self.tab_stops = Some(tabs);
        Ok(())
    }

    /// Removes explicit tabs; default equally spaced stops remain active.
    pub fn clear_all_tabs(&mut self) {
        self.tab_stops = Some(TextTabStops::default());
    }

    pub fn delete_tab(&mut self, position: f64) -> Result<(), super::DocumentError> {
        let tabs = self
            .tab_stops
            .clone()
            .unwrap_or_default()
            .without_tab(position * (96.0 / 72.0))
            .map_err(|error| super::DocumentError::InvalidFormat(error.message()))?;
        self.tab_stops = Some(tabs);
        Ok(())
    }

    pub fn tab_count(&self) -> usize {
        self.tab_stops.as_ref().map_or(0, TextTabStops::len)
    }

    /// Zero-based explicit tab query; positions are returned in TOM points.
    pub fn get_tab(
        &self,
        index: usize,
    ) -> Result<(f64, TabAlignment, TabLeader), super::DocumentError> {
        let stop = self
            .tab_stops
            .as_ref()
            .ok_or(super::DocumentError::InvalidFormat(
                "Tab index is outside the explicit tab collection",
            ))?
            .get(index)
            .map_err(|error| super::DocumentError::InvalidFormat(error.message()))?;
        Ok((stop.position * (72.0 / 96.0), stop.alignment, stop.leader))
    }

    /// Sets TOM indentation in points. Negative values are valid; paragraph
    /// margins remain nonnegative layout spacing and are stored separately.
    pub fn set_indents(
        &mut self,
        first: f64,
        left: f64,
        right: f64,
    ) -> Result<(), super::DocumentError> {
        let values = [first, left, right].map(|value| value * (96.0 / 72.0));
        if values.iter().any(|value| !value.is_finite()) {
            return Err(super::DocumentError::InvalidFormat(
                "Paragraph indents must be finite",
            ));
        }
        self.text_indent = Some(values[0]);
        self.left_indent = Some(values[1]);
        self.right_indent = Some(values[2]);
        Ok(())
    }

    /// Returns first-line, left, and right indentation in TOM points.
    pub fn get_indents(&self) -> (f64, f64, f64) {
        (
            self.text_indent.unwrap_or(0.0) * (72.0 / 96.0),
            self.left_indent.unwrap_or(0.0) * (72.0 / 96.0),
            self.right_indent.unwrap_or(0.0) * (72.0 / 96.0),
        )
    }

    /// Sets the TOM line spacing in points, converting absolute distances to
    /// layout units. Multipliers and percentages retain their native units.
    pub fn set_line_spacing(
        &mut self,
        rule: LineSpacingRule,
        spacing: f64,
    ) -> Result<(), super::DocumentError> {
        rule.validate(spacing)?;
        let spacing = if matches!(rule, LineSpacingRule::AtLeast | LineSpacingRule::Exactly) {
            spacing * (96.0 / 72.0)
        } else {
            spacing
        };
        rule.validate(spacing)?;
        self.line_spacing_rule = Some(rule);
        self.line_spacing = Some(spacing);
        Ok(())
    }

    /// Returns the TOM rule and value; absolute distances are returned in points.
    pub fn get_line_spacing(&self) -> (LineSpacingRule, f64) {
        let rule = self.line_spacing_rule.unwrap_or_default();
        let spacing = self.line_spacing.unwrap_or(1.0);
        (
            rule,
            if matches!(rule, LineSpacingRule::AtLeast | LineSpacingRule::Exactly) {
                spacing * (72.0 / 96.0)
            } else {
                spacing
            },
        )
    }

    pub fn validate(&self) -> Result<(), super::DocumentError> {
        self.character.validate()?;
        if self
            .default_tab_stop
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            return Err(super::DocumentError::InvalidFormat(
                "DefaultTabStop must be finite and positive",
            ));
        }
        self.line_spacing_rule
            .unwrap_or_default()
            .validate(self.line_spacing.unwrap_or(1.0))?;
        if [self.left_indent, self.right_indent]
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err(super::DocumentError::InvalidFormat(
                "Paragraph indents must be finite",
            ));
        }
        if self
            .line_height
            .is_some_and(|v| (!v.is_finite() && !v.is_nan()) || v < 0.0)
        {
            return Err(super::DocumentError::InvalidFormat(
                "LineHeight must be natural or nonnegative",
            ));
        }
        if self.text_indent.is_some_and(|v| !v.is_finite()) {
            return Err(super::DocumentError::InvalidFormat(
                "TextIndent must be finite",
            ));
        }
        if self.margin.is_some_and(|v| {
            [v.left, v.top, v.right, v.bottom]
                .iter()
                .any(|v| !v.is_finite() || *v < 0.0)
        }) {
            return Err(super::DocumentError::InvalidFormat(
                "Margin must be finite and nonnegative",
            ));
        }
        Ok(())
    }

    pub fn overlay(&self, patch: &Self) -> Self {
        let mut result = self.clone();
        result.character = result.character.overlay(&patch.character);
        macro_rules! overlay {
            ($($field:ident),* $(,)?) => { $(if patch.$field.is_some() { result.$field = patch.$field; })* };
        }
        overlay!(
            text_alignment,
            line_height,
            line_stacking_strategy,
            text_indent,
            margin,
            flow_direction,
            line_spacing_rule,
            line_spacing,
            left_indent,
            right_indent,
            default_tab_stop
        );
        if let Some(tabs) = &patch.tab_stops {
            result.tab_stops = Some(tabs.clone());
        }
        result
    }

    pub fn try_apply_to(&self, paragraph: &Paragraph) -> Result<(), super::DocumentError> {
        self.validate()?;
        Self::effective(paragraph).overlay(self).validate()?;
        self.apply_to(paragraph);
        Ok(())
    }

    pub(crate) fn apply_to(&self, paragraph: &Paragraph) {
        self.character.apply_to(paragraph);
        if let Some(value) = self.text_alignment {
            paragraph.set_text_alignment(value);
        }
        if let Some(value) = self.line_height {
            paragraph.set_line_height(value);
        }
        if let Some(value) = self.line_stacking_strategy {
            paragraph.set_line_stacking_strategy(value);
        }
        if let Some(value) = self.text_indent {
            paragraph.set_text_indent(value);
        }
        if let Some(value) = self.margin {
            paragraph.set_margin(value);
        }
        if let Some(value) = self.flow_direction {
            paragraph.set_flow_direction(value);
        }
        if let Some(value) = self.line_spacing {
            paragraph.set_line_spacing(value);
        }
        if let Some(value) = self.line_spacing_rule {
            paragraph.set_line_spacing_rule(value);
        }
        if let Some(value) = self.left_indent {
            paragraph.set_left_indent(value);
        }
        if let Some(value) = self.right_indent {
            paragraph.set_right_indent(value);
        }
        if let Some(tabs) = &self.tab_stops {
            paragraph.set_tab_stops(tabs.clone());
        }
        if let Some(value) = self.default_tab_stop {
            paragraph.set_default_tab_stop(value);
        }
    }

    pub(crate) fn replace_on(&self, paragraph: &Paragraph) {
        self.character.replace_on(paragraph);
        paragraph.clear_value(Block::text_alignment_property());
        paragraph.clear_value(Block::line_height_property());
        paragraph.clear_value(Block::line_stacking_strategy_property());
        paragraph.clear_value(Paragraph::text_indent_property());
        paragraph.clear_value(Block::margin_property());
        paragraph.clear_value(Block::flow_direction_property());
        paragraph.clear_value(Paragraph::line_spacing_rule_property());
        paragraph.clear_value(Paragraph::line_spacing_property());
        paragraph.clear_value(Paragraph::left_indent_property());
        paragraph.clear_value(Paragraph::right_indent_property());
        paragraph.clear_value(Paragraph::tab_stops_property());
        paragraph.clear_value(Paragraph::default_tab_stop_property());
        self.apply_to(paragraph);
    }
}

fn option_float_equal(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a == b || (a.is_nan() && b.is_nan()),
        (None, None) => true,
        _ => false,
    }
}

fn copy_features(value: &FontFeatureCollection) -> FontFeatureCollection {
    FontFeatureCollection::from_items(value.list().snapshot().iter().cloned())
}

// Framework brushes with a value snapshot are frozen. Scene/custom brushes are
// service objects and retain their identity, just as embedded controls do.
fn freeze_brush(value: &Rc<dyn IBrush>) -> Rc<dyn IBrush> {
    if (value.as_solid_color_brush().is_some()
        || value.as_image_brush().is_some()
        || value.as_gradient_brush().is_some())
        && value.as_mutable_brush().is_some()
    {
        if let Some(mutable) = value.as_mutable_brush() {
            return mutable.to_immutable();
        }
    }
    value.clone()
}

fn copy_decorations(value: &TextDecorationCollection) -> TextDecorationCollection {
    TextDecorationCollection::from_items(value.list().snapshot().iter().map(copy_decoration))
}

fn copy_decoration(value: &Ref<TextDecoration>) -> Ref<TextDecoration> {
    let result = TextDecoration::new();
    result.set_location(value.location());
    result.set_stroke(value.stroke().as_ref().map(freeze_brush));
    result.set_stroke_thickness_unit(value.stroke_thickness_unit());
    result.set_stroke_dash_array(
        value
            .stroke_dash_array()
            .map(|v| MediaCollection::from_items(v.list().snapshot().iter().copied())),
    );
    result.set_stroke_dash_offset(value.stroke_dash_offset());
    result.set_stroke_thickness(value.stroke_thickness());
    result.set_stroke_line_cap(value.stroke_line_cap());
    result.set_stroke_offset(value.stroke_offset());
    result.set_stroke_offset_unit(value.stroke_offset_unit());
    result
}

fn decorations_equal(a: &TextDecorationCollection, b: &TextDecorationCollection) -> bool {
    let a = a.list().snapshot();
    let b = b.list().snapshot();
    a.len() == b.len()
        && a.iter().zip(b.iter()).all(|(a, b)| {
            a.location() == b.location()
                && a.stroke() == b.stroke()
                && a.stroke_thickness_unit() == b.stroke_thickness_unit()
                && a.stroke_dash_array().map(|v| v.list().snapshot())
                    == b.stroke_dash_array().map(|v| v.list().snapshot())
                && a.stroke_dash_offset() == b.stroke_dash_offset()
                && a.stroke_thickness() == b.stroke_thickness()
                && a.stroke_line_cap() == b.stroke_line_cap()
                && a.stroke_offset() == b.stroke_offset()
                && a.stroke_offset_unit() == b.stroke_offset_unit()
        })
}
