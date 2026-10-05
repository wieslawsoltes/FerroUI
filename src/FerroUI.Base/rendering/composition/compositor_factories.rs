use super::animations::{CompositionAnimationGroup, ExpressionAnimation, ImplicitAnimationCollection};
use super::server::{RenderSurfaces, ServerCompositionContainerVisual};
use super::visual::CompositionVisualKind;
use super::{
    CompositionDrawingSurface, CompositionCustomVisual, ICompositionCustomVisualHandler, CompositionConicGradientBrush, CompositionGradientStop, CompositionLinearGradientBrush,
    CompositionRadialGradientBrush, CompositionSolidColorBrush, CompositionContainerVisual, CompositionSolidColorVisual, CompositionSurfaceVisual, CompositionTarget,
    CompositionVisual, Compositor,
};
use crate::media::Color;
use std::rc::Rc;

impl Compositor {
    fn this_rc(&self) -> Rc<Compositor> {
        self.this_handle()
    }

    /// Creates a composition target that renders to one of the surfaces
    /// `surfaces` returns.
    pub fn create_composition_target(&self, surfaces: RenderSurfaces) -> Rc<CompositionTarget> {
        CompositionTarget::new(&self.this_rc(), surfaces)
    }

    pub fn create_container_visual(&self) -> Rc<CompositionContainerVisual> {
        CompositionVisual::create(
            &self.this_rc(),
            CompositionVisualKind::Container(super::generated::CompositionContainerVisualProps::new()),
            || Box::new(ServerCompositionContainerVisual),
        )
    }

    pub fn create_expression_animation(&self) -> Rc<ExpressionAnimation> {
        ExpressionAnimation::new(&self.this_rc())
    }

    pub fn create_expression_animation_with(&self, expression: &str) -> Rc<ExpressionAnimation> {
        let animation = ExpressionAnimation::new(&self.this_rc());
        animation.set_expression(Some(expression.to_owned()));
        animation
    }

    pub fn create_implicit_animation_collection(&self) -> Rc<ImplicitAnimationCollection> {
        ImplicitAnimationCollection::new(&self.this_rc())
    }

    pub fn create_animation_group(&self) -> Rc<CompositionAnimationGroup> {
        CompositionAnimationGroup::new(&self.this_rc())
    }

    pub fn create_solid_color_visual(&self) -> CompositionSolidColorVisual {
        CompositionSolidColorVisual::new(&self.this_rc())
    }

    pub fn create_custom_visual(&self, handler: Rc<dyn ICompositionCustomVisualHandler>) -> CompositionCustomVisual {
        CompositionCustomVisual::new(&self.this_rc(), handler)
    }

    pub fn create_drawing_surface(&self) -> CompositionDrawingSurface {
        CompositionDrawingSurface::new(&self.this_rc())
    }

    pub fn create_surface_visual(&self) -> CompositionSurfaceVisual {
        CompositionSurfaceVisual::new(&self.this_rc())
    }

    pub fn create_solid_color_brush(&self) -> CompositionSolidColorBrush {
        CompositionSolidColorBrush::new(&self.this_rc())
    }

    pub fn create_solid_color_brush_with(&self, color: Color) -> CompositionSolidColorBrush {
        CompositionSolidColorBrush::with_color(&self.this_rc(), color)
    }

    pub fn create_linear_gradient_brush(&self) -> CompositionLinearGradientBrush {
        CompositionLinearGradientBrush::new(&self.this_rc())
    }

    pub fn create_conic_gradient_brush(&self) -> CompositionConicGradientBrush {
        CompositionConicGradientBrush::new(&self.this_rc())
    }

    pub fn create_radial_gradient_brush(&self) -> CompositionRadialGradientBrush {
        CompositionRadialGradientBrush::new(&self.this_rc())
    }

    pub fn create_gradient_stop_with(&self, offset: f64, color: Color) -> Rc<CompositionGradientStop> {
        CompositionGradientStop::with_offset_and_color(&self.this_rc(), offset, color)
    }

    pub fn create_gradient_stop(&self) -> Rc<CompositionGradientStop> {
        CompositionGradientStop::new(&self.this_rc())
    }
}
