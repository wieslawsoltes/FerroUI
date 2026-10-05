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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CompositionStretch {
    #[default]
    None = 0,
    Fill = 1,
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
}
