//! The server-side counterpart of the WinUI `CompositionViewBox`, and the
//! mapping of a view box onto the area of a visual.

use super::{
    impl_server_render_resource, surface_brush_content_rect, IAnimatedServerObject, IServerObject,
    IServerRenderResource, ServerCompositor, ServerRenderResource,
};
use crate::rendering::composition::generated::{ServerCompositionViewBoxHooks, ServerCompositionViewBoxProps};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::rendering::composition::CompositionStretch;
use crate::{Matrix, Rect, Size, Vector};
use std::any::{Any, TypeId};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// How a view box maps shape coordinates onto the area of a visual.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewBoxMapping {
    /// The transform from shape coordinates to the coordinates of the
    /// visual.
    pub transform: Matrix,
    /// The mapped view box, in the coordinates of the visual, limited to the
    /// area: what stays visible.
    pub clip: Rect,
}

/// Maps the view box `(offset, size)` onto `area`: the view box is sized by
/// `stretch` and placed by the alignment ratios, as a surface brush lays
/// out its surface (see [`surface_brush_content_rect`]). `None` when the
/// view box or the area is empty.
pub fn view_box_mapping(
    offset: Vector,
    size: Vector,
    area: Rect,
    stretch: CompositionStretch,
    horizontal_ratio: f64,
    vertical_ratio: f64,
) -> Option<ViewBoxMapping> {
    let content = surface_brush_content_rect(Size::new(size.x, size.y), area, stretch, horizontal_ratio, vertical_ratio)?;
    let transform = Matrix::create_translation(-offset.x, -offset.y)
        * Matrix::create_scale(content.width / size.x, content.height / size.y)
        * Matrix::create_translation(content.x, content.y);
    Some(ViewBoxMapping { transform, clip: content.intersect(area) })
}

/// The server-side counterpart of a
/// [`CompositionViewBox`](crate::rendering::composition::CompositionViewBox).
pub struct ServerCompositionViewBox {
    base: ServerRenderResource,
    props: ServerCompositionViewBoxProps,
}

impl ServerCompositionViewBox {
    /// Creates the view box, active: the UI-thread view box activates its
    /// server object when it is created.
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionViewBox> {
        let view_box = Rc::new_cyclic(|this: &Weak<ServerCompositionViewBox>| ServerCompositionViewBox {
            base: ServerRenderResource::new(
                compositor,
                this.clone() as Weak<dyn IAnimatedServerObject>,
                this.clone() as Weak<dyn IServerRenderResource>,
            ),
            props: ServerCompositionViewBoxProps::new(),
        });
        view_box.base.object().activate();
        view_box
    }

    /// The properties of the view box.
    pub fn props(&self) -> &ServerCompositionViewBoxProps {
        &self.props
    }

    pub fn is_disposed(&self) -> bool {
        self.base.is_disposed()
    }

    /// How the view box maps onto a visual of `size`; `None` when the view
    /// box or the visual is empty (the shapes are then drawn without a view
    /// box).
    pub fn mapping(&self, size: Vector) -> Option<ViewBoxMapping> {
        view_box_mapping(
            self.props.offset(),
            self.props.size(),
            Rect::new(0.0, 0.0, size.x, size.y),
            self.props.stretch(),
            self.props.horizontal_alignment_ratio() as f64,
            self.props.vertical_alignment_ratio() as f64,
        )
    }

    /// The mapping of the view box behind `object` (a server-side view box)
    /// onto a visual of `size`.
    pub fn mapping_of(object: &Rc<dyn IServerObject>, size: Vector) -> Option<ViewBoxMapping> {
        object.as_any().downcast_ref::<ServerCompositionViewBox>()?.mapping(size)
    }
}

impl_server_render_resource!(
    ServerCompositionViewBox,
    base,
    composition_property: ServerCompositionViewBoxProps::get_composition_property
);

impl ServerCompositionViewBoxHooks for ServerCompositionViewBox {}

impl IServerObject for ServerCompositionViewBox {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.props.deserialize_changes_core(self, reader, committed_at);
    }

    fn values_invalidated(&self) {
        self.base.core().invalidated(self);
    }

    fn dispose(&self) {
        self.base.core().dispose();
    }

    fn as_render_resource(self: Rc<Self>) -> Option<Rc<dyn IServerRenderResource>> {
        Some(self)
    }

    fn as_animated(self: Rc<Self>) -> Option<Rc<dyn IAnimatedServerObject>> {
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
