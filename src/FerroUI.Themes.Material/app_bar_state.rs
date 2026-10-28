// Copyright 2020-2026 The Android Open Source Project (reference algorithms).
// Copyright 2026 FerroUI contributors (Rust implementation).
// SPDX-License-Identifier: Apache-2.0
//! Scroll and fling arithmetic for Material top app bars.
//!
//! Reference: AndroidX revision `e4d933c98df7baa7ea2b83eea6e086915961e0ff`,
//! `compose/material3/material3/.../AppBar.kt`, and animation's
//! `SplineBasedDecay.kt` / `FlingCalculator.kt`. Logical pixels are used throughout;
//! the native density coefficient cancels when velocity and distance are in dp.

use ferroui_base::media::Color;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MaterialAppBarSize {
    #[default]
    Small,
    CenterAligned,
    Medium,
    Large,
    MediumFlexible,
    LargeFlexible,
}
ferroui_base::ferro_markup_enum!(
    MaterialAppBarSize { Small, CenterAligned, Medium, Large, MediumFlexible, LargeFlexible },
    { namespace: "FerroUI.Themes.Material" }
);

impl MaterialAppBarSize {
    pub fn has_two_rows(self) -> bool {
        !matches!(self, Self::Small | Self::CenterAligned)
    }
    pub fn expanded_height(self, subtitle: bool) -> f64 {
        match self {
            Self::Small | Self::CenterAligned => 64.0,
            Self::Medium => 112.0,
            Self::Large => 152.0,
            Self::MediumFlexible => {
                if subtitle {
                    136.0
                } else {
                    112.0
                }
            }
            Self::LargeFlexible => {
                if subtitle {
                    152.0
                } else {
                    120.0
                }
            }
        }
    }
    pub(crate) fn title_metrics(self) -> (f64, f64, f64) {
        match self {
            Self::Small | Self::CenterAligned => (22.0, 28.0, 0.0),
            Self::Medium => (24.0, 32.0, 24.0),
            Self::Large | Self::MediumFlexible => {
                (28.0, 36.0, if self == Self::Large { 28.0 } else { 24.0 })
            }
            Self::LargeFlexible => (36.0, 44.0, 28.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MaterialAppBarScrollBehavior {
    #[default]
    Pinned,
    EnterAlways,
    ExitUntilCollapsed,
}
ferroui_base::ferro_markup_enum!(
    MaterialAppBarScrollBehavior { Pinned, EnterAlways, ExitUntilCollapsed },
    { namespace: "FerroUI.Themes.Material" }
);

/// A serializable scroll snapshot. Deltas follow finger movement: up is negative.
/// The offset is always between the negative collapse limit and zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialAppBarState {
    height_offset_limit: f64,
    height_offset: f64,
    content_offset: f64,
    content_at_start: bool,
}

impl Default for MaterialAppBarState {
    fn default() -> Self {
        Self::new(-64.0)
    }
}

impl MaterialAppBarState {
    pub fn new(height_offset_limit: f64) -> Self {
        let mut result = Self {
            height_offset_limit: 0.0,
            height_offset: 0.0,
            content_offset: 0.0,
            content_at_start: true,
        };
        result.set_height_offset_limit(height_offset_limit);
        result
    }
    pub fn height_offset_limit(self) -> f64 {
        self.height_offset_limit
    }
    pub fn height_offset(self) -> f64 {
        self.height_offset
    }
    pub fn content_offset(self) -> f64 {
        self.content_offset
    }
    pub fn content_at_start(self) -> bool {
        self.content_at_start
    }
    pub fn set_height_offset_limit(&mut self, value: f64) {
        if value.is_finite() {
            self.height_offset_limit = value.min(0.0);
        }
        self.set_height_offset(self.height_offset);
    }
    pub fn set_height_offset(&mut self, value: f64) {
        if value.is_finite() {
            self.height_offset = value.clamp(self.height_offset_limit, 0.0);
        }
    }
    pub fn set_content_offset(&mut self, value: f64) {
        if value.is_finite() {
            self.content_offset = value;
        }
    }
    pub fn set_content_at_start(&mut self, value: bool) {
        self.content_at_start = value;
    }
    pub fn collapsed_fraction(self) -> f64 {
        if self.height_offset_limit == 0.0 {
            0.0
        } else {
            self.height_offset / self.height_offset_limit
        }
    }
    pub fn overlapped_fraction(self) -> f64 {
        if !self.content_at_start && self.content_offset == 0.0 {
            1.0
        } else if self.height_offset_limit == 0.0 {
            0.0
        } else {
            1.0 - (self.height_offset_limit + self.content_offset.abs())
                .clamp(self.height_offset_limit, 0.0)
                / self.height_offset_limit
        }
    }
    /// The source pre-scroll contract consumes the Y gesture when the bar changes height.
    pub fn pre_scroll(&mut self, behavior: MaterialAppBarScrollBehavior, available_y: f64) -> f64 {
        if !available_y.is_finite()
            || behavior == MaterialAppBarScrollBehavior::Pinned
            || (behavior == MaterialAppBarScrollBehavior::ExitUntilCollapsed && available_y > 0.0)
        {
            return 0.0;
        }
        let old = self.height_offset;
        self.set_height_offset(old + available_y);
        if old != self.height_offset {
            available_y
        } else {
            0.0
        }
    }
    /// Called after the child. Only exit-until-collapsed responds to leftover downward scroll.
    pub fn post_scroll(
        &mut self,
        behavior: MaterialAppBarScrollBehavior,
        consumed_y: f64,
        available_y: f64,
    ) -> f64 {
        if !consumed_y.is_finite() || !available_y.is_finite() {
            return 0.0;
        }
        self.set_content_offset(self.content_offset + consumed_y);
        if behavior != MaterialAppBarScrollBehavior::ExitUntilCollapsed {
            return 0.0;
        }
        let old = self.height_offset;
        if available_y < 0.0 || consumed_y < 0.0 {
            self.set_height_offset(old + consumed_y);
        } else if available_y > 0.0 {
            self.set_height_offset(old + available_y);
        }
        self.height_offset - old
    }
    pub fn reset_content_after_fling(&mut self, available_velocity_y: f64) {
        if available_velocity_y > 0.0 {
            self.content_offset = 0.0;
        }
    }
    /// Native rounding tolerance and nearest-anchor threshold, used after decay finishes.
    pub fn snap_target(self) -> Option<f64> {
        let fraction = self.collapsed_fraction();
        if fraction < 0.01 || fraction == 1.0 {
            None
        } else {
            Some(if fraction < 0.5 {
                0.0
            } else {
                self.height_offset_limit
            })
        }
    }
}

/// Native Android spline decay, including its 100-sample interpolation and millisecond clock.
#[derive(Clone, Copy, Debug)]
pub struct MaterialAppBarFling {
    distance: f64,
    duration_ms: u64,
}
impl MaterialAppBarFling {
    pub fn new(velocity: f64) -> Self {
        if !velocity.is_finite() || velocity.abs() <= 1.0 {
            return Self {
                distance: 0.0,
                duration_ms: 0,
            };
        }
        let physical = 9.80665 * 39.37 * 160.0 * 0.84;
        let rate = 0.78_f64.ln() / 0.9_f64.ln();
        let deceleration = (0.35 * velocity.abs() / (0.015 * physical)).ln();
        Self {
            distance: (0.015 * physical * (rate / (rate - 1.0) * deceleration).exp())
                * velocity.signum(),
            duration_ms: (1000.0 * (deceleration / (rate - 1.0)).exp()) as u64,
        }
    }
    pub fn duration_seconds(self) -> f64 {
        self.duration_ms as f64 / 1000.0
    }
    pub fn target_distance(self) -> f64 {
        self.distance
    }
    pub fn sample(self, seconds: f64) -> (f64, f64) {
        if self.duration_ms == 0 {
            return (self.distance, 0.0);
        }
        let progress =
            ((seconds.max(0.0) * 1000.0).floor() / self.duration_ms as f64).clamp(0.0, 1.0);
        let index = (100.0 * progress) as usize;
        if index >= 100 {
            return (self.distance, 0.0);
        }
        let table = fling_positions();
        let velocity = (table[index + 1] - table[index]) * 100.0;
        let distance = table[index] + (progress - index as f64 / 100.0) * velocity;
        (
            self.distance * distance,
            self.distance * velocity / self.duration_seconds(),
        )
    }
}

fn fling_positions() -> &'static [f64; 101] {
    static POSITIONS: OnceLock<[f64; 101]> = OnceLock::new();
    POSITIONS.get_or_init(|| {
        let mut table = [0.0; 101];
        let mut minimum = 0.0;
        for (i, value) in table.iter_mut().enumerate().take(100) {
            let alpha = i as f64 / 100.0;
            let mut maximum = 1.0;
            loop {
                let x = (minimum + maximum) / 2.0;
                let coefficient = 3.0 * x * (1.0 - x);
                let time = coefficient * ((1.0 - x) * 0.175 + x * 0.35) + x * x * x;
                if (time - alpha).abs() < 0.00001 {
                    *value = coefficient * ((1.0 - x) * 0.5 + x) + x * x * x;
                    break;
                }
                if time > alpha {
                    maximum = x;
                } else {
                    minimum = x;
                }
            }
        }
        table[100] = 1.0;
        table
    })
}

pub(crate) fn cubic_bezier(fraction: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction == 0.0 || fraction == 1.0 {
        return fraction;
    }
    let mut lo = 0.0_f64;
    let mut hi = 1.0_f64;
    for _ in 0..40 {
        let t = (lo + hi) / 2.0;
        let x = 3.0 * (1.0 - t).powi(2) * t * x1 + 3.0 * (1.0 - t) * t * t * x2 + t * t * t;
        if x < fraction {
            lo = t;
        } else {
            hi = t;
        }
    }
    let t = (lo + hi) / 2.0;
    3.0 * (1.0 - t).powi(2) * t * y1 + 3.0 * (1.0 - t) * t * t * y2 + t * t * t
}

/// Compose Color.lerp uses Oklab; direct sRGB channel or opacity mixing is visibly different.
pub(crate) fn interpolate_container(from: Color, to: Color, fraction: f64) -> Color {
    let amount = fraction.clamp(0.0, 1.0);
    if amount == 0.0 {
        return from;
    }
    if amount == 1.0 {
        return to;
    }
    let convert = |color: Color| {
        let linear = |channel: u8| {
            let c = channel as f64 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        let (r, g, b) = (linear(color.r), linear(color.g), linear(color.b));
        let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
        let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
        let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
        [
            0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
            1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
            0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
        ]
    };
    let a = convert(from);
    let b = convert(to);
    let c = [
        a[0] + (b[0] - a[0]) * amount,
        a[1] + (b[1] - a[1]) * amount,
        a[2] + (b[2] - a[2]) * amount,
    ];
    let l = (c[0] + 0.3963377774 * c[1] + 0.2158037573 * c[2]).powi(3);
    let m = (c[0] - 0.1055613458 * c[1] - 0.0638541728 * c[2]).powi(3);
    let s = (c[0] - 0.0894841775 * c[1] - 1.2914855480 * c[2]).powi(3);
    let encode = |x: f64| {
        let x = if x <= 0.0031308 {
            12.92 * x
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        };
        (x.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    Color::from_argb(
        (from.a as f64 + (to.a as f64 - from.a as f64) * amount).round() as u8,
        encode(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s),
        encode(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s),
        encode(-0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s),
    )
}
