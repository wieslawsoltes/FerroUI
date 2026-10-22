use crate::highlighting_brush::{resolve_highlighting_brush, HighlightingBrushError};
use ferroui_base::media::text_formatting::{GenericTextRunProperties, TextRunProperties};
use ferroui_base::media::{
    FontFamily, FontStyle, FontWeight, IBrush, TextDecoration, TextDecorationCollection,
    TextDecorationLocation, Typeface,
};
use ferroui_base::{Ref, StyledElement};
use ferroui_controls::documents::{Inline, InlineCollection, Run};
use ferroui_controls::TextBlock;
use ferroui_edit_core::highlighting::{
    HighlightingColor, HighlightingFontStyle, RichText, RichTextError,
};
use std::fmt;
use std::rc::Rc;

#[derive(Debug)]
pub enum RichTextDisplayError {
    RichText(RichTextError),
    Brush(HighlightingBrushError),
    InvalidFontFamily(String),
    InvalidFontWeight(u16),
    InvalidFontSize(f64),
}

impl fmt::Display for RichTextDisplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RichText(error) => error.fmt(f),
            Self::Brush(error) => error.fmt(f),
            Self::InvalidFontFamily(value) => write!(f, "Invalid font family {value:?}"),
            Self::InvalidFontWeight(value) => write!(f, "Font weight {value} is outside 1..=1000"),
            Self::InvalidFontSize(value) => {
                write!(f, "Font size {value} must be finite and positive")
            }
        }
    }
}

impl std::error::Error for RichTextDisplayError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::RichText(error) => Some(error),
            Self::Brush(error) => Some(error),
            _ => None,
        }
    }
}

impl From<RichTextError> for RichTextDisplayError {
    fn from(error: RichTextError) -> Self {
        Self::RichText(error)
    }
}

impl From<HighlightingBrushError> for RichTextDisplayError {
    fn from(error: HighlightingBrushError) -> Self {
        Self::Brush(error)
    }
}

/// A resolved style using FerroUI's own brush, font and decoration types.
/// Unspecified properties remain inheritable when applied to a native Run.
#[derive(Clone, Debug, Default)]
pub struct NativeHighlightingStyle {
    pub foreground: Option<Rc<dyn IBrush>>,
    pub background: Option<Rc<dyn IBrush>>,
    pub font_family: Option<FontFamily>,
    pub font_weight: Option<FontWeight>,
    pub font_style: Option<FontStyle>,
    pub font_size: Option<f64>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
}

impl NativeHighlightingStyle {
    pub fn resolve(
        color: &HighlightingColor,
        context: Option<&StyledElement>,
    ) -> Result<Self, RichTextDisplayError> {
        let font_family = color
            .font_family
            .as_deref()
            .map(|value| {
                FontFamily::parse(value)
                    .map_err(|_| RichTextDisplayError::InvalidFontFamily(value.into()))
            })
            .transpose()?;
        let font_weight = color
            .effective_font_weight()
            .map(|value| {
                if (1..=1000).contains(&value) {
                    Ok(FontWeight(i32::from(value)))
                } else {
                    Err(RichTextDisplayError::InvalidFontWeight(value))
                }
            })
            .transpose()?;
        if color
            .font_size
            .is_some_and(|size| !size.is_finite() || size <= 0.0)
        {
            return Err(RichTextDisplayError::InvalidFontSize(
                color.font_size.unwrap(),
            ));
        }
        Ok(Self {
            foreground: color
                .foreground
                .as_deref()
                .map(|value| resolve_highlighting_brush(value, context))
                .transpose()?,
            background: color
                .background
                .as_deref()
                .map(|value| resolve_highlighting_brush(value, context))
                .transpose()?,
            font_family,
            font_weight,
            font_style: color.effective_font_style().map(|style| match style {
                HighlightingFontStyle::Normal => FontStyle::Normal,
                HighlightingFontStyle::Italic => FontStyle::Italic,
                HighlightingFontStyle::Oblique => FontStyle::Oblique,
            }),
            font_size: color.font_size,
            underline: color.underline,
            strikethrough: color.strikethrough,
        })
    }

    pub fn apply_to_run(&self, run: &Run) {
        if let Some(brush) = &self.foreground {
            run.set_foreground(Some(brush.clone()));
        }
        if let Some(brush) = &self.background {
            run.set_background(Some(brush.clone()));
        }
        if let Some(family) = &self.font_family {
            run.set_font_family(family.clone());
        }
        if let Some(weight) = self.font_weight {
            run.set_font_weight(weight);
        }
        if let Some(style) = self.font_style {
            run.set_font_style(style);
        }
        if let Some(size) = self.font_size {
            run.set_font_size(size);
        }
        if self.underline.is_some() || self.strikethrough.is_some() {
            run.set_text_decorations(self.decorations(run.text_decorations().as_ref()));
        }
    }

    /// Produces native formatter properties while retaining the caller's font,
    /// culture, baseline, features and unspecified decoration values.
    pub fn text_run_properties(
        &self,
        defaults: &dyn TextRunProperties,
    ) -> Rc<dyn TextRunProperties> {
        let typeface = defaults.typeface();
        Rc::new(GenericTextRunProperties::with_all(
            Typeface::with_style(
                self.font_family
                    .clone()
                    .unwrap_or_else(|| typeface.font_family().clone()),
                self.font_style.unwrap_or(typeface.style()),
                self.font_weight.unwrap_or(typeface.weight()),
                typeface.stretch(),
            ),
            self.font_size.unwrap_or(defaults.font_rendering_em_size()),
            self.decorations(defaults.text_decorations()),
            self.foreground
                .clone()
                .or_else(|| defaults.foreground_brush().cloned()),
            self.background
                .clone()
                .or_else(|| defaults.background_brush().cloned()),
            defaults.baseline_alignment(),
            defaults.culture_info().cloned(),
            defaults.font_features().cloned(),
        ))
    }

    fn decorations(
        &self,
        inherited: Option<&TextDecorationCollection>,
    ) -> Option<TextDecorationCollection> {
        if self.underline.is_none() && self.strikethrough.is_none() {
            return inherited.cloned();
        }
        let mut decorations = inherited.map_or_else(Vec::new, |items| {
            (0..items.len())
                .map(|index| items.get(index))
                .filter(|item| {
                    !matches!(
                        (item.location(), self.underline, self.strikethrough),
                        (TextDecorationLocation::Underline, Some(false), _)
                            | (TextDecorationLocation::Strikethrough, _, Some(false))
                    )
                })
                .collect()
        });
        for (enabled, location) in [
            (self.underline, TextDecorationLocation::Underline),
            (self.strikethrough, TextDecorationLocation::Strikethrough),
        ] {
            if enabled == Some(true) && !decorations.iter().any(|item| item.location() == location)
            {
                let item = TextDecoration::new();
                item.set_location(location);
                decorations.push(item);
            }
        }
        Some(TextDecorationCollection::from_items(decorations))
    }

    /// Replaces resource names with concrete solid colors for HTML. A gradient
    /// or other non-solid brush has no single CSS text color and is omitted.
    pub fn export_color(&self, source: &HighlightingColor) -> HighlightingColor {
        fn solid(brush: &Option<Rc<dyn IBrush>>) -> Option<String> {
            let brush = brush.as_ref()?;
            let color = brush.as_solid_color_brush()?.color();
            let alpha = (f64::from(color.a) * brush.opacity())
                .round()
                .clamp(0.0, 255.0) as u8;
            Some(if alpha == 255 {
                format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
            } else {
                format!("#{alpha:02x}{:02x}{:02x}{:02x}", color.r, color.g, color.b)
            })
        }
        let mut color = source.clone();
        color.foreground = solid(&self.foreground);
        color.background = solid(&self.background);
        color
    }
}

/// Native presentation adapters for the framework-independent rich-text value.
pub trait RichTextDisplayExt {
    fn create_runs(&self) -> Result<Vec<Ref<Run>>, RichTextDisplayError>;
    fn create_runs_with_context(
        &self,
        context: Option<&StyledElement>,
    ) -> Result<Vec<Ref<Run>>, RichTextDisplayError>;
    fn to_text_block(&self) -> Result<Ref<TextBlock>, RichTextDisplayError>;
    fn apply_to_text_block(&self, target: &TextBlock) -> Result<(), RichTextDisplayError>;
}

impl RichTextDisplayExt for RichText {
    fn create_runs(&self) -> Result<Vec<Ref<Run>>, RichTextDisplayError> {
        self.create_runs_with_context(None)
    }

    fn create_runs_with_context(
        &self,
        context: Option<&StyledElement>,
    ) -> Result<Vec<Ref<Run>>, RichTextDisplayError> {
        let sections = self.get_highlighted_sections(0, self.len())?;
        if sections.is_empty() {
            return Ok(Vec::new());
        }
        let mut runs = Vec::new();
        let mut index = 0;
        let mut position = 0;
        let mut current_color = &sections[0].color;
        let mut text = String::new();
        let inherited =
            context.and_then(|context| context.get_value(Inline::text_decorations_property()));
        let mut append =
            |text: &mut String, color: &HighlightingColor| -> Result<(), RichTextDisplayError> {
                if text.is_empty() {
                    return Ok(());
                }
                let style = NativeHighlightingStyle::resolve(color, context)?;
                let run = Run::with_text(Some(text));
                if style.underline.is_some() || style.strikethrough.is_some() {
                    run.set_text_decorations(inherited.clone());
                }
                style.apply_to_run(&run);
                runs.push(run);
                text.clear();
                Ok(())
            };
        let units = self.text_utf16();
        while position < units.len() {
            while sections
                .get(index + 1)
                .is_some_and(|section| section.offset <= position)
            {
                index += 1;
            }
            let color = &sections[index].color;
            if color != current_color {
                append(&mut text, current_color)?;
                current_color = color;
            }
            let first = units[position];
            let (scalar, width) = if (0xd800..=0xdbff).contains(&first)
                && units
                    .get(position + 1)
                    .is_some_and(|unit| (0xdc00..=0xdfff).contains(unit))
            {
                let value =
                    0x10000 + (u32::from(first) - 0xd800) * 0x400 + u32::from(units[position + 1])
                        - 0xdc00;
                (
                    char::from_u32(value).expect("a validated surrogate pair"),
                    2,
                )
            } else {
                (
                    char::from_u32(u32::from(first)).unwrap_or(char::REPLACEMENT_CHARACTER),
                    1,
                )
            };
            text.push(scalar);
            position += width;
        }
        append(&mut text, current_color)?;
        Ok(runs)
    }

    fn to_text_block(&self) -> Result<Ref<TextBlock>, RichTextDisplayError> {
        let block = TextBlock::new();
        self.apply_to_text_block(&block)?;
        Ok(block)
    }

    fn apply_to_text_block(&self, target: &TextBlock) -> Result<(), RichTextDisplayError> {
        // Resolve every brush/font before replacing an existing visual value.
        let runs = self.create_runs_with_context(Some(target))?;
        let inlines = InlineCollection::new();
        for run in runs {
            inlines.add(run);
        }
        target.set_text(None);
        target.set_inlines(Some(inlines));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::media::{Colors, TextDecorations};
    use ferroui_edit_core::highlighting::RichTextModel;

    #[test]
    fn rich_text_builds_formatted_native_runs_and_measurable_text_block() {
        let _scope = crate::tests::test_scope();
        let mut model = RichTextModel::new();
        model
            .apply_highlighting(
                0,
                2,
                &HighlightingColor {
                    foreground: Some("Red".into()),
                    background: Some("Blue".into()),
                    font_family: Some("Test Sans".into()),
                    font_weight: Some(650),
                    font_style: Some(HighlightingFontStyle::Oblique),
                    font_size: Some(18.0),
                    underline: Some(true),
                    strikethrough: Some(true),
                    ..HighlightingColor::default()
                },
            )
            .unwrap();
        let text = RichText::with_model("ab cd", &model);
        let runs = text.create_runs().unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text().as_deref(), Some("ab"));
        assert_eq!(runs[0].font_weight(), FontWeight(650));
        assert_eq!(runs[0].font_style(), FontStyle::Oblique);
        assert_eq!(runs[0].font_size(), 18.0);
        assert_eq!(
            runs[0]
                .foreground()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            Colors::RED
        );
        assert_eq!(
            runs[0]
                .background()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            Colors::BLUE
        );
        assert_eq!(runs[0].text_decorations().unwrap().len(), 2);
        let block = text.to_text_block().unwrap();
        block.set_font_family(FontFamily::new("Test Sans"));
        block.measure(ferroui_base::Size::new(500.0, 100.0));
        assert!(block.desired_size().width > 0.0);
        assert_eq!(block.inlines().unwrap().text().as_deref(), Some("ab cd"));
    }

    #[test]
    fn supplementary_scalars_keep_the_leading_style_and_lone_surrogates_are_visible() {
        let _scope = crate::tests::test_scope();
        let mut model = RichTextModel::new();
        model.set_foreground(0, 1, Some("Red")).unwrap();
        model.set_foreground(1, 3, Some("Blue")).unwrap();
        let rich = RichText::from_utf16_with_model(&[0xd83d, 0xde00, b'x' as u16, 0xd800], &model);
        let runs = rich.create_runs().unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text().as_deref(), Some("😀"));
        assert_eq!(runs[1].text().as_deref(), Some("x�"));
        assert_eq!(
            runs[0]
                .foreground()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            Colors::RED
        );
    }

    #[test]
    fn display_updates_are_atomic_and_unset_properties_stay_inheritable() {
        let _scope = crate::tests::test_scope();
        let block = TextBlock::new();
        block.set_text(Some("original"));
        let mut model = RichTextModel::new();
        model.set_foreground(0, 1, Some("not-a-brush")).unwrap();
        assert!(RichText::with_model("x", &model)
            .apply_to_text_block(&block)
            .is_err());
        assert_eq!(block.text().as_deref(), Some("original"));
        RichText::new("plain").apply_to_text_block(&block).unwrap();
        block.set_font_size(27.0);
        let inline = block.inlines().unwrap().get(0);
        assert_eq!(inline.font_size(), 27.0);
    }

    #[test]
    fn explicit_decoration_changes_preserve_other_inherited_decorations() {
        let _scope = crate::tests::test_scope();
        let run = Run::with_text(Some("x"));
        run.set_text_decorations(Some(
            TextDecorationCollection::parse("Underline,Strikethrough,Overline").unwrap(),
        ));
        NativeHighlightingStyle {
            underline: Some(false),
            ..NativeHighlightingStyle::default()
        }
        .apply_to_run(&run);
        let decorations = run.text_decorations().unwrap();
        assert_eq!(decorations.len(), 2);
        assert!((0..decorations.len()).any(
            |index| decorations.get(index).location() == TextDecorationLocation::Strikethrough
        ));
        let defaults = GenericTextRunProperties::with_all(
            Typeface::from_name("Test Sans"),
            15.0,
            Some(TextDecorations::strikethrough()),
            None,
            None,
            ferroui_base::media::BaselineAlignment::Baseline,
            None,
            None,
        );
        let style = NativeHighlightingStyle {
            underline: Some(true),
            font_weight: Some(FontWeight(550)),
            ..NativeHighlightingStyle::default()
        };
        let properties = style.text_run_properties(&defaults);
        assert_eq!(properties.typeface().weight(), FontWeight(550));
        assert_eq!(properties.text_decorations().unwrap().len(), 2);
    }
}
