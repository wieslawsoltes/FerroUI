use crate::media::immutable::{ImmutableDashStyle, ImmutablePen, ImmutableSolidColorBrush};
use crate::media::{IBrush, IDashStyle, IImmutableBrush, IPen, PenLineCap, PenLineJoin};
use crate::rendering::composition::generated::{ServerCompositionSimplePenHooks, ServerCompositionSimplePenProps};
use crate::rendering::composition::server::{
    impl_simple_server_render_resource, IServerObject, IServerRenderResource, ServerCompositor, ServerResourceRef,
    SimpleServerRenderResource,
};
use crate::rendering::composition::transport::BatchStreamReader;
use std::any::{Any, TypeId};
use std::rc::Rc;
use std::time::Duration;

/// The server-side counterpart of a mutable pen.
pub struct ServerCompositionSimplePen {
    base: SimpleServerRenderResource,
    props: ServerCompositionSimplePenProps,
}

impl ServerCompositionSimplePen {
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionSimplePen> {
        Rc::new_cyclic(|this| {
            let this: std::rc::Weak<ServerCompositionSimplePen> = this.clone();
            ServerCompositionSimplePen {
                base: SimpleServerRenderResource::new(compositor, this),
                props: ServerCompositionSimplePenProps::new(),
            }
        })
    }

    pub fn is_disposed(&self) -> bool {
        self.base.is_disposed()
    }

    /// The properties of the pen.
    pub fn props(&self) -> &ServerCompositionSimplePenProps {
        &self.props
    }

    pub fn set_brush(&self, value: Option<ServerResourceRef<dyn IBrush>>) {
        self.props.set_brush(self, value)
    }

    pub fn set_dash_style(&self, value: Option<Rc<ImmutableDashStyle>>) {
        self.props.set_dash_style(self, value)
    }

    pub fn set_line_cap(&self, value: PenLineCap) {
        self.props.set_line_cap(self, value)
    }

    pub fn set_line_join(&self, value: PenLineJoin) {
        self.props.set_line_join(self, value)
    }

    pub fn set_miter_limit(&self, value: f64) {
        self.props.set_miter_limit(self, value)
    }

    pub fn set_thickness(&self, value: f64) {
        self.props.set_thickness(self, value)
    }
}

impl_simple_server_render_resource!(ServerCompositionSimplePen, base);

impl ServerCompositionSimplePenHooks for ServerCompositionSimplePen {}

impl IServerObject for ServerCompositionSimplePen {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.props.deserialize_changes_core(self, reader, committed_at);
    }

    fn values_invalidated(&self) {
        self.base.core().invalidated(self);
    }

    fn dispose(&self) {
        // Remove the pen from the brush observers.
        // Without this, the pen was being retained in memory by long lived brush resources (e.g. those defined in
        // the theme or app resources), hence was causing memory leaks.
        let brush = self.props.brush();
        self.base.core().remove_observers_from_property(self, brush.as_ref().and_then(|b| b.resource.as_ref()));
        ServerCompositionSimplePenProps::id_of_brush_property().set_field(self, None);
        self.base.core().dispose();
    }

    fn as_render_resource(self: Rc<Self>) -> Option<Rc<dyn IServerRenderResource>> {
        Some(self)
    }

    fn as_pen(self: Rc<Self>) -> Option<Rc<dyn IPen>> {
        Some(self)
    }

    fn get_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl IPen for ServerCompositionSimplePen {
    fn brush(&self) -> Option<Rc<dyn IBrush>> {
        self.props.brush().map(|brush| brush.value)
    }

    fn dash_style(&self) -> Option<Rc<dyn IDashStyle>> {
        self.props.dash_style().map(|style| style as Rc<dyn IDashStyle>)
    }

    fn line_cap(&self) -> PenLineCap {
        self.props.line_cap()
    }

    fn line_join(&self) -> PenLineJoin {
        self.props.line_join()
    }

    fn miter_limit(&self) -> f64 {
        self.props.miter_limit()
    }

    fn thickness(&self) -> f64 {
        self.props.thickness()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_immutable_pen(self: Rc<Self>) -> Rc<ImmutablePen> {
        // A snapshot of the brush: an immutable brush is taken as it is, a
        // server-side solid color brush by value. Other server-side brushes
        // have no immutable form here.
        let brush: Option<Rc<dyn IImmutableBrush>> = IPen::brush(&*self).and_then(|brush| {
            if let Some(solid) = brush.as_solid_color_brush() {
                if brush.as_any().is::<ImmutableSolidColorBrush>() {
                    return brush.clone().into_immutable_brush();
                }
                let snapshot: Rc<dyn IImmutableBrush> = Rc::new(ImmutableSolidColorBrush::from_brush(solid));
                return Some(snapshot);
            }
            brush.into_immutable_brush()
        });
        Rc::new(ImmutablePen::new(
            brush,
            self.props.thickness(),
            self.props.dash_style(),
            self.props.line_cap(),
            self.props.line_join(),
            self.props.miter_limit(),
        ))
    }
}
