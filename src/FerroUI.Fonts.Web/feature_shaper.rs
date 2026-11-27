//! The `font-feature-settings` descriptor of a face.
//!
//! Features are given to the shaper with each text run, by the properties of the text. What a
//! face declares is added here, in front of them, by a shaper that wraps the active one: it is
//! installed only when a face declares features, so nothing else pays for it.

use ferroui_base::media::text_formatting::{ShapedBuffer, TextShaper, TextShaperOptions};
use ferroui_base::media::{FontFeature, GlyphTypeface, ITextShaperTypeface};
use ferroui_base::platform::ITextShaperImpl;
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_fonts_web_core::css::FontFeatureSetting;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The descriptor's features as the shaper takes them.
pub(crate) fn to_font_features(settings: &[FontFeatureSetting]) -> Rc<Vec<FontFeature>> {
    Rc::new(
        settings
            .iter()
            .map(|setting| FontFeature {
                tag: String::from_utf8_lossy(&setting.tag).into_owned(),
                value: setting.value.min(i32::MAX as u32) as i32,
                ..FontFeature::default()
            })
            .collect(),
    )
}

/// The features of a run: those of the face first, so that those of the text, which have the
/// higher precedence in CSS Fonts 4, section 7.2, win.
pub(crate) fn merged_features(face: &[FontFeature], text: Option<&Rc<Vec<FontFeature>>>) -> Rc<Vec<FontFeature>> {
    let mut merged = Vec::with_capacity(face.len() + text.map_or(0, |text| text.len()));
    merged.extend_from_slice(face);
    if let Some(text) = text {
        merged.extend_from_slice(text);
    }
    Rc::new(merged)
}

type FaceFeatures = (Rc<GlyphTypeface>, Rc<Vec<FontFeature>>);

/// A shaper that adds the features of a face to the runs set in it.
pub(crate) struct FontFeatureShaper {
    inner: Rc<dyn ITextShaperImpl>,
    /// By the address of the typeface, which the entry keeps alive.
    faces: RefCell<HashMap<*const GlyphTypeface, FaceFeatures>>,
}

impl FontFeatureShaper {
    /// Wraps the active shaper; `None` when there is none.
    pub(crate) fn install() -> Option<Rc<Self>> {
        let inner = FerroLocator::current().get_service::<dyn ITextShaperImpl>()?;
        let shaper = Rc::new(Self { inner, faces: RefCell::new(HashMap::new()) });
        let as_impl: Rc<dyn ITextShaperImpl> = shaper.clone();
        let locator = FerroLocator::current_mutable();
        locator.bind::<dyn ITextShaperImpl>().to_constant(as_impl.clone());
        locator.bind::<TextShaper>().to_constant(Rc::new(TextShaper::new(as_impl)));
        Some(shaper)
    }

    pub(crate) fn set_features(&self, typeface: &Rc<GlyphTypeface>, features: Rc<Vec<FontFeature>>) {
        self.faces.borrow_mut().insert(Rc::as_ptr(typeface), (typeface.clone(), features));
    }

    pub(crate) fn remove(&self, typeface: &Rc<GlyphTypeface>) {
        self.faces.borrow_mut().remove(&Rc::as_ptr(typeface));
    }
}

impl ITextShaperImpl for FontFeatureShaper {
    fn shape_text(&self, text: &ReadOnlyMemory<u16>, options: &TextShaperOptions) -> Rc<ShapedBuffer> {
        let face = self.faces.borrow().get(&Rc::as_ptr(options.glyph_typeface())).map(|entry| entry.1.clone());
        let Some(face) = face else {
            return self.inner.shape_text(text, options);
        };
        let merged = TextShaperOptions::with_all(
            options.glyph_typeface().clone(),
            options.font_rendering_em_size(),
            options.bidi_level(),
            options.culture().cloned(),
            options.incremental_tab_width(),
            options.letter_spacing(),
            Some(merged_features(&face, options.font_features())),
        )
        .with_concat_flags(options.produce_concat_flags());
        self.inner.shape_text(text, &merged)
    }

    fn create_typeface(&self, glyph_typeface: &GlyphTypeface) -> Rc<dyn ITextShaperTypeface> {
        self.inner.create_typeface(glyph_typeface)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feature(tag: &str, value: i32) -> FontFeature {
        FontFeature { tag: tag.into(), value, ..FontFeature::default() }
    }

    #[test]
    fn the_descriptor_becomes_features_of_the_whole_run() {
        let settings = [FontFeatureSetting { tag: *b"smcp", value: 1 }, FontFeatureSetting { tag: *b"liga", value: 0 }];
        let features = to_font_features(&settings);
        assert_eq!(features[0].tag, "smcp");
        assert_eq!(features[1].value, 0);
        assert_eq!((features[0].start, features[0].end), (FontFeature::default().start, FontFeature::default().end));
    }

    #[test]
    fn the_features_of_the_text_come_last_so_they_win() {
        let face = [feature("liga", 0), feature("smcp", 1)];
        let text = Rc::new(vec![feature("liga", 1)]);
        let merged = merged_features(&face, Some(&text));
        let tags: Vec<(&str, i32)> = merged.iter().map(|feature| (feature.tag.as_str(), feature.value)).collect();
        assert_eq!(tags, [("liga", 0), ("smcp", 1), ("liga", 1)]);
        assert_eq!(merged_features(&face, None).len(), 2);
    }
}
