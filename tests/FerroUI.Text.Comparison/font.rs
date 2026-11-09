//! The one font every library uses: Roboto Regular of the Material theme of this repository,
//! embedded at compile time, so that no library discovers or loads a font of the system.

/// The bytes of `src/FerroUI.Themes.Material/Assets/Roboto-Regular.ttf`.
pub const ROBOTO_REGULAR: &[u8] = include_bytes!("../../src/FerroUI.Themes.Material/Assets/Roboto-Regular.ttf");

/// The family name of the font (its `name` table), by which every library selects it.
pub const FAMILY: &str = "Roboto";

/// The em size of the text, in pixels (one point is one pixel: every library lays out at a scale
/// of one).
pub const FONT_SIZE: f32 = 14.0;

/// The line height of the font in ems as its `hhea` table states it: ascender minus descender
/// plus line gap, over the units per em (2400 / 2048 for this Roboto).
///
/// cosmic-text has no line height of its own (its `Metrics` take one); it is given this one, the
/// height the other libraries derive from the same metrics by default.
pub fn hhea_line_height_in_ems() -> f32 {
    let head = table(ROBOTO_REGULAR, *b"head").expect("the font has a head table");
    let hhea = table(ROBOTO_REGULAR, *b"hhea").expect("the font has an hhea table");
    let units_per_em = f32::from(u16_at(head, 18));
    let ascender = f32::from(i16_at(hhea, 4));
    let descender = f32::from(i16_at(hhea, 6));
    let line_gap = f32::from(i16_at(hhea, 8));
    (ascender - descender + line_gap) / units_per_em
}

/// The bytes of the table `tag` of the font `data`, from its table directory.
fn table(data: &[u8], tag: [u8; 4]) -> Option<&[u8]> {
    let count = usize::from(u16_at(data, 4));
    (0..count).find_map(|index| {
        let record = 12 + 16 * index;
        if data.get(record..record + 4)? != tag {
            return None;
        }
        let offset = u32_at(data, record + 8) as usize;
        let length = u32_at(data, record + 12) as usize;
        data.get(offset..offset + length)
    })
}

fn u16_at(data: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([data[offset], data[offset + 1]])
}

fn i16_at(data: &[u8], offset: usize) -> i16 {
    i16::from_be_bytes([data[offset], data[offset + 1]])
}

fn u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_height_comes_from_the_hhea_table_of_roboto() {
        // Roboto Regular: ascender 1900, descender -500, line gap 0, 2048 units per em.
        assert_eq!(hhea_line_height_in_ems(), 2400.0 / 2048.0);
    }
}
