//! The composition facades of WinUI's `UIElement` on [`Visual`] (`Rotation`, `RotationAxis`,
//! `Scale`, `CenterPoint`, `TransformMatrix`; `Translation` is next to the other composition
//! state in `visual.rs`) and their transitions (`OpacityTransition`, `RotationTransition`,
//! `ScaleTransition`, `TranslationTransition`, with `ScalarTransition` and `Vector3Transition` of
//! the `Microsoft.UI.Xaml` namespace).
//!
//! A facade is a plain property of the element, not a styled property, as in WinUI. Setting it
//! hands the matching property of the element's composition visual to the facade, which writes it
//! whenever the element is rendered again; while a facade was never set, the composition property
//! is left to the application. The transitions are implicit animations of the element's
//! composition visual (see `docs/porting/winui-composition.md`, section 3.10).

use super::Visual;
use crate::numerics::{Matrix4x4, Quaternion, Vector3};
use crate::rendering::composition::animations::{ICompositionAnimationBase, ImplicitAnimationCollection};
use crate::rendering::composition::{AsCompositionObject, CompositionVisual, Compositor};
use crate::Matrix;
use bitflags::bitflags;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// `Microsoft.UI.Xaml.ScalarTransition`: the animation of a change of a scalar facade of an
/// element (`UIElement.OpacityTransition`, `UIElement.RotationTransition`).
///
/// Deviation (winui-composition.md): WinUI's transition is a class whose `Duration` can be
/// changed after it was assigned to an element; here it is a value, copied when it is assigned,
/// so a changed duration is assigned again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScalarTransition {
    duration: Duration,
}

impl ScalarTransition {
    /// The default duration of a transition: 300 milliseconds.
    pub const DEFAULT_DURATION: Duration = Duration::from_millis(300);

    /// A transition of the default duration (300 milliseconds).
    pub const fn new() -> ScalarTransition {
        ScalarTransition { duration: Self::DEFAULT_DURATION }
    }

    /// A transition of `duration`.
    pub const fn with_duration(duration: Duration) -> ScalarTransition {
        ScalarTransition { duration }
    }

    /// `Duration`: how long the animation of a change takes. 300 milliseconds by default.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Sets [`duration`](Self::duration).
    pub fn set_duration(&mut self, value: Duration) {
        self.duration = value;
    }
}

impl Default for ScalarTransition {
    fn default() -> Self {
        Self::new()
    }
}

bitflags! {
    /// `Microsoft.UI.Xaml.Vector3TransitionComponents`: the components of a `Vector3` facade that a
    /// [`Vector3Transition`] animates.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Vector3TransitionComponents: u32 {
        /// The X component.
        const X = 1;
        /// The Y component.
        const Y = 2;
        /// The Z component.
        const Z = 4;
    }
}

impl Default for Vector3TransitionComponents {
    /// All three components, the default of `Vector3Transition.Components`.
    fn default() -> Self {
        Self::all()
    }
}

/// `Microsoft.UI.Xaml.Vector3Transition`: the animation of a change of a `Vector3` facade of an
/// element (`UIElement.ScaleTransition`, `UIElement.TranslationTransition`).
///
/// Deviation (winui-composition.md): a value, as [`ScalarTransition`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Vector3Transition {
    duration: Duration,
    components: Vector3TransitionComponents,
}

impl Vector3Transition {
    /// A transition of the default duration (300 milliseconds) animating every component.
    pub const fn new() -> Vector3Transition {
        Vector3Transition { duration: ScalarTransition::DEFAULT_DURATION, components: Vector3TransitionComponents::all() }
    }

    /// A transition of `duration` animating `components`.
    pub const fn with(duration: Duration, components: Vector3TransitionComponents) -> Vector3Transition {
        Vector3Transition { duration, components }
    }

    /// `Duration`: how long the animation of a change takes. 300 milliseconds by default.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Sets [`duration`](Self::duration).
    pub fn set_duration(&mut self, value: Duration) {
        self.duration = value;
    }

    /// `Components`: the components that animate; the others take their new value at once. All
    /// three by default.
    pub fn components(&self) -> Vector3TransitionComponents {
        self.components
    }

    /// Sets [`components`](Self::components).
    pub fn set_components(&mut self, value: Vector3TransitionComponents) {
        self.components = value;
    }
}

impl Default for Vector3Transition {
    fn default() -> Self {
        Self::new()
    }
}

/// The facades and transitions of an element, kept in the composition extras of its visual.
/// `None` is a facade that was never set.
#[derive(Default)]
pub(super) struct CompositionFacades {
    rotation: Option<f32>,
    rotation_axis: Option<Vector3>,
    scale: Option<Vector3>,
    center_point: Option<Vector3>,
    transform_matrix: Option<Matrix4x4>,
    transitions: FacadeTransitions,
    /// The implicit animations built from the transitions, on the compositor of the composition
    /// visual they were built for; dropped when a transition changes.
    transition_animations: Option<Rc<ImplicitAnimationCollection>>,
    /// A composition visual created for the element and not synchronized yet: its first
    /// synchronization writes the initial values before the implicit animations are installed.
    fresh_visual: Option<Weak<CompositionVisual>>,
}

#[derive(Clone, Copy, Default)]
struct FacadeTransitions {
    opacity: Option<ScalarTransition>,
    rotation: Option<ScalarTransition>,
    scale: Option<Vector3Transition>,
    translation: Option<Vector3Transition>,
}

impl FacadeTransitions {
    fn is_empty(&self) -> bool {
        self.opacity.is_none() && self.rotation.is_none() && self.scale.is_none() && self.translation.is_none()
    }
}

impl Visual {
    /// `UIElement.Rotation`: the rotation of the element in degrees, clockwise, around
    /// [`rotation_axis`](Self::rotation_axis) through [`center_point`](Self::center_point). Zero
    /// until set.
    ///
    /// Around the Z axis it is the `RotationAngle` of the composition visual (in radians);
    /// around another axis it is the `Orientation` of the composition visual, a quaternion.
    /// While neither the rotation nor its axis was set, both are left to the application.
    pub fn rotation(&self) -> f32 {
        self.read_composition_extras(|extras| extras.facades.rotation).flatten().unwrap_or(0.0)
    }

    /// Sets [`rotation`](Self::rotation).
    pub fn set_rotation(&self, value: f32) {
        self.with_composition_extras(|extras| extras.facades.rotation = Some(value));
        self.invalidate_visual();
    }

    /// `UIElement.RotationAxis`: the axis [`rotation`](Self::rotation) turns the element around.
    /// The Z axis `(0, 0, 1)` until set.
    ///
    /// Deviation (winui-composition.md): the renderers are 2D, so a rotation around another axis
    /// shows only its projection on the plane of the element (no perspective). A zero axis is
    /// taken as the Z axis.
    pub fn rotation_axis(&self) -> Vector3 {
        self.read_composition_extras(|extras| extras.facades.rotation_axis).flatten().unwrap_or(Vector3::UNIT_Z)
    }

    /// Sets [`rotation_axis`](Self::rotation_axis).
    pub fn set_rotation_axis(&self, value: Vector3) {
        self.with_composition_extras(|extras| extras.facades.rotation_axis = Some(value));
        self.invalidate_visual();
    }

    /// `UIElement.Scale`: the scale of the element around [`center_point`](Self::center_point),
    /// the `Scale` of its composition visual. `(1, 1, 1)` until set.
    pub fn scale(&self) -> Vector3 {
        self.read_composition_extras(|extras| extras.facades.scale).flatten().unwrap_or(Vector3::ONE)
    }

    /// Sets [`scale`](Self::scale).
    pub fn set_scale(&self, value: Vector3) {
        self.with_composition_extras(|extras| extras.facades.scale = Some(value));
        self.invalidate_visual();
    }

    /// `UIElement.CenterPoint`: the point, in the coordinates of the element, that
    /// [`scale`](Self::scale) and [`rotation`](Self::rotation) are around; the `CenterPoint` of
    /// its composition visual. The origin until set.
    pub fn center_point(&self) -> Vector3 {
        self.read_composition_extras(|extras| extras.facades.center_point).flatten().unwrap_or(Vector3::ZERO)
    }

    /// Sets [`center_point`](Self::center_point).
    pub fn set_center_point(&self, value: Vector3) {
        self.with_composition_extras(|extras| extras.facades.center_point = Some(value));
        self.invalidate_visual();
    }

    /// `UIElement.TransformMatrix`: a transform of the element in addition to the other facades.
    /// The identity until set.
    ///
    /// The composition visual has a 2D `TransformMatrix`, which the renderer fills with the
    /// render transform of the element; the 2D affine part of this matrix (`M11`, `M12`, `M21`,
    /// `M22`, `M41`, `M42`) is applied after the render transform.
    ///
    /// Deviation (winui-composition.md): WinUI applies `TransformMatrix` after the other facade
    /// transforms; the composition visual applies its `TransformMatrix` before `Scale`,
    /// `RotationAngle`, `Orientation` and `Translation`, so here the matrix acts before those
    /// facades. The 3D part of the matrix is ignored.
    pub fn transform_matrix(&self) -> Matrix4x4 {
        self.read_composition_extras(|extras| extras.facades.transform_matrix).flatten().unwrap_or(Matrix4x4::IDENTITY)
    }

    /// Sets [`transform_matrix`](Self::transform_matrix).
    pub fn set_transform_matrix(&self, value: Matrix4x4) {
        self.with_composition_extras(|extras| extras.facades.transform_matrix = Some(value));
        self.invalidate_visual();
    }

    /// `UIElement.OpacityTransition`: animates the changes of the `Opacity` of the element.
    /// `None` (no animation) by default.
    pub fn opacity_transition(&self) -> Option<ScalarTransition> {
        self.read_composition_extras(|extras| extras.facades.transitions.opacity).flatten()
    }

    /// Sets [`opacity_transition`](Self::opacity_transition).
    pub fn set_opacity_transition(&self, value: Option<ScalarTransition>) {
        self.with_composition_extras(|extras| extras.facades.transitions.opacity = value);
        self.composition_transitions_changed();
    }

    /// `UIElement.RotationTransition`: animates the changes of [`rotation`](Self::rotation) (and
    /// of [`rotation_axis`](Self::rotation_axis)). `None` by default.
    pub fn rotation_transition(&self) -> Option<ScalarTransition> {
        self.read_composition_extras(|extras| extras.facades.transitions.rotation).flatten()
    }

    /// Sets [`rotation_transition`](Self::rotation_transition).
    pub fn set_rotation_transition(&self, value: Option<ScalarTransition>) {
        self.with_composition_extras(|extras| extras.facades.transitions.rotation = value);
        self.composition_transitions_changed();
    }

    /// `UIElement.ScaleTransition`: animates the changes of [`scale`](Self::scale). `None` by
    /// default.
    pub fn scale_transition(&self) -> Option<Vector3Transition> {
        self.read_composition_extras(|extras| extras.facades.transitions.scale).flatten()
    }

    /// Sets [`scale_transition`](Self::scale_transition).
    pub fn set_scale_transition(&self, value: Option<Vector3Transition>) {
        self.with_composition_extras(|extras| extras.facades.transitions.scale = value);
        self.composition_transitions_changed();
    }

    /// `UIElement.TranslationTransition`: animates the changes of
    /// [`translation`](Self::translation). `None` by default.
    pub fn translation_transition(&self) -> Option<Vector3Transition> {
        self.read_composition_extras(|extras| extras.facades.transitions.translation).flatten()
    }

    /// Sets [`translation_transition`](Self::translation_transition).
    pub fn set_translation_transition(&self, value: Option<Vector3Transition>) {
        self.with_composition_extras(|extras| extras.facades.transitions.translation = value);
        self.composition_transitions_changed();
    }

    /// The `TransformMatrix` facade as the 2D matrix of the composition visual, while it is set.
    pub(super) fn composition_facade_transform_matrix(&self) -> Option<Matrix> {
        let matrix = self.read_composition_extras(|extras| extras.facades.transform_matrix).flatten()?;
        Some(Matrix::new(
            matrix.m11 as f64,
            matrix.m12 as f64,
            matrix.m21 as f64,
            matrix.m22 as f64,
            matrix.m41 as f64,
            matrix.m42 as f64,
        ))
    }

    /// Writes the facades that were set to the composition visual of the element (the
    /// `TransformMatrix` facade is combined with the render transform by the caller).
    pub(super) fn synchronize_composition_facades(&self, comp: &Rc<CompositionVisual>) {
        let Some((rotation, rotation_axis, scale, center_point)) = self.read_composition_extras(|extras| {
            let facades = &extras.facades;
            (facades.rotation, facades.rotation_axis, facades.scale, facades.center_point)
        }) else {
            return;
        };

        if rotation.is_some() || rotation_axis.is_some() {
            let (angle, orientation) =
                rotation_to_composition(rotation.unwrap_or(0.0), rotation_axis.unwrap_or(Vector3::UNIT_Z));
            comp.set_rotation_angle(angle);
            comp.set_orientation(orientation);
        }
        if let Some(scale) = scale {
            comp.set_scale(scale.into());
        }
        if let Some(center_point) = center_point {
            comp.set_center_point(center_point.into());
        }
    }

    /// Records that `comp` was just created for the element, so that its first synchronization
    /// writes the initial values without animating them. Nothing is recorded for an element that
    /// uses no composition facade.
    pub(super) fn composition_visual_created(&self, comp: &Rc<CompositionVisual>) {
        if self.read_composition_extras(|_| ()).is_some() {
            self.with_composition_extras(|extras| extras.facades.fresh_visual = Some(Rc::downgrade(comp)));
        }
    }

    /// Installs the implicit animations of the transitions on the composition visual of the
    /// element, before the properties are written (`after_writes` false) so that a change made
    /// with a transition animates, and again after them (`after_writes` true). On the first
    /// synchronization of a composition visual created while the element had facades, only after
    /// them, so that the initial values are written without animating.
    ///
    /// Deviation (winui-composition.md): the collection replaces implicit animations the
    /// application set on the element's composition visual while a transition is set (WinUI does
    /// not allow a facade transition and an animation of the handoff visual's property together).
    pub(super) fn apply_composition_transitions(&self, comp: &Rc<CompositionVisual>, after_writes: bool) {
        let Some(fresh) = self.read_composition_extras(|extras| {
            extras.facades.fresh_visual.as_ref().is_some_and(|visual| std::ptr::eq(visual.as_ptr(), Rc::as_ptr(comp)))
        }) else {
            return;
        };
        if fresh {
            if !after_writes {
                return;
            }
            self.with_composition_extras(|extras| extras.facades.fresh_visual = None);
        }

        if let Some(animations) = self.composition_transition_animations(comp.compositor()) {
            if !comp.implicit_animations().is_some_and(|current| Rc::ptr_eq(&current, &animations)) {
                comp.set_implicit_animations(Some(animations));
            }
        }
    }

    /// The implicit animations of the transitions on `compositor`, built on first use; `None`
    /// while no transition is set.
    fn composition_transition_animations(&self, compositor: &Rc<Compositor>) -> Option<Rc<ImplicitAnimationCollection>> {
        let (transitions, cached) = self.read_composition_extras(|extras| {
            (extras.facades.transitions, extras.facades.transition_animations.clone())
        })?;
        if transitions.is_empty() {
            return None;
        }
        if let Some(cached) = cached {
            if Rc::ptr_eq(cached.as_composition_object().compositor(), compositor) {
                return Some(cached);
            }
        }
        let built = build_transition_animations(compositor, &transitions);
        self.with_composition_extras(|extras| extras.facades.transition_animations = Some(built.clone()));
        Some(built)
    }

    /// Drops the implicit animations built from the previous transitions (and takes them off the
    /// composition visual if they are still there); the next synchronization installs the new
    /// ones before it writes the properties, so a change made with the transition animates.
    fn composition_transitions_changed(&self) {
        let old = self.with_composition_extras(|extras| extras.facades.transition_animations.take());
        if let (Some(old), Some(comp)) = (old, self.composition_visual()) {
            if comp.implicit_animations().is_some_and(|current| Rc::ptr_eq(&current, &old)) {
                comp.set_implicit_animations(None);
            }
        }
        self.invalidate_visual();
    }
}

/// The `RotationAngle` (radians) and `Orientation` of a composition visual for the `Rotation`
/// (degrees) and `RotationAxis` facades: around the Z axis (or a zero axis) the angle and the
/// identity, around another axis no angle and the quaternion of the rotation.
fn rotation_to_composition(degrees: f32, axis: Vector3) -> (f32, Quaternion) {
    let radians = degrees.to_radians();
    if axis.x == 0.0 && axis.y == 0.0 {
        let angle = if axis.z < 0.0 { -radians } else { radians };
        (angle, Quaternion::IDENTITY)
    } else {
        (0.0, Quaternion::create_from_axis_angle(Vector3::normalize(axis), radians))
    }
}

/// The duration of a transition as a key frame animation accepts it (1 ms to 24 days).
fn key_frame_duration(duration: Duration) -> Duration {
    duration.clamp(Duration::from_millis(1), Duration::from_secs(24 * 60 * 60))
}

/// The implicit animations of `transitions`: per property of the composition visual a key frame
/// animation from the value the property had to the new value (`this.FinalValue`) over the
/// duration of the transition, with the default easing of the compositor.
fn build_transition_animations(compositor: &Rc<Compositor>, transitions: &FacadeTransitions) -> Rc<ImplicitAnimationCollection> {
    let collection = compositor.create_implicit_animation_collection();

    if let Some(transition) = transitions.opacity {
        collection.set("Opacity", scalar_animation(compositor, "Opacity", transition.duration()));
    }
    if let Some(transition) = transitions.rotation {
        collection.set("RotationAngle", scalar_animation(compositor, "RotationAngle", transition.duration()));
        let orientation = compositor.create_quaternion_key_frame_animation();
        orientation.set_target(Some("Orientation".to_owned()));
        orientation.insert_expression_key_frame(1.0, "this.FinalValue", None);
        orientation.set_duration(key_frame_duration(transition.duration()));
        let orientation: Rc<dyn ICompositionAnimationBase> = orientation;
        collection.set("Orientation", orientation);
    }
    if let Some(transition) = transitions.scale {
        if let Some(animation) = vector3_animation(compositor, "Scale", transition) {
            collection.set("Scale", animation);
        }
    }
    if let Some(transition) = transitions.translation {
        if let Some(animation) = vector3_animation(compositor, "Translation", transition) {
            collection.set("Translation", animation);
        }
    }
    collection
}

fn scalar_animation(compositor: &Rc<Compositor>, target: &str, duration: Duration) -> Rc<dyn ICompositionAnimationBase> {
    let animation = compositor.create_scalar_key_frame_animation();
    animation.set_target(Some(target.to_owned()));
    animation.insert_expression_key_frame(1.0, "this.FinalValue", None);
    animation.set_duration(key_frame_duration(duration));
    animation as Rc<dyn ICompositionAnimationBase>
}

/// The animation of a `Vector3Transition`; `None` when it animates no component (the property
/// then takes its new value at once).
///
/// With only some components, a key frame at the start sets the other components to their final
/// value (`this.FinalValue`) and keeps the animated ones at their starting value
/// (`this.StartingValue`); the frame at the end is the final value. The other components therefore
/// jump at the start of the animation.
fn vector3_animation(compositor: &Rc<Compositor>, target: &str, transition: Vector3Transition) -> Option<Rc<dyn ICompositionAnimationBase>> {
    let components = transition.components();
    if components.is_empty() {
        return None;
    }
    let animation = compositor.create_vector3d_key_frame_animation();
    animation.set_target(Some(target.to_owned()));
    if components != Vector3TransitionComponents::all() {
        animation.insert_expression_key_frame(0.0, &partial_start_expression(components), None);
    }
    animation.insert_expression_key_frame(1.0, "this.FinalValue", None);
    animation.set_duration(key_frame_duration(transition.duration()));
    let animation: Rc<dyn ICompositionAnimationBase> = animation;
    Some(animation)
}

/// The start frame of a transition of some components: the animated components at their starting
/// value, the others at their final value.
fn partial_start_expression(components: Vector3TransitionComponents) -> String {
    let component = |flag: Vector3TransitionComponents, name: &str| {
        if components.contains(flag) {
            format!("this.StartingValue.{name}")
        } else {
            format!("this.FinalValue.{name}")
        }
    };
    format!(
        "Vector3({}, {}, {})",
        component(Vector3TransitionComponents::X, "X"),
        component(Vector3TransitionComponents::Y, "Y"),
        component(Vector3TransitionComponents::Z, "Z"),
    )
}

#[cfg(test)]
pub(super) fn partial_start_expression_for_tests(components: Vector3TransitionComponents) -> String {
    partial_start_expression(components)
}
