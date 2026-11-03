//! The heat colour ramp of the pinned `HeatFunctions`: colour stops for a heat map and the colour
//! of a weight within its bounds.

use crate::measure::Bounds;
use crate::scene::ChartError;
use ferroui_base::media::Color;

/// The heat functions of the source (`HeatFunctions`).
pub struct HeatFunctions;

impl HeatFunctions {
    /// The stops of a heat map (`BuildColorStops`): the given offsets, or offsets spread evenly
    /// from 0 to 1. A heat map of fewer than two colours, or offsets of another count, is an error
    /// where the source throws.
    pub fn build_color_stops(
        heat_map: &[Color],
        color_stops: Option<&[f64]>,
    ) -> Result<Vec<(f64, Color)>, ChartError> {
        if heat_map.len() < 2 {
            return Err(ChartError::InvalidData(
                "at least 2 colors are required in a heat map".into(),
            ));
        }
        if let Some(stops) = color_stops {
            if stops.len() != heat_map.len() {
                return Err(ChartError::InvalidData(
                    "ColorStops and HeatMap must have the same length".into(),
                ));
            }
            return Ok(stops.iter().copied().zip(heat_map.iter().copied()).collect());
        }
        // The source accumulates the step, as here.
        let step = 1.0 / (heat_map.len() - 1) as f64;
        let mut offset = 0.0;
        Ok(heat_map
            .iter()
            .map(|&color| {
                let stop = (offset, color);
                offset += step;
                stop
            })
            .collect())
    }

    /// The colour of a weight (`InterpolateColor`): its position between the bounds, clamped to
    /// [0, 1], interpolated between the stops around it, each channel truncated as the source's
    /// byte conversion; past the last stop, the last colour of the heat map. A zero range puts
    /// every weight at the start.
    ///
    /// Where the source extrapolates (a position before the first stop, two stops at the same
    /// offset, a weight that is not a number) the colour is the nearest stop's.
    pub fn interpolate_color(
        weight: f64,
        weight_bounds: &Bounds,
        heat_map: &[Color],
        heat_stops: &[(f64, Color)],
    ) -> Color {
        let Some(&(first_offset, first_color)) = heat_stops.first() else {
            return heat_map.last().copied().unwrap_or(Color::from_argb(0, 0, 0, 0));
        };
        let mut range = weight_bounds.max - weight_bounds.min;
        if range == 0.0 {
            range = f64::from_bits(1);
        }
        let mut p = (weight - weight_bounds.min) / range;
        if p.is_nan() || p < 0.0 {
            p = 0.0;
        }
        if p > 1.0 {
            p = 1.0;
        }
        if p <= first_offset {
            return first_color;
        }
        let mut previous = heat_stops[0];
        for &next in &heat_stops[1..] {
            if next.0 < p {
                previous = next;
                continue;
            }
            let span = next.0 - previous.0;
            if !(span > 0.0) {
                return next.1;
            }
            let px = ((p - previous.0) / span).clamp(0.0, 1.0);
            let channel = |a: u8, b: u8| (a as f64 + px * (b as f64 - a as f64)) as u8;
            let (a, b) = (previous.1, next.1);
            return Color::from_argb(
                channel(a.a, b.a),
                channel(a.r, b.r),
                channel(a.g, b.g),
                channel(a.b, b.b),
            );
        }
        heat_map
            .last()
            .copied()
            .unwrap_or(heat_stops[heat_stops.len() - 1].1)
    }
}

#[cfg(test)]
mod tests {
    //! Ports of `CoreTests/CoreObjectsTests/HeatFunctionsTesting.cs` at the pinned source.
    use super::*;

    const RED: Color = Color::from_rgb(255, 0, 0);
    const GREEN: Color = Color::from_rgb(0, 255, 0);
    const BLUE: Color = Color::from_rgb(0, 0, 255);

    fn zero_to_hundred() -> Bounds {
        let mut bounds = Bounds::default();
        bounds.append_value(0.0);
        bounds.append_value(100.0);
        bounds
    }

    fn red_to_blue(weight: f64, bounds: &Bounds) -> Color {
        let heat_map = [RED, BLUE];
        let stops = HeatFunctions::build_color_stops(&heat_map, None).unwrap();
        HeatFunctions::interpolate_color(weight, bounds, &heat_map, &stops)
    }

    #[test]
    fn build_color_stops_with_null_stops_generates_uniform_distribution() {
        let stops = HeatFunctions::build_color_stops(&[RED, BLUE], None).unwrap();
        assert_eq!(stops.len(), 2);
        assert!(stops[0].0.abs() < 0.001);
        assert!((stops[1].0 - 1.0).abs() < 0.001);
    }

    #[test]
    fn build_color_stops_with_three_colors_and_null_stops() {
        let stops = HeatFunctions::build_color_stops(&[RED, GREEN, BLUE], None).unwrap();
        assert_eq!(stops.len(), 3);
        assert!(stops[0].0.abs() < 0.001);
        assert!((stops[1].0 - 0.5).abs() < 0.001);
        assert!((stops[2].0 - 1.0).abs() < 0.001);
    }

    #[test]
    fn build_color_stops_with_explicit_stops() {
        let stops = HeatFunctions::build_color_stops(&[RED, BLUE], Some(&[0.2, 0.8])).unwrap();
        assert_eq!(stops.len(), 2);
        assert!((stops[0].0 - 0.2).abs() < 0.001);
        assert!((stops[1].0 - 0.8).abs() < 0.001);
    }

    #[test]
    fn build_color_stops_throws_with_fewer_than_two_colors() {
        assert!(HeatFunctions::build_color_stops(&[RED], None).is_err());
    }

    #[test]
    fn build_color_stops_throws_when_stops_length_mismatch() {
        assert!(HeatFunctions::build_color_stops(&[RED, BLUE], Some(&[0.0, 0.5, 1.0])).is_err());
    }

    #[test]
    fn interpolate_color_at_start() {
        let result = red_to_blue(0.0, &zero_to_hundred());
        assert_eq!((result.r, result.g, result.b), (255, 0, 0));
    }

    #[test]
    fn interpolate_color_at_end() {
        let result = red_to_blue(100.0, &zero_to_hundred());
        assert_eq!((result.r, result.g, result.b), (0, 0, 255));
    }

    #[test]
    fn interpolate_color_at_midpoint() {
        let result = red_to_blue(50.0, &zero_to_hundred());
        // About 127 for red and blue: 127.5 truncated.
        assert!(result.r > 100 && result.r < 160);
        assert!(result.b > 100 && result.b < 160);
        assert_eq!((result.r, result.b), (127, 127));
    }

    #[test]
    fn interpolate_color_clamps_below() {
        assert_eq!(red_to_blue(-50.0, &zero_to_hundred()).r, 255);
    }

    #[test]
    fn interpolate_color_clamps_above() {
        assert_eq!(red_to_blue(200.0, &zero_to_hundred()).b, 255);
    }

    #[test]
    fn interpolate_color_handles_zero_range() {
        let mut bounds = Bounds::default();
        bounds.append_value(50.0);
        assert_eq!(red_to_blue(50.0, &bounds).a, 255);
    }

    /// Not in the source suite: explicit stops hold the first and last colours outside them.
    #[test]
    fn explicit_stops_hold_their_colours_outside_their_range() {
        let heat_map = [RED, BLUE];
        let stops = HeatFunctions::build_color_stops(&heat_map, Some(&[0.2, 0.8])).unwrap();
        let bounds = zero_to_hundred();
        let color = |weight| HeatFunctions::interpolate_color(weight, &bounds, &heat_map, &stops);
        assert_eq!(color(10.0), RED);
        assert_eq!(color(90.0), BLUE);
        assert_eq!(color(f64::NAN), RED);
        assert_eq!(color(50.0), Color::from_rgb(127, 0, 127));
    }
}
