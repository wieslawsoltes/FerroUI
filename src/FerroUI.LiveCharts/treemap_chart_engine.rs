//! Squarified weighted rectangles, hierarchical tiles, and leaf interaction.
// Ported from LiveChartsCore/TreemapLayout.cs, CoreTreemapSeries.cs and TreemapChartEngine.cs.
// Copyright (c) 2021 Alberto Rodriguez Orozco & LiveCharts Contributors. MIT.

use crate::scene::{
    palette_color, validate_size, ChartError, Geometry, HitRegion, HitShape, LegendItem,
    PaintStyle, PointId, Primitive, Scene, TextAnchor,
};
use ferroui_base::{media::Color, Point, Rect, Size};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreemapNode {
    pub id: u64,
    pub name: String,
    /// A nonzero explicit value takes precedence over the sum of the children.
    pub value: f64,
    /// `None` is a leaf; `Some(Vec::new())` is an empty internal node.
    pub children: Option<Vec<TreemapNode>>,
    pub color: Option<Color>,
}
impl TreemapNode {
    pub fn new(id: u64, name: impl Into<String>, value: f64) -> Self {
        Self {
            id,
            name: name.into(),
            value,
            children: None,
            color: None,
        }
    }
    pub fn branch(id: u64, name: impl Into<String>, children: Vec<Self>) -> Self {
        Self {
            id,
            name: name.into(),
            children: Some(children),
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreemapOptions {
    pub padding: f64,
    pub corner_radius: f64,
    pub show_labels: bool,
    pub label_size: f64,
    pub label_color: Color,
    pub stroke: Option<Color>,
    pub stroke_width: f64,
}
impl Default for TreemapOptions {
    fn default() -> Self {
        Self {
            padding: 2.0,
            corner_radius: 0.0,
            show_labels: false,
            label_size: 14.0,
            label_color: Color::from_argb(255, 255, 255, 255),
            stroke: Some(Color::from_argb(255, 255, 255, 255)),
            stroke_width: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreemapSeries {
    pub name: String,
    pub roots: Vec<TreemapNode>,
    pub options: TreemapOptions,
    pub color: Option<Color>,
    pub visible: bool,
}
impl Default for TreemapSeries {
    fn default() -> Self {
        Self {
            name: String::new(),
            roots: Vec::new(),
            options: TreemapOptions::default(),
            color: None,
            visible: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreemapTile {
    pub node_id: u64,
    /// Preorder position, stable for a fixed tree and independent of weight sorting.
    pub node_index: usize,
    pub parent_index: Option<usize>,
    /// Child indexes from the roots down to this node.
    pub path: Vec<usize>,
    pub name: String,
    pub value: f64,
    pub color: Option<Color>,
    pub is_leaf: bool,
    pub allocated_bounds: Rect,
    pub bounds: Rect,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreemapLayout {
    pub tiles: Vec<TreemapTile>,
    pub total_weight: f64,
}

struct FlatNode<'a> {
    node: &'a TreemapNode,
    parent: Option<usize>,
    children: Vec<usize>,
    path: Vec<usize>,
    value: f64,
    color: Option<Color>,
}

/// The pure squarifier is also used to assign rectangles to multiple series.
pub fn squarify<T: Clone>(items: &[(f64, T)], rect: Rect) -> Result<Vec<(T, Rect)>, ChartError> {
    validate_rect(rect)?;
    if rect.width == 0.0 || rect.height == 0.0 {
        return Ok(Vec::new());
    }
    let mut items: Vec<_> = items
        .iter()
        .filter(|(weight, _)| weight.is_finite() && *weight > 0.0)
        .collect();
    if items.is_empty() {
        return Ok(Vec::new());
    }
    items.sort_by(|a, b| b.0.total_cmp(&a.0));
    // Normalization retains ratios while avoiding overflow when summing large weights.
    let largest = items[0].0;
    let total: f64 = items.iter().map(|(weight, _)| *weight / largest).sum();
    let area = rect.width * rect.height;
    if !area.is_finite() {
        return Err(ChartError::InvalidSize);
    }
    let areas: Vec<_> = items
        .iter()
        .map(|(weight, _)| (*weight / largest / total) * area)
        .collect();
    let mut result = Vec::with_capacity(items.len());
    let mut index = 0;
    let mut current = rect;
    while index < items.len() {
        let shorter = current.width.min(current.height);
        if shorter <= 0.0 || areas[index] == 0.0 {
            // A finite positive value may underflow after normalization; its visible area is zero.
            for (_, item) in &items[index..] {
                result.push((item.clone(), Rect::new(current.x, current.y, 0.0, 0.0)));
            }
            break;
        }
        let start = index;
        let mut sum = 0.0;
        let mut min: f64 = f64::INFINITY;
        let mut max: f64 = 0.0;
        let mut worst = f64::INFINITY;
        while index < items.len() {
            let area = areas[index];
            if area == 0.0 {
                break;
            }
            let new_sum = sum + area;
            let new_min = min.min(area);
            let new_max = max.max(area);
            let new_worst = worst_ratio(new_max, new_min, shorter, new_sum);
            if index != start && new_worst > worst {
                break;
            }
            sum = new_sum;
            min = new_min;
            max = new_max;
            worst = new_worst;
            index += 1;
        }
        let horizontal = current.width <= current.height;
        let depth = (sum / shorter).min(if horizontal {
            current.height
        } else {
            current.width
        });
        let mut cursor = if horizontal { current.x } else { current.y };
        for k in start..index {
            // The final member closes its row exactly, avoiding seams caused by repeated rounding.
            let extent = if k + 1 == index {
                (if horizontal {
                    current.x + current.width
                } else {
                    current.y + current.height
                }) - cursor
            } else {
                areas[k] / depth
            };
            let child = if horizontal {
                Rect::new(cursor, current.y, extent.max(0.0), depth)
            } else {
                Rect::new(current.x, cursor, depth, extent.max(0.0))
            };
            result.push((items[k].1.clone(), child));
            cursor += extent;
        }
        if horizontal {
            current.y += depth;
            current.height = (current.height - depth).max(0.0);
        } else {
            current.x += depth;
            current.width = (current.width - depth).max(0.0);
        }
    }
    Ok(result)
}

fn worst_ratio(max: f64, min: f64, shorter: f64, sum: f64) -> f64 {
    // Algebraically identical to upstream's squared-area formula without unnecessary overflow.
    let depth = sum / shorter;
    (max / depth / depth).max(depth / (min / depth))
}

pub struct TreemapChartEngine;
impl TreemapChartEngine {
    pub fn layout(
        roots: &[TreemapNode],
        rect: Rect,
        options: &TreemapOptions,
    ) -> Result<TreemapLayout, ChartError> {
        validate_rect(rect)?;
        validate_options(options)?;
        let (flat, root_indexes) = flatten(roots);
        let total_weight = root_indexes.iter().map(|i| flat[*i].value).sum();
        let mut result = TreemapLayout {
            tiles: Vec::new(),
            total_weight,
        };
        let root_weights: Vec<_> = root_indexes.iter().map(|i| (flat[*i].value, *i)).collect();
        let mut pending = squarify(&root_weights, rect)?;
        pending.reverse();
        while let Some((index, allocated_bounds)) = pending.pop() {
            let entry = &flat[index];
            let padding = options.padding;
            let bounds = Rect::new(
                allocated_bounds.x + padding,
                allocated_bounds.y + padding,
                (allocated_bounds.width - 2.0 * padding).max(0.0),
                (allocated_bounds.height - 2.0 * padding).max(0.0),
            );
            result.tiles.push(TreemapTile {
                node_id: entry.node.id,
                node_index: index,
                parent_index: entry.parent,
                path: entry.path.clone(),
                name: entry.node.name.clone(),
                value: entry.value,
                color: entry.color,
                is_leaf: entry.node.children.is_none(),
                allocated_bounds,
                bounds,
            });
            if !entry.children.is_empty() {
                let weights: Vec<_> = entry
                    .children
                    .iter()
                    .map(|i| (flat[*i].value, *i))
                    .collect();
                let children = squarify(&weights, bounds)?;
                pending.extend(children.into_iter().rev());
            }
        }
        Ok(result)
    }

    pub fn total_weight(roots: &[TreemapNode]) -> f64 {
        let (flat, indexes) = flatten(roots);
        indexes.iter().map(|i| flat[*i].value).sum()
    }

    pub fn measure(
        roots: &[TreemapNode],
        size: Size,
        options: &TreemapOptions,
    ) -> Result<Scene, ChartError> {
        Self::measure_with_labels(roots, size, options, estimated_text_size)
    }

    /// Hosts can supply their native text measurement service for exact label culling.
    pub fn measure_with_labels(
        roots: &[TreemapNode],
        size: Size,
        options: &TreemapOptions,
        mut measure_text: impl FnMut(&str, f64) -> Size,
    ) -> Result<Scene, ChartError> {
        validate_size(size)?;
        let bounds = Rect::new(0.0, 0.0, size.width, size.height);
        let layout = Self::layout(roots, bounds, options)?;
        let mut scene = Scene::new(size, bounds);
        append_layout(&mut scene, &layout, 0, options, None, &mut measure_text);
        Ok(scene)
    }

    pub fn measure_series(series: &[TreemapSeries], size: Size) -> Result<Scene, ChartError> {
        Self::measure_series_with_labels(series, size, estimated_text_size)
    }

    pub fn measure_series_with_labels(
        series: &[TreemapSeries],
        size: Size,
        mut measure_text: impl FnMut(&str, f64) -> Size,
    ) -> Result<Scene, ChartError> {
        validate_size(size)?;
        let bounds = Rect::new(0.0, 0.0, size.width, size.height);
        let weights: Vec<_> = series
            .iter()
            .enumerate()
            .filter(|(_, s)| s.visible)
            .map(|(i, s)| (Self::total_weight(&s.roots), i))
            .collect();
        let mut scene = Scene::new(size, bounds);
        for (index, rect) in squarify(&weights, bounds)? {
            let series = &series[index];
            let layout = Self::layout(&series.roots, rect, &series.options)?;
            let color = series.color.unwrap_or_else(|| palette_color(index));
            append_layout(
                &mut scene,
                &layout,
                index,
                &series.options,
                Some(color),
                &mut measure_text,
            );
            scene.legend.push(LegendItem {
                series: index,
                name: series.name.clone(),
                color,
            });
        }
        Ok(scene)
    }
}

fn flatten(roots: &[TreemapNode]) -> (Vec<FlatNode<'_>>, Vec<usize>) {
    let mut flat: Vec<FlatNode<'_>> = Vec::new();
    let mut root_indexes = Vec::new();
    let mut pending: Vec<_> = roots
        .iter()
        .enumerate()
        .rev()
        .map(|(i, node)| (node, None, vec![i], None))
        .collect();
    while let Some((node, parent, path, inherited_color)) = pending.pop() {
        let index = flat.len();
        let color = node.color.or(inherited_color);
        flat.push(FlatNode {
            node,
            parent,
            children: Vec::new(),
            path: path.clone(),
            value: 0.0,
            color,
        });
        if let Some(parent) = parent {
            flat[parent].children.push(index);
        } else {
            root_indexes.push(index);
        }
        if let Some(children) = &node.children {
            for (child_index, child) in children.iter().enumerate().rev() {
                let mut child_path = path.clone();
                child_path.push(child_index);
                pending.push((child, Some(index), child_path, color));
            }
        }
    }
    for index in (0..flat.len()).rev() {
        let explicit = flat[index].node.value;
        // .NET double.Epsilon is the smallest positive subnormal, not machine epsilon.
        flat[index].value = if explicit.abs() > f64::from_bits(1) {
            explicit
        } else {
            flat[index].children.iter().map(|i| flat[*i].value).sum()
        };
    }
    (flat, root_indexes)
}

fn append_layout(
    scene: &mut Scene,
    layout: &TreemapLayout,
    series: usize,
    options: &TreemapOptions,
    series_color: Option<Color>,
    measure_text: &mut impl FnMut(&str, f64) -> Size,
) {
    for tile in &layout.tiles {
        let color = tile
            .color
            .or(series_color)
            .unwrap_or_else(|| palette_color(tile.path[0]));
        let style = PaintStyle {
            fill: Some(color),
            stroke: options.stroke,
            stroke_width: options.stroke_width,
            ..PaintStyle::default()
        };
        let id = PointId {
            series,
            point: tile.node_index,
        };
        scene.primitives.push(
            Primitive::new(
                Geometry::Rectangle {
                    rect: tile.bounds,
                    radius: options.corner_radius,
                },
                style,
            )
            .clipped(scene.plot_bounds)
            .identified(id),
        );
        if !tile.is_leaf || tile.bounds.width == 0.0 || tile.bounds.height == 0.0 {
            continue;
        }
        let center = Point::new(
            tile.bounds.x + tile.bounds.width * 0.5,
            tile.bounds.y + tile.bounds.height * 0.5,
        );
        let label = if tile.name.is_empty() {
            tile.value.to_string()
        } else {
            format!("{}: {}", tile.name, tile.value)
        };
        scene.hits.push(HitRegion {
            id,
            shape: HitShape::Rectangle(tile.bounds),
            center,
            value: tile.value,
            label,
        });
        if options.show_labels && !tile.name.is_empty() {
            let measured = measure_text(&tile.name, options.label_size);
            if measured.width.is_finite()
                && measured.height.is_finite()
                && measured.width <= tile.bounds.width
                && measured.height <= tile.bounds.height
            {
                let origin = Point::new(center.x, center.y - measured.height * 0.5);
                scene.primitives.push(
                    Primitive::new(
                        Geometry::Text {
                            origin,
                            text: tile.name.clone(),
                            size: options.label_size,
                            rotation: 0.0,
                            anchor: TextAnchor::Center,
                        },
                        PaintStyle::fill(options.label_color),
                    )
                    .clipped(tile.bounds),
                );
            }
        }
    }
}

fn estimated_text_size(text: &str, size: f64) -> Size {
    Size::new(text.chars().count() as f64 * size * 0.6, size * 1.2)
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
fn validate_options(options: &TreemapOptions) -> Result<(), ChartError> {
    if [
        options.padding,
        options.corner_radius,
        options.label_size,
        options.stroke_width,
    ]
    .iter()
    .any(|x| !x.is_finite() || *x < 0.0)
    {
        return Err(ChartError::InvalidData(
            "Treemap padding, radii, label size and stroke width must be finite and nonnegative"
                .into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flat_basic_tiles_preserve_relative_area_without_overlap() {
        // Data from SnapshotTests/TreemapSeriesTests.FlatBasic.
        let input: Vec<_> = [40.0, 25.0, 15.0, 10.0, 7.0, 3.0]
            .into_iter()
            .enumerate()
            .map(|(i, w)| (w, i))
            .collect();
        let placements = squarify(&input, Rect::new(20.0, 30.0, 600.0, 600.0)).unwrap();
        assert_eq!(placements.len(), 6);
        for (i, rect) in &placements {
            assert!((rect.width * rect.height / 360000.0 - input[*i].0 / 100.0).abs() < 1e-12);
            assert!(
                rect.x >= 20.0
                    && rect.y >= 30.0
                    && rect.x + rect.width <= 620.0 + 1e-9
                    && rect.y + rect.height <= 630.0 + 1e-9
            );
        }
        for (i, (_, a)) in placements.iter().enumerate() {
            for (_, b) in placements.iter().skip(i + 1) {
                let overlap_x = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
                let overlap_y = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
                assert!(overlap_x <= 1e-9 || overlap_y <= 1e-9);
            }
        }
    }
    fn population() -> Vec<TreemapNode> {
        vec![
            TreemapNode::branch(
                0,
                "Asia",
                vec![
                    TreemapNode::new(1, "China", 1400.0),
                    TreemapNode::new(2, "India", 1300.0),
                    TreemapNode::new(3, "Japan", 125.0),
                    TreemapNode::new(4, "Indonesia", 270.0),
                ],
            ),
            TreemapNode::branch(
                5,
                "Europe",
                vec![
                    TreemapNode::new(6, "Germany", 83.0),
                    TreemapNode::new(7, "France", 67.0),
                    TreemapNode::new(8, "Italy", 60.0),
                    TreemapNode::new(9, "Spain", 46.0),
                ],
            ),
        ]
    }
    #[test]
    fn hierarchical_two_level_rolls_up_values_and_insets_every_level() {
        // Fixture from SnapshotTests/TreemapSeriesTests.HierarchicalTwoLevel and HoverTooltip.
        let options = TreemapOptions {
            padding: 4.0,
            ..TreemapOptions::default()
        };
        let layout =
            TreemapChartEngine::layout(&population(), Rect::new(0.0, 0.0, 600.0, 600.0), &options)
                .unwrap();
        assert_eq!(layout.total_weight, 3351.0);
        assert_eq!(layout.tiles.len(), 10);
        let asia = layout.tiles.iter().find(|t| t.node_id == 0).unwrap();
        assert_eq!(asia.value, 3095.0);
        for child in layout
            .tiles
            .iter()
            .filter(|t| t.parent_index == Some(asia.node_index))
        {
            assert!(child.bounds.x >= asia.bounds.x + 4.0 - 1e-9);
            assert!(child.bounds.y >= asia.bounds.y + 4.0 - 1e-9);
            assert!(
                child.bounds.x + child.bounds.width
                    <= asia.bounds.x + asia.bounds.width - 4.0 + 1e-9
            );
        }
        let scene =
            TreemapChartEngine::measure(&population(), Size::new(600.0, 600.0), &options).unwrap();
        assert_eq!(scene.hits.len(), 8);
        assert!(scene
            .hits
            .iter()
            .any(|h| h.label == "China: 1400" && h.contains(Point::new(150.0, 150.0))));
    }
    #[test]
    fn explicit_parent_weight_and_empty_internal_node_preserve_semantics() {
        let mut parent =
            TreemapNode::branch(0, "Override", vec![TreemapNode::new(1, "Child", 20.0)]);
        parent.value = 100.0;
        let mut empty = TreemapNode::branch(2, "Empty", vec![]);
        empty.value = 50.0;
        let roots = vec![parent, empty];
        let scene = TreemapChartEngine::measure(
            &roots,
            Size::new(300.0, 300.0),
            &TreemapOptions::default(),
        )
        .unwrap();
        assert_eq!(TreemapChartEngine::total_weight(&roots), 150.0);
        assert_eq!(scene.hits.len(), 1);
        assert_eq!(scene.hits[0].label, "Child: 20");
    }
    #[test]
    fn leaf_to_internal_remeasurement_drops_old_hover_and_label() {
        let options = TreemapOptions {
            show_labels: true,
            ..TreemapOptions::default()
        };
        let leaf = TreemapNode::new(0, "Former leaf", 10.0);
        let scene = TreemapChartEngine::measure(&[leaf.clone()], Size::new(300.0, 300.0), &options)
            .unwrap();
        assert_eq!(scene.hits.len(), 1);
        let branch = TreemapNode {
            children: Some(vec![]),
            ..leaf
        };
        let scene =
            TreemapChartEngine::measure(&[branch], Size::new(300.0, 300.0), &options).unwrap();
        assert!(scene.hits.is_empty());
        assert!(!scene
            .primitives
            .iter()
            .any(|p| matches!(p.geometry, Geometry::Text { .. })));
    }
    #[test]
    fn two_series_partition_by_total_and_hide_invisible_series() {
        // Totals from SnapshotTests/TreemapSeriesTests.TwoSeriesWithTitleAndLegend.
        let a = TreemapSeries {
            name: "2023".into(),
            roots: vec![
                TreemapNode::new(0, "Americas", 60.0),
                TreemapNode::new(1, "EMEA", 50.0),
                TreemapNode::new(2, "APAC", 28.0),
                TreemapNode::new(3, "Other", 15.0),
            ],
            ..TreemapSeries::default()
        };
        let b = TreemapSeries {
            name: "2024".into(),
            roots: vec![
                TreemapNode::new(4, "Americas", 40.0),
                TreemapNode::new(5, "EMEA", 28.0),
                TreemapNode::new(6, "APAC", 16.0),
                TreemapNode::new(7, "Other", 8.0),
            ],
            ..TreemapSeries::default()
        };
        let slots = squarify(
            &[
                (TreemapChartEngine::total_weight(&a.roots), 0),
                (TreemapChartEngine::total_weight(&b.roots), 1),
            ],
            Rect::new(0.0, 0.0, 900.0, 600.0),
        )
        .unwrap();
        assert!(
            (slots[0].1.width * slots[0].1.height / (slots[1].1.width * slots[1].1.height)
                - 153.0 / 92.0)
                .abs()
                < 1e-12
        );
        let mut hidden = b.clone();
        hidden.visible = false;
        let scene =
            TreemapChartEngine::measure_series(&[a, b, hidden], Size::new(900.0, 600.0)).unwrap();
        assert_eq!(scene.hits.len(), 8);
        assert_eq!(scene.legend.len(), 2);
    }
    #[test]
    fn filters_invalid_weights_and_handles_large_finite_weights() {
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
        )
        .unwrap();
        assert_eq!(placements.len(), 2);
        assert_eq!(placements[0].1.width * placements[0].1.height, 5000.0);
        assert_eq!(placements[1].1.width * placements[1].1.height, 5000.0);
    }
    #[test]
    fn native_label_measurement_culls_labels_that_do_not_fit() {
        let options = TreemapOptions {
            show_labels: true,
            ..TreemapOptions::default()
        };
        let scene = TreemapChartEngine::measure_with_labels(
            &[TreemapNode::new(0, "Too wide", 5.0)],
            Size::new(100.0, 100.0),
            &options,
            |_, _| Size::new(200.0, 14.0),
        )
        .unwrap();
        assert!(!scene
            .primitives
            .iter()
            .any(|p| matches!(p.geometry, Geometry::Text { .. })));
    }
}
