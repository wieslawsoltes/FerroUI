use ferroui_base::media::{
    BackgroundSizing, BoxShadows, CombinedGeometry, DrawingContext, Geometry, GeometryCombineMode, GeometryBuilder, IBrush, IPen, Pen, PenLineCap,
    PenLineJoin, StreamGeometry,
};
use ferroui_base::platform::render_interface;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{CornerRadius, Rect, Ref, RoundedRect, Size, Thickness};
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

            if inner_rect.width != 0.0 && inner_rect.height != 0.0 {
                self.background_geometry_cache =
                    Some(Self::build_geometry(bound_rect, border_thickness, corner_radius, background_sizing));
            } else {
                self.background_geometry_cache = None;
            }

            if bound_rect.width != 0.0 && bound_rect.height != 0.0 {
                let border_inner_geometry =
                    Self::build_geometry(bound_rect, border_thickness, corner_radius, BackgroundSizing::InnerBorderEdge);
                let border_outer_geometry =
                    Self::build_geometry(bound_rect, border_thickness, corner_radius, BackgroundSizing::OuterBorderEdge);

                self.border_geometry_cache = Some(
                    CombinedGeometry::with_mode(
                        GeometryCombineMode::Exclude,
                        Some(border_outer_geometry),
                        Some(border_inner_geometry),
                    )
                    .upcast(),
                );
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
        let keypoints =
            GeometryBuilder::calculate_rounded_corners_rectangle_win_ui(bound_rect, border_thickness, corner_radius, sizing);
        let geometry = StreamGeometry::new();
        {
            let mut ctx = geometry.open();
            GeometryBuilder::draw_rounded_corners_rectangle_keypoints(&mut ctx, &keypoints);
        }
        geometry.upcast()
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
