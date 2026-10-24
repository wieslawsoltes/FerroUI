use crate::xaml_charts_resources::{solid, SERIES_COLORS};
use crate::{
    AreaSeries, Axis, BarOrientation, BarSeries, CartesianAxis, CartesianSeries, CategoryAxis,
    CategorySortKey, Chart, DateTimeAxis, GridLines, MarkerShape, NumericRange, SampleValue,
    SortOrder, StrokeDashStyle,
};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::media::immutable::{ImmutableDashStyle, ImmutablePen};
use ferroui_base::media::text_formatting::{TextLayout, TextLayoutOptions};
use ferroui_base::media::{
    BoxShadows, BrushExtensions, DrawingContext, Geometry, IBrush, IPen, PenLineCap, PenLineJoin,
    StreamGeometry, Typeface,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{Matrix, Point, Rect, Ref, Size};
use std::cell::OnceCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A source point after axis mapping. Indices refer to the original series and
/// sample, including when a category axis changes the display order.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartPoint {
    pub series_index: usize,
    pub sample_index: usize,
    pub position: Point,
    pub bar_bounds: Option<Rect>,
    pub value: f64,
}

/// A read-only snapshot of calculated chart geometry, useful for diagnostics,
/// custom interactions and deterministic layout tests.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartLayout {
    pub size: Size,
    pub plot_area: Rect,
    pub points: Vec<ChartPoint>,
    pub legend_items: Vec<String>,
    pub axis_labels: Vec<String>,
    pub data_labels: Vec<(usize, usize, String)>,
    pub path_vertex_count: usize,
}

enum Draw {
    Line(Point, Point, Rc<dyn IPen>),
    Rectangle(Rect, Option<Rc<dyn IBrush>>, Option<Rc<dyn IPen>>),
    Ellipse(Rect, Rc<dyn IBrush>, Option<Rc<dyn IPen>>),
    Path {
        points: Vec<Point>,
        closed: bool,
        fill: Option<Rc<dyn IBrush>>,
        pen: Option<Rc<dyn IPen>>,
        geometry: OnceCell<Ref<Geometry>>,
    },
    Text(Rc<TextLayout>, Point, Option<Matrix>),
}

impl Draw {
    fn draw(&self, context: &mut DrawingContext) {
        match self {
            Self::Line(a, b, pen) => context.draw_line(pen, *a, *b),
            Self::Rectangle(rect, fill, pen) => context.draw_rectangle(
                fill.as_ref(),
                pen.as_ref(),
                *rect,
                0.0,
                0.0,
                &BoxShadows::default(),
            ),
            Self::Ellipse(rect, fill, pen) => context.draw_ellipse(Some(fill), pen.as_ref(), *rect),
            Self::Path {
                points,
                closed,
                fill,
                pen,
                geometry,
            } => {
                if points.len() < 2 {
                    return;
                }
                let geometry = geometry.get_or_init(|| {
                    let geometry = StreamGeometry::new();
                    {
                        let mut path = geometry.open();
                        path.begin_figure(points[0], fill.is_some());
                        for point in &points[1..] {
                            path.line_to(*point, true);
                        }
                        path.end_figure(*closed);
                    }
                    geometry.upcast()
                });
                context.draw_geometry(fill.as_ref(), pen.as_ref(), geometry);
            }
            Self::Text(layout, origin, transform) => {
                if let Some(transform) = transform {
                    let token = context.push_transform(*transform);
                    layout.draw(context, *origin);
                    context.pop(token);
                } else {
                    layout.draw(context, *origin);
                }
            }
        }
    }
}

pub(crate) struct ChartScene {
    pub(crate) layout: ChartLayout,
    background: Vec<Draw>,
    grid: Vec<Draw>,
    series: Vec<Draw>,
    annotations: Vec<Draw>,
    foreground: Vec<Draw>,
}

struct SeriesData {
    index: usize,
    series: Ref<CartesianSeries>,
    x: Rc<[Option<SampleValue>]>,
    y: Rc<[Option<SampleValue>]>,
    x_group: usize,
    y_group: usize,
    horizontal: bool,
    bar: bool,
    area: bool,
}

#[derive(Clone)]
struct Category {
    key: String,
    label: String,
    numeric: Option<f64>,
}
struct Tick {
    position: f64,
    text: String,
}
enum Scale {
    Numeric(NumericRange),
    Category {
        indices: HashMap<String, usize>,
        count: usize,
        banded: bool,
    },
    Date(DateTimeOffset, DateTimeOffset),
}
struct AxisGroup {
    key: Option<Ref<Axis>>,
    axis: Ref<CartesianAxis>,
    horizontal: bool,
    numeric: bool,
    numbers: Vec<f64>,
    dates: Vec<DateTimeOffset>,
    categories: Vec<Category>,
    category_keys: HashMap<String, usize>,
    banded: bool,
    scale: Scale,
    ticks: Vec<Tick>,
}

fn category(value: &SampleValue) -> Option<Category> {
    match value {
        SampleValue::Number(value) if value.is_finite() => Some(Category {
            key: format!("n{:016x}", if *value == 0.0 { 0 } else { value.to_bits() }),
            label: number(*value),
            numeric: Some(*value),
        }),
        SampleValue::Category(value) => Some(Category {
            key: format!("s{value}"),
            label: value.clone(),
            numeric: None,
        }),
        SampleValue::DateTime(value) => Some(Category {
            key: format!("d{}", value.utc_date_time().ticks()),
            label: value.to_string(),
            numeric: None,
        }),
        _ => None,
    }
}

fn number(value: f64) -> String {
    if value == 0.0 {
        "0".into()
    } else if value.abs() >= 1e8 || value.abs() < 1e-5 {
        format!("{value:.3e}")
    } else {
        let text = format!("{value:.6}");
        text.trim_end_matches('0').trim_end_matches('.').into()
    }
}

fn group_for(
    groups: &mut Vec<AxisGroup>,
    key: Option<Ref<Axis>>,
    horizontal: bool,
    numeric: bool,
    dates: bool,
) -> usize {
    if let Some(index) = groups.iter().position(|group| {
        group.key == key
            && group.horizontal == horizontal
            && group.numeric == numeric
            && (key.is_some() || group.axis.is::<DateTimeAxis>() == dates)
    }) {
        return index;
    }
    let axis: Ref<CartesianAxis> = key
        .as_ref()
        .and_then(|axis| axis.cast())
        .unwrap_or_else(|| {
            if numeric {
                crate::LinearAxis::new().upcast()
            } else if dates {
                DateTimeAxis::new().upcast()
            } else {
                CategoryAxis::new().upcast()
            }
        });
    groups.push(AxisGroup {
        key,
        axis,
        horizontal,
        numeric,
        numbers: Vec::new(),
        dates: Vec::new(),
        categories: Vec::new(),
        category_keys: HashMap::new(),
        banded: false,
        scale: Scale::Numeric(NumericRange {
            minimum: 0.0,
            maximum: 1.0,
            spacing: 0.2,
        }),
        ticks: Vec::new(),
    });
    groups.len() - 1
}

impl AxisGroup {
    fn resolve(&mut self, target: usize) {
        if let Some(axis) = self.axis.cast::<crate::LinearAxis>() {
            let range = axis.resolve_range(&self.numbers, target);
            self.ticks = range
                .ticks()
                .into_iter()
                .map(|value| Tick {
                    position: range.normalize(value),
                    text: number(value),
                })
                .collect();
            self.scale = Scale::Numeric(range);
        } else if let Some(axis) = self.axis.cast::<DateTimeAxis>() {
            let (mut minimum, mut maximum) = axis.resolve_range(&self.dates);
            if self.banded && !self.dates.is_empty() {
                // Dates are bar centers. Reserve half the nearest spacing at
                // automatic boundaries so the first/last bars are complete.
                // Explicit boundaries continue to clip data exactly as set.
                let mut instants: Vec<_> = self
                    .dates
                    .iter()
                    .map(|date| date.utc_date_time().ticks())
                    .collect();
                instants.sort_unstable();
                let gap = instants
                    .windows(2)
                    .map(|pair| pair[1] - pair[0])
                    .filter(|gap| *gap > 0)
                    .min()
                    .unwrap_or(maximum.utc_date_time().ticks() - minimum.utc_date_time().ticks());
                let padding = gap / 2;
                if axis.minimum().is_none() {
                    minimum = DateTimeOffset::from_ticks(
                        (minimum.utc_date_time().ticks() - padding).max(0),
                        ferroui_base::animation::TimeSpan::ZERO,
                    );
                }
                if axis.maximum().is_none() {
                    maximum = DateTimeOffset::from_ticks(
                        (maximum.utc_date_time().ticks() + padding)
                            .min(DateTimeOffset::MAX_VALUE.utc_date_time().ticks()),
                        ferroui_base::animation::TimeSpan::ZERO,
                    );
                }
            }
            self.ticks = axis
                .labeled_ticks(minimum, maximum, target)
                .into_iter()
                .map(|(value, text)| Tick {
                    position: date_position(value, minimum, maximum),
                    text,
                })
                .collect();
            self.scale = Scale::Date(minimum, maximum);
        } else {
            let axis = self.axis.cast::<CategoryAxis>();
            if axis
                .as_ref()
                .is_some_and(|axis| axis.sort_key() == CategorySortKey::Value)
            {
                let descending = axis
                    .as_ref()
                    .is_some_and(|axis| axis.sort_order() == SortOrder::Descending);
                self.categories.sort_by(|a, b| {
                    // A single total order is required for mixed numeric/text
                    // sources. Comparing mixed pairs lexically and numeric pairs
                    // numerically would violate transitivity.
                    let order = match (a.numeric, b.numeric) {
                        (Some(a), Some(b)) => a.total_cmp(&b),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        _ => a.label.cmp(&b.label),
                    };
                    if descending {
                        order.reverse()
                    } else {
                        order
                    }
                });
            } else if axis
                .as_ref()
                .is_some_and(|axis| axis.sort_order() == SortOrder::Descending)
            {
                self.categories.reverse();
            }
            let count = self.categories.len();
            let step = count.div_ceil(target.max(1)).max(1);
            self.ticks = self
                .categories
                .iter()
                .enumerate()
                .filter(|(i, _)| i % step == 0)
                .map(|(i, c)| Tick {
                    position: category_position(i, count, self.banded),
                    text: c.label.clone(),
                })
                .collect();
            self.scale = Scale::Category {
                indices: self
                    .categories
                    .iter()
                    .enumerate()
                    .map(|(i, c)| (c.key.clone(), i))
                    .collect(),
                count,
                banded: self.banded,
            };
        }
    }
    fn position(&self, value: &SampleValue) -> Option<f64> {
        match (&self.scale, value) {
            (Scale::Numeric(range), SampleValue::Number(value)) if value.is_finite() => {
                Some(range.normalize(*value))
            }
            (Scale::Date(min, max), SampleValue::DateTime(value)) => {
                Some(date_position(*value, *min, *max))
            }
            (
                Scale::Category {
                    indices,
                    count,
                    banded,
                },
                value,
            ) => {
                let key = category(value)?.key;
                indices
                    .get(&key)
                    .map(|i| category_position(*i, *count, *banded))
            }
            _ => None,
        }
    }
}

fn category_position(index: usize, count: usize, banded: bool) -> f64 {
    if count <= 1 {
        0.5
    } else if banded {
        (index as f64 + 0.5) / count as f64
    } else {
        index as f64 / (count - 1) as f64
    }
}
fn date_position(value: DateTimeOffset, minimum: DateTimeOffset, maximum: DateTimeOffset) -> f64 {
    let low = minimum.utc_date_time().ticks();
    let span = maximum.utc_date_time().ticks() - low;
    if span == 0 {
        0.5
    } else {
        (value.utc_date_time().ticks() - low) as f64 / span as f64
    }
}

pub(crate) fn resource(chart: &Chart, name: &str) -> Option<Rc<dyn IBrush>> {
    let value = chart.try_find_resource(
        &ResourceKey::from(name),
        chart.actual_theme_variant().as_ref(),
    )??;
    let converted = ValueTypes::try_convert(Some(&value), ValueType::of::<Rc<dyn IBrush>>())??;
    converted.downcast_ref::<Rc<dyn IBrush>>().cloned()
}
fn text(chart: &Chart, value: &str, brush: Rc<dyn IBrush>) -> Rc<TextLayout> {
    Rc::new(TextLayout::new(
        value,
        Typeface::with_style(
            chart.font_family(),
            chart.font_style(),
            chart.font_weight(),
            chart.font_stretch(),
        ),
        TextLayoutOptions {
            font_size: chart.font_size(),
            foreground: Some(brush),
            font_features: chart.font_features(),
            letter_spacing: chart.letter_spacing(),
            flow_direction: chart.flow_direction(),
            ..Default::default()
        },
    ))
}
fn pen(brush: Rc<dyn IBrush>, thickness: f64, dash: StrokeDashStyle) -> Rc<dyn IPen> {
    let dashes = (!dash.dashes().is_empty()).then_some(dash.dashes());
    Rc::new(ImmutablePen::new(
        Some(BrushExtensions::to_immutable(&brush)),
        thickness,
        dashes.map(|values| Rc::new(ImmutableDashStyle::new(Some(values), 0.0))),
        PenLineCap::Flat,
        PenLineJoin::Round,
        10.0,
    ))
}
fn path(
    points: Vec<Point>,
    closed: bool,
    fill: Option<Rc<dyn IBrush>>,
    pen: Option<Rc<dyn IPen>>,
) -> Draw {
    Draw::Path {
        points,
        closed,
        fill,
        pen,
        geometry: OnceCell::new(),
    }
}

impl ChartScene {
    pub(crate) fn build(chart: &Chart, size: Size) -> Self {
        let size = Size::new(finite_size(size.width), finite_size(size.height));
        let dark = chart.actual_theme_variant() == Some(ThemeVariant::dark());
        let foreground = if chart.is_set(
            ferroui_controls::primitives::TemplatedControl::foreground_property().as_property(),
        ) {
            chart.foreground()
        } else {
            resource(chart, "ChartsControlForegroundBrush").or_else(|| chart.foreground())
        }
        .unwrap_or_else(|| solid(if dark { 0xfff3f3f3 } else { 0xff242424 }));
        let mut scene = Self {
            layout: ChartLayout {
                size,
                plot_area: Rect::new(0.0, 0.0, size.width, size.height),
                points: Vec::new(),
                legend_items: Vec::new(),
                axis_labels: Vec::new(),
                data_labels: Vec::new(),
                path_vertex_count: 0,
            },
            background: Vec::new(),
            grid: Vec::new(),
            series: Vec::new(),
            annotations: Vec::new(),
            foreground: Vec::new(),
        };
        if let Some(background) = chart
            .background()
            .or_else(|| resource(chart, "ChartsControlBackgroundBrush"))
        {
            scene.background.push(Draw::Rectangle(
                Rect::new(0.0, 0.0, size.width, size.height),
                Some(background),
                None,
            ));
        }
        if size.width <= 0.0 || size.height <= 0.0 {
            return scene;
        }
        let mut groups = Vec::new();
        let mut series_data = Vec::new();
        for (index, series) in chart
            .series()
            .snapshot()
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_visible())
        {
            let bar = series.cast::<BarSeries>();
            let horizontal = bar
                .as_ref()
                .is_some_and(|bar| bar.orientation() == BarOrientation::Horizontal);
            let x = series
                .x_values()
                .map(|source| source.snapshot())
                .unwrap_or_default();
            let y = series
                .y_values()
                .map(|source| source.snapshot())
                .unwrap_or_default();
            let implicit_x = series.x_values().is_none();
            let count = if implicit_x {
                y.len()
            } else {
                x.len().min(y.len())
            };
            let dates = x
                .iter()
                .take(count)
                .flatten()
                .next()
                .is_some_and(|v| matches!(v, SampleValue::DateTime(_)));
            let x_group = group_for(&mut groups, series.x_axis(), !horizontal, false, dates);
            let y_group = group_for(&mut groups, series.y_axis(), horizontal, true, false);
            groups[x_group].banded |= bar.is_some();
            // Numeric ranges describe renderable pairs. An unmatched or invalid X must
            // not let an invisible Y outlier compress every visible series.
            // Missing X uses source indices; a present source is zipped with Y.
            for index in 0..count {
                let fallback = SampleValue::Number(index as f64);
                let value = if implicit_x {
                    Some(&fallback)
                } else {
                    x[index].as_ref()
                };
                let Some(value) = value else {
                    continue;
                };
                let group = &mut groups[x_group];
                if group.axis.is::<DateTimeAxis>() {
                    let SampleValue::DateTime(date) = value else {
                        continue;
                    };
                    group.dates.push(*date);
                } else if let Some(category) = category(value) {
                    if !group.category_keys.contains_key(&category.key) {
                        group
                            .category_keys
                            .insert(category.key.clone(), group.categories.len());
                        group.categories.push(category);
                    }
                } else {
                    continue;
                }
                let Some(SampleValue::Number(number)) = y[index].as_ref() else {
                    continue;
                };
                if !number.is_finite() {
                    continue;
                }
                groups[y_group].numbers.push(*number);
            }
            let area = series.is::<AreaSeries>();
            if bar.is_some() || area {
                groups[y_group].numbers.push(0.0);
            }
            series_data.push(SeriesData {
                index,
                series: series.clone(),
                x,
                y,
                x_group,
                y_group,
                horizontal,
                bar: bar.is_some(),
                area,
            });
        }
        for axis in chart.axes().snapshot().iter() {
            if !groups.iter().any(|group| group.key.as_ref() == Some(axis)) {
                let numeric = axis.is::<crate::LinearAxis>();
                group_for(
                    &mut groups,
                    Some(axis.clone()),
                    !numeric,
                    numeric,
                    axis.is::<DateTimeAxis>(),
                );
            }
        }
        for group in &mut groups {
            group.resolve(
                if group.horizontal {
                    (size.width / 90.0) as usize
                } else {
                    (size.height / 55.0) as usize
                }
                .clamp(2, 100),
            );
        }
        let font_size = chart.font_size();
        let padding = chart.padding();
        let mut margins = [
            padding.left + 8.0,
            padding.top + 8.0,
            padding.right + 8.0,
            padding.bottom + 8.0,
        ];
        let mut lanes: Vec<(usize, f64)> = Vec::new();
        let mut horizontal_count = 0;
        let mut vertical_count = 0;
        for group in &groups {
            if !group.axis.is_visible() {
                lanes.push((0, 0.0));
                continue;
            }
            let side = if group.horizontal {
                let side = if horizontal_count % 2 == 0 { 3 } else { 1 };
                horizontal_count += 1;
                side
            } else {
                let side = if vertical_count % 2 == 0 { 0 } else { 2 };
                vertical_count += 1;
                side
            };
            let tick_size = if group.axis.show_tick_labels() {
                group
                    .ticks
                    .iter()
                    .map(|tick| {
                        let t = text(chart, &tick.text, foreground.clone());
                        if group.horizontal {
                            t.height()
                        } else {
                            t.width()
                        }
                    })
                    .fold(0.0, f64::max)
            } else {
                0.0
            };
            let lane = 8.0
                + tick_size
                + if group.axis.label().is_empty() {
                    0.0
                } else {
                    font_size * 1.6
                };
            lanes.push((side, margins[side]));
            margins[side] += lane;
        }
        let legend = if chart.show_legend() {
            legend_rows(
                chart,
                &series_data,
                size.width - padding.left - padding.right,
                foreground.clone(),
            )
        } else {
            Vec::new()
        };
        let legend_height = legend
            .iter()
            .map(|(_, _, position)| position.y + font_size * 1.5)
            .fold(0.0, f64::max);
        margins[3] += legend_height;
        let left = margins[0].min(size.width * 0.45);
        let right = margins[2].min(size.width * 0.45);
        let top = margins[1].min(size.height * 0.45);
        let bottom = margins[3].min(size.height * 0.45);
        let plot = Rect::new(
            left,
            top,
            (size.width - left - right).max(0.0),
            (size.height - top - bottom).max(0.0),
        );
        scene.layout.plot_area = plot;
        for (group, &(side, offset)) in groups.iter().zip(&lanes) {
            scene.axis(chart, group, side, offset, foreground.clone(), dark);
        }
        // The per-point labels of every series are drawn after the markers of every series,
        // so that no marker covers a label.
        let mut labels = Vec::new();
        for data in &series_data {
            scene.plot_series(chart, data, &series_data, &groups, foreground.clone(), &mut labels);
        }
        scene.annotations.extend(labels);
        for (title, color, position) in legend {
            let origin = Point::new(
                position.x + padding.left + 8.0,
                size.height - legend_height + position.y,
            );
            if let Some(color) = color {
                scene.foreground.push(Draw::Rectangle(
                    Rect::new(origin.x, origin.y + 3.0, 14.0, 10.0),
                    Some(color),
                    None,
                ));
                scene.foreground.push(Draw::Text(
                    title.clone(),
                    Point::new(origin.x + 21.0, origin.y),
                    None,
                ));
            } else {
                scene
                    .foreground
                    .push(Draw::Text(title.clone(), origin, None));
            }
        }
        scene.layout.legend_items = if chart.show_legend() {
            series_data.iter().map(|d| d.series.title()).collect()
        } else {
            Vec::new()
        };
        scene
    }

    fn axis(
        &mut self,
        chart: &Chart,
        group: &AxisGroup,
        side: usize,
        offset: f64,
        foreground: Rc<dyn IBrush>,
        dark: bool,
    ) {
        if !group.axis.is_visible() {
            return;
        }
        let plot = self.layout.plot_area;
        let axis = &group.axis;
        let stroke = axis
            .axis_line_brush()
            .or_else(|| resource(chart, "ChartsAxisLineBrush"))
            .unwrap_or_else(|| solid(if dark { 0xff8a8a8a } else { 0xff777777 }));
        let major = axis
            .grid_line_major_brush()
            .or_else(|| resource(chart, "ChartsGridLineMajorBrush"))
            .unwrap_or_else(|| solid(if dark { 0xff505050 } else { 0xffdedede }));
        let minor = axis
            .grid_line_minor_brush()
            .or_else(|| resource(chart, "ChartsGridLineMinorBrush"))
            .unwrap_or_else(|| solid(if dark { 0xff383838 } else { 0xffeeeeee }));
        let tick_brush = axis
            .tick_brush()
            .or_else(|| resource(chart, "ChartsTickBrush"))
            .unwrap_or_else(|| stroke.clone());
        let label_brush = axis
            .tick_label_brush()
            .or_else(|| resource(chart, "ChartsTickLabelBrush"))
            .unwrap_or_else(|| foreground.clone());
        let axis_pen = pen(stroke, 1.0, StrokeDashStyle::Solid);
        let major_pen = pen(major, 1.0, StrokeDashStyle::Solid);
        let minor_pen = pen(minor, 1.0, StrokeDashStyle::Solid);
        let tick_pen = pen(tick_brush, 1.0, StrokeDashStyle::Solid);
        let edge = match side {
            0 => plot.x - offset + 8.0,
            1 => plot.y - offset + 8.0,
            2 => plot.x + plot.width + offset - 8.0,
            _ => plot.y + plot.height + offset - 8.0,
        };
        let outward = if side == 0 || side == 1 { -1.0 } else { 1.0 };
        let (a, b) = if group.horizontal {
            (
                Point::new(plot.x, edge),
                Point::new(plot.x + plot.width, edge),
            )
        } else {
            (
                Point::new(edge, plot.y),
                Point::new(edge, plot.y + plot.height),
            )
        };
        self.foreground.push(Draw::Line(a, b, axis_pen));
        let mut previous_label_right = f64::NEG_INFINITY;
        for tick in &group.ticks {
            if !(0.0..=1.0).contains(&tick.position) {
                continue;
            }
            let coordinate = if group.horizontal {
                plot.x + tick.position * plot.width
            } else {
                plot.y + (1.0 - tick.position) * plot.height
            };
            if axis.grid_lines() != GridLines::None {
                let (a, b) = if group.horizontal {
                    (
                        Point::new(coordinate, plot.y),
                        Point::new(coordinate, plot.y + plot.height),
                    )
                } else {
                    (
                        Point::new(plot.x, coordinate),
                        Point::new(plot.x + plot.width, coordinate),
                    )
                };
                self.grid.push(Draw::Line(a, b, major_pen.clone()));
            }
            if axis.show_tick_marks() {
                let (a, b) = if group.horizontal {
                    (
                        Point::new(coordinate, edge),
                        Point::new(coordinate, edge + outward * 5.0),
                    )
                } else {
                    (
                        Point::new(edge, coordinate),
                        Point::new(edge + outward * 5.0, coordinate),
                    )
                };
                self.foreground.push(Draw::Line(a, b, tick_pen.clone()));
            }
            if axis.show_tick_labels() {
                let label = text(chart, &tick.text, label_brush.clone());
                let mut origin = if group.horizontal {
                    Point::new(
                        coordinate - label.width() * 0.5,
                        edge + if outward < 0.0 {
                            -label.height() - 7.0
                        } else {
                            7.0
                        },
                    )
                } else {
                    Point::new(
                        edge + if outward < 0.0 {
                            -label.width() - 7.0
                        } else {
                            7.0
                        },
                        coordinate - label.height() * 0.5,
                    )
                };
                if group.horizontal {
                    // A centered endpoint label can extend beyond the control,
                    // even when every plot point fits. Keep complete text in
                    // the viewport and omit colliding labels while preserving
                    // every requested tick and grid line.
                    origin.x = origin
                        .x
                        .clamp(0.0, (self.layout.size.width - label.width()).max(0.0));
                    if origin.x < previous_label_right + 4.0 {
                        continue;
                    }
                    previous_label_right = origin.x + label.width();
                }
                self.layout.axis_labels.push(tick.text.clone());
                self.foreground.push(Draw::Text(label, origin, None));
            }
        }
        if axis.grid_lines() == GridLines::Minor {
            for ticks in group.ticks.windows(2) {
                for fraction in 1..5 {
                    let p = ticks[0].position
                        + (ticks[1].position - ticks[0].position) * fraction as f64 / 5.0;
                    if !(0.0..=1.0).contains(&p) {
                        continue;
                    }
                    let (a, b) = if group.horizontal {
                        let x = plot.x + p * plot.width;
                        (Point::new(x, plot.y), Point::new(x, plot.y + plot.height))
                    } else {
                        let y = plot.y + (1.0 - p) * plot.height;
                        (Point::new(plot.x, y), Point::new(plot.x + plot.width, y))
                    };
                    self.grid.push(Draw::Line(a, b, minor_pen.clone()));
                }
            }
        }
        let label_text = axis.label();
        if !label_text.is_empty() {
            let label = text(chart, &label_text, foreground);
            self.layout.axis_labels.push(label_text);
            if group.horizontal {
                let y = if outward > 0.0 {
                    edge + chart.font_size() * 1.9
                } else {
                    edge - chart.font_size() * 3.0
                };
                self.foreground.push(Draw::Text(
                    label.clone(),
                    Point::new(plot.x + (plot.width - label.width()) * 0.5, y),
                    None,
                ));
            } else {
                let x = if outward < 0.0 {
                    offset.max(0.0)
                } else {
                    self.layout.size.width - offset - label.height()
                };
                let y = plot.y + (plot.height + label.width()) * 0.5;
                self.foreground.push(Draw::Text(
                    label,
                    Point::new(0.0, 0.0),
                    Some(Matrix::new(0.0, -1.0, 1.0, 0.0, x, y)),
                ));
            }
        }
    }

    fn plot_series(
        &mut self,
        chart: &Chart,
        data: &SeriesData,
        all: &[SeriesData],
        groups: &[AxisGroup],
        foreground: Rc<dyn IBrush>,
        label_draws: &mut Vec<Draw>,
    ) {
        let series = &data.series;
        let plot = self.layout.plot_area;
        let palette = SERIES_COLORS[data.index % SERIES_COLORS.len()];
        let stroke = series.stroke().unwrap_or_else(|| solid(palette));
        let thickness = series.effective_stroke_thickness();
        let series_pen = if thickness > 0.0 {
            Some(pen(stroke.clone(), thickness, series.stroke_dash_style()))
        } else {
            None
        };
        let fill = if let Some(area) = series.cast::<AreaSeries>() {
            area.fill()
                .unwrap_or_else(|| solid((palette & 0x00ff_ffff) | 0x5500_0000))
        } else if let Some(bar) = series.cast::<BarSeries>() {
            bar.fill().unwrap_or_else(|| stroke.clone())
        } else {
            stroke.clone()
        };
        let x_group = &groups[data.x_group];
        let y_group = &groups[data.y_group];
        let baseline = match y_group.scale {
            Scale::Numeric(range) => range.normalize(0.0).clamp(0.0, 1.0),
            _ => 0.0,
        };
        let peers: Vec<_> = all
            .iter()
            .filter(|d| d.bar && d.x_group == data.x_group && d.horizontal == data.horizontal)
            .collect();
        let bar_index = peers
            .iter()
            .position(|d| d.index == data.index)
            .unwrap_or(0);
        let count = if series.x_values().is_some() {
            data.x.len().min(data.y.len())
        } else {
            data.y.len()
        };
        let category_count = match &x_group.scale {
            Scale::Category { count, .. } => *count,
            _ => count,
        }
        .max(1);
        let band = if matches!(x_group.scale, Scale::Date(..)) {
            let mut positions: Vec<_> = x_group
                .dates
                .iter()
                .filter_map(|date| x_group.position(&SampleValue::DateTime(*date)))
                .collect();
            positions.sort_by(f64::total_cmp);
            positions.dedup();
            let gap = positions
                .windows(2)
                .map(|p| p[1] - p[0])
                .filter(|gap| *gap > 0.0 && gap.is_finite())
                .fold(if positions.len() <= 1 { 0.5 } else { 1.0 }, f64::min);
            gap * 0.8
        } else {
            0.8 / category_count as f64
        };
        let bar_width = band / peers.len().max(1) as f64;
        let labels = series.data_label_overrides();
        let markers = series.data_marker_overrides();
        let mut segment = Vec::new();
        let mut annotations = Vec::new();
        let finish_segment = |scene: &mut Self, segment: &mut Vec<Point>| {
            if segment.is_empty() {
                return;
            }
            let points = std::mem::take(segment);
            if data.area {
                let first = points[0];
                let last = *points.last().expect("nonempty segment");
                let mut area = Vec::with_capacity(points.len() + 2);
                area.push(Point::new(first.x, baseline));
                area.extend_from_slice(&points);
                area.push(Point::new(last.x, baseline));
                let area: Vec<_> = clip_polygon(area)
                    .into_iter()
                    .map(|point| project(plot, point))
                    .collect();
                // The upper edge is monotone for ordinary area series; the
                // closing baseline is handled separately by polygon reduction.
                let area = decimate_polygon(area, plot.width.max(1.0) as usize);
                scene.layout.path_vertex_count += area.len();
                scene
                    .series
                    .push(path(area, true, Some(fill.clone()), None));
            }
            if series_pen.is_some() {
                for points in clip_polyline(&points) {
                    let points = decimate(
                        points.into_iter().map(|p| project(plot, p)).collect(),
                        plot.width.max(1.0) as usize,
                    );
                    scene.layout.path_vertex_count += points.len();
                    scene
                        .series
                        .push(path(points, false, None, series_pen.clone()));
                }
            }
        };
        for index in 0..count {
            let x = if series.x_values().is_none() {
                Some(SampleValue::Number(index as f64))
            } else {
                data.x[index].clone()
            };
            let y = data.y[index].as_ref();
            let mapped = x
                .as_ref()
                .and_then(|x| x_group.position(x))
                .zip(y.and_then(|y| y_group.position(y)));
            let Some((x, y)) = mapped.filter(|(x, y)| x.is_finite() && y.is_finite()) else {
                finish_segment(self, &mut segment);
                continue;
            };
            let normalized = if data.horizontal {
                Point::new(y, x)
            } else {
                Point::new(x, y)
            };
            // Diagnostics remain finite for arbitrarily distant samples. The
            // path itself is clipped in normalized space before projection.
            let mut point = project(
                plot,
                Point::new(normalized.x.clamp(-1e6, 1e6), normalized.y.clamp(-1e6, 1e6)),
            );
            let value = match data.y[index] {
                Some(SampleValue::Number(value)) => value,
                _ => continue,
            };
            let bounds = if data.bar {
                let start = x - band * 0.5 + bar_index as f64 * bar_width;
                let end = start + bar_width * 0.92;
                let clipped_start = start.clamp(0.0, 1.0);
                let clipped_end = end.clamp(0.0, 1.0);
                let rect = if data.horizontal {
                    let zero = plot.x + baseline * plot.width;
                    let endpoint = plot.x + y.clamp(0.0, 1.0) * plot.width;
                    Rect::new(
                        zero.min(endpoint),
                        plot.y + (1.0 - clipped_end) * plot.height,
                        (endpoint - zero).abs(),
                        (clipped_end - clipped_start) * plot.height,
                    )
                } else {
                    let zero = plot.y + (1.0 - baseline) * plot.height;
                    let endpoint = plot.y + (1.0 - y.clamp(0.0, 1.0)) * plot.height;
                    Rect::new(
                        plot.x + clipped_start * plot.width,
                        zero.min(endpoint),
                        (clipped_end - clipped_start) * plot.width,
                        (endpoint - zero).abs(),
                    )
                };
                if data.horizontal {
                    point.y = rect.y + rect.height * 0.5;
                } else {
                    point.x = rect.x + rect.width * 0.5;
                }
                if rect.width > 0.0 && rect.height > 0.0 {
                    self.series.push(Draw::Rectangle(
                        rect,
                        Some(fill.clone()),
                        series_pen.clone(),
                    ));
                }
                Some(rect)
            } else {
                segment.push(normalized);
                None
            };
            self.layout.points.push(ChartPoint {
                series_index: data.index,
                sample_index: index,
                position: point,
                bar_bounds: bounds,
                value,
            });
            if series.show_data_markers() {
                let over = markers.try_get_value(&(index as u32));
                let shape = over
                    .as_ref()
                    .map(|o| o.shape())
                    .unwrap_or_else(|| series.marker_shape());
                let brush = over
                    .and_then(|o| o.brush())
                    .or_else(|| series.data_marker_brush())
                    .unwrap_or_else(|| stroke.clone());
                if (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y) {
                    marker(&mut annotations, point, shape, brush);
                }
            }
            if series.show_data_labels() {
                let over = labels.try_get_value(&(index as u32));
                let value_text = over
                    .as_ref()
                    .map(|o| o.text().to_owned())
                    .unwrap_or_else(|| number(value));
                if !value_text.is_empty() {
                    let brush = over
                        .and_then(|o| o.brush())
                        .or_else(|| series.data_label_brush())
                        .unwrap_or_else(|| foreground.clone());
                    let label = text(chart, &value_text, brush);
                    // A label goes on the side of the point its value points to (above a
                    // positive value, below a negative one; right or left on a horizontal bar),
                    // and on the other side when there is no room for it inside the plot there,
                    // so that it is not pushed back over its own point and marker.
                    let origin = if data.horizontal {
                        let after = point.x + LABEL_GAP;
                        let before = point.x - label.width() - LABEL_GAP;
                        let fits_after = after + label.width() <= plot.x + plot.width;
                        let fits_before = before >= plot.x;
                        let x = match (value < 0.0, fits_before, fits_after) {
                            (true, true, _) | (true, false, false) => before,
                            (true, false, true) => after,
                            (false, _, true) | (false, false, false) => after,
                            (false, true, false) => before,
                        };
                        Point::new(x, point.y - label.height() * 0.5)
                    } else {
                        let above = point.y - label.height() - LABEL_GAP;
                        let below = point.y + LABEL_GAP;
                        let fits_above = above >= plot.y;
                        let fits_below = below + label.height() <= plot.y + plot.height;
                        let y = match (value < 0.0, fits_above, fits_below) {
                            (true, _, true) | (true, false, false) => below,
                            (true, true, false) => above,
                            (false, true, _) | (false, false, false) => above,
                            (false, false, true) => below,
                        };
                        Point::new(point.x - label.width() * 0.5, y)
                    };
                    let origin = Point::new(
                        origin
                            .x
                            .clamp(plot.x, (plot.x + plot.width - label.width()).max(plot.x)),
                        origin
                            .y
                            .clamp(plot.y, (plot.y + plot.height - label.height()).max(plot.y)),
                    );
                    if (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y) {
                        label_draws.push(Draw::Text(label, origin, None));
                    }
                    self.layout
                        .data_labels
                        .push((data.index, index, value_text));
                }
            }
        }
        finish_segment(self, &mut segment);
        self.annotations.extend(annotations);
    }

    pub(crate) fn draw(&self, context: &mut DrawingContext) {
        let bounds = Rect::new(0.0, 0.0, self.layout.size.width, self.layout.size.height);
        let control_clip = context.push_clip(bounds);
        for draw in &self.background {
            draw.draw(context);
        }
        let plot_clip = context.push_clip(self.layout.plot_area);
        for draw in &self.grid {
            draw.draw(context);
        }
        for draw in &self.series {
            draw.draw(context);
        }
        context.pop(plot_clip);
        // Endpoint markers remain whole and labels stay visible at automatic
        // bounds. Only samples inside the plot contribute annotations, and
        // the surrounding control clip still constrains their full extent.
        for draw in &self.annotations {
            draw.draw(context);
        }
        for draw in &self.foreground {
            draw.draw(context);
        }
        context.pop(control_clip);
    }
}

fn finite_size(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1e9)
    } else {
        0.0
    }
}

fn project(plot: Rect, point: Point) -> Point {
    Point::new(
        plot.x + point.x * plot.width,
        plot.y + (1.0 - point.y) * plot.height,
    )
}

fn crossing(a: Point, b: Point, boundary: f64, vertical: bool) -> Point {
    // Interpolating by t loses an entire unit-sized plot when endpoints are
    // huge: (1e100,1e100) -> (-1e100,-1e100) rounds t to 0.5 at BOTH edges.
    // A scaled line equation retains the boundary term, and a compensated
    // determinant avoids cancellation when the line passes near the origin.
    let sx = a.x.abs().max(b.x.abs()).max(1.0);
    let sy = a.y.abs().max(b.y.abs()).max(1.0);
    let (ax, ay, bx, by) = (a.x / sx, a.y / sy, b.x / sx, b.y / sy);
    let p = ay - by;
    let q = bx - ax;
    let product = ay * bx;
    let determinant = ax.mul_add(by, -product) + (-ay).mul_add(bx, product);
    if vertical {
        Point::new(boundary, -p.mul_add(boundary / sx, determinant) / q * sy)
    } else {
        Point::new(-q.mul_add(boundary / sy, determinant) / p * sx, boundary)
    }
}
fn outcode(p: Point) -> u8 {
    (if p.x < 0.0 {
        1
    } else if p.x > 1.0 {
        2
    } else {
        0
    }) | (if p.y < 0.0 {
        4
    } else if p.y > 1.0 {
        8
    } else {
        0
    })
}
fn clip_segment(mut a: Point, mut b: Point) -> Option<(Point, Point)> {
    for _ in 0..12 {
        let ac = outcode(a);
        let bc = outcode(b);
        if ac | bc == 0 {
            return Some((a, b));
        }
        if ac & bc != 0 {
            return None;
        }
        let c = if ac != 0 { ac } else { bc };
        let p = if c & 8 != 0 {
            crossing(a, b, 1.0, false)
        } else if c & 4 != 0 {
            crossing(a, b, 0.0, false)
        } else if c & 2 != 0 {
            crossing(a, b, 1.0, true)
        } else {
            crossing(a, b, 0.0, true)
        };
        if !p.x.is_finite() || !p.y.is_finite() {
            return None;
        }
        if ac != 0 {
            a = p;
        } else {
            b = p;
        }
    }
    None
}
fn clip_polyline(points: &[Point]) -> Vec<Vec<Point>> {
    let mut result: Vec<Vec<Point>> = Vec::new();
    for pair in points.windows(2) {
        if let Some((a, b)) = clip_segment(pair[0], pair[1]) {
            if let Some(last) = result.last_mut().filter(|last| last.last() == Some(&a)) {
                last.push(b);
            } else {
                result.push(vec![a, b]);
            }
        }
    }
    result
}
fn clip_polygon(mut points: Vec<Point>) -> Vec<Point> {
    for edge in 0..4 {
        let vertical = edge < 2;
        let boundary = (edge % 2) as f64;
        let inside = |p: Point| {
            if edge % 2 == 0 {
                (if vertical { p.x } else { p.y }) >= boundary
            } else {
                (if vertical { p.x } else { p.y }) <= boundary
            }
        };
        if points.is_empty() {
            break;
        }
        let mut output = Vec::new();
        let mut previous = *points.last().expect("nonempty polygon");
        for current in points {
            if inside(current) != inside(previous) {
                output.push(crossing(previous, current, boundary, vertical));
            }
            if inside(current) {
                output.push(current);
            }
            previous = current;
        }
        points = output;
    }
    points
        .into_iter()
        .filter(|p| p.x.is_finite() && p.y.is_finite())
        .collect()
}

fn legend_rows(
    chart: &Chart,
    series: &[SeriesData],
    width: f64,
    foreground: Rc<dyn IBrush>,
) -> Vec<(Rc<TextLayout>, Option<Rc<dyn IBrush>>, Point)> {
    let mut rows = Vec::new();
    let mut x = 0.0;
    let mut y = 6.0;
    if !chart.legend_title().is_empty() {
        let title = text(chart, &chart.legend_title(), foreground.clone());
        y += title.height() + 6.0;
        rows.push((title, None, Point::new(0.0, 6.0)));
    }
    for data in series {
        let title = text(chart, &data.series.title(), foreground.clone());
        let item_width = title.width() + 42.0;
        if x > 0.0 && x + item_width > width {
            x = 0.0;
            y += chart.font_size() * 1.7;
        }
        let fill = data
            .series
            .cast::<BarSeries>()
            .and_then(|series| series.fill())
            .or_else(|| {
                data.series
                    .cast::<AreaSeries>()
                    .and_then(|series| series.fill())
            });
        let brush = fill
            .or_else(|| data.series.stroke())
            .unwrap_or_else(|| solid(SERIES_COLORS[data.index % SERIES_COLORS.len()]));
        rows.push((title, Some(brush), Point::new(x, y)));
        x += item_width;
    }
    rows
}

/// The distance between a point and its label: clear of the point's marker (radius 4 and
/// half its 1.5 outline).
const LABEL_GAP: f64 = MARKER_RADIUS + 1.5;
const MARKER_RADIUS: f64 = 4.0;

fn marker(draws: &mut Vec<Draw>, p: Point, shape: MarkerShape, brush: Rc<dyn IBrush>) {
    let r = MARKER_RADIUS;
    let outline = Some(pen(brush.clone(), 1.5, StrokeDashStyle::Solid));
    match shape {
        MarkerShape::None => {}
        MarkerShape::Circle => draws.push(Draw::Ellipse(
            Rect::new(p.x - r, p.y - r, 2.0 * r, 2.0 * r),
            brush,
            None,
        )),
        MarkerShape::Square => draws.push(Draw::Rectangle(
            Rect::new(p.x - r, p.y - r, 2.0 * r, 2.0 * r),
            Some(brush),
            None,
        )),
        MarkerShape::Diamond => draws.push(path(
            vec![
                Point::new(p.x, p.y - r),
                Point::new(p.x + r, p.y),
                Point::new(p.x, p.y + r),
                Point::new(p.x - r, p.y),
            ],
            true,
            Some(brush),
            None,
        )),
        MarkerShape::Triangle => draws.push(path(
            vec![
                Point::new(p.x, p.y - r),
                Point::new(p.x + r, p.y + r),
                Point::new(p.x - r, p.y + r),
            ],
            true,
            Some(brush),
            None,
        )),
        MarkerShape::ShortDash | MarkerShape::LongDash => {
            let r = if shape == MarkerShape::LongDash {
                7.0
            } else {
                3.0
            };
            draws.push(Draw::Line(
                Point::new(p.x - r, p.y),
                Point::new(p.x + r, p.y),
                outline.expect("marker pen"),
            ));
        }
        MarkerShape::Plus | MarkerShape::X | MarkerShape::Asterisk => {
            let pen = outline.expect("marker pen");
            if shape != MarkerShape::X {
                draws.push(Draw::Line(
                    Point::new(p.x - r, p.y),
                    Point::new(p.x + r, p.y),
                    pen.clone(),
                ));
                draws.push(Draw::Line(
                    Point::new(p.x, p.y - r),
                    Point::new(p.x, p.y + r),
                    pen.clone(),
                ));
            }
            if shape != MarkerShape::Plus {
                draws.push(Draw::Line(
                    Point::new(p.x - r, p.y - r),
                    Point::new(p.x + r, p.y + r),
                    pen.clone(),
                ));
                draws.push(Draw::Line(
                    Point::new(p.x - r, p.y + r),
                    Point::new(p.x + r, p.y - r),
                    pen,
                ));
            }
        }
    }
}

/// Min/max envelope reduction preserves extrema in each pixel column and source
/// traversal order. It is used only for monotone X line segments without a gap.
fn decimate(points: Vec<Point>, width: usize) -> Vec<Point> {
    if points.len() <= width.saturating_mul(4).max(16) || points.windows(2).any(|p| p[1].x < p[0].x)
    {
        return points;
    }
    let mut result = Vec::with_capacity(width.saturating_mul(4).min(points.len()));
    let mut start = 0;
    while start < points.len() {
        let bucket = points[start].x.floor();
        let mut end = start + 1;
        let mut min = start;
        let mut max = start;
        while end < points.len() && points[end].x.floor() == bucket {
            if points[end].y < points[min].y {
                min = end;
            }
            if points[end].y > points[max].y {
                max = end;
            }
            end += 1;
        }
        let mut indices = [start, min, max, end - 1];
        indices.sort_unstable();
        for (i, index) in indices.iter().enumerate() {
            if i == 0 || *index != indices[i - 1] {
                result.push(points[*index]);
            }
        }
        start = end;
    }
    result
}

/// Reduce each monotone run independently, keeping turns and the polygon's
/// winding. Unlike reducing the whole closed polygon as a line, this handles
/// the return along the baseline and preserves extrema at clipped boundaries.
fn decimate_polygon(points: Vec<Point>, width: usize) -> Vec<Point> {
    if points.len() <= width.saturating_mul(4).max(16) {
        return points;
    }
    let mut result = Vec::new();
    let mut start = 0;
    while start < points.len() {
        let mut direction = 0.0;
        let mut end = (start + 1).min(points.len());
        while end < points.len() {
            let step = points[end].x - points[end - 1].x;
            if step != 0.0 {
                if direction != 0.0 && step.signum() != direction {
                    break;
                }
                direction = step.signum();
            }
            end += 1;
        }
        let mut run = points[start..end].to_vec();
        if direction < 0.0 {
            run.reverse();
        }
        run = decimate(run, width);
        if direction < 0.0 {
            run.reverse();
        }
        result.extend(run);
        start = end;
    }
    result
}

#[cfg(test)]
#[path = "rendering_tests.rs"]
mod regression_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pixel_envelope_preserves_extrema_and_order() {
        let points: Vec<_> = (0..10000)
            .map(|i| {
                Point::new(
                    i as f64 / 100.0,
                    if i == 5001 { -1000.0 } else { (i % 13) as f64 },
                )
            })
            .collect();
        let reduced = decimate(points, 100);
        assert!(reduced.len() <= 400);
        assert!(reduced.iter().any(|p| p.y == -1000.0));
        assert!(reduced.windows(2).all(|p| p[0].x <= p[1].x));
    }
    #[test]
    fn category_layout_preserves_edge_and_band_positions() {
        assert_eq!(category_position(0, 3, false), 0.0);
        assert_eq!(category_position(2, 3, false), 1.0);
        assert_eq!(category_position(0, 2, true), 0.25);
        assert_eq!(category_position(1, 2, true), 0.75);
    }
}
