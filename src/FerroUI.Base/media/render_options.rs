use crate::media::imaging::{BitmapBlendingMode, BitmapInterpolationMode};
use crate::media::{EdgeMode, TextRenderingMode};
use crate::Visual;

/// Provides a set of options that control rendering behavior for visuals,
/// including text rendering, bitmap interpolation, edge rendering, blending,
/// and opacity handling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RenderOptions {
    /// The text rendering mode. Superseded by the text options.
    pub text_rendering_mode: TextRenderingMode,
    /// The interpolation mode used when rendering bitmap images.
    pub bitmap_interpolation_mode: BitmapInterpolationMode,
    /// The edge rendering mode for non-text primitives.
    pub edge_mode: EdgeMode,
    /// The blending mode used when rendering bitmap images.
    pub bitmap_blending_mode: BitmapBlendingMode,
    /// Whether full opacity handling is required for the associated content.
    pub requires_full_opacity_handling: Option<bool>,
}

impl RenderOptions {
    /// Gets the value of the `BitmapInterpolationMode` render option for a
    /// visual.
    pub fn get_bitmap_interpolation_mode(visual: &Visual) -> BitmapInterpolationMode {
        visual.render_options().bitmap_interpolation_mode
    }

    /// Sets the value of the `BitmapInterpolationMode` render option for a
    /// visual.
    pub fn set_bitmap_interpolation_mode(visual: &Visual, value: BitmapInterpolationMode) {
        visual.set_render_options(RenderOptions { bitmap_interpolation_mode: value, ..visual.render_options() });
    }

    /// Gets the value of the `BitmapBlendingMode` render option for a
    /// visual.
    pub fn get_bitmap_blending_mode(visual: &Visual) -> BitmapBlendingMode {
        visual.render_options().bitmap_blending_mode
    }

    /// Sets the value of the `BitmapBlendingMode` render option for a
    /// visual.
    pub fn set_bitmap_blending_mode(visual: &Visual, value: BitmapBlendingMode) {
        visual.set_render_options(RenderOptions { bitmap_blending_mode: value, ..visual.render_options() });
    }

    /// Gets the value of the `EdgeMode` render option for a visual.
    pub fn get_edge_mode(visual: &Visual) -> EdgeMode {
        visual.render_options().edge_mode
    }

    /// Sets the value of the `EdgeMode` render option for a visual.
    pub fn set_edge_mode(visual: &Visual, value: EdgeMode) {
        visual.set_render_options(RenderOptions { edge_mode: value, ..visual.render_options() });
    }

    /// Gets the value of the `TextRenderingMode` render option for a visual.
    /// Superseded by the text options.
    pub fn get_text_rendering_mode(visual: &Visual) -> TextRenderingMode {
        visual.render_options().text_rendering_mode
    }

    /// Sets the value of the `TextRenderingMode` render option for a visual.
    /// Superseded by the text options.
    pub fn set_text_rendering_mode(visual: &Visual, value: TextRenderingMode) {
        visual.set_render_options(RenderOptions { text_rendering_mode: value, ..visual.render_options() });
    }

    /// Gets the value of the `RequiresFullOpacityHandling` render option for
    /// a visual.
    pub fn get_requires_full_opacity_handling(visual: &Visual) -> Option<bool> {
        visual.render_options().requires_full_opacity_handling
    }

    /// Sets the value of the `RequiresFullOpacityHandling` render option for
    /// a visual.
    pub fn set_requires_full_opacity_handling(visual: &Visual, value: Option<bool>) {
        visual.set_render_options(RenderOptions { requires_full_opacity_handling: value, ..visual.render_options() });
    }

    /// Returns options where every unspecified value of `self` is taken from
    /// `other`.
    pub fn merge_with(&self, other: RenderOptions) -> RenderOptions {
        let mut bitmap_interpolation_mode = self.bitmap_interpolation_mode;
        if bitmap_interpolation_mode == BitmapInterpolationMode::Unspecified {
            bitmap_interpolation_mode = other.bitmap_interpolation_mode;
        }

        let mut edge_mode = self.edge_mode;
        if edge_mode == EdgeMode::Unspecified {
            edge_mode = other.edge_mode;
        }

        let mut text_rendering_mode = self.text_rendering_mode;
        if text_rendering_mode == TextRenderingMode::Unspecified {
            text_rendering_mode = other.text_rendering_mode;
        }

        let mut bitmap_blending_mode = self.bitmap_blending_mode;
        if bitmap_blending_mode == BitmapBlendingMode::Unspecified {
            bitmap_blending_mode = other.bitmap_blending_mode;
        }

        let requires_full_opacity_handling =
            self.requires_full_opacity_handling.or(other.requires_full_opacity_handling);

        RenderOptions {
            bitmap_interpolation_mode,
            edge_mode,
            text_rendering_mode,
            bitmap_blending_mode,
            requires_full_opacity_handling,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_with_takes_unspecified_values_from_other() {
        let a = RenderOptions { edge_mode: EdgeMode::Aliased, ..Default::default() };
        let b = RenderOptions {
            edge_mode: EdgeMode::Antialias,
            bitmap_interpolation_mode: BitmapInterpolationMode::HighQuality,
            requires_full_opacity_handling: Some(true),
            ..Default::default()
        };
        let merged = a.merge_with(b);
        assert_eq!(EdgeMode::Aliased, merged.edge_mode);
        assert_eq!(BitmapInterpolationMode::HighQuality, merged.bitmap_interpolation_mode);
        assert_eq!(Some(true), merged.requires_full_opacity_handling);
        assert_eq!(TextRenderingMode::Unspecified, merged.text_rendering_mode);
    }

    #[test]
    fn visual_accessors_update_one_option_each() {
        let visual = Visual::new();
        assert_eq!(BitmapInterpolationMode::Unspecified, RenderOptions::get_bitmap_interpolation_mode(&visual));

        RenderOptions::set_bitmap_interpolation_mode(&visual, BitmapInterpolationMode::LowQuality);
        RenderOptions::set_edge_mode(&visual, EdgeMode::Aliased);
        RenderOptions::set_bitmap_blending_mode(&visual, BitmapBlendingMode::Multiply);
        RenderOptions::set_text_rendering_mode(&visual, TextRenderingMode::Antialias);
        RenderOptions::set_requires_full_opacity_handling(&visual, Some(true));

        assert_eq!(BitmapInterpolationMode::LowQuality, RenderOptions::get_bitmap_interpolation_mode(&visual));
        assert_eq!(EdgeMode::Aliased, RenderOptions::get_edge_mode(&visual));
        assert_eq!(BitmapBlendingMode::Multiply, RenderOptions::get_bitmap_blending_mode(&visual));
        assert_eq!(TextRenderingMode::Antialias, RenderOptions::get_text_rendering_mode(&visual));
        assert_eq!(Some(true), RenderOptions::get_requires_full_opacity_handling(&visual));
    }
}
