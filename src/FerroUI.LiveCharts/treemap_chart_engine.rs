//! Treemap measurement adapted from the pinned `TreemapLayout`, `TreemapChartEngine` and
//! `CoreTreemapSeries`: the squarified layout of Bruls, Huijing and van Wijk, the partition of the
//! draw margin between the visible series by their totals, the weight roll-up of internal nodes,
//! the padding inset of every tile, hover areas and data labels on the leaves. Pure calculation
//! produces a retained scene that the native controls draw like the other chart scenes.

use crate::paint::{representative_color, snapshot_brush};
use crate::scene::*;
use crate::series::{Series, SeriesKind, DATA_LABELS_COLOR, DATA_LABELS_PADDING};
use ferroui_base::{media::Color, Point, Rect, Size};

/// A node of a treemap (`Defaults/TreemapNode`). A leaf carries its weight in `value`; an internal
/// node either carries an explicit nonzero value or rolls its children up. `children` is `None`
/// for a leaf; `Some` of an empty list is an internal node without children, which has no hover
/// area and no label, as in the source.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreemapNode {
    pub name: String,
    pub value: f64,
    pub children: Option<Vec<TreemapNode>>,
}
impl TreemapNode {
    /// A leaf of the given weight (`TreemapNode(value, name)`).
    pub fn new(value: f64, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value,
            children: None,
        }
    }
    /// An internal node whose weight is the sum of its children (`TreemapNode(name, children)`).
    pub fn with_children(name: impl Into<String>, children: Vec<TreemapNode>) -> Self {
        Self {
            name: name.into(),
            value: 0.0,
            children: Some(children),
        }
    }
}

/// A treemap series: the paints, padding (`Padding`, 2 by default), corner radius and data labels
/// of its [`Series`], and its root nodes (the source's `Values`).
#[derive(Clone, Debug, PartialEq)]
pub struct TreemapSeries {
    pub series: Series,
    pub nodes: Vec<TreemapNode>,
}
impl Default for TreemapSeries {
    fn default() -> Self {
        Self {
            series: Series::new(SeriesKind::Treemap),
            nodes: Vec::new(),
        }
    }
}
impl TreemapSeries {
    pub fn new(nodes: Vec<TreemapNode>) -> Self {
        Self {
            nodes,
            ..Self::default()
        }
    }
    /// The sum of the resolved weights of the roots (`GetTotalWeight`), which partitions the draw
    /// margin between the series.
    pub fn total_weight(&self) -> f64 {
        let flat = flatten(&self.nodes);
        flat.roots.iter().map(|&i| flat.nodes[i].value).sum()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreemapOptions {
    /// An explicit draw margin; `None` takes the whole control.
    pub draw_margin: Option<Rect>,
}

/// One tile of a laid out series.
#[derive(Clone, Debug, PartialEq)]
pub struct TreemapTile {
    /// The preorder position of the node in its series, independent of the weight order.
    pub node_index: usize,
    pub parent_index: Option<usize>,
    pub name: String,
    /// The resolved weight: the explicit value, or the sum of the children.
    pub value: f64,
    pub is_leaf: bool,
    /// The squarified rectangle of the node.
    pub allocated_bounds: Rect,
    /// The rectangle inset by the padding, which the tile draws and its children divide.
    pub bounds: Rect,
}

/// Partitions `rect` into rectangles of areas proportional to the weights
/// (`TreemapLayout.Squarify`), row by row along the shorter side, keeping each row while its worst
/// aspect ratio does not get worse. Non-positive and non-finite weights are skipped; the result is in descending weight
/// order (a stable sort, so equal weights keep their order).
///
/// The weights are normalised by the largest before they are summed, which keeps the areas of the
/// source without its overflow for weights near `f64::MAX`; a rectangle that collapses to no
/// shorter side places the rest of the items at zero size instead of dividing by zero.
pub fn squarify<T: Clone>(items: &[(f64, T)], rect: Rect) -> Vec<(T, Rect)> {
    if items.is_empty() || !(rect.width > 0.0) || !(rect.height > 0.0) {
        return Vec::new();
    }
    let mut items: Vec<_> = items
        .iter()
        .filter(|(weight, _)| weight.is_finite() && *weight > 0.0)
        .collect();
    if items.is_empty() {
        return Vec::new();
    }
    items.sort_by(|a, b| b.0.total_cmp(&a.0));
    let largest = items[0].0;
    let total: f64 = items.iter().map(|(weight, _)| *weight / largest).sum();
    let area = rect.width * rect.height;
    let areas: Vec<_> = items
        .iter()
        .map(|(weight, _)| *weight / largest / total * area)
        .collect();
    let mut placements = Vec::with_capacity(items.len());
    let mut index = 0;
    let mut current = rect;
    while index < items.len() {
        let shorter = current.width.min(current.height);
        if !(shorter > 0.0) || !(areas[index] > 0.0) {
            for (_, item) in &items[index..] {
                placements.push((item.clone(), Rect::new(current.x, current.y, 0.0, 0.0)));
            }
            break;
        }
        let start = index;
        let mut sum = 0.0;
        let mut min = f64::MAX;
        let mut max = 0.0_f64;
        let mut worst = f64::MAX;
        // Greedy row: add items while the worst aspect ratio does not get worse.
        while index < items.len() {
            let a = areas[index];
            if !(a > 0.0) {
                break;
            }
            let new_sum = sum + a;
            let new_min = min.min(a);
            let new_max = max.max(a);
            let new_worst = worst_ratio(new_max, new_min, shorter, new_sum);
            if index == start || new_worst <= worst {
                sum = new_sum;
                min = new_min;
                max = new_max;
                worst = new_worst;
                index += 1;
            } else {
                break;
            }
        }
        // The closed row lies along the shorter side: a horizontal row spans the width and grows
        // downward, a vertical one spans the height and grows rightward.
        let depth = sum / shorter;
        let horizontal = current.width <= current.height;
        let mut cursor_x = current.x;
        let mut cursor_y = current.y;
        for k in start..index {
            let extent = areas[k] / depth;
            let child = if horizontal {
                let child = Rect::new(cursor_x, cursor_y, extent, depth);
                cursor_x += extent;
                child
            } else {
                let child = Rect::new(cursor_x, cursor_y, depth, extent);
                cursor_y += extent;
                child
            };
            placements.push((items[k].1.clone(), child));
        }
        current = if horizontal {
            Rect::new(current.x, current.y + depth, current.width, current.height - depth)
        } else {
            Rect::new(current.x + depth, current.y, current.width - depth, current.height)
        };
    }
    placements
}

/// The worst aspect ratio of a row spanning `shorter` (`WorstRatio`): the larger of the ratios of
/// its biggest and its smallest tile, written through the row depth so that large areas do not
/// overflow the squares of the source.
fn worst_ratio(max: f64, min: f64, shorter: f64, sum: f64) -> f64 {
    let depth = sum / shorter;
    (max / depth / depth).max(depth / (min / depth))
}

pub struct TreemapChartEngine;
impl TreemapChartEngine {
    /// The tiles of one series laid out in `rect`, in the order the source emits them: each
    /// level squarified in descending weight, a node followed by its descendants.
    pub fn layout(nodes: &[TreemapNode], rect: Rect, padding: f64) -> Vec<TreemapTile> {
        let flat = flatten(nodes);
        let mut tiles = Vec::new();
        let roots: Vec<_> = flat.roots.iter().map(|&i| (flat.nodes[i].value, i)).collect();
        let mut pending = vec![squarify(&roots, rect).into_iter()];
        while let Some(level) = pending.last_mut() {
            let Some((index, allocated_bounds)) = level.next() else {
                pending.pop();
                continue;
            };
            let entry = &flat.nodes[index];
            // Inset by the padding so that siblings keep a gap; a tile thinner than twice the
            // padding collapses to a line.
            let bounds = Rect::new(
                allocated_bounds.x + padding,
                allocated_bounds.y + padding,
                (allocated_bounds.width - padding * 2.0).max(0.0),
                (allocated_bounds.height - padding * 2.0).max(0.0),
            );
            tiles.push(TreemapTile {
                node_index: index,
                parent_index: entry.parent,
                name: entry.node.name.clone(),
                value: entry.value,
                is_leaf: entry.node.children.is_none(),
                allocated_bounds,
                bounds,
            });
            if entry.node.children.is_some() {
                let children: Vec<_> = entry
                    .children
                    .iter()
                    .map(|&i| (flat.nodes[i].value, i))
                    .collect();
                pending.push(squarify(&children, bounds).into_iter());
            }
        }
        tiles
    }

    pub fn measure(
        size: Size,
        series: &[TreemapSeries],
        options: &TreemapOptions,
    ) -> Result<Scene, ChartError> {
        Self::measure_with_labels(size, series, options, |text, size| {
            Size::new(text.chars().count() as f64 * size * 0.6, size * 1.2)
        })
    }

    /// Native hosts supply their text metrics here, so that labels are culled with the typeface
    /// that draws them.
    pub fn measure_with_labels(
        size: Size,
        series: &[TreemapSeries],
        options: &TreemapOptions,
        mut measure_text: impl FnMut(&str, f64) -> Size,
    ) -> Result<Scene, ChartError> {
        validate_size(size)?;
        let mut plot = match options.draw_margin {
            Some(margin) => {
                if [margin.x, margin.y, margin.width, margin.height]
                    .iter()
                    .any(|v| !v.is_finite())
                {
                    return Err(ChartError::InvalidData("invalid treemap draw margin".into()));
                }
                margin
            }
            None => Rect::new(0.0, 0.0, size.width, size.height),
        };
        for s in series {
            validate_series(&s.series)?;
        }
        // A collapsed draw margin hides the plot (HidePlotZones); it is a valid frame.
        plot.width = plot.width.max(0.0);
        plot.height = plot.height.max(0.0);
        let mut scene = Scene::new(size, plot);
        scene.automatic_finding_strategy = FindingStrategy::CompareAll;
        if plot.width <= 0.0 || plot.height <= 0.0 {
            return Ok(scene);
        }
        // The draw margin is squarified between the visible series by their totals; a visible
        // series without a slot (no positive total) falls back to the whole draw margin.
        let weights: Vec<_> = series
            .iter()
            .enumerate()
            .filter(|(_, s)| s.series.visible)
            .map(|(i, s)| (s.total_weight(), i))
            .collect();
        let assigned = squarify(&weights, plot);
        let mut layers: Vec<(f64, Primitive)> = Vec::new();
        for (si, s) in series.iter().enumerate().filter(|(_, s)| s.series.visible) {
            let rect = assigned
                .iter()
                .find(|(i, r)| *i == si && r.width > 0.0 && r.height > 0.0)
                .map_or(plot, |(_, r)| *r);
            let paint = &s.series;
            scene.legend.push(LegendItem {
                series: si,
                name: if paint.name.is_empty() {
                    format!("Series {}", si + 1)
                } else {
                    paint.name.clone()
                },
                color: fill_color(paint, si),
            });
            let fill = fill_style(paint, si);
            let stroke = stroke_style(paint);
            let label_color = paint.data_labels_color.unwrap_or(DATA_LABELS_COLOR);
            // The z-indexes of the source paints: fills, then strokes, then the labels of every
            // series above all tiles.
            let z = si as f64;
            for tile in Self::layout(&s.nodes, rect, paint.padding) {
                let id = PointId {
                    series: si,
                    point: tile.node_index,
                };
                let geometry = Geometry::Rectangle {
                    rect: tile.bounds,
                    radius: paint.corner_radius,
                };
                if let Some(fill) = &fill {
                    layers.push((
                        z + 0.1,
                        Primitive::new(geometry.clone(), fill.clone()).identified(id),
                    ));
                }
                if let Some(stroke) = &stroke {
                    layers.push((
                        z + 0.2,
                        Primitive::new(geometry, stroke.clone()).identified(id),
                    ));
                }
                if !tile.is_leaf {
                    continue;
                }
                // Leaves get a hover area over the tile and the default tooltip text; internal
                // nodes are containers, so a hover does not hit a leaf and all its ancestors.
                let center = Point::new(
                    tile.bounds.x + tile.bounds.width * 0.5,
                    tile.bounds.y + tile.bounds.height * 0.5,
                );
                scene.hits.push(HitRegion {
                    id,
                    shape: HitShape::Rectangle(tile.bounds),
                    center,
                    value: tile.value,
                    label: tooltip_text(&tile.name, tile.value),
                });
                if !paint.data_labels || tile.name.is_empty() {
                    continue;
                }
                // A label that does not fit its tile (with the label padding) is culled.
                let text = text_size(&mut measure_text, &tile.name, paint.data_labels_size);
                if text.width + DATA_LABELS_PADDING.0 * 2.0 > tile.bounds.width
                    || text.height + DATA_LABELS_PADDING.1 * 2.0 > tile.bounds.height
                {
                    continue;
                }
                layers.push((
                    1000.0 + z + 0.3,
                    Primitive::new(
                        Geometry::Text {
                            origin: Point::new(center.x, center.y - text.height * 0.5),
                            text: tile.name.clone(),
                            size: paint.data_labels_size,
                            rotation: 0.0,
                            anchor: TextAnchor::Center,
                        },
                        PaintStyle::fill(label_color),
                    )
                    .identified(id),
                ));
            }
        }
        layers.sort_by(|a, b| a.0.total_cmp(&b.0));
        scene
            .primitives
            .extend(layers.into_iter().map(|(_, primitive)| primitive));
        Ok(scene)
    }
}

/// The default tooltip text of a node (`GetPrimaryToolTipText`): `<label>: <value>`, or the value
/// alone when the node has no label.
pub(crate) fn tooltip_text(label: &str, value: f64) -> String {
    if label.is_empty() {
        format!("{value}")
    } else {
        format!("{label}: {value}")
    }
}

/// The identity colour of a series that fills its shapes: its fill, or the colour of the theme's
/// palette for its position (the default theme gives the Sankey and treemap series a palette fill
/// and no stroke).
pub(crate) fn fill_color(s: &Series, index: usize) -> Color {
    s.fill_brush
        .as_ref()
        .and_then(|brush| representative_color(brush.as_ref()))
        .or(s.fill.filter(|fill| fill.a != 0))
        .unwrap_or_else(|| palette_color(index))
}

/// The fill of the shapes of a series, or `None` for an explicitly transparent fill (the source's
/// `Fill = null`).
pub(crate) fn fill_style(s: &Series, index: usize) -> Option<PaintStyle> {
    if s.fill_brush.is_none() && s.fill.is_some_and(|fill| fill.a == 0) {
        return None;
    }
    Some(PaintStyle {
        fill: Some(s.fill.unwrap_or_else(|| palette_color(index))),
        fill_brush: s.fill_brush.as_ref().map(snapshot_brush),
        brush_space: s.brush_space,
        opacity: s.opacity,
        ..PaintStyle::default()
    })
}

/// The stroke of the shapes of a series: only one it sets (the theme gives none).
pub(crate) fn stroke_style(s: &Series) -> Option<PaintStyle> {
    if s.stroke.is_none() && s.stroke_brush.is_none() {
        return None;
    }
    Some(PaintStyle {
        stroke: s.stroke,
        stroke_brush: s.stroke_brush.as_ref().map(snapshot_brush),
        brush_space: s.brush_space,
        stroke_width: s.stroke_width,
        stroke_cap: s.stroke_cap,
        stroke_join: s.stroke_join,
        stroke_miter: s.stroke_miter,
        opacity: s.opacity,
        dashes: s.dashes.clone(),
        dash_offset: s.dash_offset,
        ..PaintStyle::default()
    })
}

pub(crate) fn text_size(
    measure: &mut impl FnMut(&str, f64) -> Size,
    text: &str,
    size: f64,
) -> Size {
    let measured = measure(text, size);
    Size::new(
        if measured.width.is_finite() {
            measured.width.max(0.0)
        } else {
            0.0
        },
        if measured.height.is_finite() {
            measured.height.max(0.0)
        } else {
            0.0
        },
    )
}

pub(crate) fn validate_series(s: &Series) -> Result<(), ChartError> {
    if [
        s.padding,
        s.corner_radius,
        s.stroke_width,
        s.opacity,
        s.data_labels_size,
    ]
    .iter()
    .any(|v| !v.is_finite())
        || s.padding < 0.0
        || s.corner_radius < 0.0
        || s.data_labels_size < 0.0
    {
        return Err(ChartError::InvalidData(
            "series padding, radii, stroke and label sizes must be finite and nonnegative".into(),
        ));
    }
    Ok(())
}

struct FlatNode<'a> {
    node: &'a TreemapNode,
    parent: Option<usize>,
    children: Vec<usize>,
    /// The resolved weight (`ResolveValue`).
    value: f64,
}

struct Flat<'a> {
    nodes: Vec<FlatNode<'a>>,
    roots: Vec<usize>,
}

/// The nodes in preorder with their resolved weights: a nonzero explicit value (above .NET's
/// `double.Epsilon`, the smallest subnormal) is the weight, otherwise the sum of the children.
fn flatten(roots: &[TreemapNode]) -> Flat<'_> {
    let mut nodes: Vec<FlatNode<'_>> = Vec::new();
    let mut root_indexes = Vec::new();
    let mut pending: Vec<_> = roots.iter().rev().map(|node| (node, None)).collect();
    while let Some((node, parent)) = pending.pop() {
        let index = nodes.len();
        nodes.push(FlatNode {
            node,
            parent,
            children: Vec::new(),
            value: 0.0,
        });
        match parent {
            Some(parent) => nodes[parent].children.push(index),
            None => root_indexes.push(index),
        }
        if let Some(children) = &node.children {
            pending.extend(children.iter().rev().map(|child| (child, Some(index))));
        }
    }
    // Children follow their parent in preorder, so a reverse pass resolves them first.
    for index in (0..nodes.len()).rev() {
        let explicit = nodes[index].node.value;
        nodes[index].value = if explicit.abs() > f64::from_bits(1) {
            explicit
        } else {
            nodes[index].children.iter().map(|&i| nodes[i].value).sum()
        };
    }
    Flat {
        nodes,
        roots: root_indexes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rects_do_not_overlap(rects: &[Rect]) -> bool {
        rects.iter().enumerate().all(|(i, a)| {
            rects.iter().skip(i + 1).all(|b| {
                let x = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
                let y = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
                x <= 1e-9 || y <= 1e-9
            })
        })
    }

    fn series(nodes: Vec<TreemapNode>, padding: f64) -> TreemapSeries {
        let mut s = TreemapSeries::new(nodes);
        s.series.padding = padding;
        s.series.fill = Some(Color::from_rgb(96, 138, 218));
        s.series.stroke = Some(Color::from_rgb(255, 255, 255));
        s.series.stroke_width = 2.0;
        s
    }

    fn population() -> Vec<TreemapNode> {
        vec![
            TreemapNode::with_children(
                "Asia",
                vec![
                    TreemapNode::new(1400.0, "China"),
                    TreemapNode::new(1300.0, "India"),
                    TreemapNode::new(125.0, "Japan"),
                    TreemapNode::new(270.0, "Indonesia"),
                ],
            ),
            TreemapNode::with_children(
                "Europe",
                vec![
                    TreemapNode::new(83.0, "Germany"),
                    TreemapNode::new(67.0, "France"),
                    TreemapNode::new(60.0, "Italy"),
                    TreemapNode::new(46.0, "Spain"),
                ],
            ),
        ]
    }

    /// The fixture of SnapshotTests/TreemapSeriesTests.FlatBasic: every tile keeps the share of
    /// its weight in the area, inside the draw margin, without overlap, in descending order.
    #[test]
    fn flat_basic_squarifies_by_weight_without_overlap() {
        let input: Vec<_> = [40.0, 25.0, 15.0, 10.0, 7.0, 3.0]
            .into_iter()
            .enumerate()
            .map(|(i, w)| (w, i))
            .collect();
        let placements = squarify(&input, Rect::new(20.0, 30.0, 600.0, 600.0));
        assert_eq!(
            placements.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4, 5]
        );
        for (i, rect) in &placements {
            assert!((rect.width * rect.height / 360000.0 - input[*i].0 / 100.0).abs() < 1e-12);
            assert!(rect.x >= 20.0 - 1e-9 && rect.y >= 30.0 - 1e-9);
            assert!(rect.x + rect.width <= 620.0 + 1e-9 && rect.y + rect.height <= 630.0 + 1e-9);
        }
        let rects: Vec<_> = placements.iter().map(|(_, r)| *r).collect();
        assert!(rects_do_not_overlap(&rects));
        // The first row of a square spans its width: 40 and 25 share the top 65 % of it.
        assert_eq!(rects[0].y, 30.0);
        assert_eq!(rects[1].y, 30.0);
        assert!((rects[0].height - 390.0).abs() < 1e-9);
        assert!((rects[1].x - rects[0].x - rects[0].width).abs() < 1e-9);
    }

    /// SnapshotTests/TreemapSeriesTests.HierarchicalTwoLevel: internal nodes roll their children
    /// up, and every level is inset by the padding inside its parent.
    #[test]
    fn hierarchical_two_level_rolls_up_and_insets_every_level() {
        let mut roots = population();
        roots.push(TreemapNode::with_children(
            "Americas",
            vec![
                TreemapNode::new(330.0, "USA"),
                TreemapNode::new(220.0, "Brazil"),
                TreemapNode::new(130.0, "Mexico"),
            ],
        ));
        let s = series(roots, 4.0);
        assert_eq!(s.total_weight(), 3095.0 + 256.0 + 680.0);
        let tiles = TreemapChartEngine::layout(&s.nodes, Rect::new(0.0, 0.0, 600.0, 600.0), 4.0);
        assert_eq!(tiles.len(), 14);
        // Asia is the heaviest root: it comes first, followed by its children.
        assert_eq!(tiles[0].name, "Asia");
        assert_eq!(tiles[0].value, 3095.0);
        assert_eq!(tiles[1].name, "China");
        for tile in tiles.iter().filter(|t| t.parent_index.is_some()) {
            let parent = tiles
                .iter()
                .find(|p| Some(p.node_index) == tile.parent_index)
                .unwrap();
            let inner = parent.bounds;
            assert!(tile.bounds.x >= inner.x + 4.0 - 1e-9);
            assert!(tile.bounds.y >= inner.y + 4.0 - 1e-9);
            assert!(tile.bounds.x + tile.bounds.width <= inner.x + inner.width - 4.0 + 1e-9);
            assert!(tile.bounds.y + tile.bounds.height <= inner.y + inner.height - 4.0 + 1e-9);
        }
        let scene = TreemapChartEngine::measure(
            Size::new(600.0, 600.0),
            &[s],
            &TreemapOptions::default(),
        )
        .unwrap();
        // Hover areas on the eleven leaves only; a fill and a stroke for each of the 14 tiles.
        assert_eq!(scene.hits.len(), 11);
        assert_eq!(scene.primitives.len(), 28);
        assert!(scene.primitives[..14]
            .iter()
            .all(|p| p.style.fill.is_some() && p.style.stroke.is_none()));
        assert!(scene.primitives[14..]
            .iter()
            .all(|p| p.style.stroke.is_some() && p.style.fill.is_none()));
    }

    /// SnapshotTests/TreemapSeriesTests.HoverTooltip: the pointer at (150, 150) is over China,
    /// the largest leaf of the larger root, and the default formatter reads "China: 1400".
    #[test]
    fn hover_tooltip_finds_china_at_the_top_left() {
        let scene = TreemapChartEngine::measure(
            Size::new(600.0, 600.0),
            &[series(population(), 4.0)],
            &TreemapOptions::default(),
        )
        .unwrap();
        let hits = scene.hit_test(Point::new(150.0, 150.0), FindingStrategy::Automatic, 0.0);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].label, "China: 1400");
        assert_eq!(hits[0].value, 1400.0);
    }

    /// SnapshotTests/TreemapSeriesTests.WithDataLabels: labels are centred in their leaf, above
    /// every tile, and a tile too small for its text (with the label padding) has none.
    #[test]
    fn data_labels_are_centred_and_culled_when_they_do_not_fit() {
        let measure = |size: f64| {
            let mut s = series(
                vec![
                    TreemapNode::new(40.0, "Asia"),
                    TreemapNode::new(25.0, "Europe"),
                    TreemapNode::new(15.0, "Americas"),
                    TreemapNode::new(10.0, "Africa"),
                    TreemapNode::new(7.0, "Oceania"),
                    TreemapNode::new(3.0, "Antarctica"),
                ],
                4.0,
            );
            s.series.data_labels = true;
            s.series.data_labels_size = size;
            s.series.data_labels_color = Some(Color::from_rgb(255, 255, 255));
            TreemapChartEngine::measure(Size::new(600.0, 600.0), &[s], &TreemapOptions::default())
                .unwrap()
        };
        let labels = |scene: &Scene| -> Vec<(String, Point)> {
            scene
                .primitives
                .iter()
                .filter_map(|p| match &p.geometry {
                    Geometry::Text { origin, text, .. } => Some((text.clone(), *origin)),
                    _ => None,
                })
                .collect()
        };
        // The fixture's 18 px labels fit every tile of the 600 px square.
        let scene = measure(18.0);
        assert_eq!(labels(&scene).len(), 6);
        let asia = scene.hits.iter().find(|h| h.label == "Asia: 40").unwrap();
        let (_, origin) = labels(&scene)
            .into_iter()
            .find(|(t, _)| t == "Asia")
            .unwrap();
        assert!((origin.x - asia.center.x).abs() < 1e-9);
        let first_label = scene
            .primitives
            .iter()
            .position(|p| matches!(p.geometry, Geometry::Text { .. }))
            .unwrap();
        assert!(scene.primitives[first_label..]
            .iter()
            .all(|p| matches!(p.geometry, Geometry::Text { .. })));
        // At 40 px the labels of the two narrowest tiles no longer fit and are culled.
        let names: Vec<_> = labels(&measure(40.0)).into_iter().map(|(t, _)| t).collect();
        assert_eq!(names, vec!["Asia", "Europe", "Americas", "Africa"]);
    }

    /// SnapshotTests/TreemapSeriesTests.TwoSeriesWithTitleAndLegend: the draw margin is
    /// squarified between the series by their totals (153 and 92), each with its legend entry.
    #[test]
    fn two_series_partition_the_draw_margin_by_their_totals() {
        let a = series(
            vec![
                TreemapNode::new(60.0, "Americas"),
                TreemapNode::new(50.0, "EMEA"),
                TreemapNode::new(28.0, "APAC"),
                TreemapNode::new(15.0, "Other"),
            ],
            4.0,
        );
        let mut b = series(
            vec![
                TreemapNode::new(40.0, "Americas"),
                TreemapNode::new(28.0, "EMEA"),
                TreemapNode::new(16.0, "APAC"),
                TreemapNode::new(8.0, "Other"),
            ],
            4.0,
        );
        b.series.name = "2024".into();
        b.series.fill = Some(Color::from_rgb(232, 105, 105));
        let slots = squarify(
            &[(a.total_weight(), 0), (b.total_weight(), 1)],
            Rect::new(0.0, 0.0, 900.0, 600.0),
        );
        let ratio = slots[0].1.width * slots[0].1.height / (slots[1].1.width * slots[1].1.height);
        assert!((ratio - 153.0 / 92.0).abs() < 1e-12);
        let mut hidden = b.clone();
        hidden.series.visible = false;
        let scene = TreemapChartEngine::measure(
            Size::new(900.0, 600.0),
            &[a, b, hidden],
            &TreemapOptions::default(),
        )
        .unwrap();
        assert_eq!(scene.hits.len(), 8);
        assert_eq!(scene.legend.len(), 2);
        assert_eq!(scene.legend[1].name, "2024");
        assert_eq!(scene.legend[1].color, Color::from_rgb(232, 105, 105));
        // The tiles of the second series lie in the slot of the second series.
        let slot = slots[1].1;
        assert!(scene.hits.iter().filter(|h| h.id.series == 1).all(|h| {
            h.center.x >= slot.x && h.center.x <= slot.x + slot.width
        }));
    }

    #[test]
    fn explicit_parent_weight_and_empty_internal_node_keep_the_source_semantics() {
        let mut parent =
            TreemapNode::with_children("Override", vec![TreemapNode::new(20.0, "Child")]);
        parent.value = 100.0;
        let mut empty = TreemapNode::with_children("Empty", vec![]);
        empty.value = 50.0;
        let s = series(vec![parent, empty], 2.0);
        assert_eq!(s.total_weight(), 150.0);
        let scene = TreemapChartEngine::measure(
            Size::new(300.0, 300.0),
            &[s],
            &TreemapOptions::default(),
        )
        .unwrap();
        // The empty internal node draws a tile without a hover area.
        assert_eq!(scene.hits.len(), 1);
        assert_eq!(scene.hits[0].label, "Child: 20");
    }

    #[test]
    fn a_leaf_that_becomes_internal_loses_its_hover_area_and_label() {
        let mut s = series(vec![TreemapNode::new(10.0, "Former leaf")], 2.0);
        s.series.data_labels = true;
        let size = Size::new(300.0, 300.0);
        let scene = TreemapChartEngine::measure(size, &[s.clone()], &TreemapOptions::default())
            .unwrap();
        assert_eq!(scene.hits.len(), 1);
        s.nodes[0].children = Some(Vec::new());
        let scene = TreemapChartEngine::measure(size, &[s], &TreemapOptions::default()).unwrap();
        assert!(scene.hits.is_empty());
        assert!(!scene
            .primitives
            .iter()
            .any(|p| matches!(p.geometry, Geometry::Text { .. })));
    }

    #[test]
    fn invalid_weights_are_skipped_and_large_weights_do_not_overflow() {
        let placements = squarify(
            &[
                (f64::NAN, 0),
                (f64::INFINITY, 1),
                (-1.0, 2),
                (0.0, 3),
                (1e308, 4),
                (1e308, 5),
            ],
            Rect::new(0.0, 0.0, 100.0, 100.0),
        );
        assert_eq!(placements.len(), 2);
        assert_eq!(placements[0].1.width * placements[0].1.height, 5000.0);
        assert_eq!(placements[1].1.width * placements[1].1.height, 5000.0);
    }

    /// CollapsedDrawMarginAllChartsTests.Treemap_HidesThenResets: a draw margin collapsed past
    /// the control hides the plot, and a valid one draws it again.
    #[test]
    fn collapsed_draw_margin_hides_then_resets() {
        let s = TreemapSeries::new(vec![
            TreemapNode::new(1.0, ""),
            TreemapNode::new(2.0, ""),
            TreemapNode::new(3.0, ""),
        ]);
        let size = Size::new(300.0, 300.0);
        let collapsed = TreemapOptions {
            draw_margin: Some(Rect::new(200.0, 200.0, -100.0, -100.0)),
        };
        let scene = TreemapChartEngine::measure(size, &[s.clone()], &collapsed).unwrap();
        assert!(scene.plot_bounds.width <= 0.0 && scene.plot_bounds.height <= 0.0);
        assert!(scene.primitives.is_empty() && scene.hits.is_empty());
        let scene =
            TreemapChartEngine::measure(size, &[s], &TreemapOptions::default()).unwrap();
        assert_eq!(scene.hits.len(), 3);
        // The theme's palette fill and no stroke.
        assert_eq!(scene.primitives.len(), 3);
        assert_eq!(scene.primitives[0].style.fill, Some(palette_color(0)));
    }

    #[test]
    fn native_text_metrics_cull_labels() {
        let mut s = series(vec![TreemapNode::new(5.0, "Too wide")], 2.0);
        s.series.data_labels = true;
        let scene = TreemapChartEngine::measure_with_labels(
            Size::new(100.0, 100.0),
            &[s],
            &TreemapOptions::default(),
            |_, _| Size::new(200.0, 14.0),
        )
        .unwrap();
        assert!(!scene
            .primitives
            .iter()
            .any(|p| matches!(p.geometry, Geometry::Text { .. })));
    }
}
