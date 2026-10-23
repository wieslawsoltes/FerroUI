use super::{
    render_shapes, CompositionProperty, IServerObject, IServerRenderResourceObserver, IServerVisualContent,
    ServerCompositionShapeCollection, ServerCompositionViewBox, ServerCompositionVisual, ServerSizeDependantVisual,
    ServerVisualRenderContext, ViewBoxMapping,
};
use crate::platform::LtrbRect;
use crate::rendering::composition::generated::{
    ServerCompositionContainerVisualProps, ServerShapeVisualHooks, ServerShapeVisualProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::{Matrix, Rect};
use std::any::{Any, TypeId};
use std::rc::Rc;
use std::time::Duration;

/// Server-side counterpart of a
/// [`ShapeVisual`](crate::rendering::composition::ShapeVisual): its shapes,
/// through its view box, clipped to its size, then its children.
///
/// The visual observes its shape collection and its view box, which are
/// render resources: a change of either, or of anything a shape shows (an
/// animation included), redraws it.
#[derive(Default)]
pub struct ServerShapeVisual {
    props: ServerShapeVisualProps,
}

impl ServerShapeVisual {
    pub fn new() -> Self {
        Self::default()
    }

    /// The properties of the visual.
    pub fn props(&self) -> &ServerShapeVisualProps {
        &self.props
    }

    /// The shapes, in drawing order.
    pub fn shapes(&self) -> Vec<Rc<dyn IServerObject>> {
        self.props
            .shapes()
            .as_ref()
            .and_then(ServerCompositionShapeCollection::of)
            .map(ServerCompositionShapeCollection::shapes)
            .unwrap_or_default()
    }

    /// How the view box maps the shapes onto a visual of `size`, when the
    /// visual has a view box that maps anything.
    pub fn view_box_mapping(&self, size: crate::Vector) -> Option<ViewBoxMapping> {
        ServerCompositionViewBox::mapping_of(&self.props.view_box()?, size)
    }

    /// Starts or stops observing a render resource the visual shows.
    fn observe(visual: &ServerCompositionVisual, resource: Option<Rc<dyn IServerObject>>, observe: bool) {
        let Some(resource) = resource.and_then(|resource| resource.as_render_resource()) else { return };
        let Some(this) = visual.weak().upgrade() else { return };
        let observer: Rc<dyn IServerRenderResourceObserver> = this;
        if observe {
            resource.add_observer(&observer);
        } else {
            resource.remove_observer(&observer);
        }
    }
}

impl ServerShapeVisualHooks for ServerCompositionVisual {
    fn base_deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        ServerCompositionVisual::base_deserialize_changes_core(self, reader, committed_at);
    }

    fn on_shapes_changing(&self) {
        if let Some(content) = self.content_as::<ServerShapeVisual>() {
            ServerShapeVisual::observe(self, content.props.shapes(), false);
        }
    }

    fn on_shapes_changed(&self) {
        if let Some(content) = self.content_as::<ServerShapeVisual>() {
            ServerShapeVisual::observe(self, content.props.shapes(), true);
        }
        self.invalidate_content();
    }

    fn on_view_box_changing(&self) {
        if let Some(content) = self.content_as::<ServerShapeVisual>() {
            ServerShapeVisual::observe(self, content.props.view_box(), false);
        }
    }

    fn on_view_box_changed(&self) {
        if let Some(content) = self.content_as::<ServerShapeVisual>() {
            ServerShapeVisual::observe(self, content.props.view_box(), true);
        }
        self.invalidate_content();
    }
}

impl IServerVisualContent for ServerShapeVisual {
    fn deserialize_changes_core(
        &self,
        visual: &ServerCompositionVisual,
        reader: &mut BatchStreamReader<'_>,
        committed_at: Duration,
    ) {
        self.props.deserialize_changes_core(visual, reader, committed_at);
    }

    /// The size of the visual: the shapes are clipped to it.
    fn compute_own_content_bounds(&self, visual: &ServerCompositionVisual) -> Option<LtrbRect> {
        ServerSizeDependantVisual::compute_own_content_bounds(visual)
    }

    fn size_changed(&self, visual: &ServerCompositionVisual) {
        ServerSizeDependantVisual::size_changed(visual);
    }

    /// Draws the shapes in order, clipped to the size of the visual and to
    /// the mapped view box.
    ///
    /// Deviation (winui-composition.md): the shapes are clipped to the size
    /// of the visual (a shape visual without a size shows nothing, as in
    /// WinUI) and to the view box (the port's reading of WinUI).
    fn render_core(
        &self,
        visual: &ServerCompositionVisual,
        context: &mut ServerVisualRenderContext<'_>,
        _current_transformed_clip: LtrbRect,
    ) {
        let size = visual.size();
        if size.x <= 0.0 || size.y <= 0.0 {
            return;
        }
        let shapes = self.shapes();
        if shapes.is_empty() {
            return;
        }
        let mapping = self.view_box_mapping(size);
        let canvas = context.canvas();
        let base = canvas.transform();
        canvas.push_clip(Rect::new(0.0, 0.0, size.x, size.y));
        if let Some(mapping) = &mapping {
            canvas.push_clip(mapping.clip);
        }
        let parent = mapping.map_or(Matrix::IDENTITY, |mapping| mapping.transform);
        render_shapes(canvas, &shapes, parent, base);
        canvas.set_transform(base);
        if mapping.is_some() {
            canvas.pop_clip();
        }
        canvas.pop_clip();
    }

    /// The shape collection or the view box changed.
    fn dependency_queued_invalidate(&self, visual: &ServerCompositionVisual) {
        visual.invalidate_content();
    }

    fn dispose(&self, visual: &ServerCompositionVisual) {
        ServerShapeVisual::observe(visual, self.props.shapes(), false);
        ServerShapeVisual::observe(visual, self.props.view_box(), false);
    }

    fn find_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id)
    }

    fn get_composition_property(&self, name: &str) -> Option<&'static CompositionProperty> {
        ServerCompositionContainerVisualProps::get_composition_property(name)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
