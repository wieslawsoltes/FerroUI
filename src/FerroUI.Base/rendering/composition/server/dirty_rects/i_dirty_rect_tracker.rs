use crate::platform::{IDrawingContextImpl, LtrbPixelRect, LtrbRect};
use smallvec::SmallVec;
use std::cell::Cell;
use std::hash::{BuildHasher, Hasher};

/// Accumulates the dirty area of a frame and clips drawing to it.
///
/// Trackers are render-thread objects: every member takes `&self` and the
/// implementations use interior mutability.
pub trait IDirtyRectTracker: IDirtyRectCollector {
    /// Post-processes the dirty rect area (e. g. to account for anti-aliasing)
    fn finalize_frame(&self, bounds: LtrbRect);

    /// Adds a rectangle to the dirty area `finalize_frame` made final: an
    /// area that is drawn whole once a part of it is. `intersects` answers
    /// for the final area, so such an area is tested after `finalize_frame`
    /// and added with this. Not in the C# source (desktop-performance.md,
    /// section 9.54).
    fn include(&self, rect: LtrbRect, bounds: LtrbRect);

    /// Pushes the dirty area as a clip onto `ctx`.
    ///
    /// The C# member returns a disposable that pops the clip. Drawing
    /// contexts are passed around as `&mut dyn IDrawingContextImpl` here, so
    /// the pair is explicit instead: every `begin_draw` must be followed by
    /// exactly one [`end_draw`](Self::end_draw) on the same context once
    /// drawing is done.
    fn begin_draw(&self, ctx: &mut dyn IDrawingContextImpl);

    /// Pops the clip pushed by [`begin_draw`](Self::begin_draw).
    fn end_draw(&self, ctx: &mut dyn IDrawingContextImpl);

    fn is_empty(&self) -> bool;

    fn intersects(&self, rect: LtrbRect) -> bool;

    fn initialize(&self, bounds: LtrbRect);

    /// Draws the dirty area in a random color (debug overlay).
    fn visualize(&self, context: &mut dyn IDrawingContextImpl);

    fn combined_rect(&self) -> LtrbRect;

    /// Whether the tracker is a `SingleDirtyRectTracker` (the `is` test of
    /// the render pass, which needs no per-visual test with one).
    fn is_single_dirty_rect_tracker(&self) -> bool {
        false
    }
}

/// Receives invalidated rectangles.
pub trait IDirtyRectCollector {
    fn add_rect(&self, rect: LtrbRect);
}

// --- helpers shared by the trackers -----------------------------------------
//
// The following are members of `LtrbRect` / `LtrbPixelRect` in the C# source
// that the `platform` module does not have yet
// (see `patches/0001-platform-ltrb-rect-members.patch`).

/// C# `LtrbRect.FullUnion(LtrbRect?, LtrbRect?)`.
pub(super) fn full_union(left: Option<LtrbRect>, right: Option<LtrbRect>) -> Option<LtrbRect> {
    match (left, right) {
        (None, right) => right,
        (left, None) => left,
        (Some(left), Some(right)) => Some(right.union(left)),
    }
}

/// C# `LtrbRect.Contains(LtrbRect)`.
pub(super) fn contains_rect(outer: &LtrbRect, rect: LtrbRect) -> bool {
    rect.left >= outer.left && rect.right <= outer.right && rect.top >= outer.top && rect.bottom <= outer.bottom
}

/// C# `LtrbPixelRect.FromRectUnscaled`: left/top are truncated towards zero,
/// right/bottom are rounded up.
pub(super) fn pixel_rect_from_rect_unscaled(rect: LtrbRect) -> LtrbPixelRect {
    LtrbPixelRect::new(rect.left as i32, rect.top as i32, rect.right.ceil() as i32, rect.bottom.ceil() as i32)
}

/// C# `LtrbPixelRect.ToLtrbRectUnscaled`.
pub(super) fn pixel_rect_to_ltrb_rect_unscaled(rect: LtrbPixelRect) -> LtrbRect {
    LtrbRect::new(rect.left as f64, rect.top as f64, rect.right as f64, rect.bottom as f64)
}

// --- what a frame draws for its dirty rectangles ----------------------------
//
// Not in the C# source (desktop-performance.md, section 9.54). The dirty
// rectangles are there to draw less. A set of them that leaves out little
// costs a backend more than it saves, and so does one rectangle that is all
// but the surface.

/// The share of their union the dirty rectangles have to fill for the union
/// to be drawn in their place. A clip of several rectangles is not a
/// rectangle to a backend, and every draw inside it pays for that.
pub(super) const UNION_SHARE: f64 = 0.875;

/// The share of the surface one dirty rectangle has to cover for the whole
/// surface to be drawn: a frame that draws all of its surface clears it,
/// and keeps nothing of the frame before it.
pub(super) const SURFACE_SHARE: f64 = 0.96;

fn area(rect: LtrbRect) -> f64 {
    rect.width().max(0.0) * rect.height().max(0.0)
}

/// The area the rectangles cover together, each point counted once.
fn covered_area(rects: &[LtrbRect]) -> f64 {
    let mut xs: SmallVec<[f64; 32]> = rects.iter().flat_map(|rect| [rect.left, rect.right]).collect();
    let mut ys: SmallVec<[f64; 32]> = rects.iter().flat_map(|rect| [rect.top, rect.bottom]).collect();
    xs.sort_by(f64::total_cmp);
    ys.sort_by(f64::total_cmp);
    let mut covered = 0.0;
    for x in xs.windows(2).filter(|x| x[0] < x[1]) {
        for y in ys.windows(2).filter(|y| y[0] < y[1]) {
            let inside = |rect: &LtrbRect| rect.left <= x[0] && x[1] <= rect.right && rect.top <= y[0] && y[1] <= rect.bottom;
            if rects.iter().any(inside) {
                covered += (x[1] - x[0]) * (y[1] - y[0]);
            }
        }
    }
    covered
}

/// What a frame draws for one dirty rectangle: the surface, when the
/// rectangle covers [`SURFACE_SHARE`] of it, and the rectangle otherwise.
pub(super) fn surface_or_rect(rect: LtrbRect, bounds: LtrbRect) -> LtrbRect {
    let surface = area(bounds);
    if surface > 0.0 && area(rect.intersect_or_empty(bounds)) >= SURFACE_SHARE * surface {
        bounds
    } else {
        rect
    }
}

/// Replaces the dirty rectangles of a frame, which are inside `bounds`,
/// with their union when they fill [`UNION_SHARE`] of it, and one rectangle
/// with the surface when it covers [`SURFACE_SHARE`] of that. The area drawn
/// only grows.
pub(super) fn simplify_dirty_rects(rects: &mut Vec<LtrbRect>, bounds: LtrbRect) {
    if rects.len() > 1 {
        let union = rects.iter().filter(|rect| !rect.is_zero_size()).copied().reduce(|all, rect| all.union(rect));
        if let Some(union) = union {
            let needed = UNION_SHARE * area(union);
            // The sum is the least to compute, and the rectangles cover no
            // more than it.
            if rects.iter().map(|rect| area(*rect)).sum::<f64>() >= needed && covered_area(rects) >= needed {
                rects.clear();
                rects.push(union);
            }
        }
    }
    if let [rect] = rects.as_mut_slice() {
        *rect = surface_or_rect(*rect, bounds);
    }
}

/// The pseudo random number source of the `visualize` overlays (C#
/// `System.Random`): the colors only have to differ from frame to frame.
pub(super) struct VisualizeRandom {
    state: Cell<u64>,
}

impl VisualizeRandom {
    pub(super) fn new() -> Self {
        // Seeded from the per-process hasher keys; no clock involved.
        let seed = std::collections::hash_map::RandomState::new().build_hasher().finish();
        Self { state: Cell::new(seed | 1) }
    }

    /// A value in `0..max_value` (C# `Random.Next(maxValue)`).
    pub(super) fn next(&self, max_value: i32) -> i32 {
        // xorshift64*
        let mut x = self.state.get();
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state.set(x);
        let value = x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 33;
        if max_value <= 0 {
            0
        } else {
            (value % max_value as u64) as i32
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::media::{BoxShadows, Color, IBrush, IPen, RenderOptions};
    use crate::platform::{
        IBitmapImpl, IDrawingContextLayerImpl, IGeometryImpl, IGlyphRunImpl, IPlatformRenderInterfaceRegion,
    };
    use crate::{Matrix, PixelSize, Point, Rect, RoundedRect};
    use std::any::{Any, TypeId};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;

    /// A drawing context that records the calls it receives.
    pub(crate) struct MockDrawingContext {
        pub log: Vec<String>,
        pub transform: Matrix,
    }

    impl Default for MockDrawingContext {
        fn default() -> Self {
            Self { log: Vec::new(), transform: Matrix::IDENTITY }
        }
    }

    fn describe_brush(brush: Option<&dyn IBrush>) -> String {
        match brush {
            None => "none".to_string(),
            Some(brush) => match brush.as_solid_color_brush() {
                Some(solid) => format!("#{:08x}@{}", solid.color().to_uint32(), brush.opacity()),
                None => "brush".to_string(),
            },
        }
    }

    impl IDrawingContextImpl for MockDrawingContext {
        fn transform(&self) -> Matrix {
            self.transform
        }
        fn set_transform(&mut self, value: Matrix) {
            self.transform = value;
        }
        fn clear(&mut self, _color: Color) {
            self.log.push("clear".to_string());
        }
        fn draw_bitmap(&mut self, _: &dyn IBitmapImpl, _: f64, _: Rect, _: Rect) {
            self.log.push("bitmap".to_string());
        }
        fn draw_bitmap_with_mask(&mut self, _: &dyn IBitmapImpl, _: &dyn IBrush, _: Rect, _: Rect) {
            self.log.push("bitmap_mask".to_string());
        }
        fn draw_line(&mut self, _: Option<&dyn IPen>, p1: Point, p2: Point) {
            self.log.push(format!("line {p1} {p2}"));
        }
        fn draw_geometry(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, _: &dyn IGeometryImpl) {
            let pen = pen.map(|p| describe_brush(p.brush().as_deref())).unwrap_or("none".to_string());
            self.log.push(format!(
                "geometry {} pen {pen} at {},{}",
                describe_brush(brush),
                self.transform.m31,
                self.transform.m32
            ));
        }
        fn draw_rectangle(
            &mut self,
            brush: Option<&dyn IBrush>,
            _pen: Option<&dyn IPen>,
            rect: RoundedRect,
            _box_shadows: &BoxShadows,
        ) {
            self.log.push(format!("rect {} {}", describe_brush(brush), rect.rect));
        }
        fn draw_region(
            &mut self,
            brush: Option<&dyn IBrush>,
            _pen: Option<&dyn IPen>,
            region: &dyn IPlatformRenderInterfaceRegion,
        ) {
            let alpha = brush.and_then(|b| b.as_solid_color_brush()).map(|b| b.color().a).unwrap_or(0);
            self.log.push(format!("region alpha {alpha} rects {}", region.rects().len()));
        }
        fn draw_ellipse(&mut self, _: Option<&dyn IBrush>, _: Option<&dyn IPen>, _: Rect) {
            self.log.push("ellipse".to_string());
        }
        fn draw_glyph_run(&mut self, foreground: Option<&dyn IBrush>, glyph_run: &dyn IGlyphRunImpl) {
            use crate::rendering::composition::server::diagnostic_text_renderer::tests::MockTextGlyphRunImpl;

            // A run of shaped diagnostic text is logged with its text.
            if let Some(text) = glyph_run.as_any().downcast_ref::<MockTextGlyphRunImpl>() {
                self.log.push(format!(
                    "text {} {} at {},{}",
                    text.0,
                    describe_brush(foreground),
                    self.transform.m31,
                    self.transform.m32
                ));
                return;
            }

            self.log.push(format!(
                "glyph {} {} at {},{}",
                glyph_run.font_rendering_em_size(),
                describe_brush(foreground),
                self.transform.m31,
                self.transform.m32
            ));
        }
        fn create_layer(&mut self, _: PixelSize) -> Rc<dyn IDrawingContextLayerImpl> {
            unimplemented!()
        }
        fn push_clip(&mut self, clip: Rect) {
            self.log.push(format!("push_clip {clip}"));
        }
        fn push_clip_rounded(&mut self, clip: RoundedRect) {
            self.log.push(format!("push_clip_rounded {}", clip.rect));
        }
        fn push_clip_region(&mut self, region: &dyn IPlatformRenderInterfaceRegion) {
            self.log.push(format!("push_clip_region {:?}", region.rects()));
        }
        fn pop_clip(&mut self) {
            self.log.push("pop_clip".to_string());
        }
        fn push_layer(&mut self, _: Rect) {}
        fn pop_layer(&mut self) {}
        fn push_opacity(&mut self, _: f64, _: Option<Rect>) {}
        fn pop_opacity(&mut self) {}
        fn push_opacity_mask(&mut self, _: &dyn IBrush, _: Rect) {}
        fn pop_opacity_mask(&mut self) {}
        fn push_geometry_clip(&mut self, _: &dyn IGeometryImpl) {}
        fn pop_geometry_clip(&mut self) {}
        fn push_render_options(&mut self, _: RenderOptions) {}
        fn pop_render_options(&mut self) {}
        fn push_text_options(&mut self, _: crate::media::TextOptions) {}
        fn pop_text_options(&mut self) {}
        fn get_feature(&mut self, _: TypeId) -> Option<Rc<dyn Any>> {
            None
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
        fn dispose(&mut self) {}
    }

    /// A region that keeps the list of rectangles added to it.
    #[derive(Default)]
    pub(crate) struct MockRegion {
        pub rects: RefCell<Vec<LtrbPixelRect>>,
    }

    impl IPlatformRenderInterfaceRegion for MockRegion {
        fn add_rect(&self, rect: LtrbPixelRect) {
            self.rects.borrow_mut().push(rect);
        }
        fn reset(&self) {
            self.rects.borrow_mut().clear();
        }
        fn is_empty(&self) -> bool {
            self.rects.borrow().is_empty()
        }
        fn bounds(&self) -> LtrbPixelRect {
            let rects = self.rects.borrow();
            let mut iter = rects.iter();
            match iter.next() {
                None => LtrbPixelRect::default(),
                Some(first) => iter.fold(*first, |acc, r| acc.union(*r)),
            }
        }
        fn rects(&self) -> Vec<LtrbPixelRect> {
            self.rects.borrow().clone()
        }
        fn intersects(&self, rect: LtrbRect) -> bool {
            self.rects.borrow().iter().any(|r| pixel_rect_to_ltrb_rect_unscaled(*r).intersects(rect))
        }
        fn contains(&self, pt: Point) -> bool {
            self.rects.borrow().iter().any(|r| pixel_rect_to_ltrb_rect_unscaled(*r).contains(pt.x, pt.y))
        }
        fn dispose(&self) {}
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    /// A stream geometry built by a [`MockStreamGeometryBuilder`], with the
    /// log of the figure commands it was built from.
    pub(crate) struct MockStreamGeometry {
        pub log: Arc<std::sync::Mutex<Vec<String>>>,
    }

    /// A stream geometry builder that records the figure commands it
    /// receives.
    struct MockStreamGeometryBuilder {
        log: Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl crate::platform::IGeometryContext for MockStreamGeometryBuilder {
        fn arc_to(&mut self, _: Point, _: crate::Size, _: f64, _: bool, _: crate::media::SweepDirection, _: bool) {
            self.log.lock().unwrap().push("arc".to_string());
        }
        fn begin_figure(&mut self, start_point: Point, is_filled: bool) {
            self.log.lock().unwrap().push(format!("begin {start_point} {is_filled}"));
        }
        fn cubic_bezier_to(&mut self, _: Point, _: Point, _: Point, _: bool) {
            self.log.lock().unwrap().push("cubic".to_string());
        }
        fn quadratic_bezier_to(&mut self, _: Point, _: Point, _: bool) {
            self.log.lock().unwrap().push("quad".to_string());
        }
        fn line_to(&mut self, point: Point, is_stroked: bool) {
            self.log.lock().unwrap().push(format!("line {point} {is_stroked}"));
        }
        fn end_figure(&mut self, is_closed: bool) {
            self.log.lock().unwrap().push(format!("end {is_closed}"));
        }
        fn set_fill_rule(&mut self, _: crate::media::FillRule) {
            self.log.lock().unwrap().push("fill".to_string());
        }
        fn dispose(&mut self) {
            self.log.lock().unwrap().push("dispose".to_string());
        }
    }

    impl crate::platform::IStreamGeometryBuilder for MockStreamGeometryBuilder {
        fn to_geometry(&mut self) -> Arc<dyn IGeometryImpl> {
            Arc::new(MockStreamGeometry { log: self.log.clone() })
        }
    }

    impl IGeometryImpl for MockStreamGeometry {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn bounds(&self) -> Rect {
            Rect::default()
        }
        fn contour_length(&self) -> f64 {
            0.0
        }
        fn get_render_bounds(&self, _pen: Option<&dyn IPen>) -> Rect {
            Rect::default()
        }
        fn get_widened_geometry(&self, _pen: &dyn IPen) -> Arc<dyn IGeometryImpl> {
            unimplemented!()
        }
        fn fill_contains(&self, _point: Point) -> bool {
            false
        }
        fn get_fill_intersection_result(&self, _geometry: &dyn IGeometryImpl) -> crate::media::IntersectionResult {
            crate::media::IntersectionResult::Empty
        }
        fn intersect(&self, _geometry: &dyn IGeometryImpl) -> Option<Arc<dyn IGeometryImpl>> {
            None
        }
        fn stroke_contains(&self, _pen: Option<&dyn IPen>, _point: Point) -> bool {
            false
        }
        fn try_get_point_at_distance(&self, _distance: f64) -> Option<Point> {
            None
        }
        fn try_get_point_and_tangent_at_distance(&self, _distance: f64) -> Option<(Point, Point)> {
            None
        }
        fn try_get_segment(&self, _start: f64, _stop: f64, _begin: bool) -> Option<Arc<dyn IGeometryImpl>> {
            None
        }
        fn with_transform(&self, _: Matrix) -> Arc<dyn crate::platform::ITransformedGeometryImpl> {
            unimplemented!()
        }
    }

    /// A render interface that creates [`MockRegion`]s and
    /// [`MockStreamGeometry`]s and keeps track of them.
    #[derive(Default)]
    pub(crate) struct MockRenderInterface {
        pub regions: RefCell<Vec<Rc<MockRegion>>>,
        pub streams: RefCell<Vec<Arc<std::sync::Mutex<Vec<String>>>>>,
    }

    impl crate::platform::IPlatformRenderInterface for MockRenderInterface {
        fn build_glyph_run_geometry(&self, _: &crate::media::GlyphRun) -> Arc<dyn IGeometryImpl> {
            unimplemented!()
        }
        fn create_glyph_run(
            &self,
            _: &Rc<crate::media::GlyphTypeface>,
            _: f64,
            _: &[crate::media::text_formatting::GlyphInfo],
            _: Point,
        ) -> std::sync::Arc<dyn crate::platform::IGlyphRunImpl> {
            unimplemented!()
        }
        fn create_ellipse_geometry(&self, _: Rect) -> Arc<dyn IGeometryImpl> {
            unimplemented!()
        }
        fn create_line_geometry(&self, _: Point, _: Point) -> Arc<dyn IGeometryImpl> {
            unimplemented!()
        }
        fn create_rectangle_geometry(&self, _: Rect) -> Arc<dyn IGeometryImpl> {
            unimplemented!()
        }
        fn create_stream_geometry_builder(
            &self,
            _source: Option<&dyn IGeometryImpl>,
        ) -> Box<dyn crate::platform::IStreamGeometryBuilder> {
            let log = Arc::new(std::sync::Mutex::new(Vec::new()));
            self.streams.borrow_mut().push(log.clone());
            Box::new(MockStreamGeometryBuilder { log })
        }
        fn create_geometry_group(&self, _: crate::media::FillRule, _: &[Arc<dyn IGeometryImpl>]) -> Arc<dyn IGeometryImpl> {
            unimplemented!()
        }
        fn create_combined_geometry(
            &self,
            _: crate::media::GeometryCombineMode,
            _: Arc<dyn IGeometryImpl>,
            _: Arc<dyn IGeometryImpl>,
        ) -> Arc<dyn IGeometryImpl> {
            unimplemented!()
        }
        fn create_render_target_bitmap(
            &self,
            _: crate::PixelSize,
            _: crate::Vector,
        ) -> std::sync::Arc<dyn crate::platform::IRenderTargetBitmapImpl> {
            unimplemented!()
        }
        fn create_writeable_bitmap(
            &self,
            _: crate::PixelSize,
            _: crate::Vector,
            _: crate::platform::PixelFormat,
            _: crate::platform::AlphaFormat,
        ) -> std::sync::Arc<dyn crate::platform::IWriteableBitmapImpl> {
            unimplemented!()
        }
        fn load_bitmap_from_file(&self, _: &str) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
            unimplemented!()
        }
        fn load_bitmap(&self, _: &mut dyn std::io::Read) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
            unimplemented!()
        }
        fn load_writeable_bitmap_to_width(
            &self,
            _: &mut dyn std::io::Read,
            _: i32,
            _: crate::media::imaging::BitmapInterpolationMode,
        ) -> std::io::Result<std::sync::Arc<dyn crate::platform::IWriteableBitmapImpl>> {
            unimplemented!()
        }
        fn load_writeable_bitmap_to_height(
            &self,
            _: &mut dyn std::io::Read,
            _: i32,
            _: crate::media::imaging::BitmapInterpolationMode,
        ) -> std::io::Result<std::sync::Arc<dyn crate::platform::IWriteableBitmapImpl>> {
            unimplemented!()
        }
        fn load_writeable_bitmap_from_file(
            &self,
            _: &str,
        ) -> std::io::Result<std::sync::Arc<dyn crate::platform::IWriteableBitmapImpl>> {
            unimplemented!()
        }
        fn load_writeable_bitmap(
            &self,
            _: &mut dyn std::io::Read,
        ) -> std::io::Result<std::sync::Arc<dyn crate::platform::IWriteableBitmapImpl>> {
            unimplemented!()
        }
        fn load_bitmap_to_width(
            &self,
            _: &mut dyn std::io::Read,
            _: i32,
            _: crate::media::imaging::BitmapInterpolationMode,
        ) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
            unimplemented!()
        }
        fn load_bitmap_to_height(
            &self,
            _: &mut dyn std::io::Read,
            _: i32,
            _: crate::media::imaging::BitmapInterpolationMode,
        ) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
            unimplemented!()
        }
        fn resize_bitmap(
            &self,
            _: &dyn crate::platform::IBitmapImpl,
            _: crate::PixelSize,
            _: crate::media::imaging::BitmapInterpolationMode,
        ) -> std::sync::Arc<crate::platform::SharedBitmapImpl> {
            unimplemented!()
        }
        fn load_bitmap_from_pixels(
            &self,
            _: crate::platform::PixelFormat,
            _: crate::platform::AlphaFormat,
            _: &[u8],
            _: crate::PixelSize,
            _: crate::Vector,
            _: i32,
        ) -> std::sync::Arc<crate::platform::SharedBitmapImpl> {
            unimplemented!()
        }
        fn create_backend_context(
            &self,
            _: Option<Rc<dyn crate::platform::IPlatformGraphicsContext>>,
        ) -> Rc<dyn crate::platform::IPlatformRenderInterfaceContext> {
            unimplemented!()
        }
        fn supports_individual_round_rects(&self) -> bool {
            false
        }
        fn default_alpha_format(&self) -> crate::platform::AlphaFormat {
            crate::platform::AlphaFormat::Premul
        }
        fn default_pixel_format(&self) -> crate::platform::PixelFormat {
            crate::platform::PixelFormats::RGBA8888
        }
        fn is_supported_bitmap_pixel_format(&self, _: crate::platform::PixelFormat) -> bool {
            true
        }
        fn supports_regions(&self) -> bool {
            true
        }
        fn create_region(&self) -> Rc<dyn IPlatformRenderInterfaceRegion> {
            let region = Rc::new(MockRegion::default());
            self.regions.borrow_mut().push(region.clone());
            region
        }
    }

    const SURFACE: LtrbRect = LtrbRect::new(0.0, 0.0, 1000.0, 1000.0);

    #[test]
    fn the_covered_area_counts_every_point_once() {
        assert_eq!(0.0, covered_area(&[]));
        assert_eq!(200.0, covered_area(&[LtrbRect::new(0.0, 0.0, 10.0, 10.0), LtrbRect::new(20.0, 0.0, 30.0, 10.0)]));
        // Two rectangles that share a quarter of each, and one inside them.
        let rects = [
            LtrbRect::new(0.0, 0.0, 10.0, 10.0),
            LtrbRect::new(5.0, 5.0, 15.0, 15.0),
            LtrbRect::new(6.0, 6.0, 9.0, 9.0),
        ];
        assert_eq!(175.0, covered_area(&rects));
        // A rectangle of no size covers nothing.
        assert_eq!(100.0, covered_area(&[LtrbRect::new(0.0, 0.0, 10.0, 10.0), LtrbRect::default()]));
    }

    #[test]
    fn rectangles_that_fill_their_union_are_replaced_with_it() {
        // Two bands with a gap of a tenth of their union between them.
        let mut rects = vec![LtrbRect::new(100.0, 100.0, 500.0, 280.0), LtrbRect::new(100.0, 320.0, 500.0, 500.0)];
        simplify_dirty_rects(&mut rects, SURFACE);
        assert_eq!(vec![LtrbRect::new(100.0, 100.0, 500.0, 500.0)], rects);

        // The same bands with a gap of a fifth: the union would draw too much.
        let bands = vec![LtrbRect::new(100.0, 100.0, 500.0, 260.0), LtrbRect::new(100.0, 340.0, 500.0, 500.0)];
        let mut rects = bands.clone();
        simplify_dirty_rects(&mut rects, SURFACE);
        assert_eq!(bands, rects);

        // Rectangles that lie over each other fill what they cover, not the
        // sum of their areas: two corners of a square, and the square is not
        // filled.
        let corners = vec![LtrbRect::new(0.0, 0.0, 300.0, 300.0), LtrbRect::new(150.0, 150.0, 450.0, 450.0)];
        let mut rects = corners.clone();
        simplify_dirty_rects(&mut rects, SURFACE);
        assert_eq!(corners, rects);

        // A rectangle of no size does not stretch the union.
        let mut rects = vec![
            LtrbRect::default(),
            LtrbRect::new(600.0, 600.0, 700.0, 700.0),
            LtrbRect::new(600.0, 700.0, 700.0, 800.0),
        ];
        simplify_dirty_rects(&mut rects, SURFACE);
        assert_eq!(vec![LtrbRect::new(600.0, 600.0, 700.0, 800.0)], rects);
    }

    #[test]
    fn a_rectangle_that_is_all_but_the_surface_is_replaced_with_the_surface() {
        assert_eq!(SURFACE, surface_or_rect(LtrbRect::new(10.0, 0.0, 990.0, 1000.0), SURFACE));
        let smaller = LtrbRect::new(30.0, 0.0, 970.0, 1000.0);
        assert_eq!(smaller, surface_or_rect(smaller, SURFACE));
        // A surface of no size is no surface to draw.
        assert_eq!(smaller, surface_or_rect(smaller, LtrbRect::default()));

        // The union of the rectangles first, and the surface for the union.
        let mut rects = vec![LtrbRect::new(0.0, 0.0, 1000.0, 480.0), LtrbRect::new(0.0, 500.0, 1000.0, 990.0)];
        simplify_dirty_rects(&mut rects, SURFACE);
        assert_eq!(vec![SURFACE], rects);

        // Two rectangles in the corners of the surface cover little of it.
        let corners = vec![LtrbRect::new(0.0, 0.0, 50.0, 50.0), LtrbRect::new(950.0, 950.0, 1000.0, 1000.0)];
        let mut rects = corners.clone();
        simplify_dirty_rects(&mut rects, SURFACE);
        assert_eq!(corners, rects);
    }

    #[test]
    fn full_union_treats_none_as_identity() {
        let a = LtrbRect::new(0.0, 0.0, 10.0, 10.0);
        let b = LtrbRect::new(5.0, 5.0, 20.0, 30.0);
        assert_eq!(None, full_union(None, None));
        assert_eq!(Some(a), full_union(Some(a), None));
        assert_eq!(Some(b), full_union(None, Some(b)));
        assert_eq!(Some(LtrbRect::new(0.0, 0.0, 20.0, 30.0)), full_union(Some(a), Some(b)));
    }

    #[test]
    fn pixel_rect_conversion_truncates_left_top_and_ceils_right_bottom() {
        let px = pixel_rect_from_rect_unscaled(LtrbRect::new(1.7, 2.2, 3.1, 4.0));
        assert_eq!(LtrbPixelRect::new(1, 2, 4, 4), px);
        // Truncation is towards zero, not a floor.
        let px = pixel_rect_from_rect_unscaled(LtrbRect::new(-1.7, -0.2, -0.5, 0.5));
        assert_eq!(LtrbPixelRect::new(-1, 0, 0, 1), px);
        assert_eq!(LtrbRect::new(-1.0, 0.0, 0.0, 1.0), pixel_rect_to_ltrb_rect_unscaled(px));
    }

    #[test]
    fn contains_rect_is_inclusive() {
        let outer = LtrbRect::new(0.0, 0.0, 10.0, 10.0);
        assert!(contains_rect(&outer, outer));
        assert!(contains_rect(&outer, LtrbRect::new(1.0, 1.0, 9.0, 9.0)));
        assert!(!contains_rect(&outer, LtrbRect::new(1.0, 1.0, 10.5, 9.0)));
    }

    #[test]
    fn visualize_random_stays_in_range() {
        let random = VisualizeRandom::new();
        for _ in 0..1000 {
            let value = random.next(255);
            assert!((0..255).contains(&value));
        }
    }
}
