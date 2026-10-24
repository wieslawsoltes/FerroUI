//! The server-side counterparts of the WinUI geometries
//! (`CompositionRectangleGeometry`, `CompositionRoundedRectangleGeometry`,
//! `CompositionEllipseGeometry`, `CompositionLineGeometry`,
//! `CompositionPathGeometry`): each builds its platform geometry with the
//! render interface of the compositor, trims it, and caches the result until
//! one of its members changes.

use super::{
    impl_server_render_resource, IAnimatedServerObject, IServerObject, IServerRenderResource, ServerCompositor,
    ServerRenderResource,
};
use crate::media::{FillRule, GeometryBuilder};
use crate::platform::{IGeometryImpl, IPlatformRenderInterface};
use crate::rendering::composition::generated::{
    ServerCompositionEllipseGeometryHooks, ServerCompositionEllipseGeometryProps, ServerCompositionGeometryHooks,
    ServerCompositionGeometryProps, ServerCompositionLineGeometryHooks, ServerCompositionLineGeometryProps,
    ServerCompositionPathGeometryHooks, ServerCompositionPathGeometryProps, ServerCompositionRectangleGeometryHooks,
    ServerCompositionRectangleGeometryProps, ServerCompositionRoundedRectangleGeometryHooks,
    ServerCompositionRoundedRectangleGeometryProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::{Point, Rect};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Duration;

/// What the shapes and clips need from a server-side geometry.
pub trait IServerCompositionGeometry {
    /// The platform geometry with the trimming applied, cached until a
    /// member of the geometry changes. `None` when there is nothing to draw
    /// (no path, or a trim that leaves nothing) or when the compositor has
    /// no render interface.
    fn platform_geometry(&self) -> Option<Arc<dyn IGeometryImpl>>;

    /// The properties of `CompositionGeometry` (the trimming).
    fn geometry_props(&self) -> &ServerCompositionGeometryProps;
}

/// The server-side geometry behind `object`, if it is one.
pub fn server_geometry_of(object: &dyn IServerObject) -> Option<&dyn IServerCompositionGeometry> {
    let any = object.as_any();
    if let Some(geometry) = any.downcast_ref::<ServerCompositionRectangleGeometry>() {
        return Some(geometry);
    }
    if let Some(geometry) = any.downcast_ref::<ServerCompositionRoundedRectangleGeometry>() {
        return Some(geometry);
    }
    if let Some(geometry) = any.downcast_ref::<ServerCompositionEllipseGeometry>() {
        return Some(geometry);
    }
    if let Some(geometry) = any.downcast_ref::<ServerCompositionLineGeometry>() {
        return Some(geometry);
    }
    if let Some(geometry) = any.downcast_ref::<ServerCompositionPathGeometry>() {
        return Some(geometry);
    }
    None
}

/// The trimmed platform geometry of `object` (a server-side geometry), if
/// it is one and there is something to draw.
pub fn platform_geometry_of(object: &Rc<dyn IServerObject>) -> Option<Arc<dyn IGeometryImpl>> {
    server_geometry_of(&**object)?.platform_geometry()
}

/// The part of the outline of `geometry` that `TrimStart`, `TrimEnd` and
/// `TrimOffset` leave: the fractions `start..end` of the length of the
/// outline, both moved by `offset` and wrapping around the start of the
/// outline. `None` when nothing is left.
///
/// Deviation (winui-composition.md): the trim points are clamped to
/// `0..=1` and swapped when the start is after the end (the port's reading);
/// a part that wraps around is two figures; when the platform cannot cut
/// the outline (`try_get_segment` fails) the whole geometry is drawn.
/// A fraction of the single-precision members as the decimal it was written as (`0.2`, not
/// `0.20000000298`): the fractions select lengths of the outline, which would otherwise be off by
/// the error of the single precision.
fn widen(value: f32) -> f64 {
    value.to_string().parse().unwrap_or(value as f64)
}

pub fn trim_geometry(
    geometry: Arc<dyn IGeometryImpl>,
    trim_start: f32,
    trim_end: f32,
    trim_offset: f32,
    render_interface: &dyn IPlatformRenderInterface,
) -> Option<Arc<dyn IGeometryImpl>> {
    let mut start = widen(trim_start).clamp(0.0, 1.0);
    let mut end = widen(trim_end).clamp(0.0, 1.0);
    if start > end {
        std::mem::swap(&mut start, &mut end);
    }
    if start <= 0.0 && end >= 1.0 {
        return Some(geometry);
    }
    if end - start <= 0.0 {
        return None;
    }
    let length = geometry.contour_length();
    if length.is_nan() || length <= 0.0 {
        return Some(geometry);
    }
    let from = (start + widen(trim_offset)).rem_euclid(1.0);
    let to = from + (end - start);
    let segment = |a: f64, b: f64| geometry.try_get_segment(a * length, b * length, true);
    let trimmed = if to <= 1.0 {
        segment(from, to)
    } else {
        match (segment(from, 1.0), segment(0.0, to - 1.0)) {
            (Some(first), Some(second)) => {
                Some(render_interface.create_geometry_group(FillRule::NonZero, &[first, second]))
            }
            _ => None,
        }
    };
    Some(trimmed.unwrap_or(geometry))
}

/// The cached platform geometry of a server-side geometry.
#[derive(Default)]
struct GeometryCache {
    /// `None` until built; then the trimmed geometry, `None` for nothing.
    value: RefCell<Option<Option<Arc<dyn IGeometryImpl>>>>,
}

impl GeometryCache {
    fn clear(&self) {
        *self.value.borrow_mut() = None;
    }

    fn get_or_build(
        &self,
        compositor: Option<Rc<ServerCompositor>>,
        props: &ServerCompositionGeometryProps,
        build: impl FnOnce(&dyn IPlatformRenderInterface) -> Option<Arc<dyn IGeometryImpl>>,
    ) -> Option<Arc<dyn IGeometryImpl>> {
        if let Some(value) = &*self.value.borrow() {
            return value.clone();
        }
        let compositor = compositor?;
        // Without a render interface nothing is cached: the geometry is
        // built once one is known.
        let built = compositor.render_interface().with_platform_render_interface(|render_interface| {
            build(render_interface).and_then(|geometry| {
                trim_geometry(geometry, props.trim_start(), props.trim_end(), props.trim_offset(), render_interface)
            })
        })?;
        *self.value.borrow_mut() = Some(built.clone());
        built
    }
}

/// Declares a server-side geometry class: the struct, its constructor, the
/// render resource contracts, `IServerObject` and
/// [`IServerCompositionGeometry`] over its `build` method.
macro_rules! server_geometry {
    ($(#[$meta:meta])* $name:ident, $props:ident, $hooks:ident) => {
        $(#[$meta])*
        pub struct $name {
            base: ServerRenderResource,
            props: $props,
            cache: GeometryCache,
        }

        impl $name {
            /// Creates the geometry, active: the UI-thread geometry activates
            /// its server object when it is created.
            pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<$name> {
                let geometry = Rc::new_cyclic(|this: &Weak<$name>| $name {
                    base: ServerRenderResource::new(
                        compositor,
                        this.clone() as Weak<dyn IAnimatedServerObject>,
                        this.clone() as Weak<dyn IServerRenderResource>,
                    ),
                    props: $props::new(),
                    cache: GeometryCache::default(),
                });
                geometry.base.object().activate();
                geometry
            }

            /// The properties of the geometry.
            pub fn props(&self) -> &$props {
                &self.props
            }

            pub fn is_disposed(&self) -> bool {
                self.base.is_disposed()
            }
        }

        impl_server_render_resource!($name, base, composition_property: $props::get_composition_property);

        impl ServerCompositionGeometryHooks for $name {}

        impl $hooks for $name {}

        impl IServerCompositionGeometry for $name {
            fn platform_geometry(&self) -> Option<Arc<dyn IGeometryImpl>> {
                self.cache.get_or_build(self.base.compositor(), self.props.base(), |render_interface| {
                    self.build(render_interface)
                })
            }

            fn geometry_props(&self) -> &ServerCompositionGeometryProps {
                self.props.base()
            }
        }

        impl IServerObject for $name {
            fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
                self.props.deserialize_changes_core(self, reader, committed_at);
            }

            /// A member changed (directly or by an animation): the geometry
            /// is built again when it is next drawn.
            fn values_invalidated(&self) {
                self.cache.clear();
                self.base.core().invalidated(self);
            }

            fn dispose(&self) {
                self.cache.clear();
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

server_geometry!(
    /// The server-side counterpart of a
    /// [`CompositionRectangleGeometry`](crate::rendering::composition::CompositionRectangleGeometry).
    ServerCompositionRectangleGeometry,
    ServerCompositionRectangleGeometryProps,
    ServerCompositionRectangleGeometryHooks
);

impl ServerCompositionRectangleGeometry {
    /// The rectangle `(offset, size)`.
    pub fn rect(&self) -> Rect {
        let (offset, size) = (self.props.offset(), self.props.size());
        Rect::new(offset.x, offset.y, size.x.max(0.0), size.y.max(0.0))
    }

    fn build(&self, render_interface: &dyn IPlatformRenderInterface) -> Option<Arc<dyn IGeometryImpl>> {
        Some(render_interface.create_rectangle_geometry(self.rect()))
    }
}

server_geometry!(
    /// The server-side counterpart of a
    /// [`CompositionRoundedRectangleGeometry`](crate::rendering::composition::CompositionRoundedRectangleGeometry).
    ServerCompositionRoundedRectangleGeometry,
    ServerCompositionRoundedRectangleGeometryProps,
    ServerCompositionRoundedRectangleGeometryHooks
);

impl ServerCompositionRoundedRectangleGeometry {
    /// The rectangle `(offset, size)`.
    pub fn rect(&self) -> Rect {
        let (offset, size) = (self.props.offset(), self.props.size());
        Rect::new(offset.x, offset.y, size.x.max(0.0), size.y.max(0.0))
    }

    /// The radii of the corners, limited to half the size.
    pub fn radii(&self) -> (f64, f64) {
        let rect = self.rect();
        let radius = self.props.corner_radius();
        (radius.x.clamp(0.0, rect.width / 2.0), radius.y.clamp(0.0, rect.height / 2.0))
    }

    fn build(&self, render_interface: &dyn IPlatformRenderInterface) -> Option<Arc<dyn IGeometryImpl>> {
        let rect = self.rect();
        let (radius_x, radius_y) = self.radii();
        if radius_x <= 0.0 || radius_y <= 0.0 {
            return Some(render_interface.create_rectangle_geometry(rect));
        }
        let mut builder = render_interface.create_stream_geometry_builder(None);
        GeometryBuilder::draw_rounded_corners_rectangle(&mut *builder, rect, radius_x, radius_y);
        let geometry = builder.to_geometry();
        builder.dispose();
        Some(geometry)
    }
}

server_geometry!(
    /// The server-side counterpart of a
    /// [`CompositionEllipseGeometry`](crate::rendering::composition::CompositionEllipseGeometry).
    ServerCompositionEllipseGeometry,
    ServerCompositionEllipseGeometryProps,
    ServerCompositionEllipseGeometryHooks
);

impl ServerCompositionEllipseGeometry {
    /// The bounds of the ellipse.
    pub fn rect(&self) -> Rect {
        let (center, radius) = (self.props.center(), self.props.radius());
        let (radius_x, radius_y) = (radius.x.abs(), radius.y.abs());
        Rect::new(center.x - radius_x, center.y - radius_y, radius_x * 2.0, radius_y * 2.0)
    }

    fn build(&self, render_interface: &dyn IPlatformRenderInterface) -> Option<Arc<dyn IGeometryImpl>> {
        Some(render_interface.create_ellipse_geometry(self.rect()))
    }
}

server_geometry!(
    /// The server-side counterpart of a
    /// [`CompositionLineGeometry`](crate::rendering::composition::CompositionLineGeometry).
    ServerCompositionLineGeometry,
    ServerCompositionLineGeometryProps,
    ServerCompositionLineGeometryHooks
);

impl ServerCompositionLineGeometry {
    fn build(&self, render_interface: &dyn IPlatformRenderInterface) -> Option<Arc<dyn IGeometryImpl>> {
        let (start, end) = (self.props.start(), self.props.end());
        Some(render_interface.create_line_geometry(Point::new(start.x, start.y), Point::new(end.x, end.y)))
    }
}

server_geometry!(
    /// The server-side counterpart of a
    /// [`CompositionPathGeometry`](crate::rendering::composition::CompositionPathGeometry).
    ServerCompositionPathGeometry,
    ServerCompositionPathGeometryProps,
    ServerCompositionPathGeometryHooks
);

impl ServerCompositionPathGeometry {
    fn build(&self, _render_interface: &dyn IPlatformRenderInterface) -> Option<Arc<dyn IGeometryImpl>> {
        self.props.path().map(|path| path.geometry().clone())
    }
}
