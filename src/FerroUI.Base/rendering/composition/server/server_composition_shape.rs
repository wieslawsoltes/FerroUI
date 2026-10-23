//! The server-side counterparts of the WinUI shapes
//! (`CompositionSpriteShape`, `CompositionContainerShape`) and of
//! `CompositionShapeCollection`, the pen of a sprite shape, and the drawing
//! of a tree of shapes.

use super::{
    composition_transform, impl_server_render_resource, paint_composition_brush, platform_geometry_of,
    CompositionProperty, IAnimatedServerObject, IServerObject, IServerRenderResource, IServerRenderResourceObserver,
    ServerCompositionSurfaceBrush, ServerCompositor, ServerRenderResource,
};
use crate::media::immutable::{ImmutableDashStyle, ImmutablePen, ImmutableSolidColorBrush};
use crate::media::{IBrush, IDashStyle, IImmutableBrush, IPen, PenLineCap, PenLineJoin};
use crate::platform::{IDrawingContextImpl, IGeometryImpl};
use crate::rendering::composition::generated::{
    ServerCompositionContainerShapeHooks, ServerCompositionContainerShapeProps, ServerCompositionShapeCollectionHooks,
    ServerCompositionShapeCollectionProps, ServerCompositionShapeHooks, ServerCompositionShapeProps,
    ServerCompositionSpriteShapeHooks, ServerCompositionSpriteShapeProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::{Matrix, Vector};
use std::any::{Any, TypeId};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Duration;

/// How deep container shapes may nest before the drawing stops: a
/// container that contains itself would recurse for ever.
const MAX_SHAPE_DEPTH: usize = 256;

// --- the shape collection ------------------------------------------------------------

fn no_composition_property(_name: &str) -> Option<&'static CompositionProperty> {
    None
}

/// The server-side counterpart of a
/// [`CompositionShapeCollection`](crate::rendering::composition::CompositionShapeCollection).
///
/// A render resource that observes its shapes: a change of a shape (or of
/// what the shape shows) invalidates the collection, and so the shape
/// visual or container shape that shows it.
pub struct ServerCompositionShapeCollection {
    this: Weak<ServerCompositionShapeCollection>,
    base: ServerRenderResource,
    props: ServerCompositionShapeCollectionProps,
}

impl ServerCompositionShapeCollection {
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionShapeCollection> {
        let collection = Rc::new_cyclic(|this: &Weak<ServerCompositionShapeCollection>| {
            ServerCompositionShapeCollection {
                this: this.clone(),
                base: ServerRenderResource::new(
                    compositor,
                    this.clone() as Weak<dyn IAnimatedServerObject>,
                    this.clone() as Weak<dyn IServerRenderResource>,
                ),
                props: ServerCompositionShapeCollectionProps::new(),
            }
        });
        collection.base.object().activate();
        collection
    }

    /// The shapes, in drawing order: server-side sprite and container
    /// shapes.
    pub fn shapes(&self) -> Vec<Rc<dyn IServerObject>> {
        self.props.list().list()
    }

    pub fn count(&self) -> usize {
        self.props.list().count()
    }

    pub fn is_disposed(&self) -> bool {
        self.base.is_disposed()
    }

    /// The collection behind `object`, if it is one.
    pub fn of(object: &Rc<dyn IServerObject>) -> Option<&ServerCompositionShapeCollection> {
        object.as_any().downcast_ref::<ServerCompositionShapeCollection>()
    }

    /// Starts or stops observing the shapes.
    fn observe(&self, shapes: &[Rc<dyn IServerObject>], observe: bool) {
        let Some(this) = self.this.upgrade() else { return };
        let observer: Rc<dyn IServerRenderResourceObserver> = this;
        for shape in shapes {
            if let Some(resource) = shape.clone().as_render_resource() {
                if observe {
                    resource.add_observer(&observer);
                } else {
                    resource.remove_observer(&observer);
                }
            }
        }
    }
}

impl_server_render_resource!(ServerCompositionShapeCollection, base, composition_property: no_composition_property);

impl ServerCompositionShapeCollectionHooks for ServerCompositionShapeCollection {}

impl IServerObject for ServerCompositionShapeCollection {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        let old = self.shapes();
        self.props.deserialize_changes_core(self, reader, committed_at);
        let new = self.shapes();
        self.observe(&old, false);
        self.observe(&new, true);
    }

    fn values_invalidated(&self) {
        self.base.core().invalidated(self);
    }

    fn dispose(&self) {
        self.observe(&self.shapes(), false);
        self.props.list().clear();
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

// --- the pen -------------------------------------------------------------------------

/// The pen a sprite shape strokes its geometry with, made from the stroke
/// members of the shape when it is drawn.
#[derive(Clone)]
pub struct CompositionShapePen {
    brush: Option<Rc<dyn IBrush>>,
    thickness: f64,
    dash_style: Option<Rc<ImmutableDashStyle>>,
    line_cap: PenLineCap,
    line_join: PenLineJoin,
    miter_limit: f64,
}

impl IPen for CompositionShapePen {
    fn brush(&self) -> Option<Rc<dyn IBrush>> {
        self.brush.clone()
    }

    fn dash_style(&self) -> Option<Rc<dyn IDashStyle>> {
        self.dash_style.clone().map(|style| style as Rc<dyn IDashStyle>)
    }

    fn line_cap(&self) -> PenLineCap {
        self.line_cap
    }

    fn line_join(&self) -> PenLineJoin {
        self.line_join
    }

    fn miter_limit(&self) -> f64 {
        self.miter_limit
    }

    fn thickness(&self) -> f64 {
        self.thickness
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_immutable_pen(self: Rc<Self>) -> Rc<ImmutablePen> {
        // As the server-side simple pen: an immutable brush is taken as it
        // is, a server-side solid colour brush by value.
        let brush: Option<Rc<dyn IImmutableBrush>> = self.brush.clone().and_then(|brush| {
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
            self.thickness,
            self.dash_style.clone(),
            self.line_cap,
            self.line_join,
            self.miter_limit,
        ))
    }
}

// --- the shapes ----------------------------------------------------------------------

/// The transform of a shape in the space of its parent, from the members of
/// `CompositionShape`; `None` for the identity.
pub fn shape_transform(props: &ServerCompositionShapeProps) -> Option<Matrix> {
    composition_transform(
        Vector::new(0.0, 0.0),
        Vector::new(0.0, 0.0),
        props.center_point(),
        props.transform_matrix(),
        props.scale(),
        props.rotation_angle(),
        props.offset(),
    )
}

/// Declares a server-side shape class: the struct, its constructor, the
/// render resource contracts and `IServerObject`.
macro_rules! server_shape {
    ($(#[$meta:meta])* $name:ident, $props:ident, $hooks:ident) => {
        $(#[$meta])*
        pub struct $name {
            base: ServerRenderResource,
            props: $props,
        }

        impl $name {
            /// Creates the shape, active: the UI-thread shape activates its
            /// server object when it is created.
            pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<$name> {
                let shape = Rc::new_cyclic(|this: &Weak<$name>| $name {
                    base: ServerRenderResource::new(
                        compositor,
                        this.clone() as Weak<dyn IAnimatedServerObject>,
                        this.clone() as Weak<dyn IServerRenderResource>,
                    ),
                    props: $props::new(),
                });
                shape.base.object().activate();
                shape
            }

            /// The properties of the shape.
            pub fn props(&self) -> &$props {
                &self.props
            }

            /// The properties of `CompositionShape`.
            pub fn shape_props(&self) -> &ServerCompositionShapeProps {
                self.props.base()
            }

            /// The transform of the shape in the space of its parent; `None`
            /// for the identity.
            pub fn transform(&self) -> Option<Matrix> {
                shape_transform(self.props.base())
            }

            pub fn is_disposed(&self) -> bool {
                self.base.is_disposed()
            }
        }

        impl_server_render_resource!($name, base, composition_property: $props::get_composition_property);

        impl ServerCompositionShapeHooks for $name {}

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

server_shape!(
    /// The server-side counterpart of a
    /// [`CompositionSpriteShape`](crate::rendering::composition::CompositionSpriteShape).
    ServerCompositionSpriteShape,
    ServerCompositionSpriteShapeProps,
    ServerCompositionSpriteShapeHooks
);

/// Whether `brush` (a server-side composition brush) is a surface brush,
/// which is painted, not handed to the drawing context.
fn is_surface_brush(brush: &Rc<dyn IServerObject>) -> bool {
    brush.as_any().is::<ServerCompositionSurfaceBrush>()
}

impl ServerCompositionSpriteShape {
    /// The geometry of the shape, trimmed.
    pub fn platform_geometry(&self) -> Option<Arc<dyn IGeometryImpl>> {
        platform_geometry_of(&self.props.geometry()?)
    }

    /// The pen the shape strokes with when its transform (with the
    /// transforms of its parents and of the view box) is `transform`, with
    /// `brush` as the brush of the pen. `None` when the thickness is not
    /// positive.
    ///
    /// Deviation (winui-composition.md): a non-scaling stroke divides the
    /// thickness by the square root of the determinant of `transform`,
    /// exact for a uniform scale only; the platform pen has one cap, the
    /// start cap of the shape; the dash array and offset are in multiples of
    /// the thickness, as the dash style of the platform pen.
    pub fn pen(&self, transform: Matrix, brush: Option<Rc<dyn IBrush>>) -> Option<CompositionShapePen> {
        let props = &self.props;
        let mut thickness = props.stroke_thickness() as f64;
        if props.is_stroke_non_scaling() {
            let scale = transform.get_determinant().abs().sqrt();
            if scale > f64::EPSILON {
                thickness /= scale;
            }
        }
        if thickness.is_nan() || thickness <= 0.0 {
            return None;
        }
        let dash_style = props.stroke_dash_array().filter(|dashes| !dashes.is_empty()).map(|dashes| {
            let dashes: Vec<f64> = dashes.iter().map(|dash| *dash as f64).collect();
            Rc::new(ImmutableDashStyle::new(Some(dashes.as_slice()), props.stroke_dash_offset() as f64))
        });
        Some(CompositionShapePen {
            brush,
            thickness,
            dash_style,
            line_cap: props.stroke_start_cap().to_pen_line_cap(),
            line_join: props.stroke_line_join().to_pen_line_join(),
            miter_limit: props.stroke_miter_limit() as f64,
        })
    }

    /// Draws the shape: `transform` is its transform in the space of the
    /// visual (with its parents' and the view box), `base` the transform of
    /// the canvas for the visual. The fill first, then the stroke; a surface
    /// brush is painted inside the geometry (fill) or the widened outline
    /// (stroke), every other brush is a brush of the drawing context.
    fn render(&self, canvas: &mut dyn IDrawingContextImpl, transform: Matrix, base: Matrix) {
        let Some(geometry) = self.platform_geometry() else { return };
        let fill = self.props.fill_brush();
        let stroke = self.props.stroke_brush();
        let fill_platform = fill.as_ref().and_then(|brush| brush.clone().as_brush());
        let stroke_platform = stroke.as_ref().and_then(|brush| brush.clone().as_brush());
        let pen = stroke_platform.and_then(|brush| self.pen(transform, Some(brush)));

        canvas.set_transform(transform * base);
        if let Some(brush) = fill.as_ref().filter(|brush| is_surface_brush(brush)) {
            canvas.push_geometry_clip(&*geometry);
            paint_composition_brush(canvas, brush, geometry.bounds());
            canvas.pop_geometry_clip();
        }
        if fill_platform.is_some() || pen.is_some() {
            canvas.draw_geometry(fill_platform.as_deref(), pen.as_ref().map(|pen| pen as &dyn IPen), &*geometry);
        }
        if let Some(brush) = stroke.as_ref().filter(|brush| is_surface_brush(brush)) {
            if let Some(outline) = self.pen(transform, None) {
                let widened = geometry.get_widened_geometry(&outline);
                canvas.push_geometry_clip(&*widened);
                paint_composition_brush(canvas, brush, widened.bounds());
                canvas.pop_geometry_clip();
            }
        }
        canvas.set_transform(base);
    }
}

server_shape!(
    /// The server-side counterpart of a
    /// [`CompositionContainerShape`](crate::rendering::composition::CompositionContainerShape).
    ServerCompositionContainerShape,
    ServerCompositionContainerShapeProps,
    ServerCompositionContainerShapeHooks
);

impl ServerCompositionContainerShape {
    /// The shapes of the container, in drawing order.
    pub fn shapes(&self) -> Vec<Rc<dyn IServerObject>> {
        self.props
            .shapes()
            .as_ref()
            .and_then(ServerCompositionShapeCollection::of)
            .map(ServerCompositionShapeCollection::shapes)
            .unwrap_or_default()
    }
}

/// Draws `shapes` (server-side sprite and container shapes) in order onto
/// `canvas`: `parent` is the transform from the space of the shapes to the
/// space of the visual (the view box, the containers), `base` the transform
/// of the canvas for the visual. The transform of the canvas is `base`
/// again afterwards.
pub fn render_shapes(canvas: &mut dyn IDrawingContextImpl, shapes: &[Rc<dyn IServerObject>], parent: Matrix, base: Matrix) {
    render_shapes_at(canvas, shapes, parent, base, 0);
}

fn render_shapes_at(
    canvas: &mut dyn IDrawingContextImpl,
    shapes: &[Rc<dyn IServerObject>],
    parent: Matrix,
    base: Matrix,
    depth: usize,
) {
    if depth > MAX_SHAPE_DEPTH {
        return;
    }
    for shape in shapes {
        let any = shape.as_any();
        if let Some(sprite) = any.downcast_ref::<ServerCompositionSpriteShape>() {
            let transform = sprite.transform().map_or(parent, |local| local * parent);
            sprite.render(canvas, transform, base);
        } else if let Some(container) = any.downcast_ref::<ServerCompositionContainerShape>() {
            let transform = container.transform().map_or(parent, |local| local * parent);
            render_shapes_at(canvas, &container.shapes(), transform, base, depth + 1);
        }
    }
}
