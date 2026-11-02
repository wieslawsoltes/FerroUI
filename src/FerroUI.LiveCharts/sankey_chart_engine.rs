//! Sankey measurement adapted from the pinned `SankeyLayout`, `SankeyChartEngine` and
//! `CoreSankeySeries`, with the shapes of the SkiaSharp `SankeyRibbonGeometry`,
//! `SankeyChordRibbonGeometry` and `SankeyArcSegmentGeometry`: the d3-sankey layout (columns by
//! the longest path from the sources, the sweep relaxation of the node positions, the link bands
//! stacked in the order of their other end), the bipartite arc layout with chord ribbons, the node
//! labels and the room they reserve, and the hover areas of the nodes. Pure calculation produces a
//! retained scene that the native controls draw like the other chart scenes.
//!
//! The source identifies nodes and links by reference; here a node is identified by its `id`, and
//! a node whose id repeats an earlier one is skipped, as the source skips a repeated reference.

use crate::pie_chart_engine::{annular_sector_path, append_arc, rounded_annular_sector_path};
use crate::scene::*;
use crate::series::{Series, SeriesKind, DATA_LABELS_COLOR, DATA_LABELS_PADDING};
use crate::treemap_chart_engine::{
    fill_color, fill_style, stroke_style, text_size, tooltip_text, validate_series,
};
use ferroui_base::{media::Color, Point, Rect, Size};
use std::collections::{HashMap, VecDeque};

/// A node of a Sankey diagram (`Defaults/SankeyNode`): its label and, in place of the source's
/// `NodeColorMapper`, an optional colour of its own. Its value is computed by the layout.
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

/// A directed weighted edge (`Defaults/SankeyLink`) between the nodes of the given ids and, in
/// place of the source's `LinkColorMapper`, an optional colour of its own.
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

/// The placement of the node labels (`SankeyLabelPosition`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SankeyLabelPosition {
    /// Sources (no incoming link) outside on the left, sinks (no outgoing link) outside on the
    /// right, the nodes in between over the node.
    #[default]
    Auto,
    /// Always outside, on the side of the node away from the centre of the chart.
    Outside,
    /// Always over the node, centred.
    Inside,
}
ferroui_base::ferro_markup_enum!(SankeyLabelPosition {
    Auto,
    Outside,
    Inside
});

/// How a Sankey series arranges its nodes (`SankeyLayoutKind`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SankeyLayoutKind {
    /// Columns of stacked rectangles from left to right.
    #[default]
    Vertical,
    /// Sources on a left arc, sinks on a right arc, chord ribbons through the centre. The data
    /// must be bipartite: a node that is both a target and a source is an error.
    BipartiteArc,
}
ferroui_base::ferro_markup_enum!(SankeyLayoutKind {
    Vertical,
    BipartiteArc
});

/// A Sankey series: the name, visibility, paints and data labels of its [`Series`], its nodes
/// (the source's `Values`) and links, and the layout settings of `ISankeySeries` with the
/// source's defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct SankeySeries {
    pub series: Series,
    pub nodes: Vec<SankeyNode>,
    pub links: Vec<SankeyLink>,
    /// The width of a node rectangle, or the radial thickness of an arc node (12).
    pub node_width: f64,
    /// The gap between the nodes of a column (8).
    pub node_padding: f64,
    pub node_corner_radius: f64,
    /// The relaxation sweeps of the vertical layout (32).
    pub layout_iterations: u32,
    pub node_label_position: SankeyLabelPosition,
    pub layout: SankeyLayoutKind,
    /// The angle each arc covers in the bipartite arc layout (150).
    pub arc_span_degrees: f64,
    /// The fill of the ribbons. `None` takes the theme's, the series colour at alpha 90; a
    /// transparent colour is the source's `LinkFill = null`, which fills the ribbons like the
    /// nodes.
    pub link_fill: Option<Color>,
}
impl Default for SankeySeries {
    fn default() -> Self {
        Self {
            series: Series::new(SeriesKind::Sankey),
            nodes: Vec::new(),
            links: Vec::new(),
            node_width: 12.0,
            node_padding: 8.0,
            node_corner_radius: 0.0,
            layout_iterations: 32,
            node_label_position: SankeyLabelPosition::Auto,
            layout: SankeyLayoutKind::Vertical,
            arc_span_degrees: 150.0,
            link_fill: None,
        }
    }
}
impl SankeySeries {
    pub fn new(nodes: Vec<SankeyNode>, links: Vec<SankeyLink>) -> Self {
        Self {
            nodes,
            links,
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SankeyOptions {
    /// An explicit draw margin; `None` takes the whole control.
    pub draw_margin: Option<Rect>,
}

/// A node rectangle of the vertical layout (`SankeyLayout.NodeBox`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SankeyNodeBox {
    /// The index of the node in the input.
    pub node: usize,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// The larger of the sums of the incoming and the outgoing weights.
    pub value: f64,
}

/// The band of a link at both ends (`SankeyLayout.RibbonBox`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SankeyRibbonBox {
    /// The index of the link in the input.
    pub link: usize,
    pub source_x: f64,
    pub source_y0: f64,
    pub source_y1: f64,
    pub target_x: f64,
    pub target_y0: f64,
    pub target_y1: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SankeyLayout {
    /// In the order of the input nodes.
    pub nodes: Vec<SankeyNodeBox>,
    /// By source node, then in the order of the targets from the top.
    pub links: Vec<SankeyRibbonBox>,
}

/// An annular sector of the bipartite arc layout (`SankeyLayout.ArcNodeBox`), in degrees
/// clockwise from the positive X axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SankeyArcNodeBox {
    pub node: usize,
    pub center: Point,
    pub inner_radius: f64,
    pub outer_radius: f64,
    pub start_angle: f64,
    pub sweep_angle: f64,
    pub value: f64,
}

/// The chord of a link (`SankeyLayout.ChordRibbonBox`): the ends of its bands on the inner radius
/// and the centre that attracts its curves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SankeyChordRibbonBox {
    pub link: usize,
    pub center: Point,
    pub source_p0: Point,
    pub source_p1: Point,
    pub target_p0: Point,
    pub target_p1: Point,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SankeyArcLayout {
    /// The sources (left arc), then the sinks (right arc).
    pub nodes: Vec<SankeyArcNodeBox>,
    pub links: Vec<SankeyChordRibbonBox>,
}

struct NodeState {
    node: usize,
    depth: usize,
    value: f64,
    x: f64,
    y: f64,
    height: f64,
    /// (link, source state).
    incoming: Vec<(usize, usize)>,
    /// (link, target state).
    outgoing: Vec<(usize, usize)>,
}

/// The graph of the source's layout: the first node of each id, and the links between known
/// nodes, without self-loops and with a finite positive weight.
fn graph(nodes: &[SankeyNode], links: &[SankeyLink]) -> Vec<NodeState> {
    let mut states = Vec::new();
    let mut by_id = HashMap::new();
    for (index, node) in nodes.iter().enumerate() {
        if by_id.contains_key(&node.id) {
            continue;
        }
        by_id.insert(node.id, states.len());
        states.push(NodeState {
            node: index,
            depth: 0,
            value: 0.0,
            x: 0.0,
            y: 0.0,
            height: 0.0,
            incoming: Vec::new(),
            outgoing: Vec::new(),
        });
    }
    for (index, link) in links.iter().enumerate() {
        if link.source == link.target {
            continue;
        }
        let (Some(&source), Some(&target)) = (by_id.get(&link.source), by_id.get(&link.target))
        else {
            continue;
        };
        if !(link.weight > 0.0) || !link.weight.is_finite() {
            continue;
        }
        states[source].outgoing.push((index, target));
        states[target].incoming.push((index, source));
    }
    states
}

/// Columns by the longest path from the sources (Kahn's breadth-first order); a node that never
/// reaches the queue is on a cycle. Returns the deepest column.
fn assign_depths(states: &mut [NodeState]) -> Result<usize, ChartError> {
    let mut in_degree: Vec<_> = states.iter().map(|s| s.incoming.len()).collect();
    let mut queue: VecDeque<_> = (0..states.len()).filter(|&i| in_degree[i] == 0).collect();
    let mut processed = 0;
    while let Some(n) = queue.pop_front() {
        processed += 1;
        let depth = states[n].depth;
        for k in 0..states[n].outgoing.len() {
            let target = states[n].outgoing[k].1;
            if depth + 1 > states[target].depth {
                states[target].depth = depth + 1;
            }
            in_degree[target] -= 1;
            if in_degree[target] == 0 {
                queue.push_back(target);
            }
        }
    }
    if processed < states.len() {
        return Err(ChartError::InvalidData(
            "SankeyLayout: the graph contains a cycle; only acyclic graphs are supported".into(),
        ));
    }
    Ok(states.iter().map(|s| s.depth).max().unwrap_or(0))
}

/// The value of a node is the larger of its incoming and outgoing sums; an isolated node still
/// takes a slot of 1.
fn resolve_values(states: &mut [NodeState], links: &[SankeyLink]) {
    for state in states.iter_mut() {
        let incoming: f64 = state.incoming.iter().map(|&(l, _)| links[l].weight).sum();
        let outgoing: f64 = state.outgoing.iter().map(|&(l, _)| links[l].weight).sum();
        state.value = if incoming > outgoing {
            incoming
        } else {
            outgoing
        };
        if state.value <= 0.0 {
            state.value = 1.0;
        }
    }
}

/// The vertical (d3-sankey) layout of the nodes and links in `rect` (`SankeyLayout.Compute`).
pub fn sankey_layout(
    nodes: &[SankeyNode],
    links: &[SankeyLink],
    rect: Rect,
    node_width: f64,
    node_padding: f64,
    iterations: u32,
) -> Result<SankeyLayout, ChartError> {
    let mut states = graph(nodes, links);
    if states.is_empty() || !(rect.width > 0.0) || !(rect.height > 0.0) {
        return Ok(SankeyLayout::default());
    }
    let max_depth = assign_depths(&mut states)?;
    resolve_values(&mut states, links);
    let mut columns = vec![Vec::new(); max_depth + 1];
    for (index, state) in states.iter().enumerate() {
        columns[state.depth].push(index);
    }
    // A node width wider than the rectangle would move later columns left of earlier ones.
    let mut dx = if max_depth > 0 {
        (rect.width - node_width) / max_depth as f64
    } else {
        0.0
    };
    if dx < 0.0 {
        dx = 0.0;
    }
    for (depth, column) in columns.iter().enumerate() {
        for &n in column {
            states[n].x = rect.x + depth as f64 * dx;
        }
    }
    // The scale that fits the densest column with its padding.
    let mut max_sum = 0.0;
    let mut max_count = 0;
    for column in &columns {
        let sum: f64 = column.iter().map(|&n| states[n].value).sum();
        if sum > max_sum {
            max_sum = sum;
        }
        max_count = max_count.max(column.len());
    }
    let mut available = rect.height - node_padding * max_count.saturating_sub(1) as f64;
    if available < 1.0 {
        available = 1.0;
    }
    let y_scale = available / max_sum;
    for state in states.iter_mut() {
        state.height = state.value * y_scale;
    }
    for column in &columns {
        let mut y = rect.y;
        for &n in column {
            states[n].y = y;
            y += states[n].height + node_padding;
        }
    }
    // Alternating sweeps move each node toward the weighted centre of its neighbours, then push
    // the nodes of each column apart; the step decays so that later sweeps converge.
    let mut alpha = 1.0;
    for _ in 0..iterations {
        alpha *= 0.99;
        relax_to_targets(&mut states, &columns, links, alpha);
        resolve_collisions(&mut states, &mut columns, rect, node_padding);
        relax_to_sources(&mut states, &columns, links, alpha);
        resolve_collisions(&mut states, &mut columns, rect, node_padding);
    }
    // The bands of a node follow the order of their other ends, so ribbons do not cross there.
    let ys: Vec<_> = states.iter().map(|s| s.y).collect();
    for state in states.iter_mut() {
        state.outgoing.sort_by(|a, b| ys[a.1].total_cmp(&ys[b.1]));
        state.incoming.sort_by(|a, b| ys[a.1].total_cmp(&ys[b.1]));
    }
    let nodes = states
        .iter()
        .map(|s| SankeyNodeBox {
            node: s.node,
            x: s.x,
            y: s.y,
            width: node_width,
            height: s.height,
            value: s.value,
        })
        .collect();
    let mut out_running = ys.clone();
    let mut in_running = ys;
    let mut ribbons = Vec::new();
    for n in 0..states.len() {
        for &(link, target) in &states[n].outgoing {
            let thickness = links[link].weight * y_scale;
            let source_y0 = out_running[n];
            out_running[n] = source_y0 + thickness;
            let target_y0 = in_running[target];
            in_running[target] = target_y0 + thickness;
            ribbons.push(SankeyRibbonBox {
                link,
                source_x: states[n].x + node_width,
                source_y0,
                source_y1: source_y0 + thickness,
                target_x: states[target].x,
                target_y0,
                target_y1: target_y0 + thickness,
            });
        }
    }
    Ok(SankeyLayout {
        nodes,
        links: ribbons,
    })
}

fn relax_to_targets(
    states: &mut [NodeState],
    columns: &[Vec<usize>],
    links: &[SankeyLink],
    alpha: f64,
) {
    for column in columns.iter().take(columns.len().saturating_sub(1)).rev() {
        for &n in column {
            let mut weight_sum = 0.0;
            let mut weighted_y = 0.0;
            for &(link, target) in &states[n].outgoing {
                let weight = links[link].weight;
                weight_sum += weight;
                weighted_y += (states[target].y + states[target].height * 0.5) * weight;
            }
            if states[n].outgoing.is_empty() || weight_sum <= 0.0 {
                continue;
            }
            let center = weighted_y / weight_sum;
            let state = &mut states[n];
            state.y += (center - (state.y + state.height * 0.5)) * alpha;
        }
    }
}

fn relax_to_sources(
    states: &mut [NodeState],
    columns: &[Vec<usize>],
    links: &[SankeyLink],
    alpha: f64,
) {
    for column in columns.iter().skip(1) {
        for &n in column {
            let mut weight_sum = 0.0;
            let mut weighted_y = 0.0;
            for &(link, source) in &states[n].incoming {
                let weight = links[link].weight;
                weight_sum += weight;
                weighted_y += (states[source].y + states[source].height * 0.5) * weight;
            }
            if states[n].incoming.is_empty() || weight_sum <= 0.0 {
                continue;
            }
            let center = weighted_y / weight_sum;
            let state = &mut states[n];
            state.y += (center - (state.y + state.height * 0.5)) * alpha;
        }
    }
}

/// Per column: sort by position, push the nodes apart from the top with the padding, then pull
/// an overflowing column back up from the bottom of the rectangle.
fn resolve_collisions(
    states: &mut [NodeState],
    columns: &mut [Vec<usize>],
    rect: Rect,
    padding: f64,
) {
    let top = rect.y;
    let bottom = rect.y + rect.height;
    for column in columns.iter_mut() {
        if column.is_empty() {
            continue;
        }
        column.sort_by(|a, b| states[*a].y.total_cmp(&states[*b].y));
        let mut min_y = top;
        for &n in column.iter() {
            if states[n].y < min_y {
                states[n].y = min_y;
            }
            min_y = states[n].y + states[n].height + padding;
        }
        let mut max_y = bottom;
        for &n in column.iter().rev() {
            if states[n].y + states[n].height > max_y {
                states[n].y = max_y - states[n].height;
            }
            max_y = states[n].y - padding;
        }
    }
}

#[derive(Clone, Copy)]
struct ArcSpan {
    /// The angle of the visual top of the node.
    top: f64,
    sweep: f64,
    direction: f64,
}

/// The bipartite arc layout (`SankeyLayout.ComputeBipartiteArc`): the sources on a left arc
/// centred at 180 degrees, the sinks on a right arc centred at 0, each node a sweep proportional
/// to its value, and the four chord ends of every link on the inner radius. `label_margin` is the
/// radial room reserved outside the arcs for the labels.
pub fn sankey_arc_layout(
    nodes: &[SankeyNode],
    links: &[SankeyLink],
    rect: Rect,
    node_width: f64,
    node_padding: f64,
    arc_span_degrees: f64,
    label_margin: f64,
) -> Result<SankeyArcLayout, ChartError> {
    let mut states = graph(nodes, links);
    if states.is_empty() || !(rect.width > 0.0) || !(rect.height > 0.0) {
        return Ok(SankeyArcLayout::default());
    }
    if assign_depths(&mut states)? > 1 {
        return Err(ChartError::InvalidData(
            "SankeyLayout.BipartiteArc: the graph has more than two depth columns. BipartiteArc \
             requires strictly bipartite data: every node must be either a pure source (no \
             incoming link) or a pure sink (no outgoing link). Use SankeyLayoutKind.Vertical for \
             multi-column flows."
                .into(),
        ));
    }
    resolve_values(&mut states, links);
    let left: Vec<_> = (0..states.len()).filter(|&i| states[i].depth == 0).collect();
    let right: Vec<_> = (0..states.len()).filter(|&i| states[i].depth != 0).collect();
    let center = Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
    let available = rect.width.min(rect.height) * 0.5;
    let mut outer = available - label_margin;
    if outer < node_width + 1.0 {
        outer = node_width + 1.0;
    }
    let mut inner = outer - node_width;
    if inner < 1.0 {
        inner = 1.0;
    }
    // The node padding is an angular gap that reads as that many pixels on the inner radius.
    let angular_padding = (node_padding / inner).to_degrees();
    let mut boxes = Vec::with_capacity(states.len());
    let mut spans = vec![None; states.len()];
    // The left arc goes down counterclockwise from 180 + span/2; the right arc goes down
    // clockwise from -span/2.
    for (column, top, direction) in [
        (&left, 180.0 + arc_span_degrees * 0.5, -1.0),
        (&right, -arc_span_degrees * 0.5, 1.0),
    ] {
        if column.is_empty() {
            continue;
        }
        let mut total: f64 = column.iter().map(|&n| states[n].value).sum();
        if total <= 0.0 {
            total = 1.0;
        }
        let mut usable = arc_span_degrees - angular_padding * (column.len() - 1) as f64;
        if usable < 1.0 {
            usable = 1.0;
        }
        let mut accumulated = 0.0;
        for &n in column.iter() {
            let sweep = states[n].value / total * usable;
            let node_top = top + direction * accumulated;
            boxes.push(SankeyArcNodeBox {
                node: states[n].node,
                center,
                inner_radius: inner,
                outer_radius: outer,
                start_angle: if direction > 0.0 {
                    node_top
                } else {
                    node_top - sweep
                },
                sweep_angle: sweep,
                value: states[n].value,
            });
            spans[n] = Some(ArcSpan {
                top: node_top,
                sweep,
                direction,
            });
            accumulated += sweep + angular_padding;
        }
    }
    let span = |n: usize| spans[n].expect("every node lies on one of the arcs");
    let visual_top_y = |n: usize| center.y + span(n).top.to_radians().sin() * inner;
    let tops: Vec<_> = (0..states.len()).map(visual_top_y).collect();
    for state in states.iter_mut() {
        state.outgoing.sort_by(|a, b| tops[a.1].total_cmp(&tops[b.1]));
        state.incoming.sort_by(|a, b| tops[a.1].total_cmp(&tops[b.1]));
    }
    let at = |degrees: f64| {
        let (sin, cos) = degrees.to_radians().sin_cos();
        Point::new(center.x + cos * inner, center.y + sin * inner)
    };
    let mut out_accumulated = vec![0.0; states.len()];
    let mut in_accumulated = vec![0.0; states.len()];
    let mut chords = Vec::new();
    for n in 0..states.len() {
        let source = span(n);
        for &(link, target) in &states[n].outgoing {
            let weight = links[link].weight;
            // A band is the share of the link in the sweep of each end, in the arc's direction.
            let source_sweep = source.sweep * (weight / states[n].value);
            let source_start = source.top + source.direction * out_accumulated[n];
            let source_end = source_start + source.direction * source_sweep;
            out_accumulated[n] += source_sweep;
            let target_span = span(target);
            let target_sweep = target_span.sweep * (weight / states[target].value);
            let target_start = target_span.top + target_span.direction * in_accumulated[target];
            let target_end = target_start + target_span.direction * target_sweep;
            in_accumulated[target] += target_sweep;
            chords.push(SankeyChordRibbonBox {
                link,
                center,
                source_p0: at(source_start),
                source_p1: at(source_end),
                target_p0: at(target_start),
                target_p1: at(target_end),
            });
        }
    }
    Ok(SankeyArcLayout {
        nodes: boxes,
        links: chords,
    })
}

/// The gap between a node and an outside label (the source's `gap`).
const LABEL_GAP: f64 = 4.0;

pub struct SankeyChartEngine;
impl SankeyChartEngine {
    pub fn measure(
        size: Size,
        series: &[SankeySeries],
        options: &SankeyOptions,
    ) -> Result<Scene, ChartError> {
        Self::measure_with_labels(size, series, options, |text, size| {
            Size::new(text.chars().count() as f64 * size * 0.6, size * 1.2)
        })
    }

    /// Native hosts supply their text metrics here, so that the room reserved for the labels
    /// fits the typeface that draws them. Every series takes the whole draw margin, as in the
    /// source.
    pub fn measure_with_labels(
        size: Size,
        series: &[SankeySeries],
        options: &SankeyOptions,
        mut measure_text: impl FnMut(&str, f64) -> Size,
    ) -> Result<Scene, ChartError> {
        validate_size(size)?;
        let mut plot = match options.draw_margin {
            Some(margin) => {
                if [margin.x, margin.y, margin.width, margin.height]
                    .iter()
                    .any(|v| !v.is_finite())
                {
                    return Err(ChartError::InvalidData("invalid Sankey draw margin".into()));
                }
                margin
            }
            None => Rect::new(0.0, 0.0, size.width, size.height),
        };
        for s in series {
            validate(s)?;
        }
        // A collapsed draw margin hides the plot (HidePlotZones); it is a valid frame.
        plot.width = plot.width.max(0.0);
        plot.height = plot.height.max(0.0);
        let mut scene = Scene::new(size, plot);
        scene.automatic_finding_strategy = FindingStrategy::CompareAll;
        if plot.width <= 0.0 || plot.height <= 0.0 {
            return Ok(scene);
        }
        let mut layers: Vec<(f64, Primitive)> = Vec::new();
        for (si, s) in series.iter().enumerate().filter(|(_, s)| s.series.visible) {
            scene.legend.push(LegendItem {
                series: si,
                name: if s.series.name.is_empty() {
                    format!("Series {}", si + 1)
                } else {
                    s.series.name.clone()
                },
                color: fill_color(&s.series, si),
            });
            if s.nodes.is_empty() {
                continue;
            }
            let mut context = Context {
                scene: &mut scene,
                layers: &mut layers,
                index: si,
                series: s,
                measure_text: &mut measure_text,
            };
            match s.layout {
                SankeyLayoutKind::Vertical => context.vertical(plot)?,
                SankeyLayoutKind::BipartiteArc => context.bipartite_arc(plot)?,
            }
        }
        layers.sort_by(|a, b| a.0.total_cmp(&b.0));
        scene
            .primitives
            .extend(layers.into_iter().map(|(_, primitive)| primitive));
        Ok(scene)
    }
}

struct Context<'a, F: FnMut(&str, f64) -> Size> {
    scene: &'a mut Scene,
    /// Primitives with the z-index of their source paint: the ribbons (the series index plus
    /// 0.1), the node strokes (plus 0.2), the node fills (plus 1.1) and the labels (1000 plus the
    /// index plus 0.3), drawn in that order.
    layers: &'a mut Vec<(f64, Primitive)>,
    index: usize,
    series: &'a SankeySeries,
    measure_text: &'a mut F,
}

impl<F: FnMut(&str, f64) -> Size> Context<'_, F> {
    fn id(&self, point: usize) -> PointId {
        PointId {
            series: self.index,
            point,
        }
    }

    fn z(&self, offset: f64) -> f64 {
        self.index as f64 + offset
    }

    /// The text of a label and its size grown by the label padding.
    fn label(&mut self, node: &SankeyNode) -> Option<(Size, Size)> {
        if !self.series.series.data_labels || node.name.is_empty() {
            return None;
        }
        let text = text_size(
            &mut *self.measure_text,
            &node.name,
            self.series.series.data_labels_size,
        );
        let padded = Size::new(
            text.width + DATA_LABELS_PADDING.0 * 2.0,
            text.height + DATA_LABELS_PADDING.1 * 2.0,
        );
        Some((text, padded))
    }

    fn push_label(
        &mut self,
        node: usize,
        anchor: Point,
        rotation: f64,
        align: TextAnchor,
        text: Size,
    ) {
        let s = self.series;
        // The label box is aligned at the anchor; its text sits inside the label padding.
        let dx = match align {
            TextAnchor::Start => DATA_LABELS_PADDING.0,
            TextAnchor::End => -DATA_LABELS_PADDING.0,
            TextAnchor::Center => 0.0,
        };
        let (sin, cos) = rotation.to_radians().sin_cos();
        let dy = -text.height * 0.5;
        let origin = Point::new(anchor.x + dx * cos - dy * sin, anchor.y + dx * sin + dy * cos);
        let primitive = Primitive::new(
            Geometry::Text {
                origin,
                text: s.nodes[node].name.clone(),
                size: s.series.data_labels_size,
                rotation,
                anchor: align,
            },
            PaintStyle::fill(s.series.data_labels_color.unwrap_or(DATA_LABELS_COLOR)),
        )
        .identified(self.id(node));
        let z = 1000.0 + self.z(0.3);
        self.layers.push((z, primitive));
    }

    /// The fill of a node: the node's colour over the series fill (`NodeColorMapper`).
    fn node_fill(&self, node: &SankeyNode) -> Option<PaintStyle> {
        let mut style = fill_style(&self.series.series, self.index)?;
        if let Some(color) = node.color {
            style.fill = Some(color);
            style.fill_brush = None;
        }
        Some(style)
    }

    /// The fill of a ribbon: the link's colour (`LinkColorMapper`), else the colour of its source
    /// node at half its alpha (`NodeColorMapper`), else the ribbon paint.
    fn ribbon_fill(&self, link: &SankeyLink, by_id: &HashMap<u64, usize>) -> Option<PaintStyle> {
        let s = &self.series;
        let override_color = link.color.or_else(|| {
            by_id
                .get(&link.source)
                .and_then(|&n| s.nodes[n].color)
                .map(|c| Color::from_argb(c.a / 2, c.r, c.g, c.b))
        });
        let mut style = match s.link_fill {
            // LinkFill = null: the ribbons take a clone of the node fill.
            Some(fill) if fill.a == 0 => fill_style(&s.series, self.index)?,
            Some(fill) => PaintStyle {
                fill: Some(fill),
                opacity: s.series.opacity,
                ..PaintStyle::default()
            },
            // The theme's LinkFill: the palette colour of the series at alpha 90.
            None => {
                let color = palette_color(self.index);
                PaintStyle {
                    fill: Some(Color::from_argb(90, color.r, color.g, color.b)),
                    opacity: s.series.opacity,
                    ..PaintStyle::default()
                }
            }
        };
        if let Some(color) = override_color {
            style.fill = Some(color);
            style.fill_brush = None;
        }
        Some(style)
    }

    fn by_id(&self) -> HashMap<u64, usize> {
        let mut by_id = HashMap::new();
        for (index, node) in self.series.nodes.iter().enumerate() {
            by_id.entry(node.id).or_insert(index);
        }
        by_id
    }

    /// The links of each node id, counted as the source counts them for the label placement: any
    /// link whose end is a node, whatever its weight.
    fn degrees(&self) -> HashMap<u64, (usize, usize)> {
        let mut degrees: HashMap<u64, (usize, usize)> =
            self.series.nodes.iter().map(|n| (n.id, (0, 0))).collect();
        for link in &self.series.links {
            if let Some(d) = degrees.get_mut(&link.source) {
                d.1 += 1;
            }
            if let Some(d) = degrees.get_mut(&link.target) {
                d.0 += 1;
            }
        }
        degrees
    }

    /// The room on the left and the right for the labels placed outside the nodes
    /// (`MeasureOutsideLabelReserve`).
    fn outside_label_reserve(&mut self, degrees: &HashMap<u64, (usize, usize)>) -> (f64, f64) {
        let s = self.series;
        if s.node_label_position == SankeyLabelPosition::Inside {
            return (0.0, 0.0);
        }
        let (mut left, mut right) = (0.0_f64, 0.0_f64);
        for node in &s.nodes {
            let Some((_, size)) = self.label(node) else {
                continue;
            };
            let (incoming, outgoing) = degrees.get(&node.id).copied().unwrap_or_default();
            if s.node_label_position == SankeyLabelPosition::Outside {
                left = left.max(size.width);
                right = right.max(size.width);
            } else if incoming == 0 && outgoing > 0 {
                left = left.max(size.width);
            } else if outgoing == 0 && incoming > 0 {
                right = right.max(size.width);
            }
        }
        let reserve = |width: f64| if width > 0.0 { width + LABEL_GAP } else { 0.0 };
        (reserve(left), reserve(right))
    }

    fn vertical(&mut self, plot: Rect) -> Result<(), ChartError> {
        let s = self.series;
        let degrees = self.degrees();
        let (left, right) = self.outside_label_reserve(&degrees);
        let rect = Rect::new(
            plot.x + left,
            plot.y,
            plot.width - left - right,
            plot.height,
        );
        let layout = sankey_layout(
            &s.nodes,
            &s.links,
            rect,
            s.node_width,
            s.node_padding,
            s.layout_iterations,
        )?;
        let center_x = rect.x + rect.width * 0.5;
        let stroke = stroke_style(&s.series);
        for b in &layout.nodes {
            let node = &s.nodes[b.node];
            let id = self.id(b.node);
            let bounds = Rect::new(b.x, b.y, b.width, b.height);
            let geometry = Geometry::Rectangle {
                rect: bounds,
                radius: s.node_corner_radius,
            };
            if let Some(fill) = self.node_fill(node) {
                let z = self.z(1.1);
                let primitive = Primitive::new(geometry.clone(), fill);
                self.layers.push((z, primitive.identified(id)));
            }
            if let Some(stroke) = &stroke {
                let z = self.z(0.2);
                let primitive = Primitive::new(geometry, stroke.clone());
                self.layers.push((z, primitive.identified(id)));
            }
            let middle = Point::new(b.x + b.width * 0.5, b.y + b.height * 0.5);
            self.scene.hits.push(HitRegion {
                id,
                shape: HitShape::Rectangle(bounds),
                center: middle,
                value: b.value,
                label: tooltip_text(&node.name, b.value),
            });
            let Some((text, _)) = self.label(node) else {
                continue;
            };
            let (incoming, outgoing) = degrees.get(&node.id).copied().unwrap_or_default();
            let (inside, on_right) = match s.node_label_position {
                SankeyLabelPosition::Inside => (true, false),
                SankeyLabelPosition::Outside => (false, middle.x > center_x),
                SankeyLabelPosition::Auto => {
                    if incoming == 0 && outgoing > 0 {
                        (false, false)
                    } else if outgoing == 0 && incoming > 0 {
                        (false, true)
                    } else {
                        (true, false)
                    }
                }
            };
            let (x, align) = if inside {
                (middle.x, TextAnchor::Center)
            } else if on_right {
                (b.x + b.width + LABEL_GAP, TextAnchor::Start)
            } else {
                (b.x - LABEL_GAP, TextAnchor::End)
            };
            self.push_label(b.node, Point::new(x, middle.y), 0.0, align, text);
        }
        let by_id = self.by_id();
        let node_count = s.nodes.len();
        for r in &layout.links {
            let Some(fill) = self.ribbon_fill(&s.links[r.link], &by_id) else {
                continue;
            };
            // Control points at the horizontal middle give the S-curve that does not overshoot
            // either node.
            let middle = (r.source_x + r.target_x) * 0.5;
            let path = vec![
                PathCommand::MoveTo(Point::new(r.source_x, r.source_y0)),
                PathCommand::CubicTo {
                    control1: Point::new(middle, r.source_y0),
                    control2: Point::new(middle, r.target_y0),
                    end: Point::new(r.target_x, r.target_y0),
                },
                PathCommand::LineTo(Point::new(r.target_x, r.target_y1)),
                PathCommand::CubicTo {
                    control1: Point::new(middle, r.target_y1),
                    control2: Point::new(middle, r.source_y1),
                    end: Point::new(r.source_x, r.source_y1),
                },
                PathCommand::Close,
            ];
            let z = self.z(0.1);
            let id = self.id(node_count + r.link);
            let primitive = Primitive::new(Geometry::Path(path), fill);
            self.layers.push((z, primitive.identified(id)));
        }
        Ok(())
    }

    fn bipartite_arc(&mut self, plot: Rect) -> Result<(), ChartError> {
        let s = self.series;
        // A uniform radial margin for the widest label (`MeasureArcLabelMargin`).
        let mut widest = 0.0_f64;
        for node in &s.nodes {
            if let Some((_, size)) = self.label(node) {
                widest = widest.max(size.width);
            }
        }
        let margin = if widest > 0.0 { widest + LABEL_GAP } else { 0.0 };
        let layout = sankey_arc_layout(
            &s.nodes,
            &s.links,
            plot,
            s.node_width,
            s.node_padding,
            s.arc_span_degrees,
            margin,
        )?;
        let stroke = stroke_style(&s.series);
        for b in &layout.nodes {
            let node = &s.nodes[b.node];
            let id = self.id(b.node);
            // A node without a sweep is not drawn (`SankeyArcSegmentGeometry.Draw`).
            if b.sweep_angle > 0.0 {
                let path = if s.node_corner_radius > 0.0 {
                    rounded_annular_sector_path(
                        b.center,
                        b.inner_radius,
                        b.outer_radius,
                        b.start_angle,
                        b.sweep_angle,
                        s.node_corner_radius,
                        false,
                    )
                } else {
                    annular_sector_path(
                        b.center,
                        b.inner_radius,
                        b.outer_radius,
                        b.start_angle,
                        b.sweep_angle,
                    )
                };
                if let Some(fill) = self.node_fill(node) {
                    let z = self.z(1.1);
                    let primitive = Primitive::new(Geometry::Path(path.clone()), fill);
                    self.layers.push((z, primitive.identified(id)));
                }
                if let Some(stroke) = &stroke {
                    let z = self.z(0.2);
                    let primitive = Primitive::new(Geometry::Path(path), stroke.clone());
                    self.layers.push((z, primitive.identified(id)));
                }
            }
            let middle = b.start_angle + b.sweep_angle * 0.5;
            let (sin, cos) = middle.to_radians().sin_cos();
            // The tooltip anchors at the outer middle of the sector (`AnchorAtOuterMidpoint`).
            self.scene.hits.push(HitRegion {
                id,
                shape: HitShape::AnnularSector {
                    center: b.center,
                    inner_radius: b.inner_radius,
                    outer_radius: b.outer_radius,
                    start_angle: b.start_angle,
                    sweep_angle: b.sweep_angle,
                },
                center: Point::new(
                    b.center.x + cos * b.outer_radius,
                    b.center.y + sin * b.outer_radius,
                ),
                value: b.value,
                label: tooltip_text(&node.name, b.value),
            });
            let Some((text, _)) = self.label(node) else {
                continue;
            };
            // Radially outside the arc; on the left half the text turns half a revolution and
            // ends at the anchor, so that it still reads away from the centre.
            let theta = middle.rem_euclid(360.0);
            let (sin, cos) = theta.to_radians().sin_cos();
            let radius = b.outer_radius + LABEL_GAP;
            let anchor = Point::new(b.center.x + cos * radius, b.center.y + sin * radius);
            let flipped = theta > 90.0 && theta < 270.0;
            let (rotation, align) = if flipped {
                (theta - 180.0, TextAnchor::End)
            } else {
                (theta, TextAnchor::Start)
            };
            self.push_label(b.node, anchor, rotation, align, text);
        }
        let by_id = self.by_id();
        let node_count = s.nodes.len();
        for r in &layout.links {
            let Some(fill) = self.ribbon_fill(&s.links[r.link], &by_id) else {
                continue;
            };
            let z = self.z(0.1);
            let id = self.id(node_count + r.link);
            let primitive = Primitive::new(Geometry::Path(chord_path(r)), fill);
            self.layers.push((z, primitive.identified(id)));
        }
        Ok(())
    }
}

/// The untwisted band of a chord (`SankeyChordRibbonGeometry`): source top to target top through
/// the centre, the inner arc along the target's band, target bottom back to source bottom through
/// the centre, and the inner arc along the source's band.
fn chord_path(r: &SankeyChordRibbonBox) -> Vec<PathCommand> {
    let c = r.center;
    let radius = (r.source_p0.x - c.x).hypot(r.source_p0.y - c.y);
    let angle = |p: Point| (p.y - c.y).atan2(p.x - c.x).to_degrees();
    let target_angle = angle(r.target_p0);
    let source_angle = angle(r.source_p1);
    // The shorter direction across each band: a band is far below half a revolution.
    let target_sweep = normalize_sweep(angle(r.target_p1) - target_angle);
    let source_sweep = normalize_sweep(angle(r.source_p0) - source_angle);
    let mut path = vec![
        PathCommand::MoveTo(r.source_p0),
        PathCommand::CubicTo {
            control1: c,
            control2: c,
            end: r.target_p0,
        },
    ];
    append_arc(
        &mut path,
        c,
        radius,
        target_angle.to_radians(),
        target_sweep.to_radians(),
    );
    path.push(PathCommand::CubicTo {
        control1: c,
        control2: c,
        end: r.source_p1,
    });
    append_arc(
        &mut path,
        c,
        radius,
        source_angle.to_radians(),
        source_sweep.to_radians(),
    );
    path.push(PathCommand::Close);
    path
}

fn normalize_sweep(sweep: f64) -> f64 {
    if !sweep.is_finite() {
        return 0.0;
    }
    let mut sweep = sweep;
    while sweep > 180.0 {
        sweep -= 360.0;
    }
    while sweep < -180.0 {
        sweep += 360.0;
    }
    sweep
}

fn validate(s: &SankeySeries) -> Result<(), ChartError> {
    validate_series(&s.series)?;
    if [
        s.node_width,
        s.node_padding,
        s.node_corner_radius,
        s.arc_span_degrees,
    ]
    .iter()
    .any(|v| !v.is_finite())
        || s.node_width < 0.0
        || s.node_padding < 0.0
        || s.node_corner_radius < 0.0
    {
        return Err(ChartError::InvalidData(
            "Sankey node widths, paddings, radii and arc spans must be finite and nonnegative"
                .into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLUE: Color = Color::from_rgb(96, 138, 218);

    fn nodes(names: &[&str]) -> Vec<SankeyNode> {
        names
            .iter()
            .enumerate()
            .map(|(i, name)| SankeyNode::new(i as u64 + 1, *name))
            .collect()
    }

    fn links(pairs: &[(u64, u64, f64)]) -> Vec<SankeyLink> {
        pairs
            .iter()
            .map(|&(s, t, w)| SankeyLink::new(s, t, w))
            .collect()
    }

    fn series(nodes: Vec<SankeyNode>, links: Vec<SankeyLink>) -> SankeySeries {
        let mut s = SankeySeries::new(nodes, links);
        s.series.fill = Some(BLUE);
        s.link_fill = Some(Color::from_argb(90, 96, 138, 218));
        s
    }

    fn measure(size: Size, s: SankeySeries) -> Scene {
        SankeyChartEngine::measure(size, &[s], &SankeyOptions::default()).unwrap()
    }

    /// The A, B, C to X, Y fixture of SnapshotTests/SankeySeriesTests.TwoLevelFlow.
    fn two_level() -> SankeySeries {
        let mut s = series(
            nodes(&["A", "B", "C", "X", "Y"]),
            links(&[(1, 4, 8.0), (1, 5, 4.0), (2, 4, 6.0), (2, 5, 2.0), (3, 5, 10.0)]),
        );
        s.node_width = 16.0;
        s
    }

    /// SnapshotTests/SankeySeriesTests.TwoLevelFlow: two columns, node values the larger of the
    /// in and out sums, nodes stacked within the rectangle with the padding, and every ribbon
    /// band as thick at both ends.
    #[test]
    fn two_level_flow_lays_out_two_columns() {
        let s = two_level();
        let layout = sankey_layout(
            &s.nodes,
            &s.links,
            Rect::new(0.0, 0.0, 600.0, 400.0),
            16.0,
            8.0,
            32,
        )
        .unwrap();
        let values: Vec<_> = layout.nodes.iter().map(|b| b.value).collect();
        assert_eq!(values, vec![12.0, 8.0, 10.0, 14.0, 16.0]);
        assert!(layout.nodes[..3].iter().all(|b| b.x == 0.0));
        assert!(layout.nodes[3..].iter().all(|b| b.x == 584.0));
        // The densest column (A, B, C: 30 with two gaps) fills the height.
        let scale = (400.0 - 16.0) / 30.0;
        for b in &layout.nodes {
            assert!((b.height - b.value * scale).abs() < 1e-9);
            assert!(b.y >= -1e-9 && b.y + b.height <= 400.0 + 1e-9);
        }
        let mut left: Vec<_> = layout.nodes[..3].to_vec();
        left.sort_by(|a, b| a.y.total_cmp(&b.y));
        for pair in left.windows(2) {
            assert!(pair[1].y >= pair[0].y + pair[0].height + 8.0 - 1e-9);
        }
        assert_eq!(layout.links.len(), 5);
        for r in &layout.links {
            assert!((r.source_y1 - r.source_y0 - (r.target_y1 - r.target_y0)).abs() < 1e-9);
            assert!((r.source_y1 - r.source_y0 - s.links[r.link].weight * scale).abs() < 1e-9);
            assert_eq!(r.source_x, 16.0);
            assert_eq!(r.target_x, 584.0);
        }
        // The bands of a node tile it from its top, in the order of their other ends.
        let a = layout.nodes[0];
        let bands: Vec<_> = layout.links.iter().filter(|r| r.link < 2).collect();
        assert!((bands[0].source_y0 - a.y).abs() < 1e-9);
        assert!((bands[1].source_y1 - (a.y + a.height)).abs() < 1e-9);
        let scene = measure(Size::new(600.0, 400.0), s);
        // Ribbons under the nodes.
        assert_eq!(scene.primitives.len(), 10);
        assert!(scene.primitives[..5]
            .iter()
            .all(|p| matches!(p.geometry, Geometry::Path(_))));
        assert!(scene.primitives[5..]
            .iter()
            .all(|p| matches!(p.geometry, Geometry::Rectangle { .. })));
        assert_eq!(scene.hits.len(), 5);
    }

    /// SnapshotTests/SankeySeriesTests.ThreeColumnsWithBranching: three columns, and the
    /// relaxation keeps every column inside the rectangle without overlap.
    #[test]
    fn three_columns_with_branching() {
        let mut s = series(
            nodes(&["A", "B", "C", "M", "N", "P", "Q"]),
            links(&[
                (1, 4, 10.0),
                (2, 4, 5.0),
                (2, 5, 3.0),
                (3, 5, 12.0),
                (4, 6, 8.0),
                (4, 7, 7.0),
                (5, 6, 5.0),
                (5, 7, 10.0),
            ]),
        );
        s.node_width = 14.0;
        s.node_padding = 12.0;
        let layout = sankey_layout(
            &s.nodes,
            &s.links,
            Rect::new(0.0, 0.0, 700.0, 400.0),
            14.0,
            12.0,
            32,
        )
        .unwrap();
        let xs: Vec<_> = layout.nodes.iter().map(|b| b.x).collect();
        assert_eq!(xs, vec![0.0, 0.0, 0.0, 343.0, 343.0, 686.0, 686.0]);
        for column in [&layout.nodes[..3], &layout.nodes[3..5], &layout.nodes[5..]] {
            let mut column = column.to_vec();
            column.sort_by(|a, b| a.y.total_cmp(&b.y));
            assert!(column[0].y >= -1e-9);
            assert!(column.last().map_or(0.0, |b| b.y + b.height) <= 400.0 + 1e-9);
            for pair in column.windows(2) {
                assert!(pair[1].y >= pair[0].y + pair[0].height + 12.0 - 1e-9);
            }
        }
        assert_eq!(measure(Size::new(700.0, 400.0), s).hits.len(), 7);
    }

    /// SnapshotTests/SankeySeriesTests.WithLabels: the sources' labels on the left and the
    /// sinks' on the right, with the room they take reserved at both edges.
    #[test]
    fn auto_labels_reserve_room_outside_the_columns() {
        let mut s = series(
            nodes(&["Alice", "Bob", "Carol", "Trips", "Other"]),
            links(&[(1, 4, 8.0), (1, 5, 4.0), (2, 4, 6.0), (2, 5, 2.0), (3, 5, 10.0)]),
        );
        s.node_width = 16.0;
        s.series.data_labels = true;
        s.series.data_labels_size = 16.0;
        s.series.data_labels_color = Some(Color::from_rgb(0, 0, 0));
        let scene = measure(Size::new(700.0, 400.0), s);
        let width = |text: &str| text.chars().count() as f64 * 16.0 * 0.6;
        // The widest source label (Carol) and the widest sink label (Trips, Other) with the
        // label padding and the gap.
        let left = width("Carol") + 12.0 + 4.0;
        let right = width("Other") + 12.0 + 4.0;
        let sources: Vec<_> = scene.hits.iter().filter(|h| h.id.point < 3).collect();
        let sinks: Vec<_> = scene.hits.iter().filter(|h| h.id.point >= 3).collect();
        assert!(sources
            .iter()
            .all(|h| (h.shape.hover_bounds().x - left).abs() < 1e-9));
        assert!(sinks.iter().all(|h| {
            let b = h.shape.hover_bounds();
            (b.x + b.width - (700.0 - right)).abs() < 1e-9
        }));
        let labels: Vec<_> = scene
            .primitives
            .iter()
            .filter_map(|p| match &p.geometry {
                Geometry::Text {
                    text,
                    origin,
                    anchor,
                    ..
                } => Some((text.as_str(), *origin, *anchor)),
                _ => None,
            })
            .collect();
        assert_eq!(labels.len(), 5);
        assert!(labels[..3]
            .iter()
            .all(|(_, origin, anchor)| *anchor == TextAnchor::End && origin.x < left));
        assert!(labels[3..].iter().all(|(_, origin, anchor)| {
            *anchor == TextAnchor::Start && origin.x > 700.0 - right
        }));
    }

    /// SnapshotTests/SankeySeriesTests.JobApplicationFunnel: four columns, nodes that are both a
    /// sink and a source, labels over the pass-through nodes, and the node colours (the source's
    /// NodeColorMapper) tinting their outgoing ribbons at half alpha.
    #[test]
    fn job_application_funnel_colours_nodes_and_their_ribbons() {
        let mut ns = nodes(&[
            "9 Applications",
            "4 1st Interviews",
            "3 Rejected",
            "2 No Reply",
            "2 2nd Interviews",
            "1 Dropped by Myself",
            "1 No Offer Received",
            "2 Offers",
            "1 Accepted",
            "1 Declined",
        ]);
        let green = Color::from_rgb(64, 175, 110);
        ns[8].color = Some(green);
        ns[0].color = Some(BLUE);
        let mut s = series(
            ns,
            links(&[
                (1, 2, 4.0),
                (1, 3, 3.0),
                (1, 4, 2.0),
                (2, 5, 2.0),
                (2, 6, 1.0),
                (2, 7, 1.0),
                (5, 8, 2.0),
                (8, 9, 1.0),
                (8, 10, 1.0),
            ]),
        );
        s.node_width = 18.0;
        s.node_padding = 14.0;
        s.node_corner_radius = 3.0;
        s.link_fill = Some(Color::from_rgb(255, 255, 255));
        s.series.data_labels = true;
        s.series.data_labels_size = 13.0;
        let scene = measure(Size::new(900.0, 600.0), s);
        let columns: std::collections::BTreeSet<_> = scene
            .hits
            .iter()
            .map(|h| h.shape.hover_bounds().x.round() as i64)
            .collect();
        assert_eq!(columns.len(), 5);
        let accepted = scene.hits.iter().find(|h| h.id.point == 8).unwrap();
        assert_eq!(accepted.label, "1 Accepted: 1");
        let fill_of = |point: usize| {
            scene
                .primitives
                .iter()
                .find(|p| p.id.map(|id| id.point) == Some(point))
                .and_then(|p| p.style.fill)
        };
        // The first link leaves Applications: its ribbon takes the node colour at half alpha.
        assert_eq!(fill_of(10), Some(Color::from_argb(127, 96, 138, 218)));
        // The links of 1st Interviews take the white LinkFill.
        assert_eq!(fill_of(13), Some(Color::from_rgb(255, 255, 255)));
        let label = |text: &str| {
            scene.primitives.iter().find_map(|p| match &p.geometry {
                Geometry::Text {
                    text: t, anchor, ..
                } if t == text => Some(*anchor),
                _ => None,
            })
        };
        assert_eq!(label("9 Applications"), Some(TextAnchor::End));
        assert_eq!(label("4 1st Interviews"), Some(TextAnchor::Center));
        assert_eq!(label("1 Accepted"), Some(TextAnchor::Start));
    }

    fn bipartite() -> SankeySeries {
        let mut s = series(
            nodes(&[
                "Furniture",
                "Technology",
                "Office Supplies",
                "Chairs",
                "Phones",
                "Storage",
                "Tables",
                "Paper",
            ]),
            links(&[
                (1, 4, 12.0),
                (1, 7, 6.0),
                (2, 5, 14.0),
                (2, 6, 8.0),
                (3, 6, 4.0),
                (3, 8, 9.0),
                (3, 4, 2.0),
            ]),
        );
        s.layout = SankeyLayoutKind::BipartiteArc;
        s.node_width = 20.0;
        s
    }

    /// SnapshotTests/SankeySeriesTests.BipartiteArc: the sources share the left arc and the
    /// sinks the right arc, each a sweep proportional to its value with the angular padding, and
    /// the chord ends lie on the inner radius.
    #[test]
    fn bipartite_arc_divides_the_arcs_by_value() {
        let s = bipartite();
        let layout = sankey_arc_layout(
            &s.nodes,
            &s.links,
            Rect::new(0.0, 0.0, 700.0, 600.0),
            20.0,
            8.0,
            150.0,
            0.0,
        )
        .unwrap();
        assert_eq!(layout.nodes.len(), 8);
        let center = Point::new(350.0, 300.0);
        let (outer, inner) = (300.0, 280.0);
        let padding = (8.0_f64 / inner).to_degrees();
        // Furniture 18, Technology 22, Office Supplies 15 on the left arc.
        let usable = 150.0 - 2.0 * padding;
        let left: Vec<_> = layout.nodes[..3].iter().map(|b| b.sweep_angle).collect();
        for (sweep, value) in left.iter().zip([18.0, 22.0, 15.0]) {
            assert!((sweep - value / 55.0 * usable).abs() < 1e-9);
        }
        // The first source starts at the visual top of the left arc and runs down.
        let furniture = layout.nodes[0];
        assert!((furniture.start_angle + furniture.sweep_angle - 255.0).abs() < 1e-9);
        let chairs = layout.nodes[3];
        assert!((chairs.start_angle + 75.0).abs() < 1e-9);
        for b in &layout.nodes {
            assert_eq!(b.center, center);
            assert_eq!((b.outer_radius, b.inner_radius), (outer, inner));
        }
        assert_eq!(layout.links.len(), 7);
        for r in &layout.links {
            for p in [r.source_p0, r.source_p1, r.target_p0, r.target_p1] {
                assert!(((p.x - center.x).hypot(p.y - center.y) - inner).abs() < 1e-9);
            }
        }
        let scene = measure(Size::new(700.0, 600.0), s);
        assert_eq!(scene.hits.len(), 8);
        assert!(scene
            .primitives
            .iter()
            .all(|p| matches!(p.geometry, Geometry::Path(_))));
    }

    /// SnapshotTests/SankeySeriesTests.BipartiteArcWithLabels: the arcs shrink by the widest
    /// label, and a label on the left half turns half a revolution and ends at its anchor.
    #[test]
    fn bipartite_arc_labels_reserve_a_radial_margin() {
        let mut s = series(
            nodes(&["Alpha", "Beta", "Gamma", "Xenon", "Yttrium", "Zinc"]),
            links(&[
                (1, 4, 8.0),
                (1, 5, 4.0),
                (2, 4, 6.0),
                (2, 6, 5.0),
                (3, 5, 10.0),
                (3, 6, 3.0),
            ]),
        );
        s.layout = SankeyLayoutKind::BipartiteArc;
        s.node_width = 14.0;
        s.series.data_labels = true;
        s.series.data_labels_size = 14.0;
        let scene = measure(Size::new(700.0, 600.0), s);
        // Yttrium is the widest label: 7 characters at the estimated metrics, with the padding.
        let margin = 7.0 * 14.0 * 0.6 + 12.0 + 4.0;
        let HitShape::AnnularSector { outer_radius, .. } = &scene.hits[0].shape else {
            panic!("an arc node hovers its sector");
        };
        assert!((*outer_radius - (300.0 - margin)).abs() < 1e-9);
        let alpha = scene
            .primitives
            .iter()
            .find_map(|p| match &p.geometry {
                Geometry::Text {
                    text,
                    anchor,
                    rotation,
                    ..
                } if text == "Alpha" => Some((*anchor, *rotation)),
                _ => None,
            })
            .unwrap();
        assert_eq!(alpha.0, TextAnchor::End);
        assert!(alpha.1.abs() < 90.0);
        let xenon = scene
            .primitives
            .iter()
            .find_map(|p| match &p.geometry {
                Geometry::Text { text, anchor, .. } if text == "Xenon" => Some(*anchor),
                _ => None,
            })
            .unwrap();
        assert_eq!(xenon, TextAnchor::Start);
    }

    /// SnapshotTests/SankeySeriesTests.BipartiteArc_ThrowsOnMultiColumn: a pass-through node makes
    /// three columns, which the bipartite arc layout rejects with a message naming the layouts.
    #[test]
    fn bipartite_arc_rejects_multi_column_data() {
        let mut s = series(
            nodes(&["A", "B", "M", "P"]),
            links(&[(1, 3, 5.0), (2, 3, 3.0), (3, 4, 8.0)]),
        );
        s.layout = SankeyLayoutKind::BipartiteArc;
        let error =
            SankeyChartEngine::measure(Size::new(400.0, 400.0), &[s], &SankeyOptions::default())
                .unwrap_err()
                .to_string();
        assert!(error.contains("BipartiteArc"));
        assert!(error.contains("Vertical"));
    }

    /// SnapshotTests/SankeySeriesTests.HoverTooltip_Vertical: the pointer over the top source
    /// node reads "A: 12".
    #[test]
    fn hover_tooltip_vertical_reads_the_node_value() {
        let mut s = series(
            nodes(&["A", "B", "C", "X", "Y"]),
            links(&[(1, 4, 8.0), (1, 5, 4.0), (2, 4, 6.0), (3, 5, 10.0)]),
        );
        s.node_width = 16.0;
        let scene = measure(Size::new(600.0, 400.0), s);
        let a = scene.hits.iter().find(|h| h.id.point == 0).unwrap();
        let probe = a.center;
        let hits = scene.hit_test(probe, FindingStrategy::Automatic, 0.0);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].label, "A: 12");
    }

    /// SnapshotTests/SankeySeriesTests.HoverTooltip_BipartiteArc: the pointer at the east edge
    /// of a 600 px chart is inside the ring of the right arc.
    #[test]
    fn hover_tooltip_bipartite_arc_hits_the_right_ring() {
        let mut s = series(
            nodes(&["Furniture", "Technology", "Chairs", "Phones", "Storage"]),
            links(&[(1, 3, 12.0), (2, 4, 14.0), (2, 5, 8.0)]),
        );
        s.layout = SankeyLayoutKind::BipartiteArc;
        s.node_width = 20.0;
        let scene = measure(Size::new(600.0, 600.0), s);
        let hits = scene.hit_test(Point::new(590.0, 300.0), FindingStrategy::Automatic, 0.0);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].id.point >= 2);
        assert!(hits[0].label.contains(": "));
    }

    /// SnapshotTests/SankeySeriesTests.DefaultTheme_AppliesPaletteColor: without fills, the nodes
    /// take the palette colour and the ribbons the theme's LinkFill, the same colour at alpha 90.
    #[test]
    fn default_theme_applies_the_palette_colour() {
        let mut s = SankeySeries::new(
            nodes(&["A", "B", "X", "Y"]),
            links(&[(1, 3, 5.0), (1, 4, 3.0), (2, 4, 7.0)]),
        );
        s.node_width = 16.0;
        let scene = measure(Size::new(600.0, 400.0), s);
        let color = palette_color(0);
        let fills: Vec<_> = scene.primitives.iter().filter_map(|p| p.style.fill).collect();
        assert_eq!(fills.len(), 7);
        assert!(fills[..3]
            .iter()
            .all(|c| *c == Color::from_argb(90, color.r, color.g, color.b)));
        assert!(fills[3..].iter().all(|c| *c == color));
        assert!(scene.primitives.iter().all(|p| p.style.stroke.is_none()));
        assert_eq!(scene.legend[0].color, color);
    }

    fn hit_fixture(layout: SankeyLayoutKind) -> Scene {
        let mut s = SankeySeries::new(
            nodes(&["A", "B", "X", "Y"]),
            links(&[(1, 3, 4.0), (1, 4, 2.0), (2, 4, 6.0)]),
        );
        s.node_width = 24.0;
        s.layout = layout;
        measure(Size::new(1000.0, 1000.0), s)
    }

    /// CoreTests/SeriesTests/SeriesHitTests.Sankey_Vertical_CompareAll_HitsLeftColumnNode.
    #[test]
    fn sankey_vertical_compare_all_hits_left_column_node() {
        let scene = hit_fixture(SankeyLayoutKind::Vertical);
        let hits = scene.hit_test(Point::new(12.0, 200.0), FindingStrategy::CompareAll, 0.0);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].id.point < 2);
    }

    /// CoreTests/SeriesTests/SeriesHitTests.Sankey_Vertical_BetweenColumns_Misses.
    #[test]
    fn sankey_vertical_between_columns_misses() {
        let scene = hit_fixture(SankeyLayoutKind::Vertical);
        let hits = scene.hit_test(Point::new(500.0, 500.0), FindingStrategy::CompareAll, 0.0);
        assert!(hits.is_empty());
    }

    /// CoreTests/SeriesTests/SeriesHitTests.Sankey_BipartiteArc_EastEdgeHitsRightArcNode.
    #[test]
    fn sankey_bipartite_arc_east_edge_hits_right_arc_node() {
        let scene = hit_fixture(SankeyLayoutKind::BipartiteArc);
        let hits = scene.hit_test(Point::new(990.0, 500.0), FindingStrategy::CompareAll, 0.0);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].id.point >= 2);
    }

    /// CoreTests/SeriesTests/SeriesHitTests.Sankey_BipartiteArc_InsideInnerRadius_Misses.
    #[test]
    fn sankey_bipartite_arc_inside_inner_radius_misses() {
        let scene = hit_fixture(SankeyLayoutKind::BipartiteArc);
        let hits = scene.hit_test(Point::new(500.0, 500.0), FindingStrategy::CompareAll, 0.0);
        assert!(hits.is_empty());
    }

    /// CollapsedDrawMarginAllChartsTests.Sankey_HidesThenResets: a draw margin collapsed past
    /// the control hides the plot, and a valid one draws it again.
    #[test]
    fn collapsed_draw_margin_hides_then_resets() {
        let s = SankeySeries::new(
            nodes(&["A", "B", "X", "Y"]),
            links(&[(1, 3, 4.0), (2, 4, 6.0)]),
        );
        let size = Size::new(300.0, 300.0);
        let collapsed = SankeyOptions {
            draw_margin: Some(Rect::new(200.0, 200.0, -100.0, -100.0)),
        };
        let scene = SankeyChartEngine::measure(size, &[s.clone()], &collapsed).unwrap();
        assert!(scene.plot_bounds.width <= 0.0 && scene.plot_bounds.height <= 0.0);
        assert!(scene.primitives.is_empty() && scene.hits.is_empty());
        let scene = SankeyChartEngine::measure(size, &[s], &SankeyOptions::default()).unwrap();
        assert_eq!(scene.hits.len(), 4);
        assert_eq!(scene.primitives.len(), 6);
    }

    #[test]
    fn cycles_self_loops_unknown_ends_and_invalid_weights() {
        let s = series(nodes(&["A", "B"]), links(&[(1, 2, 3.0), (2, 1, 3.0)]));
        let error =
            SankeyChartEngine::measure(Size::new(300.0, 300.0), &[s], &SankeyOptions::default())
                .unwrap_err();
        assert!(error.to_string().contains("cycle"));
        // Dropped silently: a self-loop, an unknown end, a nonpositive or non-finite weight, and
        // a repeated node id.
        let mut ns = nodes(&["A", "B"]);
        ns.push(SankeyNode::new(1, "A again"));
        let s = series(
            ns,
            links(&[
                (1, 1, 5.0),
                (1, 9, 5.0),
                (1, 2, 0.0),
                (1, 2, f64::NAN),
                (1, 2, 2.0),
            ]),
        );
        let layout = sankey_layout(
            &s.nodes,
            &s.links,
            Rect::new(0.0, 0.0, 300.0, 300.0),
            12.0,
            8.0,
            32,
        )
        .unwrap();
        assert_eq!(layout.nodes.len(), 2);
        assert_eq!(layout.links.len(), 1);
        assert_eq!(layout.links[0].link, 4);
    }

    #[test]
    fn chord_sweeps_take_the_shorter_direction() {
        assert_eq!(normalize_sweep(350.0), -10.0);
        assert_eq!(normalize_sweep(-190.0), 170.0);
        assert_eq!(normalize_sweep(f64::NAN), 0.0);
    }
}
