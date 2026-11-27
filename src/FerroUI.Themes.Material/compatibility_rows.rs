//! The packed installer of the compatibility resources of the Material theme (#856): the
//! resources that the generated `install_compatibility` of `color_scheme.rs` adds for every
//! scheme dictionary (the keys of the other themes, mapped to Material roles), read from a
//! table instead of one call each.
//!
//! The table is written by `scripts/material-theme/generate.py` (with `PACKED_INSTALLER`) from
//! `scripts/material-theme/compatibility-map.json`: the keys concatenated in the order of the
//! map, a pool of the opacities, and one record of three bytes for each entry:
//!
//! | Byte | Meaning |
//! |---|---|
//! | 0 | the length of the key in bytes (the keys follow each other in the pool) |
//! | 1 | the source: the index of a `MaterialColorRole`, or `0x80` and the index of a `MaterialStatusColor` |
//! | 2 | bit 4: a brush (else a colour); bits 0 to 3: the index of the opacity |
//!
//! The generator checks the widths (a key longer than 255 bytes, more than 128 roles or 16
//! opacities fail the generation) and that the records decode to the map, key for key. The
//! installer adds the entries in the order of the map, with the values the calls it replaces
//! add: a brush of the colour of the source with the opacity, or the colour with its alpha
//! scaled by the opacity.

use ferroui_base::controls::ResourceDictionary;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::metadata::into_markup_value;
use ferroui_base::Ref;
use std::rc::Rc;

use crate::color_scheme::{MaterialColorRole, MaterialColorScheme, MaterialStatusColor};

/// The source byte of a status colour.
pub(crate) const STATUS: u8 = 0x80;
/// The form bit of a brush.
pub(crate) const BRUSH: u8 = 0x10;

/// A packed table of compatibility resources (module documentation).
pub(crate) struct CompatibilityRows {
    pub(crate) keys: &'static str,
    pub(crate) opacities: &'static [f64],
    pub(crate) records: &'static [[u8; 3]],
}

/// The colour of the source byte `source` in `scheme`, if it names a role or a status colour.
fn source_color(scheme: &MaterialColorScheme, source: u8) -> Option<Color> {
    let index = usize::from(source & !STATUS);
    match source & STATUS {
        0 => MaterialColorRole::ALL.get(index).map(|role| scheme.color(*role)),
        _ => MaterialStatusColor::ALL.get(index).map(|status| scheme.status_color(*status)),
    }
}

/// Adds the resources of `rows` for `scheme` to `dictionary`, in order. A record that does not
/// decode (a key beyond the pool or not on a character boundary, an unknown source or opacity)
/// ends the table: the generator checks every record, so it does not happen in a built theme.
pub(crate) fn install(rows: &CompatibilityRows, scheme: &MaterialColorScheme, dictionary: &Ref<ResourceDictionary>) {
    let mut start = 0;
    for &[length, source, form] in rows.records {
        let end = start + usize::from(length);
        let (Some(key), Some(color), Some(&opacity)) =
            (rows.keys.get(start..end), source_color(scheme, source), rows.opacities.get(usize::from(form & 0x0F)))
        else {
            return;
        };
        start = end;
        if form & BRUSH != 0 {
            let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::with_opacity(color, opacity));
            dictionary.set(key, into_markup_value(brush));
        } else {
            let alpha = (color.a as f64 * opacity).round() as u8;
            dictionary.set(key, into_markup_value(Color::from_argb(alpha, color.r, color.g, color.b)));
        }
    }
}

/// The keys of `rows`, in order (for the tests of the generated installer).
#[cfg(test)]
pub(crate) fn keys(rows: &CompatibilityRows) -> Vec<&'static str> {
    let mut start = 0;
    let mut keys = Vec::with_capacity(rows.records.len());
    for &[length, ..] in rows.records {
        let end = start + usize::from(length);
        keys.extend(rows.keys.get(start..end));
        start = end;
    }
    keys
}

/// The resource `key` of `dictionary` as text: a colour, or the opacity and the colour of a
/// solid colour brush (for the tests).
#[cfg(test)]
pub(crate) fn describe(dictionary: &Ref<ResourceDictionary>, key: &str) -> String {
    use ferroui_base::controls::ResourceKey;
    use ferroui_base::media::ISolidColorBrush;
    use ferroui_base::metadata::from_markup_value;
    let value = dictionary.try_get_value(&ResourceKey::from(key)).flatten();
    assert!(value.is_some(), "the resource {key}");
    if let Some(color) = from_markup_value::<Color>(&value) {
        return format!("color {color:?}");
    }
    let brush = from_markup_value::<Rc<dyn IBrush>>(&value).unwrap_or_else(|| panic!("{key} is a brush or a colour"));
    format!("brush {:?} {:?}", brush.opacity(), brush.as_solid_color_brush().map(|solid| solid.color()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The calls the generated installer writes without the packed table, for the same entries.
    fn by_calls(scheme: &MaterialColorScheme, dictionary: &Ref<ResourceDictionary>) {
        let add = |key: &str, color: Color, opacity: f64| {
            let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::with_opacity(color, opacity));
            dictionary.set(key, into_markup_value(brush));
        };
        add("AccentFillColorDefaultBrush", scheme.color(MaterialColorRole::Primary), 1.0);
        {
            let c = scheme.color(MaterialColorRole::OnSurface);
            dictionary.set("AccentFillColorDisabled", into_markup_value(Color::from_argb((c.a as f64 * 0.38).round() as u8, c.r, c.g, c.b)));
        }
        add("SystemFillColorSuccessBrush", scheme.status_color(MaterialStatusColor::Success), 1.0);
        add("SubtleFillColorSecondaryBrush", scheme.color(MaterialColorRole::OnSurface), 0.08);
    }

    const fn role(role: MaterialColorRole) -> u8 {
        role as u8
    }

    static ROWS: CompatibilityRows = CompatibilityRows {
        keys: "AccentFillColorDefaultBrushAccentFillColorDisabledSystemFillColorSuccessBrushSubtleFillColorSecondaryBrush",
        opacities: &[1.0, 0.38, 0.08],
        records: &[
            [27, role(MaterialColorRole::Primary), BRUSH],
            [23, role(MaterialColorRole::OnSurface), 1],
            [27, STATUS | MaterialStatusColor::Success as u8, BRUSH],
            [29, role(MaterialColorRole::OnSurface), BRUSH | 2],
        ],
    };

    /// The table adds what the calls add, key for key, for the light, the dark and an arbitrary
    /// palette.
    #[test]
    fn the_packed_table_adds_what_the_calls_add() {
        let mut arbitrary = MaterialColorScheme::light();
        arbitrary.set_color(MaterialColorRole::Primary, Color::from_uint32(0x8012_3456));
        arbitrary.set_color(MaterialColorRole::OnSurface, Color::from_uint32(0xC0FE_DCBA));
        for scheme in [MaterialColorScheme::light(), MaterialColorScheme::dark(), arbitrary] {
            let (packed, calls) = (ResourceDictionary::new(), ResourceDictionary::new());
            install(&ROWS, &scheme, &packed);
            by_calls(&scheme, &calls);
            assert_eq!(packed.count(), 4);
            for key in ["AccentFillColorDefaultBrush", "AccentFillColorDisabled", "SystemFillColorSuccessBrush", "SubtleFillColorSecondaryBrush"] {
                assert_eq!(describe(&packed, key), describe(&calls, key), "{key}");
            }
        }
    }

    /// A record that does not decode ends the table.
    #[test]
    fn a_record_that_does_not_decode_ends_the_table() {
        static BROKEN: CompatibilityRows =
            CompatibilityRows { keys: "OneTwo", opacities: &[1.0], records: &[[3, 0, BRUSH], [9, 0, BRUSH], [3, 0, BRUSH]] };
        static UNKNOWN: CompatibilityRows = CompatibilityRows { keys: "One", opacities: &[1.0], records: &[[3, 0x7F, BRUSH]] };
        let dictionary = ResourceDictionary::new();
        install(&BROKEN, &MaterialColorScheme::light(), &dictionary);
        assert_eq!(dictionary.count(), 1);
        install(&UNKNOWN, &MaterialColorScheme::light(), &dictionary);
        assert_eq!(dictionary.count(), 1);
    }
}
