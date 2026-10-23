//! Chart axis snapshots and stable value/pixel conversion.

use crate::scene::ChartError;
use ferroui_base::media::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AxisPosition {
    #[default]
    Start,
    End,
}
ferroui_base::ferro_markup_enum!(AxisPosition { Start, End });

#[derive(Clone, Debug, PartialEq)]
pub struct Axis {
    pub name: String,
    pub labels: Vec<String>,
    pub min_limit: Option<f64>,
    pub max_limit: Option<f64>,
    pub min_step: f64,
    pub force_step: bool,
    pub unit_width: f64,
    pub inverted: bool,
    pub visible: bool,
    pub show_separator_lines: bool,
    pub position: AxisPosition,
    pub text_size: f64,
    pub labels_rotation: f64,
    pub labels_color: Color,
    pub separators_color: Color,
    pub logarithmic_base: Option<f64>,
    pub padding: f64,
}
impl Default for Axis {
    fn default() -> Self {
        Self {
            name: String::new(),
            labels: Vec::new(),
            min_limit: None,
            max_limit: None,
            min_step: 0.0,
            force_step: false,
            unit_width: 1.0,
            inverted: false,
            visible: true,
            show_separator_lines: true,
            position: AxisPosition::Start,
            text_size: 12.0,
            labels_rotation: 0.0,
            labels_color: Color::from_rgb(75, 85, 99),
            separators_color: Color::from_argb(50, 107, 114, 128),
            logarithmic_base: None,
            padding: 0.05,
        }
    }
}
impl Axis {
    pub fn label(&self, value: f64) -> String {
        if !self.labels.is_empty() {
            let index = value.round();
            if (index - value).abs() < 1e-7 && index >= 0.0 {
                return self.labels.get(index as usize).cloned().unwrap_or_default();
            }
        }
        format_number(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisScale {
    pub min: f64,
    pub max: f64,
    pub pixel_start: f64,
    pub pixel_end: f64,
    pub logarithmic_base: Option<f64>,
}
impl AxisScale {
    pub fn new(
        axis: &Axis,
        mut min: f64,
        mut max: f64,
        pixel_start: f64,
        pixel_end: f64,
    ) -> Result<Self, ChartError> {
        if axis.min_limit.is_some_and(|v| !v.is_finite())
            || axis.max_limit.is_some_and(|v| !v.is_finite())
        {
            return Err(ChartError::InvalidAxis("axis limits must be finite".into()));
        }
        if let Some(base) = axis.logarithmic_base {
            if !base.is_finite() || base <= 1.0 {
                return Err(ChartError::InvalidAxis(
                    "logarithmic base must be finite and greater than one".into(),
                ));
            }
            if !min.is_finite() || min <= 0.0 {
                min = 1.0;
            }
            if !max.is_finite() || max <= min {
                max = min * base;
            }
        } else {
            if !min.is_finite() || !max.is_finite() {
                min = 0.0;
                max = 10.0;
            }
            if min == max {
                let delta = if min == 0.0 { 1.0 } else { min.abs() * 0.1 };
                min -= delta;
                max += delta;
            }
            if axis.min_limit.is_none() && axis.max_limit.is_none() {
                let span = (max - min) * axis.padding.max(0.0);
                if span.is_finite() {
                    min -= span;
                    max += span;
                }
            }
        }
        min = axis.min_limit.unwrap_or(min);
        max = axis.max_limit.unwrap_or(max);
        if !min.is_finite()
            || !max.is_finite()
            || min >= max
            || axis.logarithmic_base.is_some() && min <= 0.0
        {
            return Err(ChartError::InvalidAxis(
                "axis minimum must be below maximum (and positive for logarithmic axes)".into(),
            ));
        }
        let (pixel_start, pixel_end) = if axis.inverted {
            (pixel_end, pixel_start)
        } else {
            (pixel_start, pixel_end)
        };
        Ok(Self {
            min,
            max,
            pixel_start,
            pixel_end,
            logarithmic_base: axis.logarithmic_base,
        })
    }
    pub fn to_pixels(self, value: f64) -> f64 {
        let (min, max, value) = if let Some(base) = self.logarithmic_base {
            (self.min.log(base), self.max.log(base), value.log(base))
        } else {
            (self.min, self.max, value)
        };
        // Scale first to keep ranges such as [-1e308,1e308] representable.
        let magnitude = min.abs().max(max.abs()).max(value.abs()).max(1.0);
        let fraction = (value / magnitude - min / magnitude) / (max / magnitude - min / magnitude);
        self.pixel_start * (1.0 - fraction) + self.pixel_end * fraction
    }
    pub fn to_chart_values(self, pixel: f64) -> f64 {
        let fraction = if self.pixel_start == self.pixel_end {
            0.0
        } else {
            (pixel - self.pixel_start) / (self.pixel_end - self.pixel_start)
        };
        if let Some(base) = self.logarithmic_base {
            base.powf(self.min.log(base) * (1.0 - fraction) + self.max.log(base) * fraction)
        } else {
            self.min * (1.0 - fraction) + self.max * fraction
        }
    }
    pub fn ticks(self, axis: &Axis, pixel_length: f64) -> Vec<f64> {
        if let Some(base) = self.logarithmic_base {
            let first = self.min.log(base).ceil() as i32;
            let last = self.max.log(base).floor() as i32;
            return (first..=last)
                .take(1024)
                .map(|i| base.powi(i))
                .filter(|v| v.is_finite())
                .collect();
        }
        let count = (pixel_length / 80.0).clamp(2.0, 20.0);
        let raw = self.max / count - self.min / count;
        let magnitude = 10.0_f64.powf(raw.log10().floor());
        let residual = raw / magnitude;
        let step = if residual > 5.0 {
            10.0
        } else if residual > 2.0 {
            5.0
        } else if residual > 1.0 {
            2.0
        } else {
            1.0
        } * magnitude;
        let step = if axis.force_step && axis.min_step > 0.0 {
            axis.min_step
        } else {
            step.max(axis.min_step)
                .max(if axis.labels.is_empty() { 0.0 } else { 1.0 })
        };
        if !step.is_finite() || step <= 0.0 {
            return vec![self.min, self.max];
        }
        let start = (self.min / step).ceil() * step;
        (0..1024)
            .map(|i| start + i as f64 * step)
            .take_while(|v| v.is_finite() && *v <= self.max + step * 1e-9)
            .collect()
    }
}

pub fn format_number(value: f64) -> String {
    if value == 0.0 {
        return "0".into();
    }
    if value.abs() >= 1e7 || value.abs() < 1e-4 {
        return format!("{value:.3e}");
    }
    let mut s = format!("{value:.4}");
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s
}
