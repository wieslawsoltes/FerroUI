use super::transport::{BatchValue, BatchValueReader};
use crate::media::imaging::BitmapInterpolationMode;
use bitflags::bitflags;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CompositionBlendMode {
    /// No regions are enabled.
    #[default]
    Clear,
    /// Only the source will be present.
    Src,
    /// Only the destination will be present.
    Dst,
    /// Source is placed over the destination.
    SrcOver,
    /// Destination is placed over the source.
    DstOver,
    /// The source that overlaps the destination, replaces the destination.
    SrcIn,
    /// Destination which overlaps the source, replaces the source.
    DstIn,
    /// Source is placed, where it falls outside of the destination.
    SrcOut,
    /// Destination is placed, where it falls outside of the source.
    DstOut,
    /// Source which overlaps the destination, replaces the destination.
    SrcATop,
    /// Destination which overlaps the source replaces the source.
    DstATop,
    /// The non-overlapping regions of source and destination are combined.
    Xor,
    /// Display the sum of the source image and destination image.
    Plus,
    /// Multiplies all components (= alpha and color).
    Modulate,
    /// Multiplies the complements of the backdrop and source color values,
    /// then complements the result.
    Screen,
    /// Multiplies or screens the colors, depending on the backdrop color
    /// value.
    Overlay,
    /// Selects the darker of the backdrop and source colors.
    Darken,
    /// Selects the lighter of the backdrop and source colors.
    Lighten,
    /// Brightens the backdrop color to reflect the source color.
    ColorDodge,
    /// Darkens the backdrop color to reflect the source color.
    ColorBurn,
    /// Multiplies or screens the colors, depending on the source color
    /// value.
    HardLight,
    /// Darkens or lightens the colors, depending on the source color value.
    SoftLight,
    /// Subtracts the darker of the two constituent colors from the lighter
    /// color.
    Difference,
    /// Produces an effect similar to that of the Difference mode but lower
    /// in contrast.
    Exclusion,
    /// The source color is multiplied by the destination color and replaces
    /// the destination.
    Multiply,
    /// Creates a color with the hue of the source color and the saturation
    /// and luminosity of the backdrop color.
    Hue,
    /// Creates a color with the saturation of the source color and the hue
    /// and luminosity of the backdrop color.
    Saturation,
    /// Creates a color with the hue and saturation of the source color and
    /// the luminosity of the backdrop color.
    Color,
    /// Creates a color with the luminosity of the source color and the hue
    /// and saturation of the backdrop color.
    Luminosity,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CompositionGradientExtendMode {
    #[default]
    Clamp,
    Wrap,
    Mirror,
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct CompositionTileMode: i32 {
        const NONE = 0;
        const TILE_X = 1;
        const TILE_Y = 2;
        const FLIP_X = 4;
        const FLIP_Y = 8;
        const TILE = Self::TILE_X.bits() | Self::TILE_Y.bits();
        const FLIP = Self::FLIP_X.bits() | Self::FLIP_Y.bits();
    }
}

/// How the content of a composition surface brush fills the area of the
/// brush (WinUI `CompositionStretch`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CompositionStretch {
    /// The content keeps its natural size.
    #[default]
    None = 0,
    /// The content is scaled to the area, without keeping its aspect ratio.
    Fill = 1,
    /// The content is scaled to fit inside the area, keeping its aspect ratio.
    Uniform = 2,
    /// The content is scaled to cover the area, keeping its aspect ratio; the
    /// part outside the area is clipped.
    UniformToFill = 3,
}

/// How a bitmap is resampled when a composition surface brush scales it
/// (WinUI `CompositionBitmapInterpolationMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CompositionBitmapInterpolationMode {
    /// The nearest pixel.
    NearestNeighbor = 0,
    /// Linear interpolation (the WinUI default).
    #[default]
    Linear = 1,
    MagLinearMinLinearMipLinear = 2,
    MagLinearMinLinearMipNearest = 3,
    MagLinearMinNearestMipLinear = 4,
    MagLinearMinNearestMipNearest = 5,
    MagNearestMinLinearMipLinear = 6,
    MagNearestMinLinearMipNearest = 7,
    MagNearestMinNearestMipLinear = 8,
    MagNearestMinNearestMipNearest = 9,
}

impl CompositionBitmapInterpolationMode {
    const ALL: [CompositionBitmapInterpolationMode; 10] = [
        Self::NearestNeighbor,
        Self::Linear,
        Self::MagLinearMinLinearMipLinear,
        Self::MagLinearMinLinearMipNearest,
        Self::MagLinearMinNearestMipLinear,
        Self::MagLinearMinNearestMipNearest,
        Self::MagNearestMinLinearMipLinear,
        Self::MagNearestMinLinearMipNearest,
        Self::MagNearestMinNearestMipLinear,
        Self::MagNearestMinNearestMipNearest,
    ];

    /// The interpolation mode of the drawing context that comes closest.
    ///
    /// Deviation (winui-composition.md): the drawing contexts have one
    /// quality setting, not separate minification, magnification and mip
    /// filters. Nearest magnification maps to no interpolation, a linear
    /// filter with linear mip filtering to the medium quality (mipmaps),
    /// every other linear filter to the low quality (bilinear).
    pub fn to_bitmap_interpolation_mode(self) -> BitmapInterpolationMode {
        match self {
            Self::NearestNeighbor
            | Self::MagNearestMinLinearMipLinear
            | Self::MagNearestMinLinearMipNearest
            | Self::MagNearestMinNearestMipLinear
            | Self::MagNearestMinNearestMipNearest => BitmapInterpolationMode::None,
            Self::MagLinearMinLinearMipLinear => BitmapInterpolationMode::MediumQuality,
            Self::Linear
            | Self::MagLinearMinLinearMipNearest
            | Self::MagLinearMinNearestMipLinear
            | Self::MagLinearMinNearestMipNearest => BitmapInterpolationMode::LowQuality,
        }
    }
}

/// What a drop shadow is cast from (WinUI
/// `CompositionDropShadowSourcePolicy`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CompositionDropShadowSourcePolicy {
    /// The shadow uses the mask, or the shape of the visual when there is
    /// none.
    #[default]
    Default = 0,
    /// The shadow uses the alpha of the content of the visual.
    InheritFromVisualContent = 1,
}

impl BatchValue for CompositionBitmapInterpolationMode {
    fn write_to(&self, out: &mut Vec<u8>) {
        (*self as i32).write_to(out);
    }

    fn read_from(input: &mut BatchValueReader<'_>) -> Self {
        let value = i32::read_from(input);
        match Self::ALL.iter().find(|mode| **mode as i32 == value) {
            Some(mode) => *mode,
            None => panic!("the batch value stream holds {value}, which is not a CompositionBitmapInterpolationMode"),
        }
    }
}

impl BatchValue for CompositionDropShadowSourcePolicy {
    fn write_to(&self, out: &mut Vec<u8>) {
        (*self as i32).write_to(out);
    }

    fn read_from(input: &mut BatchValueReader<'_>) -> Self {
        match i32::read_from(input) {
            0 => Self::Default,
            1 => Self::InheritFromVisualContent,
            value => panic!("the batch value stream holds {value}, which is not a CompositionDropShadowSourcePolicy"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_match_the_reference() {
        assert_eq!(CompositionBlendMode::Clear as i32, 0);
        assert_eq!(CompositionBlendMode::SrcOver as i32, 3);
        assert_eq!(CompositionBlendMode::Luminosity as i32, 28);
        assert_eq!(CompositionGradientExtendMode::Mirror as i32, 2);
        assert_eq!(CompositionTileMode::TILE.bits(), 3);
        assert_eq!(CompositionTileMode::FLIP.bits(), 12);
        assert_eq!(CompositionStretch::Fill as i32, 1);
    }

    #[test]
    fn winui_values() {
        assert_eq!(CompositionStretch::Uniform as i32, 2);
        assert_eq!(CompositionStretch::UniformToFill as i32, 3);
        assert_eq!(CompositionBitmapInterpolationMode::default(), CompositionBitmapInterpolationMode::Linear);
        assert_eq!(CompositionBitmapInterpolationMode::MagNearestMinNearestMipNearest as i32, 9);
        assert_eq!(CompositionDropShadowSourcePolicy::InheritFromVisualContent as i32, 1);
    }

    #[test]
    fn enums_cross_a_batch() {
        use crate::rendering::composition::transport::{BatchValue, BatchValueReader};

        let mut out = Vec::new();
        CompositionBitmapInterpolationMode::MagLinearMinNearestMipLinear.write_to(&mut out);
        CompositionDropShadowSourcePolicy::InheritFromVisualContent.write_to(&mut out);
        CompositionStretch::UniformToFill.write_to(&mut out);
        let mut offset = 0;
        let mut input = BatchValueReader::new(&out, &mut offset);
        assert_eq!(
            CompositionBitmapInterpolationMode::MagLinearMinNearestMipLinear,
            CompositionBitmapInterpolationMode::read_from(&mut input)
        );
        assert_eq!(
            CompositionDropShadowSourcePolicy::InheritFromVisualContent,
            CompositionDropShadowSourcePolicy::read_from(&mut input)
        );
        assert_eq!(CompositionStretch::UniformToFill, CompositionStretch::read_from(&mut input));
    }
}
