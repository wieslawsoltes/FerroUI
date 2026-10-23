//! Retained region paints using the public geometry, clip and brush APIs.
//!
//! Explicit stacks replace inherited paint coverage, including empty stacks.
//! Later stable region IDs replace earlier overrides where authored contours
//! overlap. Clipping the original path keeps its brush coordinate system fixed
//! when another region changes; gradients and image crops must not jump.
use super::*;

#[cfg(all(test, not(target_os = "emscripten")))]
#[path = "region_scaling_tests.rs"]
mod scaling_tests;

struct CachedRegion {
    geometry: Ref<Geometry>,
    clip: Option<Ref<Geometry>>,
    fills: Vec<Rc<dyn IBrush>>,
    has_alpha: bool,
}

pub(super) struct CachedNetworkFills {
    inherited_clip: Ref<Geometry>,
    regions: Vec<CachedRegion>,
    #[cfg(test)]
    bounds_tests: usize,
    #[cfg(test)]
    cutter_pairs: usize,
}

#[derive(Clone, Copy)]
struct BoundsLeaf {
    slot: usize,
    rect: Option<Rect>,
}

#[derive(Clone, Copy)]
enum BoundsKind {
    Region(usize),
    Branch(usize, usize),
}

struct BoundsEntry {
    // Unknown/nonfinite bounds cannot prove disjointness.
    rect: Option<Rect>,
    last: usize,
    kind: BoundsKind,
}

/// A preparation-only balanced broad phase. A uniform cell size could put an
/// arbitrarily large number of tiny disjoint faces in one bucket. Spatial
/// median partitioning bounds tree depth without assuming a document scale.
struct RegionBoundsIndex {
    entries: Vec<BoundsEntry>,
    #[cfg(test)]
    bounds_tests: Cell<usize>,
}

impl RegionBoundsIndex {
    fn new(bounds: &[Option<Rect>]) -> Self {
        let mut index = Self {
            entries: Vec::with_capacity(bounds.len().saturating_mul(2).saturating_sub(1)),
            #[cfg(test)]
            bounds_tests: Cell::new(0),
        };
        let mut leaves: Vec<_> = bounds.iter().enumerate()
            .map(|(slot, &rect)| BoundsLeaf { slot, rect }).collect();
        if !leaves.is_empty() {
            index.build(&mut leaves);
        }
        index
    }

    fn build(&mut self, leaves: &mut [BoundsLeaf]) -> usize {
        let rect = leaves.iter().skip(1).fold(leaves[0].rect, |rect, leaf| {
            rect.zip(leaf.rect).and_then(|(a, b)| finite_bounds(a.union(b)))
        });
        let last = leaves.iter().map(|leaf| leaf.slot).max().unwrap();
        let entry = self.entries.len();
        self.entries.push(BoundsEntry {
            rect,
            last,
            kind: BoundsKind::Region(leaves[0].slot),
        });
        if leaves.len() > 1 {
            let horizontal = rect.is_none_or(|rect| rect.width >= rect.height);
            let coordinate = |leaf: &BoundsLeaf| leaf.rect.map_or(0.0, |rect| {
                if horizontal { rect.x + rect.width * 0.5 } else { rect.y + rect.height * 0.5 }
            });
            let middle = leaves.len() / 2;
            leaves.select_nth_unstable_by(middle, |a, b| {
                coordinate(a).total_cmp(&coordinate(b)).then(a.slot.cmp(&b.slot))
            });
            let (left, right) = leaves.split_at_mut(middle);
            let left = self.build(left);
            let right = self.build(right);
            self.entries[entry].kind = BoundsKind::Branch(left, right);
        }
        entry
    }

    fn later_intersections(&self, rect: Option<Rect>, after: usize, output: &mut Vec<usize>) {
        output.clear();
        if !self.entries.is_empty() {
            self.query(0, rect, after, output);
        }
        // Keep Boolean grouping deterministic in stable region order even
        // though the broad phase partitions by position rather than paint order.
        output.sort_unstable();
    }

    fn query(&self, entry: usize, rect: Option<Rect>, after: usize, output: &mut Vec<usize>) {
        #[cfg(test)]
        self.bounds_tests.set(self.bounds_tests.get() + 1);
        let entry = &self.entries[entry];
        if entry.last <= after || entry.rect.zip(rect).is_some_and(|(a, b)| !a.intersects(b)) {
            return;
        }
        match entry.kind {
            BoundsKind::Region(slot) => output.push(slot),
            BoundsKind::Branch(left, right) => {
                self.query(left, rect, after, output);
                self.query(right, rect, after, output);
            }
        }
    }
}

fn finite_bounds(bounds: Rect) -> Option<Rect> {
    (bounds.is_finite() && bounds.width >= 0.0 && bounds.height >= 0.0
        && bounds.right().is_finite() && bounds.bottom().is_finite()).then_some(bounds)
}

/// Boolean trees must also be balanced when the platform geometry is lazy:
/// a retained linear Union chain otherwise recurses once per region on its
/// first evaluation and stores every progressively larger suffix path.
fn balanced_union(geometries: &[Ref<Geometry>]) -> Option<Ref<Geometry>> {
    match geometries.len() {
        0 => None,
        1 => Some(geometries[0].clone()),
        count => {
            let middle = count / 2;
            Some(combined(GeometryCombineMode::Union,
                balanced_union(&geometries[..middle]).unwrap(),
                balanced_union(&geometries[middle..]).unwrap()))
        }
    }
}

pub(super) fn prepare(
    node: &Node,
    geometry: &Ref<Geometry>,
    images: &mut HashMap<String, ImageAsset>,
    missing_images: &mut usize,
) -> Option<CachedNetworkFills> {
    let network = node.vector_network.as_ref()?;
    if !network.regions.values().any(|region| region.fills.is_some()) {
        return None;
    }
    let inherited = path_geometry(&network.inherited_fill_path());
    let mut regions = Vec::new();
    let mut paths = Vec::new();
    let mut bounds = Vec::new();
    // Empty or hidden stacks still reserve coverage; clearing one face must
    // not reveal the layer's old fill underneath.
    for (&id, region) in &network.regions {
        let Some(stack) = region.fills.as_ref() else { continue; };
        let Some(path) = network.region_fill_path(id) else { continue; };
        let path = path_geometry(&path);
        bounds.push(finite_bounds(core_rect(path.bounds())));
        paths.push(path.clone());
        let mut fills = Vec::new();
        for fill in stack.iter().filter(|fill| fill.visible && fill.opacity > 0.0) {
            let (brush, missing) = make_brush(&fill.paint, fill.opacity, images);
            *missing_images += usize::from(missing);
            fills.push(brush as Rc<dyn IBrush>);
        }
        regions.push(CachedRegion {
            geometry: path,
            clip: None,
            fills,
            has_alpha: stack.iter().any(|fill| {
                fill.visible && fill.opacity > 0.0 && masks::paint_has_alpha(&fill.paint)
            }),
        });
    }
    let index = RegionBoundsIndex::new(&bounds);
    let mut candidates = Vec::new();
    let mut cutters = Vec::new();
    #[cfg(test)]
    let mut cutter_pairs = 0;
    for (slot, region) in regions.iter_mut().enumerate() {
        if slot + 1 == paths.len() { continue; }
        index.later_intersections(bounds[slot], slot, &mut candidates);
        #[cfg(test)]
        { cutter_pairs += candidates.len(); }
        cutters.clear();
        cutters.extend(candidates.iter().map(|&slot| paths[slot].clone()));
        region.clip = Some(match balanced_union(&cutters) {
            Some(cutters) => combined(GeometryCombineMode::Exclude, region.geometry.clone(), cutters),
            // Retain the original redundant clip's antialiased boundary when
            // every later footprint is disjoint. The brush still targets the
            // unmodified original path, never the clipped geometry.
            None => region.geometry.clone(),
        });
    }
    let inherited_clip = match balanced_union(&paths) {
        Some(overrides) => combined(GeometryCombineMode::Exclude, inherited, overrides),
        None => geometry.clone(),
    };
    Some(CachedNetworkFills {
        inherited_clip,
        regions,
        #[cfg(test)]
        bounds_tests: index.bounds_tests.get(),
        #[cfg(test)]
        cutter_pairs,
    })
}

impl CachedNetworkFills {
    pub(super) fn paint(
        &self,
        context: &mut DrawingContext<'_>,
        geometry: &Ref<Geometry>,
        inherited: &[Rc<dyn IBrush>],
    ) {
        if !inherited.is_empty() {
            let clip = context.push_geometry_clip(&self.inherited_clip);
            for fill in inherited.iter().rev() {
                context.draw_geometry(Some(fill), None, geometry);
            }
            context.pop(clip);
        }
        for region in &self.regions {
            if region.fills.is_empty() { continue; }
            let clip = region.clip.as_ref().map(|geometry| context.push_geometry_clip(geometry))
                .unwrap_or(PushedState::NONE);
            for fill in region.fills.iter().rev() {
                context.draw_geometry(Some(fill), None, &region.geometry);
            }
            context.pop(clip);
        }
    }

    pub(super) fn vector_mask(&self, node: &Node, _geometry: &Ref<Geometry>) -> Ref<Geometry> {
        let mut parts = Vec::with_capacity(self.regions.len() + 1);
        if node.fills.iter().any(|fill| {
            fill.visible && fill.opacity > 0.0 && masks::paint_has_alpha(&fill.paint)
        }) {
            parts.push(self.inherited_clip.clone());
        }
        for region in &self.regions {
            if region.has_alpha {
                parts.push(region.clip.as_ref().unwrap_or(&region.geometry).clone());
            }
        }
        balanced_union(&parts).unwrap_or_else(|| StreamGeometry::new().upcast())
    }
}
