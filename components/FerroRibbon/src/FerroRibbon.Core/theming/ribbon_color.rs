use crate::{RibbonString, text::trim};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RibbonColor {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
ferroui_system::styled_value!(RibbonColor);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RibbonColorParseError {
    pub message: RibbonString,
}
impl RibbonColor {
    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);
    pub const WHITE: Self = Self::new(255, 255, 255, 255);
    pub const BLACK: Self = Self::new(255, 0, 0, 0);
    pub const fn new(a: u8, r: u8, g: u8, b: u8) -> Self {
        Self { a, r, g, b }
    }
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(255, r, g, b)
    }
    pub fn parse(value: Option<&RibbonString>) -> Result<Self, RibbonColorParseError> {
        let mut color = Self::default();
        if Self::try_parse(value, &mut color) {
            return Ok(color);
        }
        let mut text = vec![39];
        if let Some(value) = value {
            text.extend_from_slice(value.as_utf16());
        }
        text.extend("' is not a valid color. Use #RRGGBB or #AARRGGBB.".encode_utf16());
        Err(RibbonColorParseError {
            message: RibbonString::from_utf16(text),
        })
    }
    pub fn try_parse(value: Option<&RibbonString>, color: &mut Self) -> bool {
        *color = Self::default();
        let Some(value) = value else { return false };
        let text = trim(value.as_utf16());
        if text.is_empty() {
            return false;
        }
        let mut hex = text
            .iter()
            .skip_while(|&&c| c == 35)
            .copied()
            .collect::<Vec<_>>();
        if hex.len() == 3 {
            hex = hex.into_iter().flat_map(|c| [c, c]).collect();
        }
        if hex.len() == 6 {
            hex.splice(0..0, [70, 70]);
        }
        if hex.len() != 8 {
            return false;
        }
        let white = |c: u16| matches!(c, 9..=13 | 32);
        let mut i = 0;
        while i < hex.len() && white(hex[i]) {
            i += 1;
        }
        let mut value = 0_u32;
        let mut digits = 0;
        while i < hex.len() {
            let digit = match hex[i] {
                48..=57 => hex[i] - 48,
                65..=70 => hex[i] - 55,
                97..=102 => hex[i] - 87,
                _ => break,
            };
            value = value * 16 + u32::from(digit);
            digits += 1;
            i += 1;
        }
        while i < hex.len() && white(hex[i]) {
            i += 1;
        }
        if digits == 0 || hex[i..].iter().any(|&c| c != 0) {
            return false;
        }
        *color = Self::new(
            (value >> 24) as u8,
            (value >> 16) as u8,
            (value >> 8) as u8,
            value as u8,
        );
        true
    }
    pub fn luminance(self) -> f64 {
        fn channel(c: u8) -> f64 {
            let v = f64::from(c) / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b)
    }
    pub fn lightness(self) -> f64 {
        self.to_hsl().2
    }
    pub fn contrast_ratio(a: Self, b: Self) -> f64 {
        let l1 = a.luminance();
        let l2 = b.luminance();
        (l1.max(l2) + 0.05) / (l1.min(l2) + 0.05)
    }
    pub fn prefers_dark_foreground(self) -> bool {
        Self::contrast_ratio(self, Self::BLACK) >= Self::contrast_ratio(self, Self::WHITE)
    }
    pub const fn with_alpha(self, alpha: u8) -> Self {
        Self { a: alpha, ..self }
    }
    pub fn lighten(self, amount: f64) -> Self {
        let (h, s, l) = self.to_hsl();
        Self::from_hsl_with_alpha(h, s, l + (1.0 - l) * amount.clamp(0.0, 1.0), self.a)
    }
    pub fn darken(self, amount: f64) -> Self {
        let (h, s, l) = self.to_hsl();
        Self::from_hsl_with_alpha(h, s, l * (1.0 - amount.clamp(0.0, 1.0)), self.a)
    }
    pub fn blend(self, other: Self, amount: f64) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        let mix = |x: u8, y: u8| {
            (f64::from(x) + f64::from(i32::from(y) - i32::from(x)) * amount).round_ties_even() as u8
        };
        Self::new(
            mix(self.a, other.a),
            mix(self.r, other.r),
            mix(self.g, other.g),
            mix(self.b, other.b),
        )
    }
    pub fn to_hsl(self) -> (f64, f64, f64) {
        let r = f64::from(self.r) / 255.0;
        let g = f64::from(self.g) / 255.0;
        let b = f64::from(self.b) / 255.0;
        let max = r.max(g.max(b));
        let min = r.min(g.min(b));
        let l = (max + min) / 2.0;
        if (max - min).abs() < 1e-9 {
            return (0.0, 0.0, l);
        }
        let d = max - min;
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };
        let h = if max == r {
            (g - b) / d + if g < b { 6.0 } else { 0.0 }
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };
        (h * 60.0, s, l)
    }
    pub fn from_hsl(h: f64, s: f64, l: f64) -> Self {
        Self::from_hsl_with_alpha(h, s, l, 255)
    }
    pub fn from_hsl_with_alpha(h: f64, s: f64, l: f64, alpha: u8) -> Self {
        let h = ((h % 360.0) + 360.0) % 360.0 / 360.0;
        let s = s.clamp(0.0, 1.0);
        let l = l.clamp(0.0, 1.0);
        if s <= 0.0 {
            let v = (l * 255.0).round_ties_even() as u8;
            return Self::new(alpha, v, v, v);
        }
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;
        fn hue(p: f64, q: f64, mut t: f64) -> f64 {
            if t < 0.0 {
                t += 1.0;
            }
            if t > 1.0 {
                t -= 1.0;
            }
            if t < 1.0 / 6.0 {
                return p + (q - p) * 6.0 * t;
            }
            if t < 0.5 {
                return q;
            }
            if t < 2.0 / 3.0 {
                return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
            }
            p
        }
        Self::new(
            alpha,
            (hue(p, q, h + 1.0 / 3.0) * 255.0).round_ties_even() as u8,
            (hue(p, q, h) * 255.0).round_ties_even() as u8,
            (hue(p, q, h - 1.0 / 3.0) * 255.0).round_ties_even() as u8,
        )
    }
    pub fn to_hex(self, include_alpha: bool) -> String {
        if include_alpha || self.a != 255 {
            format!("#{:02X}{:02X}{:02X}{:02X}", self.a, self.r, self.g, self.b)
        } else {
            format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        }
    }
}
impl std::fmt::Display for RibbonColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex(false))
    }
}
