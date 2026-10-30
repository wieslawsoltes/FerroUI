use super::{AxisLimit, AxisOrientation};
use ferroui_base::Size;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AxisTick {
    pub value: f64,
    pub magnitude: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TickOptions {
    pub orientation: AxisOrientation,
    pub label_size: Size,
    pub labels_rotation: f64,
    pub labels_density: f64,
    pub unit_width: f64,
    pub min_step: f64,
    pub min_separators: u32,
    pub force_step_to_min: bool,
}

impl Default for TickOptions {
    fn default() -> Self {
        Self {
            orientation: AxisOrientation::X,
            label_size: Size::new(50.0, 20.0),
            labels_rotation: 0.0,
            labels_density: 1.0,
            unit_width: 1.0,
            min_step: 0.0,
            min_separators: 0,
            force_step_to_min: false,
        }
    }
}

impl AxisTick {
    /// Computes the source nice-number ladder (1, 2, 5, 10), including the
    /// density, rotated-label and minimum-separator adjustments.
    pub fn cartesian(control_size: Size, mut min: f64, mut max: f64, options: TickOptions) -> Self {
        AxisLimit::validate_limits(&mut min, &mut max, options.min_step);
        let mut w = options.label_size.width;
        let mut h = options.label_size.height;
        let rotation = (options.labels_rotation % 90.0).abs();
        if (20.0..=70.0).contains(&rotation) {
            let diagonal = 0.35 * w.hypot(h);
            w = diagonal;
            h = diagonal;
        } else {
            let density = (1.0 + options.labels_density) / 2.0;
            match options.orientation {
                AxisOrientation::X => w *= density,
                AxisOrientation::Y => h *= density,
                _ => {}
            }
        }
        w = w.max(10.0);
        h = h.max(10.0);
        let unit = if options.unit_width.is_finite() && options.unit_width > 0.0 {
            options.unit_width
        } else {
            1.0
        };
        max /= unit;
        min /= unit;
        let mut range = max - min;
        if range == 0.0 {
            range = 0.15 * max / unit;
        }
        let separations = if options.orientation == AxisOrientation::Y {
            (control_size.height / h).round_ties_even()
        } else {
            (control_size.width / w).round_ties_even()
        };
        let mut tick = Self::nice(
            range,
            separations,
            options.min_separators,
            options.force_step_to_min,
        );
        tick.value *= unit;
        tick.magnitude *= unit;
        tick
    }

    pub fn polar(
        control_size: Size,
        min: f64,
        max: f64,
        inner_radius: f64,
        total_angle: f64,
        is_angle: bool,
        min_separators: u32,
        force_step_to_min: bool,
    ) -> Self {
        let min_d = control_size.width.min(control_size.height);
        let separations = if is_angle {
            (min_d * total_angle / 360.0 / 30.0).round_ties_even()
        } else {
            ((min_d - inner_radius) / 90.0).round_ties_even()
        };
        Self::nice(max - min, separations, min_separators, force_step_to_min)
    }

    pub fn nice(
        range: f64,
        separations: f64,
        min_separators: u32,
        force_step_to_min: bool,
    ) -> Self {
        // Collapsed/zero-sized views have no useful inferred step. A finite
        // default avoids propagating NaN into tick-generation loops.
        if !range.is_finite() || range <= 0.0 || !separations.is_finite() || separations <= 0.0 {
            return Self {
                value: 1.0,
                magnitude: 1.0,
            };
        }
        let minimum = range / separations;
        let mut magnitude = 10.0_f64.powf(minimum.log10().floor());
        if !magnitude.is_finite() || magnitude <= 0.0 {
            return Self {
                value: minimum.max(f64::MIN_POSITIVE),
                magnitude: minimum.max(f64::MIN_POSITIVE),
            };
        }
        let residual = minimum / magnitude;
        let mut mul = if residual > 5.0 {
            10.0
        } else if residual > 2.0 {
            5.0
        } else if residual > 1.0 {
            2.0
        } else {
            1.0
        };
        if !force_step_to_min && min_separators > 0 {
            for _ in 0..32 {
                if range / (mul * magnitude) >= f64::from(min_separators) {
                    break;
                }
                if mul > 7.5 {
                    mul = 5.0;
                } else if mul > 3.5 {
                    mul = 2.0;
                } else if mul > 1.5 {
                    mul = 1.0;
                } else {
                    mul = 5.0;
                    magnitude /= 10.0;
                }
                if magnitude <= 0.0 {
                    break;
                }
            }
        }
        Self {
            value: mul * magnitude,
            magnitude,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nice_ladder_and_minimum_separator_count() {
        for (range, expected) in [(1.0, 0.1), (15.0, 2.0), (21.0, 5.0), (51.0, 10.0)] {
            assert!((AxisTick::nice(range, 10.0, 0, false).value - expected).abs() < 1e-12);
        }
        assert_eq!(AxisTick::nice(100.0, 2.0, 10, false).value, 10.0);
        assert_eq!(AxisTick::nice(100.0, 2.0, 10, true).value, 50.0);
    }
    #[test]
    fn collapsed_chart_tick_is_finite() {
        let tick = AxisTick::cartesian(Size::new(0.0, 0.0), 0.0, 0.0, TickOptions::default());
        assert!(tick.value.is_finite() && tick.value > 0.0);
    }
    #[test]
    fn category_unit_width_scales_ticks() {
        let tick = AxisTick::cartesian(
            Size::new(500.0, 400.0),
            0.0,
            1000.0,
            TickOptions {
                unit_width: 10.0,
                ..TickOptions::default()
            },
        );
        assert_eq!(tick.value, 100.0);
    }
}
