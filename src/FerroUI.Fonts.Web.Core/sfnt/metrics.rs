//! `ascent-override`, `descent-override`, `line-gap-override` and `size-adjust`.
//!
//! They are applied to the decoded font itself, in place. Every backend then lays out and draws
//! the font from its own tables, on the path it always uses, with nothing to add for them.

use super::{update_checksums, ReadBe};
use crate::css::{FontFaceDescriptors, MetricOverride};
use crate::FontDecodeError;

const MALFORMED: FontDecodeError = FontDecodeError::Malformed("metric override");

fn table_range(font: &[u8], tag: &[u8; 4]) -> Option<(usize, usize)> {
    let count = font.u16_at(4)? as usize;
    (0..count).find_map(|index| {
        let record = 12 + index * 16;
        (font.get(record..record + 4)? == tag)
            .then(|| Some((font.u32_at(record + 8)? as usize, font.u32_at(record + 12)? as usize)))?
    })
}

fn write_i16(font: &mut [u8], at: usize, value: i32) {
    let value = value.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    font[at..at + 2].copy_from_slice(&value.to_be_bytes());
}

fn write_u16(font: &mut [u8], at: usize, value: i32) {
    let value = value.clamp(0, u16::MAX as i32) as u16;
    font[at..at + 2].copy_from_slice(&value.to_be_bytes());
}

fn override_value(value: MetricOverride, units_per_em: u16) -> Option<i32> {
    match value {
        MetricOverride::Normal => None,
        // The first value is the one that applies to horizontal text.
        MetricOverride::Value { first, .. } => Some((first as f64 * units_per_em as f64).round() as i32),
    }
}

/// Whether a face has anything to apply.
pub fn has_metric_overrides(face: &FontFaceDescriptors) -> bool {
    face.ascent_override != MetricOverride::Normal
        || face.descent_override != MetricOverride::Normal
        || face.line_gap_override != MetricOverride::Normal
        || face.size_adjust != 1.0
}

/// Applies the metric descriptors of a face to a decoded font in canonical layout.
///
/// `size-adjust` scales outlines and metrics together by changing the units per em, so it has
/// the resolution of that integer. An override is a fraction of the adjusted em, which is the
/// original number of units. A font with an explicit `FontMatrix` in CFF may ignore the units
/// per em in some rasterizers.
pub fn apply_metric_overrides(font: &mut [u8], face: &FontFaceDescriptors) -> Result<(), FontDecodeError> {
    if !has_metric_overrides(face) {
        return Ok(());
    }
    let (head, head_length) = table_range(font, b"head").ok_or(FontDecodeError::MissingTable(*b"head"))?;
    let (hhea, hhea_length) = table_range(font, b"hhea").ok_or(FontDecodeError::MissingTable(*b"hhea"))?;
    if head_length < 54 || hhea_length < 36 || head + head_length > font.len() || hhea + hhea_length > font.len() {
        return Err(MALFORMED);
    }
    let units_per_em = font.u16_at(head + 18).ok_or(MALFORMED)?;
    let ascent = override_value(face.ascent_override, units_per_em);
    let descent = override_value(face.descent_override, units_per_em);
    let line_gap = override_value(face.line_gap_override, units_per_em);
    if let Some(value) = ascent {
        write_i16(font, hhea + 4, value);
    }
    if let Some(value) = descent {
        write_i16(font, hhea + 6, -value);
    }
    if let Some(value) = line_gap {
        write_i16(font, hhea + 8, value);
    }
    if let Some((os2, length)) = table_range(font, b"OS/2").filter(|(os2, length)| os2 + length <= font.len()) {
        if length >= 78 {
            if let Some(value) = ascent {
                write_i16(font, os2 + 68, value);
                write_u16(font, os2 + 74, value);
            }
            if let Some(value) = descent {
                write_i16(font, os2 + 70, -value);
                write_u16(font, os2 + 76, value);
            }
            if let Some(value) = line_gap {
                write_i16(font, os2 + 72, value);
            }
        }
    }
    if face.size_adjust != 1.0 {
        // A size of zero is as small as the format can go.
        let adjust = (face.size_adjust as f64).max(f64::MIN_POSITIVE);
        let adjusted = (units_per_em as f64 / adjust).round().clamp(16.0, 16384.0) as u16;
        font[head + 18..head + 20].copy_from_slice(&adjusted.to_be_bytes());
    }
    update_checksums(font);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::parse_font_face_block;
    use crate::sfnt::{sanitize, SanitizeOptions};
    use std::path::PathBuf;

    fn font() -> Vec<u8> {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../FerroUI.Base/test_assets/fonts/Inter-Regular.ttf");
        sanitize(&std::fs::read(path).unwrap(), &SanitizeOptions::default()).unwrap()
    }

    fn face(descriptors: &str) -> FontFaceDescriptors {
        parse_font_face_block(&format!("font-family: F; src: url(a); {descriptors}")).unwrap()
    }

    fn field(font: &[u8], tag: &[u8; 4], offset: usize) -> i16 {
        let (table, _) = table_range(font, tag).unwrap();
        font.i16_at(table + offset).unwrap()
    }

    #[test]
    fn nothing_to_apply_leaves_the_font_alone() {
        let original = font();
        let mut copy = original.clone();
        apply_metric_overrides(&mut copy, &face("")).unwrap();
        assert!(copy == original);
        assert!(!has_metric_overrides(&face("ascent-override: normal; size-adjust: 100%")));
    }

    #[test]
    fn overrides_reach_every_table_that_holds_the_metric() {
        let mut data = font();
        let em = field(&data, b"head", 18) as f64;
        apply_metric_overrides(&mut data, &face("ascent-override: 90%; descent-override: 20%; line-gap-override: 5%"))
            .unwrap();
        let (ascent, descent, gap) = ((em * 0.9).round() as i16, (em * 0.2).round() as i16, (em * 0.05).round() as i16);
        assert_eq!(
            (field(&data, b"hhea", 4), field(&data, b"hhea", 6), field(&data, b"hhea", 8)),
            (ascent, -descent, gap)
        );
        assert_eq!(
            (field(&data, b"OS/2", 68), field(&data, b"OS/2", 70), field(&data, b"OS/2", 72)),
            (ascent, -descent, gap)
        );
        assert_eq!((field(&data, b"OS/2", 74), field(&data, b"OS/2", 76)), (ascent, descent));
    }

    #[test]
    fn only_what_is_overridden_changes() {
        let original = font();
        let mut data = original.clone();
        apply_metric_overrides(&mut data, &face("line-gap-override: 0%")).unwrap();
        assert_eq!(field(&data, b"hhea", 4), field(&original, b"hhea", 4));
        assert_eq!(field(&data, b"hhea", 8), 0);
    }

    #[test]
    fn size_adjust_scales_through_the_units_per_em() {
        let original = font();
        let em = field(&original, b"head", 18) as f64;
        let mut data = original.clone();
        apply_metric_overrides(&mut data, &face("size-adjust: 200%")).unwrap();
        assert_eq!(field(&data, b"head", 18) as f64, (em / 2.0).round());
        assert_eq!(field(&data, b"hhea", 4), field(&original, b"hhea", 4), "metrics scale with it");
    }

    #[test]
    fn an_override_is_a_fraction_of_the_adjusted_em() {
        let original = font();
        let em = field(&original, b"head", 18) as f64;
        let mut data = original;
        apply_metric_overrides(&mut data, &face("size-adjust: 50%; ascent-override: 100%")).unwrap();
        assert_eq!(field(&data, b"hhea", 4) as f64, em, "one adjusted em, in the units of the outlines");
    }

    #[test]
    fn extreme_values_stay_valid() {
        for descriptors in
            ["size-adjust: 0%", "size-adjust: 100000%", "ascent-override: 100000%", "descent-override: 0%"]
        {
            let mut data = font();
            apply_metric_overrides(&mut data, &face(descriptors)).unwrap();
            let em = field(&data, b"head", 18);
            assert!((16..=16384).contains(&em), "{descriptors}: {em}");
            assert!(sanitize(&data, &SanitizeOptions::default()).is_ok(), "{descriptors}");
        }
    }

    #[test]
    fn a_font_without_the_tables_is_refused() {
        let face = face("size-adjust: 90%");
        assert!(apply_metric_overrides(&mut [0u8; 16], &face).is_err());
    }
}
