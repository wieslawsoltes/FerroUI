//! The variation space of a web font in the model of `ferroui-base`.
//!
//! The decoder reads `fvar` itself, before a backend has the font, and keeps the names of each
//! named instance in every language for `font-named-instance`. What a face shows of its
//! variation space is `FontVariationDesign`, the model every backend reports, with the first name
//! of each instance.

use ferroui_base::media::fonts::OpenTypeTag;
use ferroui_base::media::{
    FontVariation, FontVariationAxis, FontVariationDesign, FontVariationInstance, FontVariationSettings,
};
use ferroui_fonts_web_core::variations::FvarTable;

fn tag(tag: [u8; 4]) -> OpenTypeTag {
    OpenTypeTag::new(u32::from_be_bytes(tag))
}

/// The variation space of an `fvar` table as `ferroui-base` models it.
pub fn variation_design(table: &FvarTable) -> FontVariationDesign {
    let axes = table
        .axes
        .iter()
        .map(|axis| FontVariationAxis {
            tag: tag(axis.tag),
            name: axis.name.clone(),
            min_value: f64::from(axis.min),
            default_value: f64::from(axis.default),
            max_value: f64::from(axis.max),
            is_hidden: axis.hidden,
        })
        .collect();
    let named_instances = table
        .instances
        .iter()
        .map(|instance| FontVariationInstance {
            name: instance.names.first().cloned(),
            postscript_name: instance.postscript_name.clone(),
            coordinates: FontVariationSettings::new(
                table
                    .axes
                    .iter()
                    .zip(&instance.coordinates)
                    .map(|(axis, value)| FontVariation::new(tag(axis.tag), f64::from(*value))),
            ),
        })
        .collect();
    FontVariationDesign { axes, named_instances }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FontDataStream;
    use ferroui_base::media::FontSimulations;
    use ferroui_base::platform::IFontManagerImpl;
    use ferroui_fonts_web_core::variations::read_fvar;
    use std::path::PathBuf;

    fn font(name: &str) -> Vec<u8> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../FerroUI.Base/test_assets/fonts").join(name);
        std::fs::read(path).unwrap()
    }

    #[test]
    fn the_design_of_the_reader_is_the_one_the_backend_reports() {
        let bytes = font("InterVariable.ttf");
        let ours = variation_design(&read_fvar(&bytes).expect("a variable font"));
        let platform = ferroui_vello::FontManagerImpl::new();
        let mut stream = FontDataStream::new(bytes);
        let typeface = platform.try_create_glyph_typeface_from_stream(&mut stream, FontSimulations::None).unwrap();
        let theirs = typeface.variation_design();
        assert!(!ours.is_empty());
        let axes = |design: &FontVariationDesign| {
            let axes = design.axes.iter();
            axes.map(|axis| (axis.tag, axis.min_value, axis.default_value, axis.max_value)).collect::<Vec<_>>()
        };
        assert_eq!(axes(&ours), axes(&theirs));
        let coordinates = |design: &FontVariationDesign| {
            design.named_instances.iter().map(|instance| instance.coordinates.clone()).collect::<Vec<_>>()
        };
        assert_eq!(coordinates(&ours), coordinates(&theirs));
        assert!(ours.named_instances.iter().all(|instance| instance.name.is_some()));
    }

    #[test]
    fn a_font_that_is_not_variable_has_no_design() {
        assert!(read_fvar(&font("Inter-Regular.ttf")).is_none());
    }
}
