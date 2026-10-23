use crate::measure::Bounds;
use crate::motion::Interpolate;
use ferroui_base::media::Color;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorStop {
    pub offset: f64,
    pub color: Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeatError {
    TooFewColors,
    LengthMismatch,
    InvalidStops,
    InvalidWeight,
}
impl fmt::Display for HeatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TooFewColors => "At least two colors are required in a heat map.",
            Self::LengthMismatch => "Color stops and heat map must have the same length.",
            Self::InvalidStops => {
                "Color stops must be finite, ordered, distinct offsets in [0, 1]."
            }
            Self::InvalidWeight => "Heat weights and bounds must be finite.",
        })
    }
}
impl std::error::Error for HeatError {}

pub struct HeatFunctions;
impl HeatFunctions {
    pub fn build_color_stops(
        heat_map: &[Color],
        color_stops: Option<&[f64]>,
    ) -> Result<Vec<ColorStop>, HeatError> {
        if heat_map.len() < 2 {
            return Err(HeatError::TooFewColors);
        }
        if let Some(stops) = color_stops {
            if stops.len() != heat_map.len() {
                return Err(HeatError::LengthMismatch);
            }
            if stops
                .iter()
                .any(|x| !x.is_finite() || !(0.0..=1.0).contains(x))
                || stops.windows(2).any(|w| w[0] >= w[1])
            {
                return Err(HeatError::InvalidStops);
            }
        }
        Ok(heat_map
            .iter()
            .copied()
            .enumerate()
            .map(|(i, color)| ColorStop {
                color,
                offset: color_stops
                    .map_or(i as f64 / (heat_map.len() - 1) as f64, |stops| stops[i]),
            })
            .collect())
    }

    pub fn interpolate_color(
        weight: f64,
        bounds: &Bounds,
        heat_map: &[Color],
        stops: &[ColorStop],
    ) -> Result<Color, HeatError> {
        if stops.len() < 2 || heat_map.len() < 2 {
            return Err(HeatError::TooFewColors);
        }
        if stops.len() != heat_map.len() {
            return Err(HeatError::LengthMismatch);
        }
        if !weight.is_finite() || !bounds.min.is_finite() || !bounds.max.is_finite() {
            return Err(HeatError::InvalidWeight);
        }
        if stops
            .iter()
            .any(|s| !s.offset.is_finite() || !(0.0..=1.0).contains(&s.offset))
            || stops.windows(2).any(|w| w[0].offset >= w[1].offset)
        {
            return Err(HeatError::InvalidStops);
        }
        let range = bounds.max - bounds.min;
        let range = if range == 0.0 {
            f64::from_bits(1)
        } else {
            range
        };
        let progress = ((weight - bounds.min) / range).clamp(0.0, 1.0);
        Ok(sample_stops(progress, stops))
    }
}

fn sample_stops(progress: f64, stops: &[ColorStop]) -> Color {
    // The checked constructors guarantee nonempty, strictly ordered stops.
    if progress <= stops[0].offset {
        return stops[0].color;
    }
    for pair in stops.windows(2) {
        if pair[1].offset < progress {
            continue;
        }
        let p = (progress - pair[0].offset) / (pair[1].offset - pair[0].offset);
        return Color::interpolate(&pair[0].color, &pair[1].color, p);
    }
    stops[stops.len() - 1].color
}

/// A validated, reusable palette; construction checks stops once, sampling
/// allocates nothing and does not repeat their validation on every heat cell.
#[derive(Clone, Debug, PartialEq)]
pub struct HeatPalette {
    stops: Vec<ColorStop>,
}
impl HeatPalette {
    pub fn new(colors: &[Color], stops: Option<&[f64]>) -> Result<Self, HeatError> {
        Ok(Self {
            stops: HeatFunctions::build_color_stops(colors, stops)?,
        })
    }
    pub fn stops(&self) -> &[ColorStop] {
        &self.stops
    }
    pub fn sample(&self, progress: f64) -> Color {
        sample_stops(
            if progress.is_nan() {
                0.0
            } else {
                progress.clamp(0.0, 1.0)
            },
            &self.stops,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const RED: Color = Color::from_rgb(255, 0, 0);
    const GREEN: Color = Color::from_rgb(0, 255, 0);
    const BLUE: Color = Color::from_rgb(0, 0, 255);
    #[test]
    fn uniform_and_explicit_stops_match_source() {
        let stops = HeatFunctions::build_color_stops(&[RED, BLUE], None).unwrap();
        assert_eq!(stops.len(), 2);
        assert_eq!((stops[0].offset, stops[1].offset), (0.0, 1.0));
        let stops = HeatFunctions::build_color_stops(&[RED, GREEN, BLUE], None).unwrap();
        assert_eq!(
            stops.iter().map(|s| s.offset).collect::<Vec<_>>(),
            vec![0.0, 0.5, 1.0]
        );
        let stops = HeatFunctions::build_color_stops(&[RED, BLUE], Some(&[0.2, 0.8])).unwrap();
        assert_eq!((stops[0].offset, stops[1].offset), (0.2, 0.8));
    }
    #[test]
    fn invalid_palette_is_rejected() {
        assert_eq!(
            HeatFunctions::build_color_stops(&[RED], None),
            Err(HeatError::TooFewColors)
        );
        assert_eq!(
            HeatFunctions::build_color_stops(&[RED, BLUE], Some(&[0.0, 0.5, 1.0])),
            Err(HeatError::LengthMismatch)
        );
        assert!(HeatPalette::new(&[RED, BLUE], Some(&[0.5, 0.5])).is_err());
        assert!(HeatPalette::new(&[RED, BLUE], Some(&[1.0, 0.0])).is_err());
    }
    #[test]
    fn interpolation_clamps_and_truncates_channels() {
        let colors = [RED, BLUE];
        let stops = HeatFunctions::build_color_stops(&colors, None).unwrap();
        let bounds = Bounds::new(0.0, 100.0);
        for (weight, expected) in [
            (-50.0, RED),
            (0.0, RED),
            (50.0, Color::from_rgb(127, 0, 127)),
            (100.0, BLUE),
            (200.0, BLUE),
        ] {
            assert_eq!(
                HeatFunctions::interpolate_color(weight, &bounds, &colors, &stops).unwrap(),
                expected
            );
        }
    }
    #[test]
    fn zero_range_is_opaque_and_custom_stop_edges_are_bounded() {
        let colors = [RED, BLUE];
        let stops = HeatFunctions::build_color_stops(&colors, None).unwrap();
        assert_eq!(
            HeatFunctions::interpolate_color(50.0, &Bounds::new(50.0, 50.0), &colors, &stops)
                .unwrap(),
            RED
        );
        let palette = HeatPalette::new(&colors, Some(&[0.2, 0.8])).unwrap();
        assert_eq!(palette.sample(0.0), RED);
        assert_eq!(palette.sample(1.0), BLUE);
    }
}
