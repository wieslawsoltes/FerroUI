//! Portable vector-region paint replacement. Original paths remain the paint
//! coordinate reference; masks remove later overrides without rescaling them.
use super::*;

pub(super) fn paint_has_alpha(paint: &Paint) -> bool {
    match paint {
        Paint::Solid(color) => color.a != 0,
        Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. }
            | Paint::AngularGradient { stops, .. } => stops.iter().any(|stop| stop.color.a != 0),
        Paint::Image { .. } => true,
    }
}

pub(super) fn whiten_vector_fill(fill: &mut Fill) {
    fill.visible &= fill.opacity > 0.0 && paint_has_alpha(&fill.paint);
    fill.paint = Paint::Solid(Color::WHITE);
    fill.opacity = 1.0;
}

fn shape(path: &VectorPath, attributes: &str) -> String {
    let rule = if path.fill_rule == FillRule::EvenOdd { "evenodd" } else { "nonzero" };
    format!(r#"<path d="{}" fill-rule="{rule}" clip-rule="{rule}" {attributes}/>"#,
        path_data(path, Transform::IDENTITY))
}

struct RegionPaint<'a> {
    id: VectorRegionId,
    path: VectorPath,
    fills: &'a [Fill],
}

/// Suffix cutters use a balanced reusable union tree. N overrides create O(N)
/// definitions and O(N log N) references, instead of copying each later path
/// into every preceding region's mask. Nested source-reference depth is log N.
struct CutTree {
    node: NodeId,
    count: usize,
}

impl CutTree {
    fn build(node: NodeId, regions: &[RegionPaint<'_>], defs: &mut String) -> Self {
        let tree = Self { node, count: regions.len() };
        if !regions.is_empty() { tree.define(regions, 0, regions.len(), 1, defs); }
        tree
    }

    fn name(&self, key: usize) -> String { format!("network-cut-{}-{key}", self.node) }

    fn define(&self, regions: &[RegionPaint<'_>], start: usize, end: usize, key: usize, defs: &mut String) {
        let name = self.name(key);
        if end - start == 1 {
            defs.push_str(&shape(&regions[start].path, &format!(r#"id="{name}" fill="black""#)));
        } else {
            let middle = start + (end - start) / 2;
            self.define(regions, start, middle, key * 2, defs);
            self.define(regions, middle, end, key * 2 + 1, defs);
            let _ = write!(defs, r##"<g id="{name}"><use href="#{}"/><use href="#{}"/></g>"##,
                self.name(key * 2), self.name(key * 2 + 1));
        }
    }

    fn suffix(&self, start: usize, end: usize, from: usize, key: usize, output: &mut String) {
        if from >= end { return; }
        if from <= start {
            let _ = write!(output, r##"<use href="#{}"/>"##, self.name(key));
            return;
        }
        let middle = start + (end - start) / 2;
        self.suffix(start, middle, from, key * 2, output);
        self.suffix(middle, end, from, key * 2 + 1, output);
    }

    fn mask(&self, from: usize, id: &str, bounds: Rect, defs: &mut String) -> bool {
        if from >= self.count { return false; }
        let bounds = bounds.inflate(1.0);
        let _ = write!(defs, r#"<mask id="{id}" maskUnits="userSpaceOnUse" maskContentUnits="userSpaceOnUse" mask-type="luminance" color-interpolation="sRGB" x="{}" y="{}" width="{}" height="{}"><rect x="{}" y="{}" width="{}" height="{}" fill="white"/>"#,
            number(bounds.x), number(bounds.y), number(bounds.width), number(bounds.height),
            number(bounds.x), number(bounds.y), number(bounds.width), number(bounds.height));
        self.suffix(0, self.count, from, 1, defs);
        defs.push_str("</mask>");
        true
    }
}

fn fills(node: &Node, path: &VectorPath, stack: &[Fill], prefix: &str, defs: &mut String, body: &mut String) {
    let bounds = path.tight_bounds().unwrap_or_else(|| node.local_bounds());
    // First paint is topmost, matching the native prepared paint stacks.
    for (index, fill) in stack.iter().enumerate().rev().filter(|(_, fill)| fill.visible && fill.opacity > 0.0) {
        let id = format!("{prefix}-{index}");
        paint_definition_in_bounds(&fill.paint, &id, node, None, Some(bounds), defs);
        body.push_str(&shape(path, &format!(r#"fill="{}" fill-opacity="{}" stroke="none""#,
            paint_color(&fill.paint, &id), number(paint_opacity(&fill.paint, fill.opacity)))));
    }
}

pub(super) fn export_network_fills(node: &Node, network: &VectorNetwork, defs: &mut String, body: &mut String) {
    let regions: Vec<_> = network.regions.iter().filter_map(|(&id, region)| {
        Some(RegionPaint { id, path: network.region_fill_path(id)?, fills: region.fills.as_ref()? })
    }).collect();
    let cutters = CutTree::build(node.id, &regions, defs);
    let inherited = network.inherited_fill_path();
    if !inherited.segments.is_empty() && node.fills.iter().any(|fill| fill.visible && fill.opacity > 0.0) {
        let original = network.fill_path();
        let bounds = original.tight_bounds().unwrap_or_else(|| node.local_bounds());
        let clip_id = format!("network-inherited-{}", node.id);
        let _ = write!(defs, r#"<clipPath id="{clip_id}" clipPathUnits="userSpaceOnUse">{}</clipPath>"#, shape(&inherited, ""));
        let cut_id = format!("network-inherited-cut-{}", node.id);
        let masked = cutters.mask(0, &cut_id, bounds, defs);
        let _ = write!(body, r#"<g clip-path="url(#{clip_id})"{}>"#,
            if masked { format!(r#" mask="url(#{cut_id})""#) } else { String::new() });
        fills(node, &original, &node.fills, &format!("fill-{}", node.id), defs, body);
        body.push_str("</g>");
    }
    for (index, region) in regions.iter().enumerate() {
        if !region.fills.iter().any(|fill| fill.visible && fill.opacity > 0.0) { continue; }
        let bounds = region.path.tight_bounds().unwrap_or_else(|| node.local_bounds());
        let cut_id = format!("network-region-cut-{}-{}", node.id, region.id);
        let masked = cutters.mask(index + 1, &cut_id, bounds, defs);
        if masked { let _ = write!(body, r#"<g mask="url(#{cut_id})">"#); }
        fills(node, &region.path, region.fills, &format!("region-{}-{}", node.id, region.id), defs, body);
        if masked { body.push_str("</g>"); }
    }
}

#[cfg(test)]
#[path = "svg_regions_tests.rs"]
mod tests;
