//! Color-contrast calculation ported from the pinned Gallery's
//! `Samples/AccessibilityColorContrast/AccessibilityColorContrastPage.xaml.cs`.
//! Copyright (c) Microsoft Corporation. MIT licensed; see `NOTICE.md`.

use ferroui_base::media::Color;

/// The original page evaluates opaque RGB colors; transparent compositing needs
/// an explicit backdrop and is deliberately outside this sample's input model.
pub fn parse_rgb(text: &str) -> Option<Color> {
    let digits = text.trim().strip_prefix('#').unwrap_or(text.trim());
    if digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let rgb = u32::from_str_radix(digits, 16).ok()?;
    Some(Color::from_rgb(
        (rgb >> 16) as u8,
        (rgb >> 8) as u8,
        rgb as u8,
    ))
}

pub fn relative_luminance(color: Color) -> f64 {
    fn linear(component: u8) -> f64 {
        let srgb = f64::from(component) / 255.0;
        if srgb <= 0.04045 {
            srgb / 12.92
        } else {
            ((srgb + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

pub fn contrast_ratio(first: Color, second: Color) -> f64 {
    let first = relative_luminance(first);
    let second = relative_luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContrastResult {
    pub ratio: f64,
    pub normal_text: bool,
    pub large_text: bool,
    pub components: bool,
}

impl ContrastResult {
    pub fn calculate(first: Color, second: Color) -> Self {
        let ratio = contrast_ratio(first, second);
        Self {
            ratio,
            normal_text: ratio >= 4.5,
            large_text: ratio >= 3.0,
            components: ratio >= 3.0,
        }
    }
}
