//! The WinUI clip objects: `CompositionClip` and its subclasses `InsetClip`,
//! `RectangleClip` and `GeometricClip`.
//!
//! As for the brushes, every clip is one [`CompositionClip`] behind an `Rc`,
//! which holds what the concrete class adds in a kind; the concrete classes
//! are typed handles around the same `Rc` ([`InsetClip`],
//! [`RectangleClip`]). A clip is set on a visual with
//! [`CompositionVisual::set_composition_clip`](super::CompositionVisual::set_composition_clip).

use super::composition_object_impls::composition_object_impls;
use super::generated::{
    CompositionClipHooks, CompositionClipProps, GeometricClipHooks, GeometricClipProps, InsetClipHooks,
    InsetClipProps, RectangleClipHooks, RectangleClipProps, ServerGeometricClipProps, ServerInsetClipProps,
    ServerRectangleClipProps,
};
use super::server::{
    IServerObject, ServerCompositor, ServerGeometricClip, ServerInsetClip, ServerObjectId, ServerRectangleClip,
};
use super::{CompositionGeometry, CompositionObject, CompositionViewBox, Compositor, ICompositionObject};
use crate::{Matrix, Vector};
use std::ops::Deref;
use std::rc::{Rc, Weak};

/// What the concrete class of a clip adds to [`CompositionClip`].
pub(crate) enum CompositionClipKind {
    Inset(InsetClipProps),
    Rectangle(RectangleClipProps),
    Geometric(GeometricClipProps),
}

/// A clip of a visual (WinUI `Microsoft.UI.Composition.CompositionClip`):
/// the area of the visual that stays visible, with its own animatable
/// transform (`AnchorPoint`, `CenterPoint`, `Offset`, `RotationAngle`,
/// `Scale`, `TransformMatrix`) in the space of the visual.
///
/// Deviation (winui-composition.md): `AnchorPoint` is relative to the size of
/// the visual the clip is set on; `RotationAngleInDegrees` is a view of
/// `RotationAngle` and cannot be the target of an animation.
pub struct CompositionClip {
    this: Weak<CompositionClip>,
    object: CompositionObject,
    kind: CompositionClipKind,
}

impl CompositionClip {
    fn create(
        compositor: &Rc<Compositor>,
        kind: CompositionClipKind,
        server: impl FnOnce(&Rc<ServerCompositor>) -> Rc<dyn IServerObject> + Send + 'static,
    ) -> Rc<CompositionClip> {
        let server = compositor.create_server_object(move |compositor, _| server(compositor));
        let clip = Rc::new_cyclic(|this: &Weak<CompositionClip>| CompositionClip {
            this: this.clone(),
            object: CompositionObject::new(compositor, Some(server)),
            kind,
        });
        match &clip.kind {
            CompositionClipKind::Inset(props) => props.initialize_defaults(&*clip),
            CompositionClipKind::Rectangle(props) => props.initialize_defaults(&*clip),
            CompositionClipKind::Geometric(props) => props.initialize_defaults(&*clip),
        }
        clip
    }

    /// The generated properties of `CompositionClip`.
    pub fn clip_props(&self) -> &CompositionClipProps {
        match &self.kind {
            CompositionClipKind::Inset(props) => props.base(),
            CompositionClipKind::Rectangle(props) => props.base(),
            CompositionClipKind::Geometric(props) => props.base(),
        }
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    /// The embedded `CompositionObject`.
    pub fn object(&self) -> &CompositionObject {
        &self.object
    }

    /// The id of the server-side clip.
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
            CompositionClipKind::Inset(_) => "InsetClip",
            CompositionClipKind::Rectangle(_) => "RectangleClip",
            CompositionClipKind::Geometric(_) => "GeometricClip",
        }
    }

    /// The point of the clip, relative to the size of the visual, that is
    /// placed at `offset`.
    pub fn anchor_point(&self) -> Vector {
        self.clip_props().anchor_point()
    }

    pub fn set_anchor_point(&self, value: Vector) {
        self.clip_props().set_anchor_point(self, value)
    }

    /// The point the clip is scaled and rotated about.
    pub fn center_point(&self) -> Vector {
        self.clip_props().center_point()
    }

    pub fn set_center_point(&self, value: Vector) {
        self.clip_props().set_center_point(self, value)
    }

    /// The offset of the clip.
    pub fn offset(&self) -> Vector {
        self.clip_props().offset()
    }

    pub fn set_offset(&self, value: Vector) {
        self.clip_props().set_offset(self, value)
    }

    /// The rotation of the clip about its center point, in radians.
    pub fn rotation_angle(&self) -> f32 {
        self.clip_props().rotation_angle()
    }

    pub fn set_rotation_angle(&self, value: f32) {
        self.clip_props().set_rotation_angle(self, value)
    }

    /// The rotation of the clip in degrees: a view of
    /// [`rotation_angle`](Self::rotation_angle).
    pub fn rotation_angle_in_degrees(&self) -> f32 {
        self.rotation_angle().to_degrees()
    }

    pub fn set_rotation_angle_in_degrees(&self, value: f32) {
        self.set_rotation_angle(value.to_radians())
    }

    /// The scale of the clip about its center point (default `(1, 1)`).
    pub fn scale(&self) -> Vector {
        self.clip_props().scale()
    }

    pub fn set_scale(&self, value: Vector) {
        self.clip_props().set_scale(self, value)
    }

    /// The transform matrix of the clip (WinUI `Matrix3x2`).
    pub fn transform_matrix(&self) -> Matrix {
        self.clip_props().transform_matrix()
    }

    pub fn set_transform_matrix(&self, value: Matrix) {
        self.clip_props().set_transform_matrix(self, value)
    }
}

impl CompositionClipHooks for CompositionClip {}

impl InsetClipHooks for CompositionClip {}

impl RectangleClipHooks for CompositionClip {}

impl GeometricClipHooks for CompositionClip {}

composition_object_impls!(
    CompositionClip, "CompositionClip",
    serialize: |this, writer| match &this.kind {
        CompositionClipKind::Inset(props) => props.serialize_changes_core(this, writer),
        CompositionClipKind::Rectangle(props) => props.serialize_changes_core(this, writer),
        CompositionClipKind::Geometric(props) => props.serialize_changes_core(this, writer),
    },
    start_animation: |this, name, animation, final_value| match &this.kind {
        CompositionClipKind::Inset(props) => props.start_animation(this, name, animation, final_value),
        CompositionClipKind::Rectangle(props) => props.start_animation(this, name, animation, final_value),
        CompositionClipKind::Geometric(props) => props.start_animation(this, name, animation, final_value),
    },
    composition_property: |this, name| match &this.kind {
        CompositionClipKind::Inset(_) => ServerInsetClipProps::get_composition_property(name),
        CompositionClipKind::Rectangle(_) => ServerRectangleClipProps::get_composition_property(name),
        CompositionClipKind::Geometric(_) => ServerGeometricClipProps::get_composition_property(name),
    },
);

/// A clip that leaves visible the rectangle `(left_inset, top_inset,
/// width - right_inset, height - bottom_inset)` of the visual it is set on
/// (WinUI `Microsoft.UI.Composition.InsetClip`). The insets are animatable
/// (`LeftInset`, `TopInset`, `RightInset`, `BottomInset`).
#[derive(Clone)]
pub struct InsetClip(Rc<CompositionClip>);

impl Deref for InsetClip {
    type Target = Rc<CompositionClip>;

    fn deref(&self) -> &Rc<CompositionClip> {
        &self.0
    }
}

impl InsetClip {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> InsetClip {
        InsetClip(CompositionClip::create(
            compositor,
            CompositionClipKind::Inset(InsetClipProps::new()),
            |c| ServerInsetClip::new(c),
        ))
    }

    pub(crate) fn with_insets(compositor: &Rc<Compositor>, left: f32, top: f32, right: f32, bottom: f32) -> InsetClip {
        let clip = Self::new(compositor);
        clip.set_left_inset(left);
        clip.set_top_inset(top);
        clip.set_right_inset(right);
        clip.set_bottom_inset(bottom);
        clip
    }

    /// The handle of `clip` as an inset clip, if it is one.
    pub fn from_clip(clip: &Rc<CompositionClip>) -> Option<InsetClip> {
        matches!(clip.kind, CompositionClipKind::Inset(_)).then(|| InsetClip(clip.clone()))
    }

    fn props(&self) -> &InsetClipProps {
        match &self.0.kind {
            CompositionClipKind::Inset(props) => props,
            _ => unreachable!("an inset clip handle wraps an inset clip"),
        }
    }

    pub fn left_inset(&self) -> f32 {
        self.props().left_inset()
    }

    pub fn set_left_inset(&self, value: f32) {
        self.props().set_left_inset(&*self.0, value)
    }

    pub fn top_inset(&self) -> f32 {
        self.props().top_inset()
    }

    pub fn set_top_inset(&self, value: f32) {
        self.props().set_top_inset(&*self.0, value)
    }

    pub fn right_inset(&self) -> f32 {
        self.props().right_inset()
    }

    pub fn set_right_inset(&self, value: f32) {
        self.props().set_right_inset(&*self.0, value)
    }

    pub fn bottom_inset(&self) -> f32 {
        self.props().bottom_inset()
    }

    pub fn set_bottom_inset(&self, value: f32) {
        self.props().set_bottom_inset(&*self.0, value)
    }
}

/// A clip that leaves visible a rectangle, with optional rounded corners,
/// in the coordinates of the visual it is set on (WinUI
/// `Microsoft.UI.Composition.RectangleClip`). Every member is animatable
/// (`Left`, `Top`, `Right`, `Bottom`, `TopLeftRadius`, `TopRightRadius`,
/// `BottomLeftRadius`, `BottomRightRadius`); the radii are elliptical
/// (WinUI `Vector2`).
#[derive(Clone)]
pub struct RectangleClip(Rc<CompositionClip>);

impl Deref for RectangleClip {
    type Target = Rc<CompositionClip>;

    fn deref(&self) -> &Rc<CompositionClip> {
        &self.0
    }
}

impl RectangleClip {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> RectangleClip {
        RectangleClip(CompositionClip::create(
            compositor,
            CompositionClipKind::Rectangle(RectangleClipProps::new()),
            |c| ServerRectangleClip::new(c),
        ))
    }

    pub(crate) fn with_edges(compositor: &Rc<Compositor>, left: f32, top: f32, right: f32, bottom: f32) -> RectangleClip {
        let clip = Self::new(compositor);
        clip.set_left(left);
        clip.set_top(top);
        clip.set_right(right);
        clip.set_bottom(bottom);
        clip
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn with_edges_and_radii(
        compositor: &Rc<Compositor>,
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
        top_left_radius: Vector,
        top_right_radius: Vector,
        bottom_right_radius: Vector,
        bottom_left_radius: Vector,
    ) -> RectangleClip {
        let clip = Self::with_edges(compositor, left, top, right, bottom);
        clip.set_top_left_radius(top_left_radius);
        clip.set_top_right_radius(top_right_radius);
        clip.set_bottom_right_radius(bottom_right_radius);
        clip.set_bottom_left_radius(bottom_left_radius);
        clip
    }

    /// The handle of `clip` as a rectangle clip, if it is one.
    pub fn from_clip(clip: &Rc<CompositionClip>) -> Option<RectangleClip> {
        matches!(clip.kind, CompositionClipKind::Rectangle(_)).then(|| RectangleClip(clip.clone()))
    }

    fn props(&self) -> &RectangleClipProps {
        match &self.0.kind {
            CompositionClipKind::Rectangle(props) => props,
            _ => unreachable!("a rectangle clip handle wraps a rectangle clip"),
        }
    }

    pub fn left(&self) -> f32 {
        self.props().left()
    }

    pub fn set_left(&self, value: f32) {
        self.props().set_left(&*self.0, value)
    }

    pub fn top(&self) -> f32 {
        self.props().top()
    }

    pub fn set_top(&self, value: f32) {
        self.props().set_top(&*self.0, value)
    }

    pub fn right(&self) -> f32 {
        self.props().right()
    }

    pub fn set_right(&self, value: f32) {
        self.props().set_right(&*self.0, value)
    }

    pub fn bottom(&self) -> f32 {
        self.props().bottom()
    }

    pub fn set_bottom(&self, value: f32) {
        self.props().set_bottom(&*self.0, value)
    }

    pub fn top_left_radius(&self) -> Vector {
        self.props().top_left_radius()
    }

    pub fn set_top_left_radius(&self, value: Vector) {
        self.props().set_top_left_radius(&*self.0, value)
    }

    pub fn top_right_radius(&self) -> Vector {
        self.props().top_right_radius()
    }

    pub fn set_top_right_radius(&self, value: Vector) {
        self.props().set_top_right_radius(&*self.0, value)
    }

    pub fn bottom_left_radius(&self) -> Vector {
        self.props().bottom_left_radius()
    }

    pub fn set_bottom_left_radius(&self, value: Vector) {
        self.props().set_bottom_left_radius(&*self.0, value)
    }

    pub fn bottom_right_radius(&self) -> Vector {
        self.props().bottom_right_radius()
    }

    pub fn set_bottom_right_radius(&self, value: Vector) {
        self.props().set_bottom_right_radius(&*self.0, value)
    }
}

/// A clip that leaves visible the inside of a geometry (WinUI
/// `Microsoft.UI.Composition.GeometricClip`), in the coordinates of the
/// visual it is set on, or of its [`view_box`](Self::view_box) when it has
/// one: the view box maps the geometry onto the size of the visual as for a
/// shape visual. The transform of the clip applies after the view box. A
/// change of the geometry (its trimming and animations included) or of the
/// view box changes the clip.
///
/// Deviation (winui-composition.md): a geometric clip without a geometry
/// clips nothing; the trimming of the geometry applies to the clip.
#[derive(Clone)]
pub struct GeometricClip(Rc<CompositionClip>);

impl Deref for GeometricClip {
    type Target = Rc<CompositionClip>;

    fn deref(&self) -> &Rc<CompositionClip> {
        &self.0
    }
}

impl GeometricClip {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> GeometricClip {
        GeometricClip(CompositionClip::create(
            compositor,
            CompositionClipKind::Geometric(GeometricClipProps::new()),
            |c| ServerGeometricClip::new(c),
        ))
    }

    pub(crate) fn with_geometry(compositor: &Rc<Compositor>, geometry: Option<Rc<CompositionGeometry>>) -> GeometricClip {
        let clip = Self::new(compositor);
        clip.set_geometry(geometry);
        clip
    }

    /// The handle of `clip` as a geometric clip, if it is one.
    pub fn from_clip(clip: &Rc<CompositionClip>) -> Option<GeometricClip> {
        matches!(clip.kind, CompositionClipKind::Geometric(_)).then(|| GeometricClip(clip.clone()))
    }

    fn props(&self) -> &GeometricClipProps {
        match &self.0.kind {
            CompositionClipKind::Geometric(props) => props,
            _ => unreachable!("a geometric clip handle wraps a geometric clip"),
        }
    }

    /// The geometry whose inside stays visible.
    pub fn geometry(&self) -> Option<Rc<CompositionGeometry>> {
        self.props().geometry().and_then(|geometry| geometry.into_any_rc().downcast::<CompositionGeometry>().ok())
    }

    pub fn set_geometry(&self, value: Option<Rc<CompositionGeometry>>) {
        self.props().set_geometry(&*self.0, value.map(|geometry| geometry as Rc<dyn ICompositionObject>))
    }

    /// The area of the geometry coordinates mapped onto the visual.
    pub fn view_box(&self) -> Option<Rc<CompositionViewBox>> {
        self.props().view_box().and_then(|view_box| view_box.into_any_rc().downcast::<CompositionViewBox>().ok())
    }

    pub fn set_view_box(&self, value: Option<Rc<CompositionViewBox>>) {
        self.props().set_view_box(&*self.0, value.map(|view_box| view_box as Rc<dyn ICompositionObject>))
    }
}
