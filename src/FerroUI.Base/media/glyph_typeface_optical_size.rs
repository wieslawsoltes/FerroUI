//! The optical size of a font follows the size of the text (#859).
//!
//! A variable font with an optical size axis (`opsz`: the system font of macOS, 17 to 96;
//! Segoe UI Variable) has a design for each size of text: small sizes are wider and looser,
//! large ones tighter. Core Text and the browsers (`font-optical-sizing: auto`) move along the
//! axis with the size of the text; a typeface of a backend stays at the one optical size it was
//! made at. So a text is shaped and drawn with the instance of its typeface at its optical size:
//! the size of the text in points (a device-independent pixel is a point), rounded to a whole
//! point and clamped to the axis. The instances are kept by the typeface they are made from, one
//! a whole point, so that the glyph caches and the font of the text shaper are shared by the texts
//! of a size class (docs/porting/skia-native-text.md, section 3).
//!
//! A font without the axis, a backend that cannot make an instance
//! ([`IPlatformTypeface::with_variation_settings`](crate::media::IPlatformTypeface)), and a
//! family whose name names an optical size (`Segoe UI Variable Display`) keep the typeface they
//! have. Not from upstream, which draws a typeface at the instance it was made at.

use std::rc::Rc;

use crate::media::fonts::OpenTypeTag;
use crate::media::{FontFamily, FontVariation, FontVariationSettings, GlyphTypeface};

/// The optical size axis.
const OPSZ: OpenTypeTag = OpenTypeTag::from_chars('o', 'p', 's', 'z');

/// The font variations table, which lists the axes.
const FVAR: OpenTypeTag = OpenTypeTag::from_chars('f', 'v', 'a', 'r');

/// The words that end the name of a family that names an optical size (the named optical
/// sizes of Segoe UI Variable, SF and other families with an `opsz` axis).
const OPTICAL_SIZE_NAMES: [&str; 8] = ["Caption", "Display", "Micro", "Poster", "Small", "Subhead", "Text", "Banner"];

/// The range of the optical size axis of a font, from its `fvar` table; `None` for a font
/// without the axis.
pub(crate) fn optical_size_axis(fvar: &[u8]) -> Option<(f64, f64)> {
    let read_u16 = |at: usize| Some(u16::from_be_bytes([*fvar.get(at)?, *fvar.get(at + 1)?]) as usize);
    let read_fixed = |at: usize| {
        let bytes = [*fvar.get(at)?, *fvar.get(at + 1)?, *fvar.get(at + 2)?, *fvar.get(at + 3)?];
        Some(i32::from_be_bytes(bytes) as f64 / 65536.0)
    };
    // The header: version, the offset of the axes, a reserved field, their count and size.
    let axes = read_u16(4)?;
    let count = read_u16(8)?;
    let size = read_u16(10)?;
    if size < 20 {
        return None;
    }
    (0..count).find_map(|index| {
        let at = axes + index * size;
        let tag = u32::from_be_bytes([*fvar.get(at)?, *fvar.get(at + 1)?, *fvar.get(at + 2)?, *fvar.get(at + 3)?]);
        if tag != OPSZ.value() {
            return None;
        }
        let (min, max) = (read_fixed(at + 4)?, read_fixed(at + 12)?);
        (min.is_finite() && max.is_finite() && min <= max).then_some((min, max))
    })
}

/// Whether the name of a family names an optical size of it (`Segoe UI Variable Display`): its
/// texts keep that size.
pub(crate) fn names_optical_size(family: &FontFamily) -> bool {
    family
        .name()
        .split(',')
        .next()
        .and_then(|name| name.split_whitespace().last())
        .is_some_and(|word| OPTICAL_SIZE_NAMES.iter().any(|name| name.eq_ignore_ascii_case(word)))
}

/// The glyph typeface of a text of a family at a size: the typeface at the optical size of the
/// text, unless the family names an optical size.
pub(crate) fn glyph_typeface_for_text(
    glyph_typeface: Rc<GlyphTypeface>,
    family: &FontFamily,
    requested_family: &FontFamily,
    em_size: f64,
) -> Rc<GlyphTypeface> {
    if requested_family.automatic_optical_sizing() {
        return glyph_typeface.at_optical_size_in_points(em_size * (72.0 / 96.0));
    }
    // The axis first: it is read once per typeface, and most fonts have none, so the name of the
    // family is looked at only for a font that has the axis.
    if glyph_typeface.optical_size_axis().is_none() || names_optical_size(family) {
        return glyph_typeface;
    }
    glyph_typeface.at_optical_size(em_size)
}

impl GlyphTypeface {
    /// The range of the optical size axis of the font (`opsz` of its `fvar` table), read once.
    fn optical_size_axis(&self) -> Option<(f64, f64)> {
        *self.optical_size_axis.get_or_init(|| {
            let fvar = self.platform_typeface().try_get_table(FVAR)?;
            optical_size_axis(fvar.span())
        })
    }

    /// The typeface at the optical size of a text of `em_size` (in points: device-independent
    /// pixels), rounded to a whole point and clamped to the optical size axis of the font. The
    /// typeface itself for a font without the axis, for a typeface at that optical size already,
    /// and where the backend makes no instance. The instances are kept by this typeface.
    pub fn at_optical_size(self: &Rc<Self>, em_size: f64) -> Rc<GlyphTypeface> {
        let Some((min, max)) = self.optical_size_axis() else { return self.clone() };
        if !em_size.is_finite() || em_size <= 0.0 {
            return self.clone();
        }
        let optical_size = em_size.clamp(min, max).round().clamp(min.ceil(), max.floor().max(min.ceil()));
        self.at_optical_axis_value(optical_size, 0.5)
    }

    /// An explicit request uses physical points, with the fractional size preserved.
    fn at_optical_size_in_points(self: &Rc<Self>, points: f64) -> Rc<GlyphTypeface> {
        let Some((min, max)) = self.optical_size_axis() else { return self.clone() };
        if !points.is_finite() || points <= 0.0 {
            return self.clone();
        }
        self.at_optical_axis_value(points.clamp(min, max), f32::EPSILON.into())
    }

    fn at_optical_axis_value(self: &Rc<Self>, optical_size: f64, tolerance: f64) -> Rc<GlyphTypeface> {
        if let Some((_, instance)) = self.optical_sizes.borrow().iter().find(|(size, _)| *size == optical_size) {
            return instance.clone();
        }
        let current = self.platform_typeface().variation_settings().try_get_value(OPSZ);
        if current.is_some_and(|current| (current - optical_size).abs() < tolerance) {
            return self.clone();
        }
        let settings = FontVariationSettings::new([FontVariation::new(OPSZ, optical_size)]);
        let Some(platform_typeface) = self.platform_typeface().with_variation_settings(&settings) else {
            return self.clone();
        };
        let Ok(instance) = GlyphTypeface::new(platform_typeface, self.font_simulations()) else {
            return self.clone();
        };
        // The instance is at its optical size: it is not asked for others.
        let _ = instance.optical_size_axis.set(None);
        let mut variants = self.optical_sizes.borrow_mut();
        if variants.len() == 128 {
            variants.clear();
        }
        variants.push((optical_size, instance.clone()));
        instance
    }
}

#[cfg(test)]
mod tests {
    use super::{names_optical_size, optical_size_axis};
    use crate::media::FontFamily;

    /// An `fvar` table of two axes: `wght` 100 to 900 and `opsz` 17 to 96.
    fn fvar() -> Vec<u8> {
        let mut table = vec![0, 1, 0, 0, 0, 16, 0, 2, 0, 2, 0, 20, 0, 0, 0, 0];
        for (tag, min, default, max) in [(*b"wght", 100, 400, 900), (*b"opsz", 17, 28, 96)] {
            table.extend_from_slice(&tag);
            for value in [min, default, max] {
                table.extend_from_slice(&((value as i32) << 16).to_be_bytes());
            }
            table.extend_from_slice(&[0, 0, 0, 0]);
        }
        table
    }

    #[test]
    fn the_optical_size_axis_is_read_from_the_variations_table() {
        assert_eq!(optical_size_axis(&fvar()), Some((17.0, 96.0)));
        let mut weight_only = fvar();
        weight_only[8..10].copy_from_slice(&[0, 1]);
        assert_eq!(optical_size_axis(&weight_only), None);
        assert_eq!(optical_size_axis(&[0, 1]), None);
    }

    #[test]
    fn a_family_that_names_an_optical_size_keeps_it() {
        assert!(names_optical_size(&FontFamily::from("Segoe UI Variable Display")));
        assert!(names_optical_size(&FontFamily::from("Segoe UI Variable Small, Segoe UI")));
        assert!(!names_optical_size(&FontFamily::from("Segoe UI Variable")));
        assert!(!names_optical_size(&FontFamily::from(".AppleSystemUIFont")));
    }
}
