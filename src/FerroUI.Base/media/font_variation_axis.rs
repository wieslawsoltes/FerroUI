//! The variation space of a variable font: its axes and its named instances, as the font backend
//! reports them (`FontVariationAxis` and `FontVariationInstance` of upstream). The backends read
//! them with their own font code (Skia, `skrifa` for Vello); no table is parsed here.

use crate::media::fonts::OpenTypeTag;
use crate::media::FontVariationSettings;

/// An axis of a variable font (a record of its `fvar` table, with the `avar` mapping applied by
/// the backend when it makes an instance), in the user-space units of the axis.
#[derive(Clone, Debug, PartialEq)]
pub struct FontVariationAxis {
    /// The four-character tag of the axis (`wght`, `wdth`, `opsz`, ...).
    pub tag: OpenTypeTag,
    /// The name of the axis in the `name` table of the font, where the backend reads it.
    pub name: Option<String>,
    /// The lowest value of the axis.
    pub min_value: f64,
    /// The value of the default instance.
    pub default_value: f64,
    /// The highest value of the axis.
    pub max_value: f64,
    /// The font asks for the axis not to be shown in user interfaces.
    pub is_hidden: bool,
}

impl FontVariationAxis {
    /// `value` clamped to the range of the axis; the default value for a NaN.
    pub fn clamp(&self, value: f64) -> f64 {
        if value.is_nan() {
            return self.default_value;
        }
        value.clamp(self.min_value.min(self.max_value), self.max_value.max(self.min_value))
    }
}

/// A named instance of a variable font (`Bold`, `Condensed Light`): a position in its variation
/// space that the font names.
#[derive(Clone, Debug, PartialEq)]
pub struct FontVariationInstance {
    /// The subfamily name of the instance, where the backend reads it.
    pub name: Option<String>,
    /// The PostScript name of the instance, where the font has one.
    pub postscript_name: Option<String>,
    /// The value of each axis at the instance.
    pub coordinates: FontVariationSettings,
}

/// The variation space of a typeface: its axes and its named instances. Empty for a font that is
/// not variable and for a backend that makes no instances.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FontVariationDesign {
    /// The axes, in the order of the font.
    pub axes: Vec<FontVariationAxis>,
    /// The named instances, in the order of the font.
    pub named_instances: Vec<FontVariationInstance>,
}

impl FontVariationDesign {
    /// Whether the font has no axis.
    pub fn is_empty(&self) -> bool {
        self.axes.is_empty()
    }

    /// The axis with the tag.
    pub fn axis(&self, tag: OpenTypeTag) -> Option<&FontVariationAxis> {
        self.axes.iter().find(|axis| axis.tag == tag)
    }

    /// `settings` with the axes the font does not have dropped and every value clamped to the
    /// range of its axis.
    pub fn clamp(&self, settings: &FontVariationSettings) -> FontVariationSettings {
        FontVariationSettings::new(settings.variations().iter().filter_map(|variation| {
            let axis = self.axis(variation.tag)?;
            Some(crate::media::FontVariation::new(variation.tag, axis.clamp(variation.value)))
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::FontVariation;

    fn weight_axis() -> FontVariationAxis {
        FontVariationAxis {
            tag: OpenTypeTag::new(u32::from_be_bytes(*b"wght")),
            name: Some("Weight".to_string()),
            min_value: 100.0,
            default_value: 400.0,
            max_value: 900.0,
            is_hidden: false,
        }
    }

    #[test]
    fn values_are_clamped_to_the_axis() {
        let axis = weight_axis();
        assert_eq!(axis.clamp(50.0), 100.0);
        assert_eq!(axis.clamp(1000.0), 900.0);
        assert_eq!(axis.clamp(650.0), 650.0);
        assert_eq!(axis.clamp(f64::NAN), 400.0);
    }

    #[test]
    fn settings_keep_the_axes_of_the_font_only() {
        let design = FontVariationDesign { axes: vec![weight_axis()], named_instances: Vec::new() };
        let settings = FontVariationSettings::new([
            FontVariation::new(OpenTypeTag::new(u32::from_be_bytes(*b"wght")), 1200.0),
            FontVariation::new(OpenTypeTag::new(u32::from_be_bytes(*b"wdth")), 75.0),
        ]);
        let clamped = design.clamp(&settings);
        assert_eq!(clamped.variations(), &[FontVariation::new(OpenTypeTag::new(u32::from_be_bytes(*b"wght")), 900.0)]);
        assert!(FontVariationDesign::default().is_empty());
    }
}
