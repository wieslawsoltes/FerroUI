use super::animations::{CompositionAnimationGroup, ExpressionAnimation, ImplicitAnimationCollection};
use super::server::{RenderSurfaces, ServerCompositionContainerVisual};
use super::visual::CompositionVisualKind;
use super::{
    CompositionContainerVisual, CompositionSolidColorVisual, CompositionSurfaceVisual, CompositionTarget,
    CompositionVisual, Compositor,
};
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

    pub fn create_surface_visual(&self) -> CompositionSurfaceVisual {
        CompositionSurfaceVisual::new(&self.this_rc())
    }
}
