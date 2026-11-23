use ferroui_base::media::{
    BackgroundSizing, BoxShadows, CombinedGeometry, DrawingContext, Geometry, GeometryCombineMode, GeometryBuilder, IBrush, IPen, Pen, PenLineCap,
    PenLineJoin, StreamGeometry,
};
use ferroui_base::platform::render_interface;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{CornerRadius, Rect, Ref, RoundedRect, Size, Thickness};
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

/// Renders a border with a background, caching the geometries between
/// renders.
#[derive(Default)]
pub struct BorderRenderHelper {
    use_complex_rendering: bool,
    backend_supports_individual_corners: Option<bool>,
    background_geometry_cache: Option<Ref<Geometry>>,
    border_geometry_cache: Option<Ref<Geometry>>,
    size: Size,
    border_thickness: Thickness,
    corner_radius: CornerRadius,
    background_sizing: Option<BackgroundSizing>,
    initialized: bool,
    cached_pen: Option<Rc<dyn IPen>>,
}

/// The border renderer of an element, made by the first render that has something to draw.
// Not upstream (docs/porting/desktop-performance.md, section 9.69): the renderer is 144 bytes, and
// an element without a background, a border brush and a shadow never asks it for anything. A
// content presenter is such an element nearly always.
#[derive(Default)]
pub(crate) struct LazyBorderRenderHelper(OnceCell<Box<RefCell<BorderRenderHelper>>>);

impl LazyBorderRenderHelper {
    /// As [`BorderRenderHelper::render`]. A render with no brush and no shadow draws nothing
    /// (`DrawingContext` leaves such a draw out), and makes no renderer.
    pub(crate) fn render(
        &self,
        context: &mut DrawingContext,
        final_size: Size,
        border_thickness: Thickness,
        corner_radius: CornerRadius,
        background_sizing: BackgroundSizing,
        background: Option<&Rc<dyn IBrush>>,
        border_brush: Option<&Rc<dyn IBrush>>,
        box_shadows: &BoxShadows,
    ) {
        if background.is_none() && border_brush.is_none() && box_shadows.count() == 0 {
            return;
        }
        self.0.get_or_init(Default::default).borrow_mut().render(
            context,
            final_size,
            border_thickness,
            corner_radius,
            background_sizing,
            background,
            border_brush,
            box_shadows,
        );
    }

    /// Whether a render made the renderer.
    #[cfg(test)]
    pub(crate) fn is_made(&self) -> bool {
        self.0.get().is_some()
    }
}

impl BorderRenderHelper {
    pub fn new() -> Self {
        Self::default()
    }

    fn update(
        &mut self,
        final_size: Size,
        border_thickness: Thickness,
        corner_radius: CornerRadius,
        background_sizing: BackgroundSizing,
    ) {
        let backend_supports_individual_corners = *self
            .backend_supports_individual_corners
            .get_or_insert_with(|| render_interface().supports_individual_round_rects());
        self.size = final_size;
        self.border_thickness = border_thickness;
        self.corner_radius = corner_radius;
        self.background_sizing = Some(background_sizing);
        self.initialized = true;

        if border_thickness.is_uniform()
            && (corner_radius.is_uniform() || backend_supports_individual_corners)
            && background_sizing == BackgroundSizing::CenterBorder
        {
            self.background_geometry_cache = None;
            self.border_geometry_cache = None;
            self.use_complex_rendering = false;
        } else {
            self.use_complex_rendering = true;

            let bound_rect = Rect::from_size(final_size);
            let inner_rect = bound_rect.deflate_thickness(border_thickness);

            // The geometries the helper has are written again in place where there are some: a
            // border with unequal corners or thicknesses that is resized made three stream
            // geometries and a combined geometry for every size (#805).
            if inner_rect.width != 0.0 && inner_rect.height != 0.0 {
                let kept =
                    self.background_geometry_cache.as_ref().and_then(|geometry| geometry.cast::<StreamGeometry>());
                match kept {
                    Some(geometry) => {
                        Self::write_geometry(&geometry, bound_rect, border_thickness, corner_radius, background_sizing)
                    }
                    None => {
                        self.background_geometry_cache =
                            Some(Self::build_geometry(bound_rect, border_thickness, corner_radius, background_sizing))
                    }
                }
            } else {
                self.background_geometry_cache = None;
            }

            if bound_rect.width != 0.0 && bound_rect.height != 0.0 {
                if !self.write_border_geometry(bound_rect, border_thickness, corner_radius) {
                    let border_inner_geometry = Self::build_geometry(
                        bound_rect,
                        border_thickness,
                        corner_radius,
                        BackgroundSizing::InnerBorderEdge,
                    );
                    let border_outer_geometry = Self::build_geometry(
                        bound_rect,
                        border_thickness,
                        corner_radius,
                        BackgroundSizing::OuterBorderEdge,
                    );

                    self.border_geometry_cache = Some(
                        CombinedGeometry::with_mode(
                            GeometryCombineMode::Exclude,
                            Some(border_outer_geometry),
                            Some(border_inner_geometry),
                        )
                        .upcast(),
                    );
                }
            } else {
                self.border_geometry_cache = None;
            }
        }
    }

    fn build_geometry(
        bound_rect: Rect,
        border_thickness: Thickness,
        corner_radius: CornerRadius,
        sizing: BackgroundSizing,
    ) -> Ref<Geometry> {
        let geometry = StreamGeometry::new();
        Self::write_geometry(&geometry, bound_rect, border_thickness, corner_radius, sizing);
        geometry.upcast()
    }

    /// Writes the rounded rectangle of `sizing` into `geometry`, in place of what it has.
    fn write_geometry(
        geometry: &StreamGeometry,
        bound_rect: Rect,
        border_thickness: Thickness,
        corner_radius: CornerRadius,
        sizing: BackgroundSizing,
    ) {
        let keypoints =
            GeometryBuilder::calculate_rounded_corners_rectangle_win_ui(bound_rect, border_thickness, corner_radius, sizing);
        let mut ctx = geometry.reopen();
        GeometryBuilder::draw_rounded_corners_rectangle_keypoints(&mut ctx, &keypoints);
        drop(ctx);
    }

    /// Writes the outer and the inner edge of the border into the two stream geometries of the
    /// combined geometry the helper has, and says whether it could (it has none before the first
    /// size). The combined geometry follows its two geometries.
    fn write_border_geometry(
        &self,
        bound_rect: Rect,
        border_thickness: Thickness,
        corner_radius: CornerRadius,
    ) -> bool {
        let cache = self.border_geometry_cache.as_ref();
        let Some(combined) = cache.and_then(|geometry| geometry.cast::<CombinedGeometry>()) else {
            return false;
        };
        let outer = combined.geometry1().and_then(|geometry| geometry.cast::<StreamGeometry>());
        let inner = combined.geometry2().and_then(|geometry| geometry.cast::<StreamGeometry>());
        let (Some(outer), Some(inner)) = (outer, inner) else { return false };
        let (thickness, radius) = (border_thickness, corner_radius);
        Self::write_geometry(&outer, bound_rect, thickness, radius, BackgroundSizing::OuterBorderEdge);
        Self::write_geometry(&inner, bound_rect, thickness, radius, BackgroundSizing::InnerBorderEdge);
        true
    }

    /// Renders the border and its background.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        context: &mut DrawingContext,
        final_size: Size,
        border_thickness: Thickness,
        corner_radius: CornerRadius,
        background_sizing: BackgroundSizing,
        background: Option<&Rc<dyn IBrush>>,
        border_brush: Option<&Rc<dyn IBrush>>,
        box_shadows: &BoxShadows,
    ) {
        if self.size != final_size
            || self.border_thickness != border_thickness
            || self.corner_radius != corner_radius
            || self.background_sizing != Some(background_sizing)
            || !self.initialized
        {
            self.update(final_size, border_thickness, corner_radius, background_sizing);
        }

        if self.use_complex_rendering {
            if let Some(background_geometry) = &self.background_geometry_cache {
                context.draw_geometry(background, None, background_geometry);
            }

            if let Some(border_geometry) = &self.border_geometry_cache {
                context.draw_geometry(border_brush, None, border_geometry);
            }
        } else {
            let thickness = self.border_thickness.top;

            Pen::try_modify_or_create(
                &mut self.cached_pen,
                border_brush.cloned(),
                thickness,
                None,
                0.0,
                PenLineCap::Flat,
                PenLineJoin::Miter,
                10.0,
            );

            let mut rect = Rect::from_size(self.size);
            if !MathUtilities::is_zero(thickness) {
                rect = rect.deflate(thickness * 0.5);
            }
            let rrect = RoundedRect::from_corner_radius(rect, self.corner_radius);

            context.draw_rounded_rectangle(background, self.cached_pen.as_ref(), rrect, box_shadows);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #805: a border drawn with geometries (unequal corners on a backend that draws no single
    /// corner, unequal thicknesses, a sizing other than the centre of the border) that is resized
    /// writes the geometries it has again; it makes new ones only after a size without them.
    #[test]
    fn a_border_drawn_with_geometries_that_is_resized_writes_them_again() {
        let _scope = crate::test_support::test_scope();
        let _app = crate::test_support_shapes::MockRenderInterfaceScope::install();
        let thickness = Thickness::new(1.0, 2.0, 1.0, 2.0);
        let radius = CornerRadius::new(2.0, 4.0, 6.0, 1.0);
        let sizing = BackgroundSizing::CenterBorder;
        let parts = |helper: &BorderRenderHelper| {
            let background = helper.background_geometry_cache.clone().expect("a background geometry");
            let border = helper.border_geometry_cache.clone().expect("a border geometry");
            let combined = border.cast::<CombinedGeometry>().expect("a combined geometry");
            let outer = combined.geometry1().expect("the outer edge");
            let inner = combined.geometry2().expect("the inner edge");
            [background, border, outer, inner]
        };

        let mut helper = BorderRenderHelper::new();
        helper.update(Size::new(40.0, 20.0), thickness, radius, sizing);
        assert!(helper.use_complex_rendering);
        let first = parts(&helper);
        let bounds: Vec<Rect> = first.iter().map(|geometry| geometry.bounds()).collect();

        helper.update(Size::new(60.0, 30.0), thickness, radius, sizing);
        let second = parts(&helper);
        for (index, (before, after)) in first.iter().zip(&second).enumerate() {
            assert!(before.ptr_eq(after), "geometry {index} is kept");
            assert_ne!(after.bounds(), bounds[index], "geometry {index} has the new size");
        }

        // A size without geometries lets them go; the next size makes new ones.
        helper.update(Size::new(0.0, 0.0), thickness, radius, sizing);
        assert!(helper.background_geometry_cache.is_none() && helper.border_geometry_cache.is_none());
        helper.update(Size::new(60.0, 30.0), thickness, radius, sizing);
        let third = parts(&helper);
        assert!(!third[0].ptr_eq(&second[0]));
        assert_eq!(third[0].bounds(), second[0].bounds());
    }
}
