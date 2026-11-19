//! The gradient heat legend of the pinned `SKHeatLegend` and the sources it reads
//! (`IHeatLegendSource`, `Chart.EnumerateHeatLegendSources`): a bar of the colours of the heat map
//! of the first heat series of a chart, from its lowest weight to its highest, with the two bounds
//! written over its ends. The legend is a list of scene primitives (a rectangle filled with a
//! linear gradient and two texts), drawn by the native renderer as the rest of the chart.

use crate::geo_map_chart_engine::HeatLandSeries;
use crate::label_lines::LabelLines;
use crate::labelers::{Labeler, Labelers};
use crate::measure::Bounds;
use crate::paint::BrushSpace;
use crate::scene::{Geometry, PaintStyle, Primitive, TextAnchor};
use crate::series::{Series, SeriesKind, DATA_LABELS_MAX_WIDTH};
use ferroui_base::media::immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush};
use ferroui_base::media::{Color, GradientSpreadMethod, IBrush};
use ferroui_base::{Point, Rect, RelativePoint, RelativeUnit, Size, Thickness};
use std::rc::Rc;

/// The padding of the two labels of the legend (`LabelGeometry.Padding` of `GetLayout`): 8 on the
/// left and right, 4 above and below.
pub const HEAT_LEGEND_LABELS_PADDING: (f64, f64) = (8.0, 4.0);

/// What a heat legend shows (`IHeatLegendSource`): the colours of a heat map from low to high and
/// the bounds of the weights its gradient maps over. The source's legend spreads the colours
/// evenly and does not read the colour stops.
#[derive(Clone, Debug, PartialEq)]
pub struct HeatLegendSource {
    pub heat_map: Vec<Color>,
    pub weight_bounds: Bounds,
}

impl HeatLegendSource {
    /// The source of a Cartesian chart (`Chart.EnumerateHeatLegendSources`): its first visible
    /// heat series with weights, with the bounds of [`Series::weight_bounds`].
    pub fn of_series(series: &[Series]) -> Option<Self> {
        series
            .iter()
            .filter(|s| s.visible && s.kind == SeriesKind::Heat)
            .find_map(|s| Self::new(&s.heat_map, s.weight_bounds()))
    }

    /// The source of a map (`GeoMapChart.EnumerateHeatLegendSources`): its first visible heat
    /// land series with weights, with the bounds of [`HeatLandSeries::weight_bounds`].
    pub fn of_heat_lands(series: &[HeatLandSeries]) -> Option<Self> {
        series
            .iter()
            .filter(|s| s.series.visible)
            .find_map(|s| Self::new(&s.series.heat_map, s.weight_bounds()))
    }

    fn new(heat_map: &[Color], weight_bounds: Bounds) -> Option<Self> {
        let bounded = weight_bounds.min.is_finite()
            && weight_bounds.max.is_finite()
            && weight_bounds.min <= weight_bounds.max;
        (!heat_map.is_empty() && bounded).then(|| Self {
            heat_map: heat_map.to_vec(),
            weight_bounds,
        })
    }
}

/// The gradient heat legend of the source (`SKHeatLegend`), with its defaults: a padding of 10
/// around the bar and around the legend (`BadgePadding`), a bar as thick as the larger label
/// (`BadgeWidth` of `None`), labels by `Labelers.SixRepresentativeDigits` (`Formatter`) in the 16
/// px of the theme's `LegendTextSize`. A legend at the left or right of a chart is vertical.
#[derive(Clone, Debug, PartialEq)]
pub struct HeatLegend {
    pub badge_padding: Thickness,
    pub badge_width: Option<f64>,
    pub formatter: Labeler,
    pub text_size: f64,
}

impl Default for HeatLegend {
    fn default() -> Self {
        Self {
            badge_padding: Thickness::uniform(10.0),
            badge_width: None,
            formatter: Labeler::new(Labelers::six_representative_digits),
            text_size: 16.0,
        }
    }
}

/// The parts of a laid out legend, relative to its top left corner.
struct Layout {
    size: Size,
    bar: Rect,
    labels: [(Rect, LabelLines, Color); 2],
}

impl HeatLegend {
    /// The size of the legend (`Measure`) beside a draw margin (the plot) of `plot` size. A
    /// vertical legend is as tall as the plot and a horizontal one as wide; without a source it
    /// is its padding alone, as the source's empty layout.
    pub fn measure(
        &self,
        source: Option<&HeatLegendSource>,
        vertical: bool,
        plot: Size,
        measure_text: impl FnMut(&str, f64) -> Size,
    ) -> Size {
        match source {
            Some(source) => self.layout(source, vertical, plot, measure_text).size,
            None => Size::new(self.horizontal_padding(), self.vertical_padding()),
        }
    }

    /// The primitives of the legend with its top left corner at `origin` (`GetLayout`): the
    /// gradient bar, then the label of the lower bound at its start (the top of a vertical bar)
    /// and that of the upper bound at its end, each in black or white, whichever contrasts with
    /// the colour under it. Nothing without a source.
    pub fn draw(
        &self,
        source: Option<&HeatLegendSource>,
        vertical: bool,
        origin: Point,
        plot: Size,
        measure_text: impl FnMut(&str, f64) -> Size,
    ) -> Vec<Primitive> {
        let Some(source) = source else {
            return Vec::new();
        };
        let layout = self.layout(source, vertical, plot, measure_text);
        let offset = |rect: Rect| {
            Rect::new(
                rect.x + origin.x,
                rect.y + origin.y,
                rect.width,
                rect.height,
            )
        };
        let mut primitives = vec![Primitive::new(
            Geometry::Rectangle {
                rect: offset(layout.bar),
                radius: 0.0,
            },
            PaintStyle {
                fill_brush: Some(gradient(&source.heat_map, vertical)),
                brush_space: BrushSpace::Geometry,
                ..PaintStyle::default()
            },
        )];
        // A label of several lines takes a text of the scene for each, from the left of its block.
        for (rect, lines, color) in layout.labels {
            let rect = offset(rect);
            for (i, text) in lines.lines.into_iter().enumerate() {
                if text.is_empty() {
                    continue;
                }
                primitives.push(Primitive::new(
                    Geometry::Text {
                        origin: Point::new(
                            rect.x + HEAT_LEGEND_LABELS_PADDING.0,
                            rect.y + HEAT_LEGEND_LABELS_PADDING.1 + i as f64 * lines.line_height,
                        ),
                        text,
                        size: self.text_size,
                        rotation: 0.0,
                        anchor: TextAnchor::Start,
                    },
                    PaintStyle::fill(color),
                ));
            }
        }
        primitives
    }

    fn horizontal_padding(&self) -> f64 {
        self.badge_padding.left + self.badge_padding.right
    }

    fn vertical_padding(&self) -> f64 {
        self.badge_padding.top + self.badge_padding.bottom
    }

    /// `GetLayout`: an absolute layout (as large as its largest child) of the bar and the two
    /// labels, in a stack layout padded by `BadgePadding`.
    fn layout(
        &self,
        source: &HeatLegendSource,
        vertical: bool,
        plot: Size,
        mut measure_text: impl FnMut(&str, f64) -> Size,
    ) -> Layout {
        // A label breaks into lines at `MaxTooltipsAndLegendsLabelsWidth` (170) less its padding,
        // as the `MaxWidth` of the source's `LabelGeometry`.
        let mut label = |value: f64, color: Color| {
            let text = self.formatter.label(value);
            let lines = LabelLines::wrap(
                &mut measure_text,
                &text,
                self.text_size,
                DATA_LABELS_MAX_WIDTH - HEAT_LEGEND_LABELS_PADDING.0 * 2.0,
            );
            let size = Size::new(
                lines.size.width + HEAT_LEGEND_LABELS_PADDING.0 * 2.0,
                lines.size.height + HEAT_LEGEND_LABELS_PADDING.1 * 2.0,
            );
            (size, lines, contrasting_color(color))
        };
        let first = source.heat_map[0];
        let last = source.heat_map[source.heat_map.len() - 1];
        let (min_size, min_text, min_color) = label(source.weight_bounds.min, first);
        let (max_size, max_text, max_color) = label(source.weight_bounds.max, last);
        let thickness = self.badge_width.unwrap_or(if vertical {
            min_size.width.max(max_size.width)
        } else {
            min_size.height.max(max_size.height)
        });
        let s = if vertical {
            self.vertical_padding()
        } else {
            self.horizontal_padding()
        };
        let (bar, min, max) = if vertical {
            let length = (plot.height - s).max(0.0);
            (
                Size::new(thickness, length),
                Point::new((min_size.width - thickness).abs() * 0.5, 0.0),
                Point::new(
                    (max_size.width - thickness).abs() * 0.5,
                    length - max_size.height,
                ),
            )
        } else {
            let length = (plot.width - s).max(0.0);
            (
                Size::new(length, thickness),
                Point::new(0.0, (min_size.height - thickness).abs() * 0.5),
                Point::new(
                    length - max_size.width,
                    (max_size.height - thickness).abs() * 0.5,
                ),
            )
        };
        let content = Size::new(
            bar.width.max(min_size.width).max(max_size.width),
            bar.height.max(min_size.height).max(max_size.height),
        );
        let padding = self.badge_padding;
        let at = |p: Point, size: Size| {
            Rect::new(
                p.x + padding.left,
                p.y + padding.top,
                size.width,
                size.height,
            )
        };
        Layout {
            size: Size::new(
                content.width + self.horizontal_padding(),
                content.height + self.vertical_padding(),
            ),
            bar: at(Point::new(0.0, 0.0), bar),
            labels: [
                (at(min, min_size), min_text, min_color),
                (at(max, max_size), max_text, max_color),
            ],
        }
    }
}

/// The linear gradient of the heat map along the bar (`LinearGradientPaint` from the middle of
/// one end to the middle of the other), its colours spread evenly.
fn gradient(heat_map: &[Color], vertical: bool) -> Rc<dyn IBrush> {
    let last = heat_map.len().saturating_sub(1).max(1) as f64;
    let stops: Vec<_> = heat_map
        .iter()
        .enumerate()
        .map(|(i, color)| ImmutableGradientStop::new(i as f64 / last, *color))
        .collect();
    let (start, end) = if vertical {
        ((0.5, 0.0), (0.5, 1.0))
    } else {
        ((0.0, 0.5), (1.0, 0.5))
    };
    Rc::new(ImmutableLinearGradientBrush::new(
        &stops,
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        Some(RelativePoint::new(start.0, start.1, RelativeUnit::Relative)),
        Some(RelativePoint::new(end.0, end.1, RelativeUnit::Relative)),
        None,
    ))
}

/// Black over a light colour and white over a dark one (`GetContrastingColor`: the integer luma of
/// the source above 128).
pub fn contrasting_color(color: Color) -> Color {
    let luma =
        (299 * u32::from(color.r) + 587 * u32::from(color.g) + 114 * u32::from(color.b)) / 1000;
    if luma > 128 {
        Color::from_rgb(0, 0, 0)
    } else {
        Color::from_rgb(255, 255, 255)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo_map_chart_engine::HeatLand;
    use crate::kernel::Coordinate;

    fn metrics(text: &str, size: f64) -> Size {
        Size::new(text.chars().count() as f64 * size * 0.6, size * 1.2)
    }

    fn heat(weights: &[f64]) -> Series {
        let mut series = Series::new(SeriesKind::Heat);
        series.values = weights
            .iter()
            .enumerate()
            .map(|(i, w)| Coordinate::weighted(i as f64, 0.0, *w))
            .collect();
        series.heat_map = vec![Color::from_rgb(255, 241, 118), Color::from_rgb(0, 0, 255)];
        series
    }

    fn assert_rect(actual: Rect, expected: (f64, f64, f64, f64)) {
        let actual = (actual.x, actual.y, actual.width, actual.height);
        let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
        assert!(
            close(actual.0, expected.0)
                && close(actual.1, expected.1)
                && close(actual.2, expected.2)
                && close(actual.3, expected.3),
            "{actual:?} != {expected:?}"
        );
    }

    /// `Chart.EnumerateHeatLegendSources` and `IsVisibleAtLegend`: the first visible heat series
    /// of a Cartesian chart, its weight bounds with `MinValue` and `MaxValue` over its weights;
    /// other series and a heat series without weights show nothing.
    #[test]
    fn the_source_of_a_chart_is_its_first_visible_heat_series() {
        let line = Series::from_values(SeriesKind::Line, [1.0, 2.0]);
        let mut hidden = heat(&[100.0, 200.0]);
        hidden.visible = false;
        let mut shown = heat(&[10.0, 20.0, 40.0]);
        shown.max_value = Some(50.0);
        let source = HeatLegendSource::of_series(&[line.clone(), hidden, shown]).unwrap();
        assert_eq!(
            (source.weight_bounds.min, source.weight_bounds.max),
            (10.0, 50.0)
        );
        assert_eq!(source.heat_map.len(), 2);
        assert_eq!(HeatLegendSource::of_series(&[line, heat(&[])]), None);
    }

    /// GeoMapTests.GeoMap_EnumerateHeatLegendSources_ReturnsHeatLandSeries: the heat land series
    /// of a map is the source of its legend, with the value of its land as both bounds.
    #[test]
    fn source_geo_map_enumerate_heat_legend_sources_returns_heat_land_series() {
        let lands = HeatLandSeries::new(vec![HeatLand::new("rus", 7.0)]);
        let source = HeatLegendSource::of_heat_lands(std::slice::from_ref(&lands)).unwrap();
        assert_eq!(source.heat_map, lands.series.heat_map);
        assert_eq!(
            (source.weight_bounds.min, source.weight_bounds.max),
            (7.0, 7.0)
        );
        let mut hidden = lands;
        hidden.series.visible = false;
        assert_eq!(HeatLegendSource::of_heat_lands(&[hidden]), None);
    }

    /// `GetLayout` at the right of a chart (vertical): the bar as thick as the larger label (the
    /// text grown by the label padding of 8 and 4) and as long as the plot less the padding, the
    /// lower bound at its top and the upper bound at its bottom, centred across it; the legend is
    /// the bar with its padding of 10 on every side.
    #[test]
    fn a_vertical_legend_runs_along_the_plot() {
        let legend = HeatLegend::default();
        let source = HeatLegendSource::of_series(&[heat(&[10.0, 20.0, 40.0])]).unwrap();
        let plot = Size::new(300.0, 200.0);
        let label = Size::new(2.0 * 16.0 * 0.6 + 16.0, 16.0 * 1.2 + 8.0);
        let size = legend.measure(Some(&source), true, plot, metrics);
        assert!((size.width - (label.width + 20.0)).abs() < 1e-9);
        assert!((size.height - 200.0).abs() < 1e-9);
        let primitives = legend.draw(Some(&source), true, Point::new(500.0, 50.0), plot, metrics);
        assert_eq!(primitives.len(), 3);
        let Geometry::Rectangle { rect, .. } = primitives[0].geometry else {
            panic!("the bar")
        };
        assert_rect(rect, (510.0, 60.0, label.width, 180.0));
        assert!(primitives[0].style.fill_brush.is_some());
        assert_eq!(primitives[0].style.brush_space, BrushSpace::Geometry);
        let texts: Vec<_> = primitives[1..]
            .iter()
            .map(|p| match &p.geometry {
                Geometry::Text { origin, text, .. } => (*origin, text.clone(), p.style.fill),
                _ => panic!("a label"),
            })
            .collect();
        // The light yellow of the low end takes black, the blue of the high end white.
        assert_eq!(texts[0].1, "10");
        assert_eq!(texts[0].2, Some(Color::from_rgb(0, 0, 0)));
        assert_eq!((texts[0].0.x, texts[0].0.y), (518.0, 64.0));
        assert_eq!(texts[1].1, "40");
        assert_eq!(texts[1].2, Some(Color::from_rgb(255, 255, 255)));
        assert!((texts[1].0.y - (60.0 + 180.0 - label.height + 4.0)).abs() < 1e-9);
    }

    /// `GetLayout` at the bottom of a chart (horizontal) with a `BadgeWidth` of 25 (the source's
    /// SKChartingVisualsTests): the bar is 25 high and as wide as the plot less the padding, the
    /// lower bound at its left and the upper bound at its right, centred across it.
    #[test]
    fn a_horizontal_legend_takes_its_badge_width() {
        let legend = HeatLegend {
            badge_width: Some(25.0),
            ..HeatLegend::default()
        };
        let source = HeatLegendSource::of_series(&[heat(&[5.0, 15.0])]).unwrap();
        let plot = Size::new(300.0, 200.0);
        let label = Size::new(2.0 * 16.0 * 0.6 + 16.0, 16.0 * 1.2 + 8.0);
        let size = legend.measure(Some(&source), false, plot, metrics);
        assert!((size.width - 300.0).abs() < 1e-9);
        assert!((size.height - (label.height.max(25.0) + 20.0)).abs() < 1e-9);
        let primitives = legend.draw(Some(&source), false, Point::new(0.0, 0.0), plot, metrics);
        let Geometry::Rectangle { rect, .. } = primitives[0].geometry else {
            panic!("the bar")
        };
        assert_rect(rect, (10.0, 10.0, 280.0, 25.0));
        let Geometry::Text { origin, .. } = &primitives[2].geometry else {
            panic!("the upper bound")
        };
        let offset = (label.height - 25.0).abs() * 0.5;
        assert!((origin.x - (10.0 + 280.0 - label.width + 8.0)).abs() < 1e-9);
        assert!((origin.y - (10.0 + offset + 4.0)).abs() < 1e-9);
    }

    /// Without a source the legend draws nothing and measures its padding (the source's empty
    /// layout); the labels take the formatter.
    #[test]
    fn an_empty_legend_measures_its_padding_and_labels_take_the_formatter() {
        let legend = HeatLegend {
            badge_padding: Thickness::new(30.0, 20.0, 8.0, 20.0),
            formatter: Labeler::new(|value| format!("{value} people")),
            ..HeatLegend::default()
        };
        let plot = Size::new(300.0, 200.0);
        assert_eq!(
            legend.measure(None, true, plot, metrics),
            Size::new(38.0, 40.0)
        );
        assert!(legend
            .draw(None, true, Point::new(0.0, 0.0), plot, metrics)
            .is_empty());
        let source = HeatLegendSource::of_series(&[heat(&[1.0, 2.0])]).unwrap();
        let primitives = legend.draw(Some(&source), true, Point::new(0.0, 0.0), plot, metrics);
        assert!(matches!(
            &primitives[1].geometry,
            Geometry::Text { text, .. } if text == "1 people"
        ));
    }

    /// `GetContrastingColor`: the integer luma of the source against 128.
    /// The labels of the legend break into lines at `MaxTooltipsAndLegendsLabelsWidth` (170) less
    /// their padding (the `MaxWidth` of their `LabelGeometry`): a text of the scene for each
    /// line, one line height apart from the left of the label, and a bar as thick as the block.
    #[test]
    fn labels_break_at_the_max_width_of_the_tooltips_and_legends() {
        let legend = HeatLegend {
            formatter: Labeler::new(|value| format!("{value} people in the area of the map")),
            ..HeatLegend::default()
        };
        let source = HeatLegendSource::of_series(&[heat(&[10.0, 20.0])]).unwrap();
        let plot = Size::new(300.0, 400.0);
        let block = Size::new(16.0 * 16.0 * 0.6, 3.0 * 16.0 * 1.2);
        let size = legend.measure(Some(&source), true, plot, metrics);
        assert!((size.width - (block.width + 16.0 + 20.0)).abs() < 1e-9);
        let primitives = legend.draw(Some(&source), true, Point::new(0.0, 0.0), plot, metrics);
        let texts: Vec<_> = primitives[1..]
            .iter()
            .map(|p| match &p.geometry {
                Geometry::Text { origin, text, .. } => (*origin, text.clone()),
                _ => panic!("a label"),
            })
            .collect();
        assert_eq!(texts.len(), 6);
        let lines: Vec<_> = texts[..3].iter().map(|(_, text)| text.as_str()).collect();
        assert_eq!(lines, ["10 people in ", "the area of the ", "map"]);
        for (i, (origin, _)) in texts[..3].iter().enumerate() {
            assert!((origin.x - 18.0).abs() < 1e-9);
            assert!((origin.y - (14.0 + i as f64 * 16.0 * 1.2)).abs() < 1e-9);
        }
        // The upper label ends at the end of the bar by the height of its three lines.
        assert!((texts[3].0.y - (10.0 + 380.0 - (block.height + 8.0) + 4.0)).abs() < 1e-9);
    }

    #[test]
    fn contrasting_colors_follow_the_luma() {
        assert_eq!(
            contrasting_color(Color::from_rgb(128, 128, 128)),
            Color::from_rgb(255, 255, 255)
        );
        assert_eq!(
            contrasting_color(Color::from_rgb(129, 129, 129)),
            Color::from_rgb(0, 0, 0)
        );
        assert_eq!(
            contrasting_color(Color::from_rgb(0, 0, 255)),
            Color::from_rgb(255, 255, 255)
        );
    }
}
