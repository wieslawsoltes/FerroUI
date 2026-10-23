//! Acyclic flow layout, relaxation, annular chords, and retained drawing.
// Ported from LiveChartsCore/SankeyLayout.cs and CoreSankeySeries.cs.
// Copyright (c) 2021 Alberto Rodriguez Orozco & LiveCharts Contributors. MIT.

use crate::scene::{
    palette_color, validate_size, ChartError, Geometry, HitRegion, HitShape, LegendItem,
    PaintStyle, PathCommand, PointId, Primitive, Scene, TextAnchor,
};
use ferroui_base::{media::Color, Point, Rect, Size};
use std::collections::{HashMap, VecDeque};

/// Stable identity replaces managed reference identity; duplicate IDs use the first node.
#[derive(Clone, Debug, PartialEq)]
pub struct SankeyNode {
    pub id: u64,
    pub name: String,
    pub color: Option<Color>,
}
impl SankeyNode {
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            color: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SankeyLink {
    pub source: u64,
    pub target: u64,
    pub weight: f64,
    pub color: Option<Color>,
}
impl SankeyLink {
    pub fn new(source: u64, target: u64, weight: f64) -> Self {
        Self {
            source,
            target,
            weight,
            color: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SankeyLayoutKind {
    #[default]
    Vertical,
    BipartiteArc,
}
ferroui_base::ferro_markup_enum!(SankeyLayoutKind {
    Vertical,
    BipartiteArc
});

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SankeyLabelPosition {
    #[default]
    Auto,
    Outside,
    Inside,
    Left,
    Right,
    Top,
    Bottom,
    Center,
}
ferroui_base::ferro_markup_enum!(SankeyLabelPosition {
    Auto,
    Outside,
    Inside,
    Left,
    Right,
    Top,
    Bottom,
    Center
});

#[derive(Clone, Debug, PartialEq)]
pub struct SankeyOptions {
    pub node_width: f64,
    pub node_padding: f64,
    pub node_corner_radius: f64,
    pub iterations: usize,
    pub layout: SankeyLayoutKind,
    pub arc_span_degrees: f64,
    pub label_margin: f64,
    pub show_labels: bool,
    pub label_size: f64,
    pub label_color: Color,
    pub label_position: SankeyLabelPosition,
    pub link_opacity: f64,
}
impl Default for SankeyOptions {
    fn default() -> Self {
        Self {
            node_width: 12.0,
            node_padding: 8.0,
            node_corner_radius: 0.0,
            iterations: 32,
            layout: SankeyLayoutKind::Vertical,
            arc_span_degrees: 150.0,
            label_margin: 0.0,
            show_labels: false,
            label_size: 14.0,
            label_color: Color::from_argb(255, 32, 32, 32),
            label_position: SankeyLabelPosition::Auto,
            link_opacity: 0.35,
        }
    }
}

/// Angles are clockwise degrees in the control's coordinate system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SankeyArc {
    pub center: Point,
    pub inner_radius: f64,
    pub outer_radius: f64,
    pub start_angle: f64,
    pub sweep_angle: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SankeyNodeGeometry {
    Rectangle(Rect),
    Arc(SankeyArc),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SankeyNodePlacement {
    /// Index of the first occurrence in the input node slice.
    pub node_index: usize,
    pub depth: usize,
    pub incoming_count: usize,
    pub outgoing_count: usize,
    pub value: f64,
    pub geometry: SankeyNodeGeometry,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SankeyRibbon {
    pub link_index: usize,
    pub source_index: usize,
    pub target_index: usize,
    pub weight: f64,
    pub source_start: Point,
    pub source_end: Point,
    pub target_start: Point,
    pub target_end: Point,
    /// Chords curve through this center; vertical flows use horizontal midpoints.
    pub center: Option<Point>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SankeyLayout {
    pub nodes: Vec<SankeyNodePlacement>,
    pub ribbons: Vec<SankeyRibbon>,
}

#[derive(Default)]
struct State {
    input: usize,
    depth: usize,
    value: f64,
    x: f64,
    y: f64,
    height: f64,
    incoming: Vec<usize>,
    outgoing: Vec<usize>,
}
struct Edge {
    input: usize,
    source: usize,
    target: usize,
    weight: f64,
}

/// Stateless calculation can run without a window, renderer, timer, or executor.
pub struct SankeyChartEngine;
impl SankeyChartEngine {
    pub fn layout(
        nodes: &[SankeyNode],
        links: &[SankeyLink],
        rect: Rect,
        options: &SankeyOptions,
    ) -> Result<SankeyLayout, ChartError> {
        validate_rect(rect)?;
        validate_options(options)?;
        if nodes.is_empty() || rect.width == 0.0 || rect.height == 0.0 {
            return Ok(SankeyLayout::default());
        }
        let (mut states, edges) = build_graph(nodes, links)?;
        let max_depth = states.iter().map(|s| s.depth).max().unwrap_or(0);
        if options.layout == SankeyLayoutKind::BipartiteArc {
            if max_depth > 1 {
                return Err(ChartError::InvalidData("BipartiteArc requires pure sources and pure sinks; use Vertical for flows with more than two columns".into()));
            }
            arc_layout(&mut states, &edges, rect, options)
        } else {
            vertical_layout(&mut states, &edges, max_depth, rect, options)
        }
    }

    pub fn measure(
        nodes: &[SankeyNode],
        links: &[SankeyLink],
        size: Size,
        options: &SankeyOptions,
    ) -> Result<Scene, ChartError> {
        Self::measure_with_labels(nodes, links, size, options, |text, size| {
            Size::new(text.chars().count() as f64 * size * 0.6, size * 1.2)
        })
    }

    /// Hosts supply native font metrics to reserve space for labels and arc margins.
    pub fn measure_with_labels(
        nodes: &[SankeyNode],
        links: &[SankeyLink],
        size: Size,
        options: &SankeyOptions,
        mut measure_text: impl FnMut(&str, f64) -> Size,
    ) -> Result<Scene, ChartError> {
        validate_size(size)?;
        let bounds = Rect::new(0.0, 0.0, size.width, size.height);
        let mut plot_bounds = bounds;
        let mut layout_options = options.clone();
        let mut widest: f64 = 0.0;
        let mut label_sizes = Vec::new();
        if options.show_labels {
            for node in nodes {
                let measured = measure_text(&node.name, options.label_size);
                if !measured.width.is_finite()
                    || !measured.height.is_finite()
                    || measured.width < 0.0
                    || measured.height < 0.0
                {
                    return Err(ChartError::InvalidData(
                        "Sankey label measurement must be finite and nonnegative".into(),
                    ));
                }
                widest = widest.max(measured.width);
                label_sizes.push(measured);
            }
            if options.layout == SankeyLayoutKind::BipartiteArc {
                layout_options.label_margin = layout_options.label_margin.max(widest + 4.0);
            } else if matches!(
                options.label_position,
                SankeyLabelPosition::Auto
                    | SankeyLabelPosition::Outside
                    | SankeyLabelPosition::Left
                    | SankeyLabelPosition::Right
            ) {
                let margin = (widest + 4.0).min(size.width * 0.45);
                plot_bounds = Rect::new(
                    margin,
                    0.0,
                    (size.width - 2.0 * margin).max(0.0),
                    size.height,
                );
            }
        }
        let layout = Self::layout(nodes, links, plot_bounds, &layout_options)?;
        let mut scene = Scene::new(size, plot_bounds);
        // Ribbons are behind nodes, and each input link retains its original hit ID.
        for ribbon in &layout.ribbons {
            let source = &nodes[ribbon.source_index];
            let target = &nodes[ribbon.target_index];
            let color = links[ribbon.link_index]
                .color
                .or(source.color)
                .unwrap_or_else(|| palette_color(ribbon.source_index));
            let mut style = PaintStyle::fill(color);
            style.opacity = options.link_opacity;
            let id = PointId {
                series: 1,
                point: ribbon.link_index,
            };
            scene.primitives.push(
                Primitive::new(Geometry::Path(ribbon_path(ribbon)), style)
                    .clipped(bounds)
                    .identified(id),
            );
            let polygon = ribbon_polygon(ribbon);
            let center = cubic_point(
                ribbon.source_start,
                ribbon_controls(ribbon).0,
                ribbon_controls(ribbon).1,
                ribbon.target_start,
                0.5,
            );
            scene.hits.push(HitRegion {
                id,
                shape: HitShape::Polygon(polygon),
                center,
                value: ribbon.weight,
                label: format!("{} → {}: {}", source.name, target.name, ribbon.weight),
            });
        }
        for placement in &layout.nodes {
            let node = &nodes[placement.node_index];
            let color = node
                .color
                .unwrap_or_else(|| palette_color(placement.node_index));
            let id = PointId {
                series: 0,
                point: placement.node_index,
            };
            let (geometry, hit, center) = match placement.geometry {
                SankeyNodeGeometry::Rectangle(rect) => (
                    Geometry::Rectangle {
                        rect,
                        radius: options.node_corner_radius,
                    },
                    HitShape::Rectangle(rect),
                    Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5),
                ),
                SankeyNodeGeometry::Arc(arc) => (
                    Geometry::Path(annular_path(arc)),
                    HitShape::AnnularSector {
                        center: arc.center,
                        inner_radius: arc.inner_radius,
                        outer_radius: arc.outer_radius,
                        start_angle: arc.start_angle,
                        sweep_angle: arc.sweep_angle,
                    },
                    polar(
                        arc.center,
                        (arc.inner_radius + arc.outer_radius) * 0.5,
                        arc.start_angle + arc.sweep_angle * 0.5,
                    ),
                ),
            };
            scene.primitives.push(
                Primitive::new(geometry, PaintStyle::fill(color))
                    .clipped(bounds)
                    .identified(id),
            );
            scene.hits.push(HitRegion {
                id,
                shape: hit,
                center,
                value: placement.value,
                label: format!("{}: {}", node.name, placement.value),
            });
            scene.legend.push(LegendItem {
                series: placement.node_index,
                name: node.name.clone(),
                color,
            });
            if options.show_labels && !node.name.is_empty() {
                let (mut origin, anchor, rotation) =
                    label_placement(placement, center, plot_bounds, options);
                origin.y -= label_sizes[placement.node_index].height * 0.5;
                scene.primitives.push(
                    Primitive::new(
                        Geometry::Text {
                            origin,
                            text: node.name.clone(),
                            size: options.label_size,
                            rotation,
                            anchor,
                        },
                        PaintStyle::fill(options.label_color),
                    )
                    .clipped(bounds),
                );
            }
        }
        Ok(scene)
    }
}

fn validate_rect(rect: Rect) -> Result<(), ChartError> {
    validate_size(Size::new(rect.width, rect.height))?;
    if !rect.x.is_finite()
        || !rect.y.is_finite()
        || !(rect.x + rect.width).is_finite()
        || !(rect.y + rect.height).is_finite()
    {
        return Err(ChartError::InvalidSize);
    }
    Ok(())
}
fn validate_options(options: &SankeyOptions) -> Result<(), ChartError> {
    if [
        options.node_width,
        options.node_padding,
        options.node_corner_radius,
        options.label_margin,
        options.label_size,
    ]
    .iter()
    .any(|x| !x.is_finite() || *x < 0.0)
        || !options.arc_span_degrees.is_finite()
        || !(0.0..=360.0).contains(&options.arc_span_degrees)
        || options.arc_span_degrees == 0.0
        || !options.link_opacity.is_finite()
        || !(0.0..=1.0).contains(&options.link_opacity)
    {
        return Err(ChartError::InvalidData("Sankey sizes must be finite and nonnegative; arc span must be in (0, 360] and opacity in [0, 1]".into()));
    }
    Ok(())
}

fn build_graph(
    nodes: &[SankeyNode],
    links: &[SankeyLink],
) -> Result<(Vec<State>, Vec<Edge>), ChartError> {
    let mut lookup = HashMap::new();
    let mut states = Vec::<State>::new();
    for (input, node) in nodes.iter().enumerate() {
        if let std::collections::hash_map::Entry::Vacant(entry) = lookup.entry(node.id) {
            entry.insert(states.len());
            states.push(State {
                input,
                ..State::default()
            });
        }
    }
    let mut edges = Vec::new();
    for (input, link) in links.iter().enumerate() {
        if link.source == link.target || !link.weight.is_finite() || link.weight <= 0.0 {
            continue;
        }
        let (Some(&source), Some(&target)) = (lookup.get(&link.source), lookup.get(&link.target))
        else {
            continue;
        };
        let edge = edges.len();
        states[source].outgoing.push(edge);
        states[target].incoming.push(edge);
        edges.push(Edge {
            input,
            source,
            target,
            weight: link.weight,
        });
    }
    let mut degrees: Vec<_> = states.iter().map(|s| s.incoming.len()).collect();
    let mut queue: VecDeque<_> = degrees
        .iter()
        .enumerate()
        .filter_map(|(i, d)| (*d == 0).then_some(i))
        .collect();
    let mut processed = 0;
    while let Some(index) = queue.pop_front() {
        processed += 1;
        for offset in 0..states[index].outgoing.len() {
            let edge = &edges[states[index].outgoing[offset]];
            states[edge.target].depth = states[edge.target].depth.max(states[index].depth + 1);
            degrees[edge.target] -= 1;
            if degrees[edge.target] == 0 {
                queue.push_back(edge.target);
            }
        }
    }
    if processed != states.len() {
        return Err(ChartError::InvalidData(
            "Sankey graph contains a cycle; only acyclic graphs are supported".into(),
        ));
    }
    for state in &mut states {
        let incoming: f64 = state.incoming.iter().map(|i| edges[*i].weight).sum();
        let outgoing: f64 = state.outgoing.iter().map(|i| edges[*i].weight).sum();
        state.value = incoming.max(outgoing);
        if !state.value.is_finite() {
            return Err(ChartError::InvalidData(
                "Sankey flow sum exceeds the finite numeric range".into(),
            ));
        }
        if state.value <= 0.0 {
            state.value = 1.0;
        }
    }
    Ok((states, edges))
}

fn vertical_layout(
    states: &mut [State],
    edges: &[Edge],
    max_depth: usize,
    rect: Rect,
    options: &SankeyOptions,
) -> Result<SankeyLayout, ChartError> {
    let mut columns = vec![Vec::new(); max_depth + 1];
    for (i, state) in states.iter().enumerate() {
        columns[state.depth].push(i);
    }
    let dx = if max_depth > 0 {
        ((rect.width - options.node_width) / max_depth as f64).max(0.0)
    } else {
        0.0
    };
    let mut max_column_sum: f64 = 0.0;
    let mut max_column_nodes = 0;
    for column in &columns {
        let sum: f64 = column.iter().map(|i| states[*i].value).sum();
        max_column_sum = max_column_sum.max(sum);
        max_column_nodes = max_column_nodes.max(column.len());
    }
    if !max_column_sum.is_finite() {
        return Err(ChartError::InvalidData(
            "Sankey column total exceeds the finite numeric range".into(),
        ));
    }
    let available_height =
        (rect.height - options.node_padding * max_column_nodes.saturating_sub(1) as f64).max(1.0);
    let scale = available_height / max_column_sum;
    for column in &columns {
        let mut y = rect.y;
        for &index in column {
            let state = &mut states[index];
            state.x = rect.x + state.depth as f64 * dx;
            state.height = state.value * scale;
            state.y = y;
            y += state.height + options.node_padding;
        }
    }
    let mut alpha = 1.0;
    for _ in 0..options.iterations {
        alpha *= 0.99;
        for column in columns.iter().rev().skip(1) {
            for &index in column {
                relax(states, edges, index, false, alpha);
            }
        }
        resolve_collisions(states, &mut columns, rect, options.node_padding);
        for column in columns.iter().skip(1) {
            for &index in column {
                relax(states, edges, index, true, alpha);
            }
        }
        resolve_collisions(states, &mut columns, rect, options.node_padding);
    }
    let mut result = SankeyLayout::default();
    for state in states.iter() {
        result.nodes.push(SankeyNodePlacement {
            node_index: state.input,
            depth: state.depth,
            incoming_count: state.incoming.len(),
            outgoing_count: state.outgoing.len(),
            value: state.value,
            geometry: SankeyNodeGeometry::Rectangle(Rect::new(
                state.x,
                state.y,
                options.node_width,
                state.height,
            )),
        });
    }
    let mut outgoing: Vec<_> = states.iter().map(|s| s.y).collect();
    let mut incoming = outgoing.clone();
    // Keep the upstream source iteration order and sort source bands by target Y.
    for index in 0..states.len() {
        let mut ordered = states[index].outgoing.clone();
        ordered.sort_by(|a, b| {
            states[edges[*a].target]
                .y
                .total_cmp(&states[edges[*b].target].y)
        });
        for edge_index in ordered {
            let edge = &edges[edge_index];
            let thickness = edge.weight * scale;
            let source_start = Point::new(states[index].x + options.node_width, outgoing[index]);
            let target_start = Point::new(states[edge.target].x, incoming[edge.target]);
            outgoing[index] += thickness;
            incoming[edge.target] += thickness;
            result.ribbons.push(SankeyRibbon {
                link_index: edge.input,
                source_index: states[index].input,
                target_index: states[edge.target].input,
                weight: edge.weight,
                source_start,
                source_end: Point::new(source_start.x, outgoing[index]),
                target_start,
                target_end: Point::new(target_start.x, incoming[edge.target]),
                center: None,
            });
        }
    }
    Ok(result)
}

fn relax(states: &mut [State], edges: &[Edge], index: usize, to_sources: bool, alpha: f64) {
    let state = &states[index];
    let links = if to_sources {
        &state.incoming
    } else {
        &state.outgoing
    };
    if links.is_empty() {
        return;
    }
    let sum: f64 = links.iter().map(|i| edges[*i].weight).sum();
    // Divide weights before multiplying coordinates to avoid intermediate overflow.
    let center: f64 = links
        .iter()
        .map(|i| {
            let edge = &edges[*i];
            let other = &states[if to_sources { edge.source } else { edge.target }];
            (other.y + other.height * 0.5) * (edge.weight / sum)
        })
        .sum();
    let state = &mut states[index];
    state.y += (center - (state.y + state.height * 0.5)) * alpha;
}

fn resolve_collisions(states: &mut [State], columns: &mut [Vec<usize>], rect: Rect, padding: f64) {
    for column in columns {
        column.sort_by(|a, b| states[*a].y.total_cmp(&states[*b].y));
        let mut min_y = rect.y;
        for &index in column.iter() {
            let state = &mut states[index];
            state.y = state.y.max(min_y);
            min_y = state.y + state.height + padding;
        }
        let mut max_y = rect.y + rect.height;
        for &index in column.iter().rev() {
            let state = &mut states[index];
            state.y = state.y.min(max_y - state.height);
            max_y = state.y - padding;
        }
    }
}

#[derive(Clone, Copy, Default)]
struct AngularSpan {
    top: f64,
    sweep: f64,
    direction: f64,
}

fn arc_layout(
    states: &mut [State],
    edges: &[Edge],
    rect: Rect,
    options: &SankeyOptions,
) -> Result<SankeyLayout, ChartError> {
    let center = Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
    let outer_radius =
        (rect.width.min(rect.height) * 0.5 - options.label_margin).max(options.node_width + 1.0);
    let inner_radius = (outer_radius - options.node_width).max(1.0);
    let angular_padding = (options.node_padding / inner_radius).to_degrees();
    let mut spans = vec![AngularSpan::default(); states.len()];
    let mut placements = vec![None; states.len()];
    for side in 0..=1 {
        let column: Vec<_> = states
            .iter()
            .enumerate()
            .filter_map(|(i, s)| (s.depth == side).then_some(i))
            .collect();
        if column.is_empty() {
            continue;
        }
        let total: f64 = column.iter().map(|i| states[*i].value).sum();
        if !total.is_finite() {
            return Err(ChartError::InvalidData(
                "Sankey column total exceeds the finite numeric range".into(),
            ));
        }
        let usable = (options.arc_span_degrees
            - angular_padding * column.len().saturating_sub(1) as f64)
            .max(1.0);
        let direction = if side == 0 { -1.0 } else { 1.0 };
        let top = if side == 0 {
            180.0 + options.arc_span_degrees * 0.5
        } else {
            -options.arc_span_degrees * 0.5
        };
        let mut accumulated = 0.0;
        for index in column {
            let sweep = (states[index].value / total) * usable;
            let node_top = top + direction * accumulated;
            spans[index] = AngularSpan {
                top: node_top,
                sweep,
                direction,
            };
            let arc = SankeyArc {
                center,
                inner_radius,
                outer_radius,
                start_angle: if direction > 0.0 {
                    node_top
                } else {
                    node_top - sweep
                },
                sweep_angle: sweep,
            };
            placements[index] = Some(SankeyNodePlacement {
                node_index: states[index].input,
                depth: side,
                incoming_count: states[index].incoming.len(),
                outgoing_count: states[index].outgoing.len(),
                value: states[index].value,
                geometry: SankeyNodeGeometry::Arc(arc),
            });
            accumulated += sweep + angular_padding;
        }
    }
    let mut result = SankeyLayout {
        nodes: placements.into_iter().flatten().collect(),
        ribbons: Vec::with_capacity(edges.len()),
    };
    let mut outgoing = vec![0.0; states.len()];
    let mut incoming = vec![0.0; states.len()];
    for index in 0..states.len() {
        let mut ordered = states[index].outgoing.clone();
        ordered.sort_by(|a, b| {
            spans[edges[*a].target]
                .top
                .to_radians()
                .sin()
                .total_cmp(&spans[edges[*b].target].top.to_radians().sin())
        });
        for edge_index in ordered {
            let edge = &edges[edge_index];
            let source = spans[index];
            let target = spans[edge.target];
            let source_sweep = source.sweep * (edge.weight / states[index].value);
            let target_sweep = target.sweep * (edge.weight / states[edge.target].value);
            let source_start = source.top + source.direction * outgoing[index];
            let target_start = target.top + target.direction * incoming[edge.target];
            result.ribbons.push(SankeyRibbon {
                link_index: edge.input,
                source_index: states[index].input,
                target_index: states[edge.target].input,
                weight: edge.weight,
                source_start: polar(center, inner_radius, source_start),
                source_end: polar(
                    center,
                    inner_radius,
                    source_start + source.direction * source_sweep,
                ),
                target_start: polar(center, inner_radius, target_start),
                target_end: polar(
                    center,
                    inner_radius,
                    target_start + target.direction * target_sweep,
                ),
                center: Some(center),
            });
            outgoing[index] += source_sweep;
            incoming[edge.target] += target_sweep;
        }
    }
    Ok(result)
}

fn polar(center: Point, radius: f64, angle: f64) -> Point {
    let (sin, cos) = angle.to_radians().sin_cos();
    Point::new(center.x + cos * radius, center.y + sin * radius)
}

fn ribbon_controls(r: &SankeyRibbon) -> (Point, Point, Point, Point) {
    if let Some(center) = r.center {
        (center, center, center, center)
    } else {
        let mid = (r.source_start.x + r.target_start.x) * 0.5;
        (
            Point::new(mid, r.source_start.y),
            Point::new(mid, r.target_start.y),
            Point::new(mid, r.target_end.y),
            Point::new(mid, r.source_end.y),
        )
    }
}
fn ribbon_path(r: &SankeyRibbon) -> Vec<PathCommand> {
    let (a, b, c, d) = ribbon_controls(r);
    vec![
        PathCommand::MoveTo(r.source_start),
        PathCommand::CubicTo {
            control1: a,
            control2: b,
            end: r.target_start,
        },
        PathCommand::LineTo(r.target_end),
        PathCommand::CubicTo {
            control1: c,
            control2: d,
            end: r.source_end,
        },
        PathCommand::Close,
    ]
}
fn cubic_point(a: Point, b: Point, c: Point, d: Point, t: f64) -> Point {
    let u = 1.0 - t;
    Point::new(
        u * u * u * a.x + 3.0 * u * u * t * b.x + 3.0 * u * t * t * c.x + t * t * t * d.x,
        u * u * u * a.y + 3.0 * u * u * t * b.y + 3.0 * u * t * t * c.y + t * t * t * d.y,
    )
}
fn ribbon_polygon(r: &SankeyRibbon) -> Vec<Point> {
    let (a, b, c, d) = ribbon_controls(r);
    let mut result = Vec::with_capacity(66);
    for step in 0..=32 {
        result.push(cubic_point(
            r.source_start,
            a,
            b,
            r.target_start,
            step as f64 / 32.0,
        ));
    }
    for step in 0..=32 {
        result.push(cubic_point(
            r.target_end,
            c,
            d,
            r.source_end,
            step as f64 / 32.0,
        ));
    }
    result
}

fn annular_path(arc: SankeyArc) -> Vec<PathCommand> {
    let mut result = vec![PathCommand::MoveTo(polar(
        arc.center,
        arc.outer_radius,
        arc.start_angle,
    ))];
    append_arc(
        &mut result,
        arc.center,
        arc.outer_radius,
        arc.start_angle,
        arc.sweep_angle,
    );
    result.push(PathCommand::LineTo(polar(
        arc.center,
        arc.inner_radius,
        arc.start_angle + arc.sweep_angle,
    )));
    append_arc(
        &mut result,
        arc.center,
        arc.inner_radius,
        arc.start_angle + arc.sweep_angle,
        -arc.sweep_angle,
    );
    result.push(PathCommand::Close);
    result
}
fn append_arc(commands: &mut Vec<PathCommand>, center: Point, radius: f64, start: f64, sweep: f64) {
    let steps = (sweep.abs() / 90.0).ceil().max(1.0) as usize;
    let step = sweep.to_radians() / steps as f64;
    for index in 0..steps {
        let a = start.to_radians() + index as f64 * step;
        let b = a + step;
        let k = 4.0 / 3.0 * (step / 4.0).tan();
        commands.push(PathCommand::CubicTo {
            control1: Point::new(
                center.x + radius * (a.cos() - k * a.sin()),
                center.y + radius * (a.sin() + k * a.cos()),
            ),
            control2: Point::new(
                center.x + radius * (b.cos() + k * b.sin()),
                center.y + radius * (b.sin() - k * b.cos()),
            ),
            end: Point::new(center.x + radius * b.cos(), center.y + radius * b.sin()),
        });
    }
}
fn label_placement(
    placement: &SankeyNodePlacement,
    center: Point,
    bounds: Rect,
    options: &SankeyOptions,
) -> (Point, TextAnchor, f64) {
    if let SankeyNodeGeometry::Arc(arc) = placement.geometry {
        let angle = (arc.start_angle + arc.sweep_angle * 0.5).rem_euclid(360.0);
        let flipped = angle > 90.0 && angle < 270.0;
        return (
            polar(arc.center, arc.outer_radius + 4.0, angle),
            if flipped {
                TextAnchor::End
            } else {
                TextAnchor::Start
            },
            if flipped { angle - 180.0 } else { angle },
        );
    }
    let position = match options.label_position {
        SankeyLabelPosition::Auto => {
            if placement.incoming_count == 0 && placement.outgoing_count > 0 {
                SankeyLabelPosition::Left
            } else if placement.outgoing_count == 0 && placement.incoming_count > 0 {
                SankeyLabelPosition::Right
            } else {
                SankeyLabelPosition::Center
            }
        }
        SankeyLabelPosition::Outside => {
            if center.x < bounds.x + bounds.width * 0.5 {
                SankeyLabelPosition::Left
            } else {
                SankeyLabelPosition::Right
            }
        }
        position => position,
    };
    let extent = match placement.geometry {
        SankeyNodeGeometry::Rectangle(r) => r.width * 0.5,
        SankeyNodeGeometry::Arc(a) => (a.outer_radius - a.inner_radius) * 0.5,
    };
    let (origin, anchor) = match position {
        SankeyLabelPosition::Left => (
            Point::new(center.x - extent - 4.0, center.y),
            TextAnchor::End,
        ),
        SankeyLabelPosition::Right => (
            Point::new(center.x + extent + 4.0, center.y),
            TextAnchor::Start,
        ),
        SankeyLabelPosition::Top => (
            Point::new(center.x, center.y - options.label_size),
            TextAnchor::Center,
        ),
        SankeyLabelPosition::Bottom => (
            Point::new(center.x, center.y + options.label_size),
            TextAnchor::Center,
        ),
        _ => (center, TextAnchor::Center),
    };
    (origin, anchor, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn nodes(count: usize) -> Vec<SankeyNode> {
        (0..count)
            .map(|i| SankeyNode::new(i as u64, format!("N{i}")))
            .collect()
    }
    fn box_of(layout: &SankeyLayout, index: usize) -> Rect {
        match layout
            .nodes
            .iter()
            .find(|n| n.node_index == index)
            .unwrap()
            .geometry
        {
            SankeyNodeGeometry::Rectangle(rect) => rect,
            _ => panic!("expected rectangle"),
        }
    }
    #[test]
    fn two_level_flow_preserves_node_values_and_ribbon_thickness() {
        // Fixture: SnapshotTests/SankeySeriesTests.TwoLevelFlow; assert its layout contract.
        let links = vec![
            SankeyLink::new(0, 3, 8.0),
            SankeyLink::new(0, 4, 4.0),
            SankeyLink::new(1, 3, 6.0),
            SankeyLink::new(1, 4, 2.0),
            SankeyLink::new(2, 4, 10.0),
        ];
        let layout = SankeyChartEngine::layout(
            &nodes(5),
            &links,
            Rect::new(20.0, 30.0, 700.0, 400.0),
            &SankeyOptions::default(),
        )
        .unwrap();
        assert_eq!(
            layout.nodes.iter().map(|n| n.value).collect::<Vec<_>>(),
            vec![12.0, 8.0, 10.0, 14.0, 16.0]
        );
        for ribbon in &layout.ribbons {
            assert!(
                (ribbon.source_end.y
                    - ribbon.source_start.y
                    - (ribbon.target_end.y - ribbon.target_start.y))
                    .abs()
                    < 1e-10
            );
            let source = box_of(&layout, ribbon.source_index);
            assert!(
                ribbon.source_start.y >= source.y - 1e-9
                    && ribbon.source_end.y <= source.y + source.height + 1e-9
            );
        }
        for depth in 0..2 {
            let mut rectangles: Vec<_> = layout
                .nodes
                .iter()
                .filter(|n| n.depth == depth)
                .map(|n| box_of(&layout, n.node_index))
                .collect();
            rectangles.sort_by(|a, b| a.y.total_cmp(&b.y));
            assert!(rectangles.first().unwrap().y >= 30.0 - 1e-9);
            assert!(
                rectangles.last().unwrap().y + rectangles.last().unwrap().height <= 430.0 + 1e-9
            );
            for pair in rectangles.windows(2) {
                assert!(pair[1].y - pair[0].y - pair[0].height >= 8.0 - 1e-9);
            }
        }
    }
    #[test]
    fn longest_path_assignment_and_isolated_nodes() {
        let links = [
            SankeyLink::new(0, 1, 4.0),
            SankeyLink::new(1, 2, 3.0),
            SankeyLink::new(0, 2, 1.0),
        ];
        let layout = SankeyChartEngine::layout(
            &nodes(4),
            &links,
            Rect::new(0.0, 0.0, 300.0, 200.0),
            &SankeyOptions::default(),
        )
        .unwrap();
        assert_eq!(
            layout.nodes.iter().map(|n| n.depth).collect::<Vec<_>>(),
            vec![0, 1, 2, 0]
        );
        assert_eq!(layout.nodes[3].value, 1.0);
        assert_eq!(layout.nodes[2].value, 4.0);
    }
    #[test]
    fn cycles_are_errors_and_invalid_links_are_ignored() {
        let mut input = nodes(2);
        input.push(SankeyNode::new(0, "duplicate"));
        let links = [
            SankeyLink::new(0, 0, 5.0),
            SankeyLink::new(0, 9, 5.0),
            SankeyLink::new(0, 1, f64::NAN),
            SankeyLink::new(0, 1, -2.0),
            SankeyLink::new(0, 1, 3.0),
        ];
        let rect = Rect::new(0.0, 0.0, 300.0, 200.0);
        let layout =
            SankeyChartEngine::layout(&input, &links, rect, &SankeyOptions::default()).unwrap();
        assert_eq!((layout.nodes.len(), layout.ribbons.len()), (2, 1));
        assert_eq!(layout.ribbons[0].link_index, 4);
        let cycle = [SankeyLink::new(0, 1, 1.0), SankeyLink::new(1, 0, 1.0)];
        assert!(
            SankeyChartEngine::layout(&nodes(2), &cycle, rect, &SankeyOptions::default())
                .unwrap_err()
                .to_string()
                .contains("cycle")
        );
    }
    #[test]
    fn bipartite_arc_throws_on_multicolumn() {
        // Direct behavioral port of SnapshotTests/SankeySeriesTests.BipartiteArc_ThrowsOnMultiColumn.
        let links = [SankeyLink::new(0, 1, 5.0), SankeyLink::new(1, 2, 5.0)];
        let options = SankeyOptions {
            layout: SankeyLayoutKind::BipartiteArc,
            ..SankeyOptions::default()
        };
        assert!(SankeyChartEngine::layout(
            &nodes(3),
            &links,
            Rect::new(0.0, 0.0, 500.0, 500.0),
            &options
        )
        .unwrap_err()
        .to_string()
        .contains("BipartiteArc"));
    }
    #[test]
    fn bipartite_arc_bands_follow_proportional_angular_spans() {
        // Layout assertions for SnapshotTests/SankeySeriesTests.BipartiteArc fixture.
        let links = [
            SankeyLink::new(0, 3, 12.0),
            SankeyLink::new(0, 6, 6.0),
            SankeyLink::new(1, 4, 14.0),
            SankeyLink::new(1, 5, 8.0),
            SankeyLink::new(2, 5, 4.0),
            SankeyLink::new(2, 7, 10.0),
        ];
        let options = SankeyOptions {
            layout: SankeyLayoutKind::BipartiteArc,
            ..SankeyOptions::default()
        };
        let layout = SankeyChartEngine::layout(
            &nodes(8),
            &links,
            Rect::new(0.0, 0.0, 600.0, 500.0),
            &options,
        )
        .unwrap();
        assert_eq!(layout.ribbons.len(), 6);
        for ribbon in &layout.ribbons {
            for point in [
                ribbon.source_start,
                ribbon.source_end,
                ribbon.target_start,
                ribbon.target_end,
            ] {
                assert!(((point.x - 300.0).hypot(point.y - 250.0) - 238.0).abs() < 1e-9);
            }
        }
        let sums: Vec<_> = (0..2)
            .map(|depth| {
                layout
                    .nodes
                    .iter()
                    .filter(|n| n.depth == depth)
                    .map(|n| match n.geometry {
                        SankeyNodeGeometry::Arc(a) => a.sweep_angle,
                        _ => 0.0,
                    })
                    .sum::<f64>()
            })
            .collect();
        let pad = (8.0_f64 / 238.0).to_degrees();
        assert!((sums[0] + 2.0 * pad - 150.0).abs() < 1e-9);
        assert!((sums[1] + 4.0 * pad - 150.0).abs() < 1e-9);
    }
    #[test]
    fn scene_has_node_and_ribbon_hit_regions_and_opt_in_labels() {
        let options = SankeyOptions {
            show_labels: true,
            ..SankeyOptions::default()
        };
        let scene = SankeyChartEngine::measure(
            &nodes(2),
            &[SankeyLink::new(0, 1, 5.0)],
            Size::new(400.0, 250.0),
            &options,
        )
        .unwrap();
        assert_eq!(scene.hits.len(), 3);
        assert_eq!(
            scene
                .primitives
                .iter()
                .filter(|p| matches!(p.geometry, Geometry::Text { .. }))
                .count(),
            2
        );
        assert!(scene
            .hits
            .iter()
            .any(|h| h.id.series == 1 && h.contains(Point::new(200.0, 125.0))));
    }

    #[test]
    fn auto_labels_follow_source_sink_and_intermediate_node_roles() {
        let options = SankeyOptions {
            show_labels: true,
            ..SankeyOptions::default()
        };
        let links = [SankeyLink::new(0, 1, 5.0), SankeyLink::new(1, 2, 5.0)];
        let scene = SankeyChartEngine::measure_with_labels(
            &nodes(3),
            &links,
            Size::new(600.0, 300.0),
            &options,
            |_, _| Size::new(30.0, 16.0),
        )
        .unwrap();
        let labels: Vec<_> = scene
            .primitives
            .iter()
            .filter_map(|p| match &p.geometry {
                Geometry::Text { anchor, .. } => Some(*anchor),
                _ => None,
            })
            .collect();
        assert_eq!(
            labels,
            vec![TextAnchor::End, TextAnchor::Center, TextAnchor::Start]
        );
        assert_eq!(scene.plot_bounds.x, 34.0);
        let options = SankeyOptions {
            label_position: SankeyLabelPosition::Inside,
            ..options
        };
        let scene =
            SankeyChartEngine::measure(&nodes(3), &links, Size::new(600.0, 300.0), &options)
                .unwrap();
        assert!(scene
            .primitives
            .iter()
            .filter_map(|p| match &p.geometry {
                Geometry::Text { anchor, .. } => Some(*anchor),
                _ => None,
            })
            .all(|a| a == TextAnchor::Center));
    }
}
