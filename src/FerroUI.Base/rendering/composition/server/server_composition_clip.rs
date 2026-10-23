//! The server-side counterparts of the WinUI clip objects (`InsetClip`,
//! `RectangleClip`, `GeometricClip`) and the shape a clip cuts out of the
//! visual it is set on.

use super::{
    impl_server_render_resource, platform_geometry_of, IAnimatedServerObject, IServerObject, IServerRenderResource,
    ServerCompositionViewBox, ServerCompositor, ServerRenderResource,
};
use crate::numerics::Quaternion;
use crate::platform::{IDrawingContextImpl, IGeometryImpl, LtrbRect};
use crate::rendering::composition::generated::{
    ServerCompositionClipHooks, ServerCompositionClipProps, ServerGeometricClipHooks, ServerGeometricClipProps,
    ServerInsetClipHooks, ServerInsetClipProps, ServerRectangleClipHooks, ServerRectangleClipProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::rendering::composition::MatrixUtils;
use crate::{Matrix, Rect, RoundedRect, Vector, Vector3D};
use std::any::{Any, TypeId};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Duration;

/// Declares a server-side clip class: the struct, its constructor, the
/// render resource contracts and `IServerObject`.
macro_rules! server_clip {
    ($(#[$meta:meta])* $name:ident, $props:ident, $hooks:ident) => {
        $(#[$meta])*
        pub struct $name {
            base: ServerRenderResource,
            props: $props,
        }

        impl $name {
            /// Creates the clip, active: the UI-thread clip activates its
            /// server object when it is created.
            pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<$name> {
                let clip = Rc::new_cyclic(|this: &Weak<$name>| $name {
                    base: ServerRenderResource::new(
                        compositor,
                        this.clone() as Weak<dyn IAnimatedServerObject>,
                        this.clone() as Weak<dyn IServerRenderResource>,
                    ),
                    props: $props::new(),
                });
                clip.base.object().activate();
                clip
            }

            /// The properties of the clip.
            pub fn props(&self) -> &$props {
                &self.props
            }

            /// The properties of `CompositionClip`.
            pub fn clip_props(&self) -> &ServerCompositionClipProps {
                self.props.base()
            }

            pub fn is_disposed(&self) -> bool {
                self.base.is_disposed()
            }
        }

        impl_server_render_resource!($name, base, composition_property: $props::get_composition_property);

        impl ServerCompositionClipHooks for $name {}

        impl $hooks for $name {}

        impl IServerObject for $name {
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
    };
}

server_clip!(
    /// The server-side counterpart of an
    /// [`InsetClip`](crate::rendering::composition::InsetClip).
    ServerInsetClip,
    ServerInsetClipProps,
    ServerInsetClipHooks
);

server_clip!(
    /// The server-side counterpart of a
    /// [`RectangleClip`](crate::rendering::composition::RectangleClip).
    ServerRectangleClip,
    ServerRectangleClipProps,
    ServerRectangleClipHooks
);

server_clip!(
    /// The server-side counterpart of a
    /// [`GeometricClip`](crate::rendering::composition::GeometricClip).
    ServerGeometricClip,
    ServerGeometricClipProps,
    ServerGeometricClipHooks
);

/// The transform of a WinUI clip or brush from its own properties, in the
/// space of an area of `size`: the anchor point is relative to the size,
/// then the transform matrix, the scale and the rotation about the center
/// point and the offset apply, as for a visual. `None` for the identity.
#[allow(clippy::too_many_arguments)]
pub fn composition_transform(
    size: Vector,
    anchor_point: Vector,
    center_point: Vector,
    transform_matrix: Matrix,
    scale: Vector,
    rotation_angle: f32,
    offset: Vector,
) -> Option<Matrix> {
    MatrixUtils::compute_transform(
        size,
        anchor_point,
        Vector3D::new(center_point.x, center_point.y, 0.0),
        transform_matrix,
        Vector3D::new(scale.x, scale.y, 1.0),
        rotation_angle,
        Quaternion::IDENTITY,
        Vector3D::new(offset.x, offset.y, 0.0),
    )
}

/// The area a composition clip leaves visible, in the coordinates of the
/// visual it is set on: a rectangle (rounded for a rectangle clip) or, for
/// a geometric clip, a geometry in the space of the clip, and the transform
/// of the clip.
#[derive(Clone)]
pub struct CompositionClipShape {
    /// The visible area in the space of the clip; for a geometric clip the
    /// bounds of its geometry.
    pub rect: RoundedRect,
    /// The transform from the space of the clip to the space of the visual,
    /// `None` for the identity. For a geometric clip it includes the
    /// mapping of its view box.
    pub transform: Option<Matrix>,
    /// The geometry of a geometric clip: the visible area is its inside.
    pub geometry: Option<Arc<dyn IGeometryImpl>>,
}

impl CompositionClipShape {
    /// The shape of `clip`, a server-side clip object, on a visual of the
    /// given size. `None` when `clip` is not a clip object, or is a
    /// geometric clip without a geometry (which clips nothing).
    ///
    /// Deviation (winui-composition.md): the anchor point of the clip is
    /// relative to the size of the visual, which is the reference box of
    /// both clip classes here.
    pub fn of(clip: &dyn IServerObject, size: Vector) -> Option<CompositionClipShape> {
        let any = clip.as_any();
        if let Some(geometric) = any.downcast_ref::<ServerGeometricClip>() {
            return Self::of_geometric(geometric, size);
        }
        let (rect, base) = if let Some(inset) = any.downcast_ref::<ServerInsetClip>() {
            let props = inset.props();
            let rect = inset_rect(
                size,
                props.left_inset() as f64,
                props.top_inset() as f64,
                props.right_inset() as f64,
                props.bottom_inset() as f64,
            );
            (RoundedRect::from_rect(rect), props.base())
        } else if let Some(rectangle) = any.downcast_ref::<ServerRectangleClip>() {
            let props = rectangle.props();
            let (left, top) = (props.left() as f64, props.top() as f64);
            let width = (props.right() as f64 - left).max(0.0);
            let height = (props.bottom() as f64 - top).max(0.0);
            let rect = RoundedRect::new(
                Rect::new(left, top, width, height),
                props.top_left_radius(),
                props.top_right_radius(),
                props.bottom_right_radius(),
                props.bottom_left_radius(),
            );
            (rect, props.base())
        } else {
            return None;
        };
        let transform = composition_transform(
            size,
            base.anchor_point(),
            base.center_point(),
            base.transform_matrix(),
            base.scale(),
            base.rotation_angle(),
            base.offset(),
        );
        Some(CompositionClipShape { rect, transform, geometry: None })
    }

    /// The shape of a geometric clip: its geometry, mapped onto the visual
    /// by its view box, then transformed by the clip.
    fn of_geometric(clip: &ServerGeometricClip, size: Vector) -> Option<CompositionClipShape> {
        let geometry = platform_geometry_of(&clip.props().geometry()?)?;
        let base = clip.clip_props();
        let own = composition_transform(
            size,
            base.anchor_point(),
            base.center_point(),
            base.transform_matrix(),
            base.scale(),
            base.rotation_angle(),
            base.offset(),
        );
        let view_box = clip
            .props()
            .view_box()
            .and_then(|view_box| ServerCompositionViewBox::mapping_of(&view_box, size))
            .map(|mapping| mapping.transform);
        let transform = match (view_box, own) {
            (Some(view_box), Some(own)) => Some(view_box * own),
            (view_box, own) => view_box.or(own),
        };
        Some(CompositionClipShape { rect: RoundedRect::from_rect(geometry.bounds()), transform, geometry: Some(geometry) })
    }

    /// The bounds of the visible area in the space of the visual.
    pub fn bounds(&self) -> LtrbRect {
        let rect = LtrbRect::from_rect(self.rect.rect);
        match self.transform {
            Some(transform) => rect.transform_to_aabb(transform),
            None => rect,
        }
    }

    /// Pushes the clip onto `canvas`, whose transform is the one of the
    /// visual; popped with [`pop`](Self::pop). The clip is taken in the space of
    /// the clip: the transform of the canvas is changed for the push and
    /// restored after it.
    pub fn push(&self, canvas: &mut dyn IDrawingContextImpl) {
        let saved = self.transform.map(|transform| {
            let saved = canvas.transform();
            canvas.set_transform(transform * saved);
            saved
        });
        if let Some(geometry) = &self.geometry {
            canvas.push_geometry_clip(&**geometry);
        } else if self.rect.is_rounded() {
            canvas.push_clip_rounded(self.rect);
        } else {
            canvas.push_clip(self.rect.rect);
        }
        if let Some(saved) = saved {
            canvas.set_transform(saved);
        }
    }

    /// Pops what [`push`](Self::push) pushed.
    pub fn pop(&self, canvas: &mut dyn IDrawingContextImpl) {
        if self.geometry.is_some() {
            canvas.pop_geometry_clip();
        } else {
            canvas.pop_clip();
        }
    }
}

/// The rectangle an inset clip leaves visible on an area of `size`: `(left,
/// top, size.x - right, size.y - bottom)`, empty when the insets overlap.
pub fn inset_rect(size: Vector, left: f64, top: f64, right: f64, bottom: f64) -> Rect {
    let width = (size.x - right - left).max(0.0);
    let height = (size.y - bottom - top).max(0.0);
    Rect::new(left, top, width, height)
}
