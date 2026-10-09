use ferroui_base::platform::{IPlatformRenderInterfaceRegion, LtrbPixelRect, LtrbRect};
use ferroui_base::Point;
use kurbo::BezPath;
use std::any::Any;
use std::cell::RefCell;

/// An implementation of a platform region: a set of pixels held as the
/// rectangles that were added to it.
///
/// The Vello project has no region type (Skia has `SkRegion`, which the
/// Skia backend wraps): the set operations a region needs are written out
/// here, over bands of rows as a region of Skia has them.
pub struct VelloRegionImpl {
    /// The rectangles as they were added; they may overlap.
    added: RefCell<Option<Vec<LtrbPixelRect>>>,
    /// The region as rectangles that do not overlap, in bands from the top,
    /// each band from the left.
    rects: RefCell<Option<Vec<LtrbPixelRect>>>,
}

impl VelloRegionImpl {
    /// Creates an empty region.
    pub fn new() -> Self {
        Self { added: RefCell::new(Some(Vec::new())), rects: RefCell::new(None) }
    }

    fn with_added<R>(&self, f: impl FnOnce(&mut Vec<LtrbPixelRect>) -> R) -> R {
        let mut added = self.added.borrow_mut();
        f(added.as_mut().expect("VelloRegionImpl has been disposed"))
    }

    /// The area of the region as a path: one closed figure per rectangle of
    /// [`rects`](IPlatformRenderInterfaceRegion::rects). The figures do not
    /// overlap, so either fill rule fills the region.
    pub fn path(&self) -> BezPath {
        let mut path = BezPath::new();
        for rect in self.rects() {
            let (left, top, right, bottom) = (rect.left as f64, rect.top as f64, rect.right as f64, rect.bottom as f64);
            path.move_to((left, top));
            path.line_to((right, top));
            path.line_to((right, bottom));
            path.line_to((left, bottom));
            path.close_path();
        }
        path
    }

    /// Decomposes rectangles that may overlap into bands of rectangles that
    /// do not: between two neighbouring edges of rows the covered columns
    /// are the same for every row, and bands that cover the same columns
    /// and touch are one band.
    fn decompose(added: &[LtrbPixelRect]) -> Vec<LtrbPixelRect> {
        let mut edges: Vec<i32> = added.iter().flat_map(|rect| [rect.top, rect.bottom]).collect();
        edges.sort_unstable();
        edges.dedup();

        // The spans of the band above, to join bands with.
        let mut previous: Vec<(i32, i32)> = Vec::new();
        let mut previous_start = 0usize;
        let mut rects: Vec<LtrbPixelRect> = Vec::new();

        for band in edges.windows(2) {
            let (top, bottom) = (band[0], band[1]);

            let mut spans: Vec<(i32, i32)> = added
                .iter()
                .filter(|rect| rect.top <= top && rect.bottom >= bottom)
                .map(|rect| (rect.left, rect.right))
                .collect();
            spans.sort_unstable();

            let mut merged: Vec<(i32, i32)> = Vec::new();
            for (left, right) in spans {
                match merged.last_mut() {
                    Some(last) if left <= last.1 => last.1 = last.1.max(right),
                    _ => merged.push((left, right)),
                }
            }

            let touches = rects.get(previous_start).is_some_and(|rect| rect.bottom == top);
            if !merged.is_empty() && merged == previous && touches {
                for rect in &mut rects[previous_start..] {
                    rect.bottom = bottom;
                }
            } else {
                previous_start = rects.len();
                rects.extend(merged.iter().map(|(left, right)| LtrbPixelRect::new(*left, top, *right, bottom)));
            }
            previous = merged;
        }

        rects
    }
}

impl Default for VelloRegionImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl IPlatformRenderInterfaceRegion for VelloRegionImpl {
    fn add_rect(&self, rect: LtrbPixelRect) {
        *self.rects.borrow_mut() = None;
        if !rect.is_empty() {
            self.with_added(|added| added.push(rect));
        }
    }

    fn reset(&self) {
        *self.rects.borrow_mut() = None;
        self.with_added(Vec::clear);
    }

    fn is_empty(&self) -> bool {
        self.with_added(|added| added.is_empty())
    }

    fn bounds(&self) -> LtrbPixelRect {
        self.with_added(|added| {
            added.iter().copied().reduce(|bounds, rect| bounds.union(rect)).unwrap_or(LtrbPixelRect::new(0, 0, 0, 0))
        })
    }

    fn rects(&self) -> Vec<LtrbPixelRect> {
        let mut rects = self.rects.borrow_mut();
        rects.get_or_insert_with(|| self.with_added(|added| Self::decompose(added))).clone()
    }

    fn intersects(&self, rect: LtrbRect) -> bool {
        let (left, top) = (rect.left as i32, rect.top as i32);
        let (right, bottom) = (rect.right.ceil() as i32, rect.bottom.ceil() as i32);
        if left >= right || top >= bottom {
            return false;
        }

        self.with_added(|added| {
            added.iter().any(|r| r.left < right && left < r.right && r.top < bottom && top < r.bottom)
        })
    }

    fn contains(&self, pt: Point) -> bool {
        let (x, y) = (pt.x as i32, pt.y as i32);
        self.with_added(|added| added.iter().any(|r| x >= r.left && x < r.right && y >= r.top && y < r.bottom))
    }

    fn dispose(&self) {
        *self.added.borrow_mut() = None;
        *self.rects.borrow_mut() = None;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
