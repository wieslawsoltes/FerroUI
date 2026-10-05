use crate::media::{BaselinePixelAlignment, TextHintingMode, TextRenderingMode};
use crate::Visual;

/// Provides options for controlling text rendering and hinting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextOptions {
    /// Gets the text rendering mode.
    pub text_rendering_mode: TextRenderingMode,
    /// Gets the text hinting mode.
    pub text_hinting_mode: TextHintingMode,
    /// Gets a value indicating whether the text baseline should be aligned to
    /// the pixel grid.
    pub baseline_pixel_alignment: BaselinePixelAlignment,
}

impl TextOptions {
    /// Merges this instance with `other`: every unspecified value of `self`
    /// is taken from `other`.
    pub fn merge_with(&self, other: TextOptions) -> TextOptions {
        let mut text_rendering_mode = self.text_rendering_mode;

        if text_rendering_mode == TextRenderingMode::Unspecified {
            text_rendering_mode = other.text_rendering_mode;
        }

        let mut text_hinting_mode = self.text_hinting_mode;

        if text_hinting_mode == TextHintingMode::Unspecified {
            text_hinting_mode = other.text_hinting_mode;
        }

        let mut baseline_pixel_alignment = self.baseline_pixel_alignment;

        if baseline_pixel_alignment == BaselinePixelAlignment::Unspecified {
            baseline_pixel_alignment = other.baseline_pixel_alignment;
        }

        TextOptions { text_rendering_mode, text_hinting_mode, baseline_pixel_alignment }
    }

    /// Gets the text options of a visual.
    pub fn get_text_options(visual: &Visual) -> TextOptions {
        visual.text_options()
    }

    /// Sets the text options of a visual.
    pub fn set_text_options(visual: &Visual, value: TextOptions) {
        visual.set_text_options(value);
    }

    /// Gets the value of the `TextRenderingMode` text option for a visual.
    pub fn get_text_rendering_mode(visual: &Visual) -> TextRenderingMode {
        visual.text_options().text_rendering_mode
    }

    /// Sets the value of the `TextRenderingMode` text option for a visual.
    pub fn set_text_rendering_mode(visual: &Visual, value: TextRenderingMode) {
        visual.set_text_options(TextOptions { text_rendering_mode: value, ..visual.text_options() });
    }

    /// Gets the value of the `TextHintingMode` text option for a visual.
    pub fn get_text_hinting_mode(visual: &Visual) -> TextHintingMode {
        visual.text_options().text_hinting_mode
    }

    /// Sets the value of the `TextHintingMode` text option for a visual.
    pub fn set_text_hinting_mode(visual: &Visual, value: TextHintingMode) {
        visual.set_text_options(TextOptions { text_hinting_mode: value, ..visual.text_options() });
    }

    /// Gets the value of the `BaselinePixelAlignment` text option for a
    /// visual.
    pub fn get_baseline_pixel_alignment(visual: &Visual) -> BaselinePixelAlignment {
        visual.text_options().baseline_pixel_alignment
    }

    /// Sets the value of the `BaselinePixelAlignment` text option for a
    /// visual.
    pub fn set_baseline_pixel_alignment(visual: &Visual, value: BaselinePixelAlignment) {
        visual.set_text_options(TextOptions { baseline_pixel_alignment: value, ..visual.text_options() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_with_takes_unspecified_values_from_other() {
        let a = TextOptions { text_hinting_mode: TextHintingMode::Light, ..Default::default() };
        let b = TextOptions {
            text_rendering_mode: TextRenderingMode::Antialias,
            text_hinting_mode: TextHintingMode::Strong,
            baseline_pixel_alignment: BaselinePixelAlignment::Aligned,
        };

        let merged = a.merge_with(b);

        assert_eq!(merged.text_rendering_mode, TextRenderingMode::Antialias);
        assert_eq!(merged.text_hinting_mode, TextHintingMode::Light);
        assert_eq!(merged.baseline_pixel_alignment, BaselinePixelAlignment::Aligned);
    }
}
