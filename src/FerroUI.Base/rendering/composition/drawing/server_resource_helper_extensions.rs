use super::{ICompositionRenderResource, IRenderDataGeometry, RenderDataResource};
use crate::media::immutable::{ImmutablePen, ImmutableTransform};
use crate::media::{Geometry, IBrush, IPen, ITransform};
use crate::platform::IGeometryImpl;
use crate::rendering::composition::transport::BatchResource;
use crate::rendering::composition::Compositor;
use crate::Ref;
use std::rc::Rc;

/// The composition render resource behind a brush, if the brush is a
/// mutable brush with server-side counterparts.
pub(crate) fn brush_render_resource(brush: &dyn IBrush) -> Option<&dyn ICompositionRenderResource> {
    brush.as_composition_render_resource()
}

/// The composition render resource behind a pen, if the pen is a mutable
/// pen with server-side counterparts.
pub(crate) fn pen_render_resource(pen: &dyn IPen) -> Option<&dyn ICompositionRenderResource> {
    pen.as_composition_render_resource()
}

/// The composition render resource behind a geometry object: every
/// geometry object has a server-side counterpart on the compositors it is
/// drawn with.
pub(crate) fn geometry_render_resource(geometry: &Ref<Geometry>) -> Option<&dyn ICompositionRenderResource> {
    Some(&**geometry)
}

/// The composition render resource behind a transform, if the transform is
/// a mutable transform with server-side counterparts.
pub(crate) fn transform_render_resource(transform: &dyn ITransform) -> Option<&dyn ICompositionRenderResource> {
    transform.as_composition_render_resource()
}

/// A geometry object as render data keeps it on the UI thread, for hit
/// testing: it resolves to the current platform geometry of the object.
struct ClientGeometry(Ref<Geometry>);

impl IRenderDataGeometry for ClientGeometry {
    fn geometry_impl(&self) -> Option<Rc<dyn IGeometryImpl>> {
        self.0.platform_impl()
    }
}

fn is_immutable_brush(brush: &Rc<dyn IBrush>) -> bool {
    brush.as_mutable_brush().is_none()
        && brush.as_composition_render_resource().is_none()
        && brush.as_composition_brush().is_none()
}

fn is_immutable_pen(pen: &Rc<dyn IPen>) -> bool {
    pen.as_any().is::<ImmutablePen>()
}

fn not_compatible(what: &str) -> ! {
    panic!("{what} is not compatible with composition");
}

/// The form of a brush that is sent to a compositor (`GetServer`): the
/// brush itself without a compositor or when it is immutable, otherwise the
/// id of its server-side counterpart on the compositor.
pub(crate) fn brush_get_server_resource(
    brush: Option<&Rc<dyn IBrush>>,
    compositor: Option<&Compositor>,
) -> Option<BatchResource<dyn IBrush>> {
    let brush = brush?;
    let Some(compositor) = compositor else {
        return Some(BatchResource::Value(brush.clone()));
    };
    if is_immutable_brush(brush) {
        return Some(BatchResource::Value(brush.clone()));
    }
    if let Some(resource) = brush_render_resource(&**brush) {
        return Some(BatchResource::Server(resource.get_for_compositor(compositor)));
    }
    if let Some(composition_brush) = brush.as_composition_brush() {
        // The server object belongs to its own compositor's render loop;
        // handing it to another compositor would share one resource across
        // two render threads.
        if !std::ptr::eq(&**composition_brush.compositor(), compositor) {
            panic!("{} belongs to a different compositor", composition_brush.type_name());
        }
        return Some(BatchResource::Server(composition_brush.server()));
    }
    not_compatible("The brush")
}

/// The form of a brush that render data refers to (`GetServer`).
pub(crate) fn brush_get_server(brush: Option<&Rc<dyn IBrush>>, compositor: Option<&Compositor>) -> Option<RenderDataResource> {
    match brush_get_server_resource(brush, compositor)? {
        BatchResource::Value(brush) => Some(RenderDataResource::Brush(brush)),
        BatchResource::Server(server) => Some(RenderDataResource::ServerBrush { server, client: brush?.clone() }),
    }
}

/// The form of a transform that is sent to a compositor (`GetServer`): the
/// transform itself without a compositor or when it is immutable, otherwise
/// an immutable copy of its current value.
pub(crate) fn transform_get_server(
    transform: Option<&Rc<dyn ITransform>>,
    compositor: Option<&Compositor>,
) -> Option<BatchResource<dyn ITransform>> {
    let Some(compositor) = compositor else {
        return transform.map(|transform| BatchResource::Value(transform.clone()));
    };
    let transform = transform?;
    if transform.as_any().is::<ImmutableTransform>() {
        return Some(BatchResource::Value(transform.clone()));
    }
    if let Some(resource) = transform_render_resource(&**transform) {
        // As upstream: the counterpart must exist on the compositor, but
        // what is sent is a copy of the current value.
        resource.get_for_compositor(compositor);
    }
    Some(BatchResource::Value(Rc::new(ImmutableTransform::new(transform.value()))))
}

/// The form of a pen that render data draws with (`GetServer`).
pub(crate) fn pen_get_server(pen: Option<&Rc<dyn IPen>>, compositor: Option<&Compositor>) -> Option<RenderDataResource> {
    let pen = pen?;
    let Some(compositor) = compositor else {
        return Some(RenderDataResource::Pen(pen.clone()));
    };
    if is_immutable_pen(pen) {
        return Some(RenderDataResource::Pen(pen.clone()));
    }
    match pen_render_resource(&**pen) {
        Some(resource) => {
            Some(RenderDataResource::ServerPen { server: resource.get_for_compositor(compositor), client: pen.clone() })
        }
        None => not_compatible("The pen"),
    }
}

/// The form of a pen that render data keeps for hit testing on the UI
/// thread: the pen itself. A mutable pen is not sent to the server.
pub(crate) fn pen_get_client(pen: Option<&Rc<dyn IPen>>, compositor: Option<&Compositor>) -> Option<RenderDataResource> {
    let pen = pen?;
    if compositor.is_none() || is_immutable_pen(pen) {
        Some(RenderDataResource::Pen(pen.clone()))
    } else {
        Some(RenderDataResource::ClientPen(pen.clone()))
    }
}

/// The form of a geometry object that render data refers to (`GetServer`).
pub(crate) fn geometry_get_server(geometry: &Ref<Geometry>, compositor: Option<&Compositor>) -> Option<RenderDataResource> {
    let Some(compositor) = compositor else {
        return geometry.platform_impl().map(RenderDataResource::GeometryImpl);
    };
    match geometry_render_resource(geometry) {
        Some(resource) => Some(RenderDataResource::ServerGeometry {
            server: resource.get_for_compositor(compositor),
            client: Rc::new(ClientGeometry(geometry.clone())),
        }),
        None => not_compatible("The geometry"),
    }
}
