//! The WinUI shapes: `CompositionShape` and its subclasses
//! `CompositionSpriteShape` and `CompositionContainerShape`, the
//! `CompositionShapeCollection` of a shape visual or a container shape, and
//! the stroke enums.
//!
//! As for the clips, every shape is one [`CompositionShape`] behind an
//! `Rc`, which holds what the concrete class adds in a kind; the concrete
//! classes are typed handles around the same `Rc`
//! ([`CompositionSpriteShape`], [`CompositionContainerShape`]).

use super::composition_object_impls::composition_object_impls;
use super::generated::{
    CompositionContainerShapeHooks, CompositionContainerShapeProps, CompositionShapeCollectionHooks,
    CompositionShapeCollectionProps, CompositionShapeHooks, CompositionShapeProps, CompositionSpriteShapeHooks,
    CompositionSpriteShapeProps, ServerCompositionContainerShapeProps, ServerCompositionSpriteShapeProps,
};
use super::server::{
    IServerObject, ServerCompositionContainerShape, ServerCompositionShapeCollection, ServerCompositionSpriteShape,
    ServerCompositor, ServerObjectId, ServerPropertyValue,
};
use super::transport::{BatchValue, BatchValueReader};
use super::{CompositionBrush, CompositionGeometry, CompositionObject, Compositor, ICompositionObject};
use crate::media::{PenLineCap, PenLineJoin};
use crate::{Matrix, Vector};
use std::ops::Deref;
use std::rc::{Rc, Weak};
use std::sync::Arc;

// --- the stroke enums ----------------------------------------------------------------

/// The shape at an end of a stroke or of a dash (WinUI
/// `CompositionStrokeCap`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CompositionStrokeCap {
    /// The stroke ends at the end point.
    #[default]
    Flat = 0,
    /// A square half the thickness long beyond the end point.
    Square = 1,
    /// A half circle beyond the end point.
    Round = 2,
    /// A triangle half the thickness high beyond the end point.
    Triangle = 3,
}

impl CompositionStrokeCap {
    /// The line cap of the platform pen.
    ///
    /// Deviation (winui-composition.md): the pens of FerroUI have no
    /// triangle cap; `Triangle` is drawn as `Round`, the cap that comes
    /// closest.
    pub fn to_pen_line_cap(self) -> PenLineCap {
        match self {
            CompositionStrokeCap::Flat => PenLineCap::Flat,
            CompositionStrokeCap::Square => PenLineCap::Square,
            CompositionStrokeCap::Round | CompositionStrokeCap::Triangle => PenLineCap::Round,
        }
    }
}

impl BatchValue for CompositionStrokeCap {
    fn write_to(&self, out: &mut Vec<u8>) {
        (*self as i32).write_to(out);
    }

    fn read_from(input: &mut BatchValueReader<'_>) -> Self {
        match i32::read_from(input) {
            0 => Self::Flat,
            1 => Self::Square,
            2 => Self::Round,
            3 => Self::Triangle,
            value => panic!("the batch value stream holds {value}, which is not a CompositionStrokeCap"),
        }
    }
}

impl ServerPropertyValue for CompositionStrokeCap {
    fn differs(&self, other: &Self) -> bool {
        self != other
    }
}

/// How the segments of a stroke are joined (WinUI
/// `CompositionStrokeLineJoin`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CompositionStrokeLineJoin {
    /// A sharp corner, cut at the miter limit.
    #[default]
    Miter = 0,
    /// A corner cut off at the outer ends of the segments.
    Bevel = 1,
    /// A rounded corner.
    Round = 2,
    /// A sharp corner, or a bevel beyond the miter limit.
    MiterOrBevel = 3,
}

impl CompositionStrokeLineJoin {
    /// The line join of the platform pen.
    ///
    /// Deviation (winui-composition.md): the miter join of the platform pens
    /// becomes a bevel beyond the miter limit, so `Miter` (cut at the limit
    /// in WinUI) and `MiterOrBevel` both draw that join.
    pub fn to_pen_line_join(self) -> PenLineJoin {
        match self {
            CompositionStrokeLineJoin::Miter | CompositionStrokeLineJoin::MiterOrBevel => PenLineJoin::Miter,
            CompositionStrokeLineJoin::Bevel => PenLineJoin::Bevel,
            CompositionStrokeLineJoin::Round => PenLineJoin::Round,
        }
    }
}

impl BatchValue for CompositionStrokeLineJoin {
    fn write_to(&self, out: &mut Vec<u8>) {
        (*self as i32).write_to(out);
    }

    fn read_from(input: &mut BatchValueReader<'_>) -> Self {
        match i32::read_from(input) {
            0 => Self::Miter,
            1 => Self::Bevel,
            2 => Self::Round,
            3 => Self::MiterOrBevel,
            value => panic!("the batch value stream holds {value}, which is not a CompositionStrokeLineJoin"),
        }
    }
}

impl ServerPropertyValue for CompositionStrokeLineJoin {
    fn differs(&self, other: &Self) -> bool {
        self != other
    }
}

// --- the shape collection ------------------------------------------------------------

/// The shapes of a shape visual or of a container shape (WinUI
/// `Microsoft.UI.Composition.CompositionShapeCollection`), drawn in order.
pub struct CompositionShapeCollection {
    this: Weak<CompositionShapeCollection>,
    object: CompositionObject,
    props: CompositionShapeCollectionProps<CompositionShape>,
}

impl CompositionShapeCollection {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> Rc<CompositionShapeCollection> {
        let server = compositor.create_server_object(|compositor, _| ServerCompositionShapeCollection::new(compositor));
        let collection = Rc::new_cyclic(|this: &Weak<CompositionShapeCollection>| CompositionShapeCollection {
            this: this.clone(),
            object: CompositionObject::new(compositor, Some(server)),
            props: CompositionShapeCollectionProps::new(),
        });
        collection.props.initialize_defaults(&*collection);
        collection
    }

    /// The id of the server-side collection.
    pub fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    /// A snapshot of the shapes, for enumeration.
    pub fn items(&self) -> Vec<Rc<CompositionShape>> {
        self.props.items()
    }

    /// `Append`: adds a shape at the end (drawn last).
    pub fn append(&self, item: Rc<CompositionShape>) {
        self.add(item);
    }

    pub fn add(&self, item: Rc<CompositionShape>) {
        self.props.add(self, item);
    }

    pub fn clear(&self) {
        self.props.clear(self);
    }

    pub fn contains(&self, item: &Rc<CompositionShape>) -> bool {
        self.props.contains(item)
    }

    pub fn remove(&self, item: &Rc<CompositionShape>) -> bool {
        self.props.remove(self, item)
    }

    /// `Size` of the WinUI vector.
    pub fn count(&self) -> usize {
        self.props.count()
    }

    pub fn index_of(&self, item: &Rc<CompositionShape>) -> Option<usize> {
        self.props.index_of(item)
    }

    pub fn insert(&self, index: usize, item: Rc<CompositionShape>) {
        self.props.insert(self, index, item);
    }

    pub fn remove_at(&self, index: usize) {
        self.props.remove_at(self, index);
    }

    pub fn get(&self, index: usize) -> Rc<CompositionShape> {
        self.props.get(index)
    }

    pub fn set(&self, index: usize, value: Rc<CompositionShape>) {
        self.props.set(self, index, value);
    }
}

impl CompositionShapeCollectionHooks<CompositionShape> for CompositionShapeCollection {}

composition_object_impls!(
    CompositionShapeCollection, "CompositionShapeCollection",
    serialize: |this, writer| this.props.serialize_changes_core(this, writer),
    start_animation: |_this, _name, _animation, _final_value| false,
    composition_property: |_this, _name| None,
);

// --- the shapes ----------------------------------------------------------------------

/// What the concrete class of a shape adds to [`CompositionShape`].
pub(crate) enum CompositionShapeKind {
    Sprite(CompositionSpriteShapeProps),
    Container(CompositionContainerShapeProps),
}

/// A shape of a shape visual (WinUI
/// `Microsoft.UI.Composition.CompositionShape`), with its own animatable
/// transform in the space of its parent (`CenterPoint`, `Offset`,
/// `RotationAngle`, `Scale`, `TransformMatrix`), applied as the transform of
/// a visual: the transform matrix, the scale and the rotation about the
/// center point, then the offset.
pub struct CompositionShape {
    this: Weak<CompositionShape>,
    object: CompositionObject,
    kind: CompositionShapeKind,
    /// The shapes of a container shape.
    shapes: Option<Rc<CompositionShapeCollection>>,
}

impl CompositionShape {
    fn create(
        compositor: &Rc<Compositor>,
        kind: CompositionShapeKind,
        shapes: Option<Rc<CompositionShapeCollection>>,
        server: impl FnOnce(&Rc<ServerCompositor>) -> Rc<dyn IServerObject> + Send + 'static,
    ) -> Rc<CompositionShape> {
        let server = compositor.create_server_object(move |compositor, _| server(compositor));
        let shape = Rc::new_cyclic(|this: &Weak<CompositionShape>| CompositionShape {
            this: this.clone(),
            object: CompositionObject::new(compositor, Some(server)),
            kind,
            shapes,
        });
        match &shape.kind {
            CompositionShapeKind::Sprite(props) => props.initialize_defaults(&*shape),
            CompositionShapeKind::Container(props) => {
                props.initialize_defaults(&*shape);
                let shapes = shape.shapes.clone().map(|shapes| shapes as Rc<dyn ICompositionObject>);
                props.set_shapes(&*shape, shapes);
            }
        }
        shape
    }

    /// The generated properties of `CompositionShape`.
    pub fn shape_props(&self) -> &CompositionShapeProps {
        match &self.kind {
            CompositionShapeKind::Sprite(props) => props.base(),
            CompositionShapeKind::Container(props) => props.base(),
        }
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    /// The embedded `CompositionObject`.
    pub fn object(&self) -> &CompositionObject {
        &self.object
    }

    /// The id of the server-side shape.
    pub fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    pub fn is_disposed(&self) -> bool {
        self.object.is_disposed()
    }

    pub fn dispose(&self) {
        self.object.dispose();
    }

    /// The name of the concrete class, for messages.
    pub fn type_name(&self) -> &'static str {
        match &self.kind {
            CompositionShapeKind::Sprite(_) => "CompositionSpriteShape",
            CompositionShapeKind::Container(_) => "CompositionContainerShape",
        }
    }

    /// The point the shape is scaled and rotated about.
    pub fn center_point(&self) -> Vector {
        self.shape_props().center_point()
    }

    pub fn set_center_point(&self, value: Vector) {
        self.shape_props().set_center_point(self, value)
    }

    /// The offset of the shape in the space of its parent.
    pub fn offset(&self) -> Vector {
        self.shape_props().offset()
    }

    pub fn set_offset(&self, value: Vector) {
        self.shape_props().set_offset(self, value)
    }

    /// The rotation of the shape about its center point, in radians.
    pub fn rotation_angle(&self) -> f32 {
        self.shape_props().rotation_angle()
    }

    pub fn set_rotation_angle(&self, value: f32) {
        self.shape_props().set_rotation_angle(self, value)
    }

    /// The rotation of the shape in degrees: a view of
    /// [`rotation_angle`](Self::rotation_angle).
    ///
    /// Deviation (winui-composition.md): not an animation target, as for the
    /// clips.
    pub fn rotation_angle_in_degrees(&self) -> f32 {
        self.rotation_angle().to_degrees()
    }

    pub fn set_rotation_angle_in_degrees(&self, value: f32) {
        self.set_rotation_angle(value.to_radians())
    }

    /// The scale of the shape about its center point (default `(1, 1)`).
    pub fn scale(&self) -> Vector {
        self.shape_props().scale()
    }

    pub fn set_scale(&self, value: Vector) {
        self.shape_props().set_scale(self, value)
    }

    /// The transform matrix of the shape (WinUI `Matrix3x2`).
    pub fn transform_matrix(&self) -> Matrix {
        self.shape_props().transform_matrix()
    }

    pub fn set_transform_matrix(&self, value: Matrix) {
        self.shape_props().set_transform_matrix(self, value)
    }
}

impl CompositionShapeHooks for CompositionShape {}

impl CompositionSpriteShapeHooks for CompositionShape {}

impl CompositionContainerShapeHooks for CompositionShape {}

composition_object_impls!(
    CompositionShape, "CompositionShape",
    serialize: |this, writer| match &this.kind {
        CompositionShapeKind::Sprite(props) => props.serialize_changes_core(this, writer),
        CompositionShapeKind::Container(props) => props.serialize_changes_core(this, writer),
    },
    start_animation: |this, name, animation, final_value| match &this.kind {
        CompositionShapeKind::Sprite(props) => props.start_animation(this, name, animation, final_value),
        CompositionShapeKind::Container(props) => props.start_animation(this, name, animation, final_value),
    },
    composition_property: |this, name| match &this.kind {
        CompositionShapeKind::Sprite(_) => ServerCompositionSpriteShapeProps::get_composition_property(name),
        CompositionShapeKind::Container(_) => ServerCompositionContainerShapeProps::get_composition_property(name),
    },
);

/// A shape that fills and strokes a geometry (WinUI
/// `Microsoft.UI.Composition.CompositionSpriteShape`).
///
/// The fill and the stroke are composition brushes: the colour and gradient
/// brushes are brushes of the drawing context, a
/// [`CompositionSurfaceBrush`](super::CompositionSurfaceBrush) is painted
/// inside the geometry (the fill) or inside the widened outline (the
/// stroke). `StrokeThickness`, `StrokeDashOffset` and `StrokeMiterLimit` are
/// animatable.
///
/// Deviation (winui-composition.md): the platform pen has one line cap, so
/// `stroke_start_cap` caps both ends of the stroke and the dashes;
/// `stroke_end_cap` and `stroke_dash_cap` are stored only. A non-scaling
/// stroke divides the thickness by the scale of the transforms of the shape
/// (the square root of the determinant), which is exact for a uniform scale
/// only.
#[derive(Clone)]
pub struct CompositionSpriteShape(Rc<CompositionShape>);

impl Deref for CompositionSpriteShape {
    type Target = Rc<CompositionShape>;

    fn deref(&self) -> &Rc<CompositionShape> {
        &self.0
    }
}

impl CompositionSpriteShape {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> CompositionSpriteShape {
        CompositionSpriteShape(CompositionShape::create(
            compositor,
            CompositionShapeKind::Sprite(CompositionSpriteShapeProps::new()),
            None,
            |c| ServerCompositionSpriteShape::new(c),
        ))
    }

    pub(crate) fn with_geometry(
        compositor: &Rc<Compositor>,
        geometry: Option<Rc<CompositionGeometry>>,
    ) -> CompositionSpriteShape {
        let shape = Self::new(compositor);
        shape.set_geometry(geometry);
        shape
    }

    /// The handle of `shape` as a sprite shape, if it is one.
    pub fn from_shape(shape: &Rc<CompositionShape>) -> Option<CompositionSpriteShape> {
        matches!(shape.kind, CompositionShapeKind::Sprite(_)).then(|| CompositionSpriteShape(shape.clone()))
    }

    /// The shape as a [`CompositionShape`].
    pub fn shape(&self) -> &Rc<CompositionShape> {
        &self.0
    }

    fn props(&self) -> &CompositionSpriteShapeProps {
        match &self.0.kind {
            CompositionShapeKind::Sprite(props) => props,
            _ => unreachable!("a sprite shape handle wraps a sprite shape"),
        }
    }

    /// The brush the geometry is filled with.
    pub fn fill_brush(&self) -> Option<Rc<CompositionBrush>> {
        self.props().fill_brush().and_then(|brush| brush.into_any_rc().downcast::<CompositionBrush>().ok())
    }

    pub fn set_fill_brush(&self, value: Option<Rc<CompositionBrush>>) {
        self.props().set_fill_brush(&*self.0, value.map(|brush| brush as Rc<dyn ICompositionObject>))
    }

    /// The geometry the shape draws.
    pub fn geometry(&self) -> Option<Rc<CompositionGeometry>> {
        self.props().geometry().and_then(|geometry| geometry.into_any_rc().downcast::<CompositionGeometry>().ok())
    }

    pub fn set_geometry(&self, value: Option<Rc<CompositionGeometry>>) {
        self.props().set_geometry(&*self.0, value.map(|geometry| geometry as Rc<dyn ICompositionObject>))
    }

    /// Whether the thickness of the stroke ignores the transforms of the
    /// shape (and of the view box).
    pub fn is_stroke_non_scaling(&self) -> bool {
        self.props().is_stroke_non_scaling()
    }

    pub fn set_is_stroke_non_scaling(&self, value: bool) {
        self.props().set_is_stroke_non_scaling(&*self.0, value)
    }

    /// The brush the outline is stroked with.
    pub fn stroke_brush(&self) -> Option<Rc<CompositionBrush>> {
        self.props().stroke_brush().and_then(|brush| brush.into_any_rc().downcast::<CompositionBrush>().ok())
    }

    pub fn set_stroke_brush(&self, value: Option<Rc<CompositionBrush>>) {
        self.props().set_stroke_brush(&*self.0, value.map(|brush| brush as Rc<dyn ICompositionObject>))
    }

    /// The lengths of the dashes and gaps of the stroke, in multiples of the
    /// stroke thickness (WinUI `CompositionStrokeDashArray`); empty for a
    /// solid stroke.
    ///
    /// Deviation (winui-composition.md): a plain list of values, replaced as a
    /// whole, not a `CompositionObject` of its own.
    pub fn stroke_dash_array(&self) -> Vec<f32> {
        self.props().stroke_dash_array().map(|dashes| (*dashes).clone()).unwrap_or_default()
    }

    pub fn set_stroke_dash_array(&self, value: Vec<f32>) {
        let value = if value.is_empty() { None } else { Some(Arc::new(value)) };
        self.props().set_stroke_dash_array(&*self.0, value)
    }

    /// The cap of the ends of the dashes (stored only, see the type).
    pub fn stroke_dash_cap(&self) -> CompositionStrokeCap {
        self.props().stroke_dash_cap()
    }

    pub fn set_stroke_dash_cap(&self, value: CompositionStrokeCap) {
        self.props().set_stroke_dash_cap(&*self.0, value)
    }

    /// How far into the dash pattern the stroke starts, in multiples of the
    /// stroke thickness.
    pub fn stroke_dash_offset(&self) -> f32 {
        self.props().stroke_dash_offset()
    }

    pub fn set_stroke_dash_offset(&self, value: f32) {
        self.props().set_stroke_dash_offset(&*self.0, value)
    }

    /// The cap of the end of the stroke (stored only, see the type).
    pub fn stroke_end_cap(&self) -> CompositionStrokeCap {
        self.props().stroke_end_cap()
    }

    pub fn set_stroke_end_cap(&self, value: CompositionStrokeCap) {
        self.props().set_stroke_end_cap(&*self.0, value)
    }

    /// How the segments of the stroke are joined (default `Miter`).
    pub fn stroke_line_join(&self) -> CompositionStrokeLineJoin {
        self.props().stroke_line_join()
    }

    pub fn set_stroke_line_join(&self, value: CompositionStrokeLineJoin) {
        self.props().set_stroke_line_join(&*self.0, value)
    }

    /// The limit of the ratio of the miter length to half the thickness
    /// (default 2).
    pub fn stroke_miter_limit(&self) -> f32 {
        self.props().stroke_miter_limit()
    }

    pub fn set_stroke_miter_limit(&self, value: f32) {
        self.props().set_stroke_miter_limit(&*self.0, value)
    }

    /// The cap of the start of the stroke; the platform pen caps both ends
    /// and the dashes with it.
    pub fn stroke_start_cap(&self) -> CompositionStrokeCap {
        self.props().stroke_start_cap()
    }

    pub fn set_stroke_start_cap(&self, value: CompositionStrokeCap) {
        self.props().set_stroke_start_cap(&*self.0, value)
    }

    /// The thickness of the stroke (default 1).
    pub fn stroke_thickness(&self) -> f32 {
        self.props().stroke_thickness()
    }

    pub fn set_stroke_thickness(&self, value: f32) {
        self.props().set_stroke_thickness(&*self.0, value)
    }
}

/// A shape that groups shapes under its transform (WinUI
/// `Microsoft.UI.Composition.CompositionContainerShape`): its shapes are
/// drawn in order, in its space.
#[derive(Clone)]
pub struct CompositionContainerShape(Rc<CompositionShape>);

impl Deref for CompositionContainerShape {
    type Target = Rc<CompositionShape>;

    fn deref(&self) -> &Rc<CompositionShape> {
        &self.0
    }
}

impl CompositionContainerShape {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> CompositionContainerShape {
        let shapes = CompositionShapeCollection::new(compositor);
        CompositionContainerShape(CompositionShape::create(
            compositor,
            CompositionShapeKind::Container(CompositionContainerShapeProps::new()),
            Some(shapes),
            |c| ServerCompositionContainerShape::new(c),
        ))
    }

    /// The handle of `shape` as a container shape, if it is one.
    pub fn from_shape(shape: &Rc<CompositionShape>) -> Option<CompositionContainerShape> {
        matches!(shape.kind, CompositionShapeKind::Container(_)).then(|| CompositionContainerShape(shape.clone()))
    }

    /// The shape as a [`CompositionShape`].
    pub fn shape(&self) -> &Rc<CompositionShape> {
        &self.0
    }

    /// The shapes of the container.
    pub fn shapes(&self) -> &Rc<CompositionShapeCollection> {
        self.0.shapes.as_ref().expect("a container shape has a shape collection")
    }
}
