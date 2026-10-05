use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::media::{FontFamily, FontManager, FontStretch, FontStyle, FontWeight, GlyphTypeface};
use crate::utilities::SpanStringTokenizer;

/// Represents a typeface: a font family with a style, weight and stretch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Typeface {
    font_family: FontFamily,
    style: FontStyle,
    weight: FontWeight,
    stretch: FontStretch,
}

impl Typeface {
    /// Creates a normal typeface of a font family.
    pub fn new(font_family: FontFamily) -> Self {
        Self::with_style(font_family, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal)
    }

    /// Creates a typeface.
    ///
    /// Panics when `weight` is not positive.
    pub fn with_style(font_family: FontFamily, style: FontStyle, weight: FontWeight, stretch: FontStretch) -> Self {
        if weight.value() <= 0 {
            panic!("Font weight must be > 0.");
        }

        if (stretch as i32) < 1 {
            panic!("Font stretch must be > 1.");
        }

        Self { font_family, style, weight, stretch }
    }

    /// Creates a normal typeface from a font family name; an empty name gives
    /// the default font family.
    pub fn from_name(font_family_name: &str) -> Self {
        Self::from_name_with_style(font_family_name, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal)
    }

    /// Creates a typeface from a font family name; an empty name gives the
    /// default font family.
    pub fn from_name_with_style(
        font_family_name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Self {
        let font_family = if font_family_name.is_empty() {
            FontFamily::default_family()
        } else {
            FontFamily::new(font_family_name)
        };

        Self::with_style(font_family, style, weight, stretch)
    }

    /// The default typeface: the default font family, normal style.
    pub fn default_typeface() -> Typeface {
        Typeface::new(FontFamily::default_family())
    }

    /// Gets the font family.
    pub fn font_family(&self) -> &FontFamily {
        &self.font_family
    }

    /// Gets the font style.
    pub fn style(&self) -> FontStyle {
        self.style
    }

    /// Gets the font weight.
    pub fn weight(&self) -> FontWeight {
        self.weight
    }

    /// Gets the font stretch.
    pub fn stretch(&self) -> FontStretch {
        self.stretch
    }

    /// Gets the glyph typeface.
    ///
    /// Panics when the font manager cannot create one (C# throws
    /// `InvalidOperationException`); use
    /// [`FontManager::try_get_glyph_typeface`] to handle that case.
    pub fn glyph_typeface(&self) -> Rc<GlyphTypeface> {
        if let Some(glyph_typeface) = FontManager::current().try_get_glyph_typeface(self) {
            return glyph_typeface;
        }

        panic!(
            "Could not create glyphTypeface. Font family: {} (key: {}). Style: {}. Weight: {}. Stretch: {}",
            self.font_family.name(),
            self.font_family.key().map(|key| key.to_string()).unwrap_or_default(),
            self.style,
            self.weight,
            self.stretch
        );
    }

    /// Splits style, weight and stretch words (e.g. "Bold", "Italic") out of
    /// the primary family name. Returns the typeface with the matched values
    /// applied and the family name without the matched words.
    pub fn normalize(&self) -> (Typeface, String) {
        let normalized_family_name = self.font_family.family_names().primary_family_name();

        // Return early if no separator is present.
        if !normalized_family_name.contains(' ') {
            return (self.clone(), normalized_family_name.to_owned());
        }

        let mut style = self.style;
        let mut weight = self.weight;
        let mut stretch = self.stretch;

        let mut normalized_family_name_builder: Option<String> = None;
        let mut total_chars_removed = 0;

        let mut tokenizer = SpanStringTokenizer::with_separator(normalized_family_name, ' ', None);

        // Skip initial family name.
        let _ = tokenizer.read_span();

        // A malformed name ends the scan (upstream lets the format exception escape).
        while let Ok(Some(token)) = tokenizer.try_read_span() {
            // Don't try to match numbers.
            if matches!(SpanStringTokenizer::new(token).try_read_int32(), Ok(Some(_))) {
                continue;
            }

            // Try match with font style, weight or stretch and update accordingly.
            let mut matched = false;
            if let Some(new_style) = FontStyle::try_parse_ignore_case(token) {
                style = new_style;
                matched = true;
            } else if let Some(new_weight) = FontWeight::try_parse_ignore_case(token) {
                weight = new_weight;
                matched = true;
            } else if let Some(new_stretch) = FontStretch::try_parse_ignore_case(token) {
                stretch = new_stretch;
                matched = true;
            }

            if matched {
                // Carve out matched word from the normalized name.
                let builder = normalized_family_name_builder.get_or_insert_with(|| normalized_family_name.to_owned());
                let start = tokenizer.current_token_index().unwrap_or(0) - total_chars_removed;
                builder.replace_range(start..start + token.len(), "");
                total_chars_removed += token.len();
            }
        }

        // Get rid of any trailing spaces.
        let normalized = normalized_family_name_builder
            .as_deref()
            .unwrap_or(normalized_family_name)
            .trim_end()
            .to_owned();

        // Preserve old font source
        (Typeface::with_style(self.font_family.clone(), style, weight, stretch), normalized)
    }
}

impl Default for Typeface {
    fn default() -> Self {
        Typeface::default_typeface()
    }
}

impl Hash for Typeface {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.font_family.hash(state);
        (self.style as i32).hash(state);
        self.weight.value().hash(state);
        (self.stretch as i32).hash(state);
    }
}

impl From<FontFamily> for Typeface {
    fn from(font_family: FontFamily) -> Self {
        Typeface::new(font_family)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;

    use super::*;

    #[test]
    #[should_panic(expected = "Font weight must be > 0.")]
    fn exception_should_be_thrown_if_font_weight_less_than_equal_to_zero() {
        Typeface::from_name_with_style("foo", FontStyle::Normal, FontWeight(0), FontStretch::Normal);
    }

    #[test]
    fn should_be_equal() {
        assert_eq!(Typeface::from_name("Font A"), Typeface::from_name("Font A"));
    }

    #[test]
    fn should_have_equal_hash() {
        let hash_of = |typeface: &Typeface| {
            let mut hasher = DefaultHasher::new();
            typeface.hash(&mut hasher);
            hasher.finish()
        };

        assert_eq!(hash_of(&Typeface::from_name("Font A")), hash_of(&Typeface::from_name("Font A")));
    }

    #[test]
    fn should_get_implicit_typeface() {
        for (input, family_name, style, weight) in [
            ("Hello World 6", "Hello World 6", FontStyle::Normal, FontWeight::Normal),
            ("Hello World Italic", "Hello World", FontStyle::Italic, FontWeight::Normal),
            ("Hello World Italic Bold", "Hello World", FontStyle::Italic, FontWeight::Bold),
            ("FontAwesome 6 Free Regular", "FontAwesome 6 Free", FontStyle::Normal, FontWeight::Normal),
            ("FontAwesome 6 Free Solid", "FontAwesome 6 Free", FontStyle::Normal, FontWeight::Solid),
            ("FontAwesome 6 Brands", "FontAwesome 6 Brands", FontStyle::Normal, FontWeight::Normal),
        ] {
            let typeface = Typeface::from_name(input);

            let (normalized_typeface, normalized_family_name) = typeface.normalize();

            assert_eq!(normalized_family_name, family_name, "{input}");
            assert_eq!(normalized_typeface.style(), style, "{input}");
            assert_eq!(normalized_typeface.weight(), weight, "{input}");
            assert_eq!(normalized_typeface.stretch(), FontStretch::Normal, "{input}");
        }
    }

    #[test]
    fn an_empty_name_gives_the_default_font_family() {
        assert_eq!(Typeface::from_name("").font_family(), &FontFamily::default_family());
        assert_eq!(Typeface::default(), Typeface::new(FontFamily::default_family()));
    }
}
