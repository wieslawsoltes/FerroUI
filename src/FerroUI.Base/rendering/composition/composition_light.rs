//! The WinUI lights: `CompositionLight` and its subclasses `AmbientLight`,
//! `DistantLight`, `PointLight` and `SpotLight`, and the
//! `VisualUnorderedCollection` of the visuals a light targets and excludes.
//!
//! As for the clips, every light is one [`CompositionLight`] behind an `Rc`,
//! which holds what the concrete class adds in a kind; the concrete classes
//! are typed handles around the same `Rc` ([`AmbientLight`],
//! [`DistantLight`], [`PointLight`], [`SpotLight`]).
//!
//! A light lights the visuals of its [`targets`](CompositionLight::targets)
//! and their subtrees, except the visuals of its
//! [`exclusions_from_targets`](CompositionLight::exclusions_from_targets)
//! and their subtrees: the diffuse lighting of flat visuals facing the
//! viewer, see `server::LitContent` for how it is drawn.

use super::composition_object_impls::composition_object_impls;
use super::generated::{
    AmbientLightHooks, AmbientLightProps, CompositionLightHooks, CompositionLightProps, DistantLightHooks,
    DistantLightProps, PointLightHooks, PointLightProps, ServerAmbientLightProps, ServerDistantLightProps,
    ServerPointLightProps, ServerSpotLightProps, SpotLightHooks, SpotLightProps, VisualUnorderedCollectionHooks,
    VisualUnorderedCollectionProps,
};
use super::server::{
    IServerObject, ServerAmbientLight, ServerCompositor, ServerDistantLight, ServerObjectId, ServerPointLight,
    ServerSpotLight, ServerVisualUnorderedCollection,
};
use super::{CompositionObject, CompositionVisual, Compositor, ICompositionObject};
use crate::media::Color;
use crate::Vector3D;
use std::ops::Deref;
use std::rc::{Rc, Weak};

// --- the visual collection -------------------------------------------------------------

/// A set of visuals (WinUI
/// `Microsoft.UI.Composition.VisualUnorderedCollection`): the targets or the
/// exclusions of a light.
pub struct VisualUnorderedCollection {
    this: Weak<VisualUnorderedCollection>,
    object: CompositionObject,
    props: VisualUnorderedCollectionProps<CompositionVisual>,
}

impl VisualUnorderedCollection {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> Rc<VisualUnorderedCollection> {
        let server =
            compositor.create_server_object(|compositor, _| ServerVisualUnorderedCollection::new(compositor));
        let collection = Rc::new_cyclic(|this: &Weak<VisualUnorderedCollection>| VisualUnorderedCollection {
            this: this.clone(),
            object: CompositionObject::new(compositor, Some(server)),
            props: VisualUnorderedCollectionProps::new(),
        });
        collection.props.initialize_defaults(&*collection);
        collection
    }

    /// The id of the server-side collection.
    pub fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    /// A snapshot of the visuals, for enumeration (in the order they were
    /// added, which has no meaning).
    pub fn items(&self) -> Vec<Rc<CompositionVisual>> {
        self.props.items()
    }

    /// `Add`: adds a visual; a visual that is in the collection already is
    /// not added again.
    pub fn add(&self, visual: Rc<CompositionVisual>) {
        if !self.props.contains(&visual) {
            self.props.add(self, visual);
        }
    }

    /// `Remove`: removes a visual; `false` when it was not in the
    /// collection.
    pub fn remove(&self, visual: &Rc<CompositionVisual>) -> bool {
        self.props.remove(self, visual)
    }

    /// `RemoveAll`.
    pub fn remove_all(&self) {
        self.props.clear(self);
    }

    /// `Count`.
    pub fn count(&self) -> usize {
        self.props.count()
    }

    /// Whether the visual is in the collection (compared by reference).
    pub fn contains(&self, visual: &Rc<CompositionVisual>) -> bool {
        self.props.contains(visual)
    }

    pub fn is_disposed(&self) -> bool {
        self.object.is_disposed()
    }

    pub fn dispose(&self) {
        self.object.dispose();
    }
}

impl VisualUnorderedCollectionHooks<CompositionVisual> for VisualUnorderedCollection {}

composition_object_impls!(
    VisualUnorderedCollection, "VisualUnorderedCollection",
    serialize: |this, writer| this.props.serialize_changes_core(this, writer),
    start_animation: |_this, _name, _animation, _final_value| false,
    composition_property: |_this, _name| None,
);

// --- the lights ----------------------------------------------------------------------

/// What the concrete class of a light adds to [`CompositionLight`].
pub(crate) enum CompositionLightKind {
    Ambient(AmbientLightProps),
    Distant(DistantLightProps),
    Point(PointLightProps),
    Spot(SpotLightProps),
}

/// A light (WinUI `Microsoft.UI.Composition.CompositionLight`): it lights
/// the visuals of [`targets`](Self::targets) and their subtrees, minus the
/// visuals of [`exclusions_from_targets`](Self::exclusions_from_targets)
/// and their subtrees, while it [`is_enabled`](Self::is_enabled). A visual
/// that some enabled light reaches shows its content multiplied by the sum
/// of the lights that reach it, so it is dark where none of them shines.
///
/// Deviation (winui-composition.md): the lighting is a light map drawn over
/// the content of each lit visual, see `server::LitContent`.
pub struct CompositionLight {
    this: Weak<CompositionLight>,
    object: CompositionObject,
    kind: CompositionLightKind,
    targets: Rc<VisualUnorderedCollection>,
    exclusions: Rc<VisualUnorderedCollection>,
}

impl CompositionLight {
    fn create(
        compositor: &Rc<Compositor>,
        kind: CompositionLightKind,
        server: impl FnOnce(&Rc<ServerCompositor>) -> Rc<dyn IServerObject> + Send + 'static,
    ) -> Rc<CompositionLight> {
        // The collections first: the light refers to their server objects.
        let targets = VisualUnorderedCollection::new(compositor);
        let exclusions = VisualUnorderedCollection::new(compositor);
        let server = compositor.create_server_object(move |compositor, _| server(compositor));
        let light = Rc::new_cyclic(|this: &Weak<CompositionLight>| CompositionLight {
            this: this.clone(),
            object: CompositionObject::new(compositor, Some(server)),
            kind,
            targets,
            exclusions,
        });
        match &light.kind {
            CompositionLightKind::Ambient(props) => props.initialize_defaults(&*light),
            CompositionLightKind::Distant(props) => props.initialize_defaults(&*light),
            CompositionLightKind::Point(props) => props.initialize_defaults(&*light),
            CompositionLightKind::Spot(props) => props.initialize_defaults(&*light),
        }
        let targets: Rc<dyn ICompositionObject> = light.targets.clone();
        light.light_props().set_targets(&*light, Some(targets));
        let exclusions: Rc<dyn ICompositionObject> = light.exclusions.clone();
        light.light_props().set_exclusions_from_targets(&*light, Some(exclusions));
        light
    }

    /// The generated properties of `CompositionLight`.
    pub fn light_props(&self) -> &CompositionLightProps {
        match &self.kind {
            CompositionLightKind::Ambient(props) => props.base(),
            CompositionLightKind::Distant(props) => props.base(),
            CompositionLightKind::Point(props) => props.base(),
            CompositionLightKind::Spot(props) => props.base(),
        }
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    /// The embedded `CompositionObject`.
    pub fn object(&self) -> &CompositionObject {
        &self.object
    }

    /// The id of the server-side light.
    pub fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    pub fn is_disposed(&self) -> bool {
        self.object.is_disposed()
    }

    /// Disposes the light and its two collections: the visuals it lit are
    /// drawn without it from the next frame.
    pub fn dispose(&self) {
        self.object.dispose();
        self.targets.dispose();
        self.exclusions.dispose();
    }

    /// The name of the concrete class, for messages.
    pub fn type_name(&self) -> &'static str {
        match &self.kind {
            CompositionLightKind::Ambient(_) => "AmbientLight",
            CompositionLightKind::Distant(_) => "DistantLight",
            CompositionLightKind::Point(_) => "PointLight",
            CompositionLightKind::Spot(_) => "SpotLight",
        }
    }

    /// `Targets`: the visuals the light lights, with their subtrees.
    pub fn targets(&self) -> &Rc<VisualUnorderedCollection> {
        &self.targets
    }

    /// `ExclusionsFromTargets`: the visuals, with their subtrees, the light
    /// does not light although they are in the subtree of a target.
    pub fn exclusions_from_targets(&self) -> &Rc<VisualUnorderedCollection> {
        &self.exclusions
    }

    /// `IsEnabled` (default `true`): a disabled light lights nothing, and
    /// the visuals it targets are drawn as if it did not exist.
    pub fn is_enabled(&self) -> bool {
        self.light_props().is_enabled()
    }

    pub fn set_is_enabled(&self, value: bool) {
        self.light_props().set_is_enabled(self, value)
    }
}

impl CompositionLightHooks for CompositionLight {}

impl AmbientLightHooks for CompositionLight {}

impl DistantLightHooks for CompositionLight {}

impl PointLightHooks for CompositionLight {}

impl SpotLightHooks for CompositionLight {}

composition_object_impls!(
    CompositionLight, "CompositionLight",
    serialize: |this, writer| match &this.kind {
        CompositionLightKind::Ambient(props) => props.serialize_changes_core(this, writer),
        CompositionLightKind::Distant(props) => props.serialize_changes_core(this, writer),
        CompositionLightKind::Point(props) => props.serialize_changes_core(this, writer),
        CompositionLightKind::Spot(props) => props.serialize_changes_core(this, writer),
    },
    start_animation: |this, name, animation, final_value| match &this.kind {
        CompositionLightKind::Ambient(props) => props.start_animation(this, name, animation, final_value),
        CompositionLightKind::Distant(props) => props.start_animation(this, name, animation, final_value),
        CompositionLightKind::Point(props) => props.start_animation(this, name, animation, final_value),
        CompositionLightKind::Spot(props) => props.start_animation(this, name, animation, final_value),
    },
    composition_property: |this, name| match &this.kind {
        CompositionLightKind::Ambient(_) => ServerAmbientLightProps::get_composition_property(name),
        CompositionLightKind::Distant(_) => ServerDistantLightProps::get_composition_property(name),
        CompositionLightKind::Point(_) => ServerPointLightProps::get_composition_property(name),
        CompositionLightKind::Spot(_) => ServerSpotLightProps::get_composition_property(name),
    },
);

/// The visual of an object property, as a visual.
fn as_visual(object: Option<Rc<dyn ICompositionObject>>) -> Option<Rc<CompositionVisual>> {
    object.and_then(|visual| visual.into_any_rc().downcast::<CompositionVisual>().ok())
}

/// A visual for an object property.
fn as_object(visual: Option<Rc<CompositionVisual>>) -> Option<Rc<dyn ICompositionObject>> {
    visual.map(|visual| visual as Rc<dyn ICompositionObject>)
}

/// Declares the typed handle of a light class: the struct, `Deref` to the
/// light, and `from_light`.
macro_rules! light_handle {
    ($(#[$meta:meta])* $name:ident, $kind:ident, $props:ident) => {
        $(#[$meta])*
        #[derive(Clone)]
        pub struct $name(Rc<CompositionLight>);

        impl Deref for $name {
            type Target = Rc<CompositionLight>;

            fn deref(&self) -> &Rc<CompositionLight> {
                &self.0
            }
        }

        impl $name {
            /// The handle of `light` as this class, if it is one.
            pub fn from_light(light: &Rc<CompositionLight>) -> Option<$name> {
                matches!(light.kind, CompositionLightKind::$kind(_)).then(|| $name(light.clone()))
            }

            /// The generated properties of the class.
            pub fn props(&self) -> &$props {
                match &self.0.kind {
                    CompositionLightKind::$kind(props) => props,
                    _ => unreachable!("a light handle wraps a light of its class"),
                }
            }
        }
    };
}

light_handle!(
    /// A light that lights every point of its targets alike (WinUI
    /// `Microsoft.UI.Composition.AmbientLight`): `Color` times `Intensity`,
    /// both animatable.
    AmbientLight,
    Ambient,
    AmbientLightProps
);

impl AmbientLight {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> AmbientLight {
        AmbientLight(CompositionLight::create(
            compositor,
            CompositionLightKind::Ambient(AmbientLightProps::new()),
            |c| ServerAmbientLight::new(c),
        ))
    }

    /// The colour of the light (default white).
    pub fn color(&self) -> Color {
        self.props().color()
    }

    pub fn set_color(&self, value: Color) {
        self.props().set_color(&*self.0, value)
    }

    /// The intensity of the light (default 1).
    pub fn intensity(&self) -> f32 {
        self.props().intensity()
    }

    pub fn set_intensity(&self, value: f32) {
        self.props().set_intensity(&*self.0, value)
    }
}

light_handle!(
    /// A light from far away in one direction (WinUI
    /// `Microsoft.UI.Composition.DistantLight`): every point of a flat
    /// visual is lit alike, by the cosine of the angle between the direction
    /// and the visual. `Color`, `Direction` and `Intensity` are animatable.
    DistantLight,
    Distant,
    DistantLightProps
);

impl DistantLight {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> DistantLight {
        DistantLight(CompositionLight::create(
            compositor,
            CompositionLightKind::Distant(DistantLightProps::new()),
            |c| ServerDistantLight::new(c),
        ))
    }

    /// The colour of the light (default white).
    pub fn color(&self) -> Color {
        self.props().color()
    }

    pub fn set_color(&self, value: Color) {
        self.props().set_color(&*self.0, value)
    }

    /// The visual whose coordinates `Direction` is in; `None` for the
    /// targeted visual.
    pub fn coordinate_space(&self) -> Option<Rc<CompositionVisual>> {
        as_visual(self.props().coordinate_space())
    }

    pub fn set_coordinate_space(&self, value: Option<Rc<CompositionVisual>>) {
        self.props().set_coordinate_space(&*self.0, as_object(value))
    }

    /// The direction the light shines in (default `(0, 0, -1)`: straight at
    /// a visual, which faces +Z).
    pub fn direction(&self) -> Vector3D {
        self.props().direction()
    }

    pub fn set_direction(&self, value: Vector3D) {
        self.props().set_direction(&*self.0, value)
    }

    /// The intensity of the light (default 1).
    pub fn intensity(&self) -> f32 {
        self.props().intensity()
    }

    pub fn set_intensity(&self, value: f32) {
        self.props().set_intensity(&*self.0, value)
    }
}

light_handle!(
    /// A light from a point (WinUI `Microsoft.UI.Composition.PointLight`):
    /// a point of a visual is lit by the cosine of the angle of the light to
    /// the visual, weakened by the attenuation of the distance
    /// `1 / (constant + linear * d + quadratic * d^2)` between the minimum and
    /// the maximum cutoff. Every member but `CoordinateSpace` is animatable.
    PointLight,
    Point,
    PointLightProps
);

impl PointLight {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> PointLight {
        PointLight(CompositionLight::create(
            compositor,
            CompositionLightKind::Point(PointLightProps::new()),
            |c| ServerPointLight::new(c),
        ))
    }

    /// The colour of the light (default white).
    pub fn color(&self) -> Color {
        self.props().color()
    }

    pub fn set_color(&self, value: Color) {
        self.props().set_color(&*self.0, value)
    }

    /// The constant term of the attenuation (default 1).
    pub fn constant_attenuation(&self) -> f32 {
        self.props().constant_attenuation()
    }

    pub fn set_constant_attenuation(&self, value: f32) {
        self.props().set_constant_attenuation(&*self.0, value)
    }

    /// The term of the attenuation proportional to the distance (default 0).
    pub fn linear_attenuation(&self) -> f32 {
        self.props().linear_attenuation()
    }

    pub fn set_linear_attenuation(&self, value: f32) {
        self.props().set_linear_attenuation(&*self.0, value)
    }

    /// The term of the attenuation proportional to the square of the
    /// distance (default 0).
    pub fn quadratic_attenuation(&self) -> f32 {
        self.props().quadratic_attenuation()
    }

    pub fn set_quadratic_attenuation(&self, value: f32) {
        self.props().set_quadratic_attenuation(&*self.0, value)
    }

    /// The visual whose coordinates `Offset` is in; `None` for the targeted
    /// visual.
    pub fn coordinate_space(&self) -> Option<Rc<CompositionVisual>> {
        as_visual(self.props().coordinate_space())
    }

    pub fn set_coordinate_space(&self, value: Option<Rc<CompositionVisual>>) {
        self.props().set_coordinate_space(&*self.0, as_object(value))
    }

    /// The position of the light in its coordinate space; Z is the height
    /// above the visuals.
    pub fn offset(&self) -> Vector3D {
        self.props().offset()
    }

    pub fn set_offset(&self, value: Vector3D) {
        self.props().set_offset(&*self.0, value)
    }

    /// The intensity of the light (default 1).
    pub fn intensity(&self) -> f32 {
        self.props().intensity()
    }

    pub fn set_intensity(&self, value: f32) {
        self.props().set_intensity(&*self.0, value)
    }

    /// The distance up to which the light is not attenuated (default 0).
    pub fn min_attenuation_cutoff(&self) -> f32 {
        self.props().min_attenuation_cutoff()
    }

    pub fn set_min_attenuation_cutoff(&self, value: f32) {
        self.props().set_min_attenuation_cutoff(&*self.0, value)
    }

    /// The distance beyond which the light lights nothing (default
    /// `f32::MAX`).
    pub fn max_attenuation_cutoff(&self) -> f32 {
        self.props().max_attenuation_cutoff()
    }

    pub fn set_max_attenuation_cutoff(&self, value: f32) {
        self.props().set_max_attenuation_cutoff(&*self.0, value)
    }
}

light_handle!(
    /// A light from a point into a cone (WinUI
    /// `Microsoft.UI.Composition.SpotLight`): a point light whose light is
    /// the inner cone colour inside the inner cone, falls off to the outer
    /// cone colour and to nothing at the edge of the outer cone, and is
    /// nothing outside it. The cone angles are measured from the direction
    /// to the edge of the cone, in radians. Every member but
    /// `CoordinateSpace` is animatable.
    ///
    /// Deviation (winui-composition.md): `InnerConeAngleInDegrees` and
    /// `OuterConeAngleInDegrees` are views of the radians and cannot be the
    /// target of an animation.
    SpotLight,
    Spot,
    SpotLightProps
);

impl SpotLight {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> SpotLight {
        SpotLight(CompositionLight::create(
            compositor,
            CompositionLightKind::Spot(SpotLightProps::new()),
            |c| ServerSpotLight::new(c),
        ))
    }

    /// The constant term of the attenuation (default 1).
    pub fn constant_attenuation(&self) -> f32 {
        self.props().constant_attenuation()
    }

    pub fn set_constant_attenuation(&self, value: f32) {
        self.props().set_constant_attenuation(&*self.0, value)
    }

    /// The term of the attenuation proportional to the distance (default 0).
    pub fn linear_attenuation(&self) -> f32 {
        self.props().linear_attenuation()
    }

    pub fn set_linear_attenuation(&self, value: f32) {
        self.props().set_linear_attenuation(&*self.0, value)
    }

    /// The term of the attenuation proportional to the square of the
    /// distance (default 0).
    pub fn quadratic_attenuation(&self) -> f32 {
        self.props().quadratic_attenuation()
    }

    pub fn set_quadratic_attenuation(&self, value: f32) {
        self.props().set_quadratic_attenuation(&*self.0, value)
    }

    /// The visual whose coordinates `Offset` and `Direction` are in; `None`
    /// for the targeted visual.
    pub fn coordinate_space(&self) -> Option<Rc<CompositionVisual>> {
        as_visual(self.props().coordinate_space())
    }

    pub fn set_coordinate_space(&self, value: Option<Rc<CompositionVisual>>) {
        self.props().set_coordinate_space(&*self.0, as_object(value))
    }

    /// The direction the light shines in (default `(0, 0, -1)`).
    pub fn direction(&self) -> Vector3D {
        self.props().direction()
    }

    pub fn set_direction(&self, value: Vector3D) {
        self.props().set_direction(&*self.0, value)
    }

    /// The angle from the direction to the edge of the inner cone, in
    /// radians (default 0).
    pub fn inner_cone_angle(&self) -> f32 {
        self.props().inner_cone_angle()
    }

    pub fn set_inner_cone_angle(&self, value: f32) {
        self.props().set_inner_cone_angle(&*self.0, value)
    }

    /// The inner cone angle in degrees: a view of
    /// [`inner_cone_angle`](Self::inner_cone_angle).
    pub fn inner_cone_angle_in_degrees(&self) -> f32 {
        self.inner_cone_angle().to_degrees()
    }

    pub fn set_inner_cone_angle_in_degrees(&self, value: f32) {
        self.set_inner_cone_angle(value.to_radians())
    }

    /// The colour of the light inside the inner cone (default white).
    pub fn inner_cone_color(&self) -> Color {
        self.props().inner_cone_color()
    }

    pub fn set_inner_cone_color(&self, value: Color) {
        self.props().set_inner_cone_color(&*self.0, value)
    }

    /// The intensity of the light inside the inner cone (default 1).
    pub fn inner_cone_intensity(&self) -> f32 {
        self.props().inner_cone_intensity()
    }

    pub fn set_inner_cone_intensity(&self, value: f32) {
        self.props().set_inner_cone_intensity(&*self.0, value)
    }

    /// The angle from the direction to the edge of the outer cone, in
    /// radians (default pi / 4).
    pub fn outer_cone_angle(&self) -> f32 {
        self.props().outer_cone_angle()
    }

    pub fn set_outer_cone_angle(&self, value: f32) {
        self.props().set_outer_cone_angle(&*self.0, value)
    }

    /// The outer cone angle in degrees: a view of
    /// [`outer_cone_angle`](Self::outer_cone_angle).
    pub fn outer_cone_angle_in_degrees(&self) -> f32 {
        self.outer_cone_angle().to_degrees()
    }

    pub fn set_outer_cone_angle_in_degrees(&self, value: f32) {
        self.set_outer_cone_angle(value.to_radians())
    }

    /// The colour of the light between the inner and the outer cone
    /// (default white).
    pub fn outer_cone_color(&self) -> Color {
        self.props().outer_cone_color()
    }

    pub fn set_outer_cone_color(&self, value: Color) {
        self.props().set_outer_cone_color(&*self.0, value)
    }

    /// The intensity of the light between the inner and the outer cone
    /// (default 1).
    pub fn outer_cone_intensity(&self) -> f32 {
        self.props().outer_cone_intensity()
    }

    pub fn set_outer_cone_intensity(&self, value: f32) {
        self.props().set_outer_cone_intensity(&*self.0, value)
    }

    /// The position of the light in its coordinate space; Z is the height
    /// above the visuals.
    pub fn offset(&self) -> Vector3D {
        self.props().offset()
    }

    pub fn set_offset(&self, value: Vector3D) {
        self.props().set_offset(&*self.0, value)
    }

    /// The distance up to which the light is not attenuated (default 0).
    pub fn min_attenuation_cutoff(&self) -> f32 {
        self.props().min_attenuation_cutoff()
    }

    pub fn set_min_attenuation_cutoff(&self, value: f32) {
        self.props().set_min_attenuation_cutoff(&*self.0, value)
    }

    /// The distance beyond which the light lights nothing (default
    /// `f32::MAX`).
    pub fn max_attenuation_cutoff(&self) -> f32 {
        self.props().max_attenuation_cutoff()
    }

    pub fn set_max_attenuation_cutoff(&self, value: f32) {
        self.props().set_max_attenuation_cutoff(&*self.0, value)
    }
}

impl Compositor {
    /// `CreateAmbientLight`.
    pub fn create_ambient_light(&self) -> AmbientLight {
        AmbientLight::new(&self.this_handle())
    }

    /// `CreateDistantLight`.
    pub fn create_distant_light(&self) -> DistantLight {
        DistantLight::new(&self.this_handle())
    }

    /// `CreatePointLight`.
    pub fn create_point_light(&self) -> PointLight {
        PointLight::new(&self.this_handle())
    }

    /// `CreateSpotLight`.
    pub fn create_spot_light(&self) -> SpotLight {
        SpotLight::new(&self.this_handle())
    }
}
