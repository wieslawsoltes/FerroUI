//! `@font-palette-values`, applied to the `CPAL` table of the decoded font. A backend draws
//! palette 0 by default, so the chosen colors are written there, and no backend needs a change.

use super::{update_checksums, ReadBe};
use crate::css::{BasePalette, FontPaletteValues};

fn table_range(font: &[u8], tag: &[u8; 4]) -> Option<(usize, usize)> {
    let count = font.u16_at(4)? as usize;
    (0..count).find_map(|index| {
        let record = 12 + index * 16;
        (font.get(record..record + 4)? == tag)
            .then(|| Some((font.u32_at(record + 8)? as usize, font.u32_at(record + 12)? as usize)))?
    })
}

/// The palette a `base-palette` selects. One that the font does not have behaves as palette 0.
fn select_palette(cpal: &[u8], palettes: usize, base: BasePalette) -> usize {
    const USABLE_WITH_LIGHT_BACKGROUND: u32 = 1;
    const USABLE_WITH_DARK_BACKGROUND: u32 = 2;
    let flag = match base {
        BasePalette::Default => return 0,
        BasePalette::Index(index) => return if (index as usize) < palettes { index as usize } else { 0 },
        BasePalette::Light => USABLE_WITH_LIGHT_BACKGROUND,
        BasePalette::Dark => USABLE_WITH_DARK_BACKGROUND,
    };
    // The types of the palettes are in version 1 of the table.
    if cpal.u16_at(0) != Some(1) {
        return 0;
    }
    let Some(types) = cpal.u32_at(12 + palettes * 2).map(|offset| offset as usize).filter(|offset| *offset != 0) else {
        return 0;
    };
    (0..palettes).find(|index| cpal.u32_at(types + index * 4).is_some_and(|value| value & flag != 0)).unwrap_or(0)
}

/// Writes the colors of a palette rule into palette 0. Returns whether the font has a palette to
/// write to. The font must have passed the sanitizer.
pub fn apply_font_palette(font: &mut [u8], palette: &FontPaletteValues) -> bool {
    let write = |font: &mut [u8]| -> Option<()> {
        let (offset, length) = table_range(font, b"CPAL")?;
        let end = offset.checked_add(length).filter(|end| *end <= font.len())?;
        let cpal = font.get(offset..end)?;
        let entries = cpal.u16_at(2)? as usize;
        let palettes = cpal.u16_at(4)? as usize;
        let records = cpal.u32_at(8)? as usize;
        let first_of = |index: usize| cpal.u16_at(12 + index * 2).map(|value| value as usize);
        let selected = select_palette(cpal, palettes, palette.base_palette);
        let source = records.checked_add(first_of(selected)?.checked_mul(4)?)?;
        // In the table, a color is blue, green, red, alpha.
        let mut colors = cpal.get(source..source.checked_add(entries.checked_mul(4)?)?)?.to_vec();
        for (index, [red, green, blue, alpha]) in &palette.override_colors {
            // An index past the end of the palette is ignored.
            if let Some(color) = colors.get_mut(*index as usize * 4..*index as usize * 4 + 4) {
                color.copy_from_slice(&[*blue, *green, *red, *alpha]);
            }
        }
        let target = offset + records.checked_add(first_of(0)?.checked_mul(4)?)?;
        font.get_mut(target..target + colors.len())?.copy_from_slice(&colors);
        Some(())
    };
    if write(font).is_none() {
        return false;
    }
    update_checksums(font);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::{parse_font_palettes, parse_override_colors};
    use crate::sfnt::{assemble, Tag};
    use crate::WebFontLimits;

    /// A font of one table: `CPAL` version 1 with two palettes of two colors, the second marked
    /// for a dark background.
    fn font() -> Vec<u8> {
        let mut cpal = vec![0, 1, 0, 2, 0, 2, 0, 4, 0, 0, 0, 28, 0, 0, 0, 2];
        cpal.extend_from_slice(&[0, 0, 0, 44, 0, 0, 0, 0, 0, 0, 0, 0]);
        for bgra in [[1u8, 2, 3, 255], [4, 5, 6, 255], [7, 8, 9, 255], [10, 11, 12, 255]] {
            cpal.extend_from_slice(&bgra);
        }
        cpal.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 2]);
        let mut tables: Vec<(Tag, &[u8])> = vec![(*b"CPAL", &cpal)];
        assemble(0x0001_0000, &mut tables, &WebFontLimits::DEFAULT).unwrap()
    }

    fn colors(font: &[u8]) -> Vec<u8> {
        let (offset, _) = table_range(font, b"CPAL").unwrap();
        font[offset + 28..offset + 36].to_vec()
    }

    fn rule(css: &str) -> FontPaletteValues {
        parse_font_palettes(css).remove(0)
    }

    #[test]
    fn a_base_palette_becomes_the_default() {
        let mut data = font();
        assert!(apply_font_palette(&mut data, &rule("@font-palette-values --p { font-family: F; base-palette: 1 }")));
        assert_eq!(colors(&data), [7, 8, 9, 255, 10, 11, 12, 255]);
    }

    #[test]
    fn dark_and_light_read_the_types_of_the_palettes() {
        let mut data = font();
        apply_font_palette(&mut data, &rule("@font-palette-values --p { font-family: F; base-palette: dark }"));
        assert_eq!(colors(&data)[..4], [7, 8, 9, 255]);
        let mut data = font();
        apply_font_palette(&mut data, &rule("@font-palette-values --p { font-family: F; base-palette: light }"));
        assert_eq!(colors(&data)[..4], [1, 2, 3, 255]);
    }

    #[test]
    fn a_palette_the_font_does_not_have_behaves_as_zero() {
        let mut data = font();
        apply_font_palette(&mut data, &rule("@font-palette-values --p { font-family: F; base-palette: 9 }"));
        assert_eq!(colors(&data)[..4], [1, 2, 3, 255]);
    }

    #[test]
    fn override_colors_replace_single_entries() {
        let mut data = font();
        let css = "@font-palette-values --p { font-family: F; base-palette: 1; override-colors: 1 #ff0080, 7 red }";
        apply_font_palette(&mut data, &rule(css));
        assert_eq!(colors(&data), [7, 8, 9, 255, 0x80, 0, 0xFF, 255], "an index past the end is ignored");
    }

    #[test]
    fn a_font_without_a_palette_is_left_alone() {
        let mut tables: Vec<(Tag, &[u8])> = vec![(*b"name", &[0, 0, 0, 0, 0, 6])];
        let mut data = assemble(0x0001_0000, &mut tables, &WebFontLimits::DEFAULT).unwrap();
        let original = data.clone();
        assert!(!apply_font_palette(&mut data, &rule("@font-palette-values --p { font-family: F; base-palette: 1 }")));
        assert!(data == original);
    }

    #[test]
    fn the_forms_of_a_color() {
        let parsed =
            parse_override_colors("0 #abc, 1 #11223344, 2 rgb(1, 2, 3), 3 rgba(10 20 30 / 50%), 4 NAVY").unwrap();
        assert_eq!(parsed[0], (0, [0xAA, 0xBB, 0xCC, 255]));
        assert_eq!(parsed[1], (1, [0x11, 0x22, 0x33, 0x44]));
        assert_eq!(parsed[2], (2, [1, 2, 3, 255]));
        assert_eq!(parsed[3], (3, [10, 20, 30, 128]));
        assert_eq!(parsed[4], (4, [0, 0, 128, 255]));
        for invalid in ["", "red", "0", "0 nosuchcolor", "0 #12", "-1 red", "0 red 1 blue", "0 rgb(1)"] {
            assert!(parse_override_colors(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn rules_are_read_from_a_style_sheet() {
        let palettes = parse_font_palettes(
            "@font-face { font-family: F; src: url(a) }
             @font-palette-values --one { font-family: F, \"G H\"; base-palette: light }
             @font-palette-values --none { base-palette: 1 }
             @font-palette-values not-dashed { font-family: F }
             @font-palette-values --two { font-family: F; base-palette: 2; base-palette: nope }",
        );
        assert_eq!(palettes.len(), 2, "a rule needs a family and a dashed name");
        assert_eq!(palettes[0].families, ["F", "G H"]);
        assert_eq!(palettes[0].base_palette, BasePalette::Light);
        assert_eq!(palettes[1].base_palette, BasePalette::Index(2), "an invalid value keeps the last valid one");
    }
}
