// Copyright 2023 The Android Open Source Project (reference behavior).
// Copyright 2026 FerroUI contributors (Rust adaptation).
// SPDX-License-Identifier: Apache-2.0
//! Geometry and input rules translated from AndroidX Material3 TimePicker.kt at
//! e4d933c98df7baa7ea2b83eea6e086915961e0ff (1.5.0-beta02).

use ferroui_base::layout::Orientation;
use ferroui_base::{Point, Rect, Size};
use std::f64::consts::{FRAC_PI_2, PI, TAU};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MaterialTimeSelection {
    #[default]
    Hour,
    Minute,
}
ferroui_base::ferro_markup_enum!(MaterialTimeSelection { Hour, Minute }, { namespace: "FerroUI.Themes.Material" });

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MaterialTimeDisplayMode {
    #[default]
    Dial,
    Input,
}
ferroui_base::ferro_markup_enum!(MaterialTimeDisplayMode { Dial, Input }, { namespace: "FerroUI.Themes.Material" });

pub(crate) fn display_hour(hour: i32, is_24_hour: bool) -> i32 {
    if is_24_hour {
        hour
    } else if hour % 12 == 0 {
        12
    } else {
        hour % 12
    }
}

pub(crate) fn angle_for(hour: i32, minute: i32, selection: MaterialTimeSelection) -> f64 {
    TAU * match selection {
        MaterialTimeSelection::Hour => (hour % 12) as f64 / 12.0,
        MaterialTimeSelection::Minute => minute as f64 / 60.0,
    } - FRAC_PI_2
}

/// Native atan convention, including the exact antipodal choice of the negative half turn.
pub(crate) fn shortest_target(current: f64, target: f64) -> f64 {
    let difference = (current - target).rem_euclid(TAU);
    current
        - if difference > PI {
            difference - TAU
        } else {
            difference
        }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DialGeometry {
    pub diameter: f64,
    pub center: Point,
    pub scale: f64,
}
impl DialGeometry {
    pub fn new(size: Size) -> Self {
        let diameter = size.width.min(size.height).max(0.0);
        Self {
            diameter,
            center: Point::new(size.width / 2.0, size.height / 2.0),
            scale: diameter / 256.0,
        }
    }
    pub fn center_at(self, angle: f64, inner: bool) -> Point {
        let radius = if inner { 69.0 } else { 101.0 } * self.scale;
        Point::new(
            self.center.x + radius * angle.cos(),
            self.center.y + radius * angle.sin(),
        )
    }
    pub fn number_rect(self, index: usize) -> Rect {
        let p = self.center_at(index as f64 * TAU / 12.0 - FRAC_PI_2, index >= 12);
        Rect::new(p.x - 24.0, p.y - 24.0, 48.0, 48.0)
    }
    pub fn value_at(
        self,
        position: Point,
        selection: MaterialTimeSelection,
        is_24_hour: bool,
        is_pm: bool,
        tapping: bool,
    ) -> (i32, f64) {
        let dx = position.x - self.center.x;
        let dy = position.y - self.center.y;
        let angle = dy.atan2(dx);
        let fraction = (angle + FRAC_PI_2).rem_euclid(TAU) / TAU;
        match selection {
            MaterialTimeSelection::Hour => {
                let hour = (fraction * 12.0).round() as i32 % 12;
                let pm = if is_24_hour {
                    dx.hypot(dy) < 74.0 * self.scale
                } else {
                    is_pm
                };
                (hour + if pm { 12 } else { 0 }, angle)
            }
            MaterialTimeSelection::Minute => {
                let minute = if tapping {
                    ((fraction * 12.0).round() as i32 * 5) % 60
                } else {
                    (fraction * 60.0).round() as i32 % 60
                };
                (minute, angle)
            }
        }
    }
}

/// Empty input changes the selected unit to zero; invalid ranges keep the last valid time.
/// Syntax validation is separate so a temporarily invalid 99 remains editable and announced.
pub(crate) fn parse_input(
    text: &str,
    selection: MaterialTimeSelection,
    is_24_hour: bool,
    is_pm: bool,
) -> Result<Option<i32>, ()> {
    if text.len() > 2 || !text.bytes().all(|c| c.is_ascii_digit()) {
        return Err(());
    }
    let value = if text.is_empty() {
        0
    } else {
        text.parse::<i32>().map_err(|_| ())?
    };
    Ok(match selection {
        MaterialTimeSelection::Minute => (value <= 59).then_some(value),
        MaterialTimeSelection::Hour if is_24_hour => (value <= 23).then_some(value),
        MaterialTimeSelection::Hour => {
            (value <= 12).then_some(value % 12 + if is_pm { 12 } else { 0 })
        }
    })
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TimePickerMetrics {
    pub field_width: f64,
    pub field_height: f64,
    pub separator: f64,
    pub period_width: f64,
    pub period_gap: f64,
    pub period_inner_gap: f64,
    pub dial_size: f64,
    pub dial_gap: f64,
    pub bottom: f64,
    pub support_height: f64,
    pub horizontal: bool,
    pub input: bool,
}
impl TimePickerMetrics {
    pub fn new(
        vibrant: bool,
        is_24_hour: bool,
        mode: MaterialTimeDisplayMode,
        orientation: Orientation,
        available_height: f64,
    ) -> Self {
        let input = mode == MaterialTimeDisplayMode::Input;
        let horizontal = !input && orientation == Orientation::Horizontal;
        Self {
            // ClockDisplayNumbers currently uses the classic 96dp width in both formats;
            // the generated 114dp vertical-24h token is not used by the pinned component.
            field_width: if vibrant {
                if is_24_hour && !input && !horizontal {
                    132.0
                } else {
                    100.0
                }
            } else {
                96.0
            },
            field_height: if vibrant {
                120.0
            } else if input {
                72.0
            } else {
                80.0
            },
            separator: if vibrant { 16.0 } else { 24.0 },
            period_width: if vibrant { 56.0 } else { 52.0 },
            period_gap: if horizontal {
                16.0
            } else if vibrant {
                if input { 8.0 } else { 16.0 }
            } else {
                4.0
            },
            period_inner_gap: if vibrant {
                if horizontal { 16.0 } else { 8.0 }
            } else {
                4.0
            },
            dial_size: if !horizontal || vibrant || available_height >= 384.0 {
                256.0
            } else if available_height >= 330.0 {
                238.0
            } else {
                200.0
            },
            dial_gap: if vibrant {
                if horizontal { 52.0 } else { 12.0 }
            } else {
                36.0
            },
            bottom: if !vibrant && !horizontal && !input {
                24.0
            } else {
                0.0
            },
            support_height: if input && !vibrant { 39.0 } else { 0.0 },
            horizontal,
            input,
        }
    }
    pub fn fields_width(self) -> f64 {
        2.0 * self.field_width + self.separator
    }
    pub fn display_size(self, is_24_hour: bool, vibrant: bool) -> Size {
        let mut size = Size::new(self.fields_width(), self.field_height + self.support_height);
        if !is_24_hour {
            if self.horizontal {
                size.height += self.period_gap + if vibrant { 40.0 } else { 38.0 };
            } else {
                size.width += self.period_gap + self.period_width;
            }
        }
        size
    }
    pub fn desired_size(self, is_24_hour: bool, vibrant: bool) -> Size {
        let display = self.display_size(is_24_hour, vibrant);
        if self.input {
            display
        } else if self.horizontal {
            Size::new(
                display.width + self.dial_gap + self.dial_size,
                display.height.max(self.dial_size),
            )
        } else {
            Size::new(
                display.width.max(self.dial_size),
                display.height + self.dial_gap + self.dial_size + self.bottom,
            )
        }
    }
}
