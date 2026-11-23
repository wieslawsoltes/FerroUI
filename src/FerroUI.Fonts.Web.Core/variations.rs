//! What a variable font is set to for a request: the resolution of CSS Fonts 4, section 7.2,
//! for the steps that concern variations.

use crate::css::{FontFaceDescriptors, FontVariationSetting};
use crate::matching::{FontMatchRequest, FontStyleRequest};
use crate::sfnt::{ReadBe, Tag};

/// An axis of `fvar`, in its own units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontAxis {
    pub tag: Tag,
    pub min: f32,
    pub default: f32,
    pub max: f32,
}

impl FontAxis {
    fn clamp(&self, value: f32) -> f32 {
        value.clamp(self.min, self.max)
    }
}

/// A named instance of `fvar`: its names, and the position of each axis.
#[derive(Clone, Debug, PartialEq)]
pub struct FontNamedInstance {
    pub names: Vec<String>,
    pub coordinates: Vec<f32>,
}

/// The variation space of a font.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FontAxes {
    pub axes: Vec<FontAxis>,
    pub instances: Vec<FontNamedInstance>,
}

impl FontAxes {
    pub fn has(&self, tag: &Tag) -> bool {
        self.axes.iter().any(|axis| axis.tag == *tag)
    }
}

fn fixed(value: u32) -> f32 {
    (value as i32) as f32 / 65536.0
}

fn table<'a>(font: &'a [u8], tag: &Tag) -> Option<&'a [u8]> {
    let count = font.u16_at(4)? as usize;
    (0..count).find_map(|index| {
        let record = 12 + index * 16;
        if font.get(record..record + 4)? != tag {
            return None;
        }
        let (offset, length) = (font.u32_at(record + 8)? as usize, font.u32_at(record + 12)? as usize);
        font.get(offset..offset.checked_add(length)?)
    })
}

/// Every name a `name` table gives for an id.
fn names_of(name: &[u8], id: u16) -> Vec<String> {
    let mut names = Vec::new();
    let mut read = || -> Option<()> {
        let (count, strings) = (name.u16_at(2)? as usize, name.u16_at(4)? as usize);
        for index in 0..count {
            let record = 6 + index * 12;
            if name.u16_at(record + 6)? != id {
                continue;
            }
            let (length, offset) = (name.u16_at(record + 8)? as usize, name.u16_at(record + 10)? as usize);
            let start = strings.checked_add(offset)?;
            let Some(bytes) = name.get(start..start.checked_add(length)?) else {
                continue;
            };
            let text = match name.u16_at(record)? {
                0 | 3 => {
                    let units: Vec<u16> =
                        bytes.chunks_exact(2).map(|unit| u16::from_be_bytes([unit[0], unit[1]])).collect();
                    String::from_utf16_lossy(&units)
                }
                _ => bytes.iter().map(|byte| *byte as char).collect(),
            };
            if !names.contains(&text) {
                names.push(text);
            }
        }
        Some(())
    };
    let _ = read();
    names
}

/// The variation space of a sanitized font; `None` for a font that is not variable.
pub fn read_font_axes(font: &[u8]) -> Option<FontAxes> {
    let fvar = table(font, b"fvar")?;
    let axes_offset = fvar.u16_at(4)? as usize;
    let axis_count = fvar.u16_at(8)? as usize;
    let instance_count = fvar.u16_at(12)? as usize;
    let instance_size = fvar.u16_at(14)? as usize;
    let mut axes = Vec::with_capacity(axis_count);
    for index in 0..axis_count {
        let at = axes_offset + index * 20;
        let tag: Tag = fvar.get(at..at + 4)?.try_into().ok()?;
        axes.push(FontAxis {
            tag,
            min: fixed(fvar.u32_at(at + 4)?),
            default: fixed(fvar.u32_at(at + 8)?),
            max: fixed(fvar.u32_at(at + 12)?),
        });
    }
    let name = table(font, b"name").unwrap_or(&[]);
    let first = axes_offset + axis_count * 20;
    let mut instances = Vec::with_capacity(instance_count);
    for index in 0..instance_count {
        let at = first + index * instance_size;
        let coordinates =
            (0..axis_count).map(|axis| fvar.u32_at(at + 4 + axis * 4).map(fixed)).collect::<Option<Vec<f32>>>()?;
        instances.push(FontNamedInstance { names: names_of(name, fvar.u16_at(at)?), coordinates });
    }
    Some(FontAxes { axes, instances })
}

/// The position in the variation space for a request, in the order of the standard: weight,
/// width and style clamped to the descriptors and the axes, then the named instance, then the
/// `font-variation-settings` descriptor. Empty for a font without axes.
pub fn resolve_font_variations(
    space: &FontAxes,
    face: &FontFaceDescriptors,
    request: &FontMatchRequest,
) -> Vec<FontVariationSetting> {
    let mut values: Vec<(Tag, f32)> = Vec::with_capacity(space.axes.len());
    let mut set = |tag: Tag, value: f32| {
        if let Some(axis) = space.axes.iter().find(|axis| axis.tag == tag) {
            let value = axis.clamp(value);
            match values.iter_mut().find(|(known, _)| *known == tag) {
                Some(entry) => entry.1 = value,
                None => values.push((tag, value)),
            }
        }
    };
    let weight = face.weight.map_or(request.weight, |range| request.weight.clamp(range.start, range.end));
    set(*b"wght", weight);
    let width = face.width.map_or(request.width, |range| request.width.clamp(range.start, range.end));
    set(*b"wdth", width);
    match request.style {
        FontStyleRequest::Normal => {
            set(*b"ital", 0.0);
            set(*b"slnt", 0.0);
        }
        FontStyleRequest::Italic => set(*b"ital", 1.0),
        // A positive oblique angle leans right, which is a negative value of `slnt`. The
        // `ital` axis does not satisfy an oblique request.
        FontStyleRequest::Oblique(angle) => set(*b"slnt", -angle),
    }
    if let Some(wanted) = &face.named_instance {
        if let Some(instance) = space.instances.iter().find(|instance| instance.names.iter().any(|name| name == wanted))
        {
            for (axis, value) in space.axes.iter().zip(&instance.coordinates) {
                set(axis.tag, *value);
            }
        }
    }
    for setting in &face.variation_settings {
        set(setting.tag, setting.value);
    }
    values.into_iter().map(|(tag, value)| FontVariationSetting { tag, value }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::parse_font_face_block;
    use std::path::PathBuf;

    fn face(descriptors: &str) -> FontFaceDescriptors {
        parse_font_face_block(&format!("font-family: F; src: url(a); {descriptors}")).unwrap()
    }

    fn space() -> FontAxes {
        let axis = |tag: &Tag, min, default, max| FontAxis { tag: *tag, min, default, max };
        FontAxes {
            axes: vec![
                axis(b"wght", 100.0, 400.0, 900.0),
                axis(b"wdth", 75.0, 100.0, 125.0),
                axis(b"slnt", -10.0, 0.0, 0.0),
                axis(b"XHGT", 0.0, 0.5, 1.0),
            ],
            instances: vec![FontNamedInstance {
                names: vec!["Display".into()],
                coordinates: vec![250.0, 90.0, -5.0, 0.25],
            }],
        }
    }

    fn value(settings: &[FontVariationSetting], tag: &Tag) -> Option<f32> {
        settings.iter().find(|setting| setting.tag == *tag).map(|setting| setting.value)
    }

    fn request(weight: f32, width: f32, style: FontStyleRequest) -> FontMatchRequest {
        FontMatchRequest { weight, width, style }
    }

    #[test]
    fn the_request_sets_the_axes_it_names() {
        let settings = resolve_font_variations(&space(), &face(""), &request(650.0, 110.0, FontStyleRequest::Normal));
        assert_eq!(value(&settings, b"wght"), Some(650.0));
        assert_eq!(value(&settings, b"wdth"), Some(110.0));
        assert_eq!(value(&settings, b"slnt"), Some(0.0));
        assert_eq!(value(&settings, b"XHGT"), None, "an axis nothing asks for stays at its default");
        assert_eq!(value(&settings, b"ital"), None, "the font has no such axis");
    }

    #[test]
    fn values_are_clamped_to_the_descriptors_and_the_axes() {
        let face = face("font-weight: 300 500; font-width: 80% 90%");
        let settings = resolve_font_variations(&space(), &face, &request(800.0, 120.0, FontStyleRequest::Normal));
        assert_eq!((value(&settings, b"wght"), value(&settings, b"wdth")), (Some(500.0), Some(90.0)));
        let settings =
            resolve_font_variations(&space(), &self::face(""), &request(1000.0, 200.0, FontStyleRequest::Normal));
        assert_eq!((value(&settings, b"wght"), value(&settings, b"wdth")), (Some(900.0), Some(125.0)));
    }

    #[test]
    fn an_oblique_uses_slnt_with_the_opposite_sign() {
        let settings =
            resolve_font_variations(&space(), &face(""), &request(400.0, 100.0, FontStyleRequest::Oblique(8.0)));
        assert_eq!(value(&settings, b"slnt"), Some(-8.0));
        let settings =
            resolve_font_variations(&space(), &face(""), &request(400.0, 100.0, FontStyleRequest::Oblique(40.0)));
        assert_eq!(value(&settings, b"slnt"), Some(-10.0));
    }

    #[test]
    fn the_order_of_the_standard() {
        let named = face("font-named-instance: \"Display\"");
        let settings = resolve_font_variations(&space(), &named, &request(700.0, 100.0, FontStyleRequest::Normal));
        assert_eq!(value(&settings, b"wght"), Some(250.0), "the named instance is after the weight");
        assert_eq!(value(&settings, b"XHGT"), Some(0.25));
        let both = face("font-named-instance: \"Display\"; font-variation-settings: \"XHGT\" 0.75, \"wght\" 2000");
        let settings = resolve_font_variations(&space(), &both, &request(700.0, 100.0, FontStyleRequest::Normal));
        assert_eq!(value(&settings, b"XHGT"), Some(0.75), "the descriptor is after the named instance");
        assert_eq!(value(&settings, b"wght"), Some(900.0), "and is clamped to the axis");
        let unknown = face("font-named-instance: \"Nope\"");
        let settings = resolve_font_variations(&space(), &unknown, &request(700.0, 100.0, FontStyleRequest::Normal));
        assert_eq!(value(&settings, b"wght"), Some(700.0), "a name that is not found acts as auto");
    }

    #[test]
    fn a_real_variable_font() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../FerroUI.Base/test_assets/fonts/InterVariable.ttf");
        let data = std::fs::read(path).unwrap();
        let space = read_font_axes(&data).expect("a variable font");
        assert!(space.has(b"wght"));
        assert!(!space.instances.is_empty());
        assert!(space.instances.iter().any(|instance| instance.names.iter().any(|name| name.contains("Bold"))));
        let plain =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../FerroUI.Base/test_assets/fonts/Inter-Regular.ttf");
        assert!(read_font_axes(&std::fs::read(plain).unwrap()).is_none());
    }
}
