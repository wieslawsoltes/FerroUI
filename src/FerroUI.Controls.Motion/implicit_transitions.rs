//! The implicit transitions of WinUI's `UIElement` (`OpacityTransition`, `RotationTransition`,
//! `ScaleTransition`, `TranslationTransition`) and `BackgroundTransition`.
//!
//! `Visual` holds the facades and starts their composition animations. The properties markup
//! sets them with are here, as attached properties with markup aliases on the WinUI names
//! (docs/porting/xaml.md, "Property aliases"), so that an application without motion registers
//! none of them.

use ferroui_base::animation::{BrushTransition, SignedDuration, Transitions};
use ferroui_base::numerics::Vector3;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, ferro_static_type, instantiate, AttachedProperty, FerroObject,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty, Vector3D,
    Visual, WeakRef,
};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::{Border, Control};
use std::cell::RefCell;
use std::time::Duration;

/// The default of `BrushTransition.Duration` in WinUI.
const BRUSH_TRANSITION_DURATION: SignedDuration = SignedDuration::from_ticks(150 * SignedDuration::TICKS_PER_MILLISECOND);

fn std_duration(value: SignedDuration) -> Duration {
    Duration::from_nanos(u64::try_from(value.ticks()).unwrap_or(0).saturating_mul(100))
}

/// Which facade of an element a transition object animates.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Facade {
    Opacity,
    Rotation,
    Scale,
    Translation,
}

fn apply_scalar(visual: &Visual, facade: Facade, value: Option<ferroui_base::ScalarTransition>) {
    match facade {
        Facade::Opacity => visual.set_opacity_transition(value),
        _ => visual.set_rotation_transition(value),
    }
}

fn apply_vector(visual: &Visual, facade: Facade, value: Option<ferroui_base::Vector3Transition>) {
    match facade {
        Facade::Scale => visual.set_scale_transition(value),
        _ => visual.set_translation_transition(value),
    }
}

/// `Microsoft.UI.Xaml.ScalarTransition`: the animation of a change of `Opacity` or `Rotation`.
#[repr(C)]
pub struct ScalarTransition {
    base: FerroObject,
    targets: RefCell<Vec<(WeakRef<Visual>, Facade)>>,
}

ferro_class!(ScalarTransition: FerroObject);
ferro_class_info!(ScalarTransition { new: ScalarTransition::new, markup: { namespace: "FerroUI.Controls" } });

ferro_properties! {
    impl ScalarTransition {
        /// Defines the `Duration` property: 300 milliseconds by default.
        pub fn duration_property() -> StyledProperty<SignedDuration> {
            FerroProperty::register::<ScalarTransition, _>("Duration", SignedDuration::from_ticks(300 * SignedDuration::TICKS_PER_MILLISECOND))
        }
    }
}

impl FerroObjectImpl for ScalarTransition {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::duration_property().as_property() {
            let value = this.value();
            this.targets.borrow_mut().retain(|(visual, _)| visual.upgrade().is_some());
            for (visual, facade) in this.targets.borrow().iter() {
                if let Some(visual) = visual.upgrade() {
                    apply_scalar(&visual, *facade, Some(value));
                }
            }
        }
    }
}

impl ScalarTransition {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), targets: RefCell::new(Vec::new()) }
    }

    /// Creates a transition of the default duration.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// How long the animation of a change takes.
    pub fn duration(&self) -> SignedDuration {
        self.get_value(Self::duration_property())
    }

    /// Sets [`duration`](Self::duration); the elements that use the transition follow.
    pub fn set_duration(&self, value: SignedDuration) {
        self.set_value(Self::duration_property(), value);
    }

    fn value(&self) -> ferroui_base::ScalarTransition {
        ferroui_base::ScalarTransition::with_duration(std_duration(self.duration()))
    }
}

/// `Microsoft.UI.Xaml.Vector3Transition`: the animation of a change of `Scale` or `Translation`.
#[repr(C)]
pub struct Vector3Transition {
    base: FerroObject,
    targets: RefCell<Vec<(WeakRef<Visual>, Facade)>>,
}

ferro_class!(Vector3Transition: FerroObject);
ferro_class_info!(Vector3Transition { new: Vector3Transition::new, markup: { namespace: "FerroUI.Controls" } });

ferro_properties! {
    impl Vector3Transition {
        /// Defines the `Duration` property: 300 milliseconds by default.
        pub fn duration_property() -> StyledProperty<SignedDuration> {
            FerroProperty::register::<Vector3Transition, _>("Duration", SignedDuration::from_ticks(300 * SignedDuration::TICKS_PER_MILLISECOND))
        }
    }
}

impl FerroObjectImpl for Vector3Transition {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::duration_property().as_property() {
            let value = this.value();
            this.targets.borrow_mut().retain(|(visual, _)| visual.upgrade().is_some());
            for (visual, facade) in this.targets.borrow().iter() {
                if let Some(visual) = visual.upgrade() {
                    apply_vector(&visual, *facade, Some(value));
                }
            }
        }
    }
}

impl Vector3Transition {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), targets: RefCell::new(Vec::new()) }
    }

    /// Creates a transition of the default duration, of all three components.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// How long the animation of a change takes.
    pub fn duration(&self) -> SignedDuration {
        self.get_value(Self::duration_property())
    }

    /// Sets [`duration`](Self::duration); the elements that use the transition follow.
    pub fn set_duration(&self, value: SignedDuration) {
        self.set_value(Self::duration_property(), value);
    }

    fn value(&self) -> ferroui_base::Vector3Transition {
        ferroui_base::Vector3Transition::with(std_duration(self.duration()), ferroui_base::Vector3TransitionComponents::all())
    }
}

/// The properties the WinUI names of the implicit transitions are aliases of.
pub struct ImplicitTransitions;

ferro_static_type!(ImplicitTransitions);

ferro_properties! {
    impl ImplicitTransitions {
        /// Defines the attached `OpacityTransition` property.
        pub fn opacity_transition_property() -> AttachedProperty<Option<Ref<ScalarTransition>>> {
            FerroProperty::register_attached::<ImplicitTransitions, Visual, _>("OpacityTransition", None)
        }
        /// Defines the attached `RotationTransition` property.
        pub fn rotation_transition_property() -> AttachedProperty<Option<Ref<ScalarTransition>>> {
            FerroProperty::register_attached::<ImplicitTransitions, Visual, _>("RotationTransition", None)
        }
        /// Defines the attached `ScaleTransition` property.
        pub fn scale_transition_property() -> AttachedProperty<Option<Ref<Vector3Transition>>> {
            FerroProperty::register_attached::<ImplicitTransitions, Visual, _>("ScaleTransition", None)
        }
        /// Defines the attached `TranslationTransition` property.
        pub fn translation_transition_property() -> AttachedProperty<Option<Ref<Vector3Transition>>> {
            FerroProperty::register_attached::<ImplicitTransitions, Visual, _>("TranslationTransition", None)
        }
        /// Defines the attached `BackgroundTransition` property.
        pub fn background_transition_property() -> AttachedProperty<Option<Ref<BrushTransition>>> {
            FerroProperty::register_attached::<ImplicitTransitions, Control, _>("BackgroundTransition", None)
        }
        /// Defines the attached `Rotation` property: `UIElement.Rotation`, in degrees.
        pub fn rotation_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached::<ImplicitTransitions, Visual, _>("Rotation", 0.0)
        }
        /// Defines the attached `Scale` property: `UIElement.Scale`.
        pub fn scale_property() -> AttachedProperty<Vector3D> {
            FerroProperty::register_attached::<ImplicitTransitions, Visual, _>("Scale", Vector3D::new(1.0, 1.0, 1.0))
        }
    }
}

macro_rules! attached_accessors {
    ($($get:ident, $set:ident, $property:ident, $owner:ty, $value:ty;)*) => {
        impl ImplicitTransitions {
            $(
                #[doc = concat!("The value of `", stringify!($property), "` of `element`.")]
                pub fn $get(element: &$owner) -> $value {
                    element.get_value(Self::$property())
                }

                #[doc = concat!("Sets the value of `", stringify!($property), "` of `element`.")]
                pub fn $set(element: &$owner, value: $value) {
                    element.set_value(Self::$property(), value);
                }
            )*
        }
    };
}

attached_accessors! {
    get_opacity_transition, set_opacity_transition, opacity_transition_property, Visual, Option<Ref<ScalarTransition>>;
    get_rotation_transition, set_rotation_transition, rotation_transition_property, Visual, Option<Ref<ScalarTransition>>;
    get_scale_transition, set_scale_transition, scale_transition_property, Visual, Option<Ref<Vector3Transition>>;
    get_translation_transition, set_translation_transition, translation_transition_property, Visual, Option<Ref<Vector3Transition>>;
    get_background_transition, set_background_transition, background_transition_property, Control, Option<Ref<BrushTransition>>;
    get_rotation, set_rotation, rotation_property, Visual, f64;
    get_scale, set_scale, scale_property, Visual, Vector3D;
}

impl ImplicitTransitions {
    fn static_constructor() {
        for (property, facade) in
            [(Self::opacity_transition_property(), Facade::Opacity), (Self::rotation_transition_property(), Facade::Rotation)]
        {
            property
                .changed()
                .subscribe(move |change| {
                    let Some(visual) = change.sender().downcast_ref::<Visual>() else { return };
                    let (old, new) = change.get_old_and_new_value::<Option<Ref<ScalarTransition>>>();
                    if let Some(old) = old {
                        old.targets.borrow_mut().retain(|(target, kind)| {
                            *kind != facade || !target.upgrade().is_some_and(|target| std::ptr::eq(&*target, visual))
                        });
                    }
                    if let Some(new) = &new {
                        new.targets.borrow_mut().push((visual.to_ref().downgrade(), facade));
                    }
                    apply_scalar(visual, facade, new.map(|transition| transition.value()));
                })
                .detach();
        }
        for (property, facade) in
            [(Self::scale_transition_property(), Facade::Scale), (Self::translation_transition_property(), Facade::Translation)]
        {
            property
                .changed()
                .subscribe(move |change| {
                    let Some(visual) = change.sender().downcast_ref::<Visual>() else { return };
                    let (old, new) = change.get_old_and_new_value::<Option<Ref<Vector3Transition>>>();
                    if let Some(old) = old {
                        old.targets.borrow_mut().retain(|(target, kind)| {
                            *kind != facade || !target.upgrade().is_some_and(|target| std::ptr::eq(&*target, visual))
                        });
                    }
                    if let Some(new) = &new {
                        new.targets.borrow_mut().push((visual.to_ref().downgrade(), facade));
                    }
                    apply_vector(visual, facade, new.map(|transition| transition.value()));
                })
                .detach();
        }
        Self::background_transition_property().changed().subscribe(Self::background_transition_changed).detach();
        Self::rotation_property()
            .changed()
            .subscribe(|change| {
                if let Some(visual) = change.sender().downcast_ref::<Visual>() {
                    visual.set_rotation(change.get_new_value::<f64>() as f32);
                }
            })
            .detach();
        Self::scale_property()
            .changed()
            .subscribe(|change| {
                if let Some(visual) = change.sender().downcast_ref::<Visual>() {
                    let value = change.get_new_value::<Vector3D>();
                    visual.set_scale(Vector3::new(value.x as f32, value.y as f32, value.z as f32));
                }
            })
            .detach();
    }

    /// The `Background` of the element: of a border or a panel, or of a content presenter.
    fn background_of(control: &Control) -> &'static FerroProperty {
        if control.is::<ContentPresenter>() {
            ContentPresenter::background_property().as_property()
        } else {
            Border::background_property().as_property()
        }
    }

    /// The brush transition runs through the `Transitions` of the element, on its `Background`.
    fn background_transition_changed(change: &FerroPropertyChangedEventArgs<'_>) {
        let Some(control) = change.sender().downcast_ref::<Control>() else { return };
        let (old, new) = change.get_old_and_new_value::<Option<Ref<BrushTransition>>>();
        let transitions = control.transitions().unwrap_or_else(|| {
            let created = Transitions::new();
            control.set_transitions(Some(created.clone()));
            created
        });
        if let Some(old) = old {
            transitions.remove(&old.into());
        }
        if let Some(new) = new {
            new.set_property(Some(Self::background_of(control)));
            if new.duration() == SignedDuration::ZERO {
                new.set_duration(BRUSH_TRANSITION_DURATION);
            }
            transitions.add(new.into());
        }
    }
}
