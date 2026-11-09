//! Markup metadata of the easings and page transitions of this crate: plain
//! classes, held through `Rc`. An easing is an [`Easing`] (the abstract
//! class of the managed original) and an easing contract; a page transition
//! is a page transition contract.

use crate::animation::easings::*;
use crate::animation::{
    CompositePageTransition, CrossFade, FillMode, IPageTransition, PageSlide, Rotate3DTransition, SlideAxis, SignedDuration,
};
use crate::data::core::ValueTypes;
use crate::ferro_markup_type;
use crate::metadata::{MarkupType, MarkupTyped};
use crate::{Ref, animation::KeySpline};
use std::rc::Rc;

// The classes with state are reference types: an instance equals itself.
macro_rules! identity_eq {
    ($($type_:ty),*) => {
        $(impl PartialEq for $type_ {
            fn eq(&self, other: &Self) -> bool {
                std::ptr::eq(self, other)
            }
        })*
    };
}
identity_eq!(SplineEasing, SpringEasing, CrossFade, PageSlide, CompositePageTransition, Rotate3DTransition);

// FerroUI.Animation.Easings

ferro_markup_type!(class BackEaseIn {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<BackEaseIn>, Option<Rc<BackEaseIn>>],
    this: Rc<BackEaseIn>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(BackEaseIn::new())],
});

ferro_markup_type!(class BackEaseInOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<BackEaseInOut>, Option<Rc<BackEaseInOut>>],
    this: Rc<BackEaseInOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(BackEaseInOut::new())],
});

ferro_markup_type!(class BackEaseOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<BackEaseOut>, Option<Rc<BackEaseOut>>],
    this: Rc<BackEaseOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(BackEaseOut::new())],
});

ferro_markup_type!(class BounceEaseIn {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<BounceEaseIn>, Option<Rc<BounceEaseIn>>],
    this: Rc<BounceEaseIn>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(BounceEaseIn::new())],
});

ferro_markup_type!(class BounceEaseInOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<BounceEaseInOut>, Option<Rc<BounceEaseInOut>>],
    this: Rc<BounceEaseInOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(BounceEaseInOut::new())],
});

ferro_markup_type!(class BounceEaseOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<BounceEaseOut>, Option<Rc<BounceEaseOut>>],
    this: Rc<BounceEaseOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(BounceEaseOut::new())],
});

ferro_markup_type!(class CircularEaseIn {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<CircularEaseIn>, Option<Rc<CircularEaseIn>>],
    this: Rc<CircularEaseIn>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(CircularEaseIn::new())],
});

ferro_markup_type!(class CircularEaseInOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<CircularEaseInOut>, Option<Rc<CircularEaseInOut>>],
    this: Rc<CircularEaseInOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(CircularEaseInOut::new())],
});

ferro_markup_type!(class CircularEaseOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<CircularEaseOut>, Option<Rc<CircularEaseOut>>],
    this: Rc<CircularEaseOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(CircularEaseOut::new())],
});

ferro_markup_type!(class CubicEaseIn {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<CubicEaseIn>, Option<Rc<CubicEaseIn>>],
    this: Rc<CubicEaseIn>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(CubicEaseIn::new())],
});

ferro_markup_type!(class CubicEaseInOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<CubicEaseInOut>, Option<Rc<CubicEaseInOut>>],
    this: Rc<CubicEaseInOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(CubicEaseInOut::new())],
});

ferro_markup_type!(class CubicEaseOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<CubicEaseOut>, Option<Rc<CubicEaseOut>>],
    this: Rc<CubicEaseOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(CubicEaseOut::new())],
});

ferro_markup_type!(class ElasticEaseIn {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<ElasticEaseIn>, Option<Rc<ElasticEaseIn>>],
    this: Rc<ElasticEaseIn>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(ElasticEaseIn::new())],
});

ferro_markup_type!(class ElasticEaseInOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<ElasticEaseInOut>, Option<Rc<ElasticEaseInOut>>],
    this: Rc<ElasticEaseInOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(ElasticEaseInOut::new())],
});

ferro_markup_type!(class ElasticEaseOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<ElasticEaseOut>, Option<Rc<ElasticEaseOut>>],
    this: Rc<ElasticEaseOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(ElasticEaseOut::new())],
});

ferro_markup_type!(class ExponentialEaseIn {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<ExponentialEaseIn>, Option<Rc<ExponentialEaseIn>>],
    this: Rc<ExponentialEaseIn>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(ExponentialEaseIn::new())],
});

ferro_markup_type!(class ExponentialEaseInOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<ExponentialEaseInOut>, Option<Rc<ExponentialEaseInOut>>],
    this: Rc<ExponentialEaseInOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(ExponentialEaseInOut::new())],
});

ferro_markup_type!(class ExponentialEaseOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<ExponentialEaseOut>, Option<Rc<ExponentialEaseOut>>],
    this: Rc<ExponentialEaseOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(ExponentialEaseOut::new())],
});

ferro_markup_type!(class LinearEasing {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<LinearEasing>, Option<Rc<LinearEasing>>],
    this: Rc<LinearEasing>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(LinearEasing::new())],
});

ferro_markup_type!(class QuadraticEaseIn {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<QuadraticEaseIn>, Option<Rc<QuadraticEaseIn>>],
    this: Rc<QuadraticEaseIn>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(QuadraticEaseIn::new())],
});

ferro_markup_type!(class QuadraticEaseInOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<QuadraticEaseInOut>, Option<Rc<QuadraticEaseInOut>>],
    this: Rc<QuadraticEaseInOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(QuadraticEaseInOut::new())],
});

ferro_markup_type!(class QuadraticEaseOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<QuadraticEaseOut>, Option<Rc<QuadraticEaseOut>>],
    this: Rc<QuadraticEaseOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(QuadraticEaseOut::new())],
});

ferro_markup_type!(class QuarticEaseIn {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<QuarticEaseIn>, Option<Rc<QuarticEaseIn>>],
    this: Rc<QuarticEaseIn>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(QuarticEaseIn::new())],
});

ferro_markup_type!(class QuarticEaseInOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<QuarticEaseInOut>, Option<Rc<QuarticEaseInOut>>],
    this: Rc<QuarticEaseInOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(QuarticEaseInOut::new())],
});

ferro_markup_type!(class QuarticEaseOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<QuarticEaseOut>, Option<Rc<QuarticEaseOut>>],
    this: Rc<QuarticEaseOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(QuarticEaseOut::new())],
});

ferro_markup_type!(class QuinticEaseIn {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<QuinticEaseIn>, Option<Rc<QuinticEaseIn>>],
    this: Rc<QuinticEaseIn>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(QuinticEaseIn::new())],
});

ferro_markup_type!(class QuinticEaseInOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<QuinticEaseInOut>, Option<Rc<QuinticEaseInOut>>],
    this: Rc<QuinticEaseInOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(QuinticEaseInOut::new())],
});

ferro_markup_type!(class QuinticEaseOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<QuinticEaseOut>, Option<Rc<QuinticEaseOut>>],
    this: Rc<QuinticEaseOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(QuinticEaseOut::new())],
});

ferro_markup_type!(class SineEaseIn {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<SineEaseIn>, Option<Rc<SineEaseIn>>],
    this: Rc<SineEaseIn>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(SineEaseIn::new())],
});

ferro_markup_type!(class SineEaseInOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<SineEaseInOut>, Option<Rc<SineEaseInOut>>],
    this: Rc<SineEaseInOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(SineEaseInOut::new())],
});

ferro_markup_type!(class SineEaseOut {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<SineEaseOut>, Option<Rc<SineEaseOut>>],
    this: Rc<SineEaseOut>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [() => || Rc::new(SineEaseOut::new())],
});

ferro_markup_type!(class SplineEasing {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<SplineEasing>, Option<Rc<SplineEasing>>],
    this: Rc<SplineEasing>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [
        () => || Rc::new(SplineEasing::new()),
        (f64, f64, f64, f64) => |x1: f64, y1: f64, x2: f64, y2: f64| Rc::new(SplineEasing::with_points(x1, y1, x2, y2)),
        (Ref<KeySpline>) => |key_spline: Ref<KeySpline>| Rc::new(SplineEasing::with_key_spline(key_spline)),
    ],
    properties: [
        X1: f64 { get: |e: &Rc<SplineEasing>| e.x1(), set: |e: &Rc<SplineEasing>, value: f64| e.set_x1(value) },
        Y1: f64 { get: |e: &Rc<SplineEasing>| e.y1(), set: |e: &Rc<SplineEasing>, value: f64| e.set_y1(value) },
        X2: f64 { get: |e: &Rc<SplineEasing>| e.x2(), set: |e: &Rc<SplineEasing>, value: f64| e.set_x2(value) },
        Y2: f64 { get: |e: &Rc<SplineEasing>| e.y2(), set: |e: &Rc<SplineEasing>, value: f64| e.set_y2(value) },
    ],
});

ferro_markup_type!(class SpringEasing {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<SpringEasing>, Option<Rc<SpringEasing>>],
    this: Rc<SpringEasing>,
    base: Easing,
    interfaces: [Rc<dyn IEasing>],
    constructors: [
        () => || Rc::new(SpringEasing::new()),
        (f64, f64, f64, f64) => |mass: f64, stiffness: f64, damping: f64, initial_velocity: f64| {
            Rc::new(SpringEasing::with_values(mass, stiffness, damping, initial_velocity))
        },
    ],
    properties: [
        Mass: f64 { get: |e: &Rc<SpringEasing>| e.mass(), set: |e: &Rc<SpringEasing>, value: f64| e.set_mass(value) },
        Stiffness: f64 {
            get: |e: &Rc<SpringEasing>| e.stiffness(),
            set: |e: &Rc<SpringEasing>, value: f64| e.set_stiffness(value)
        },
        Damping: f64 {
            get: |e: &Rc<SpringEasing>| e.damping(),
            set: |e: &Rc<SpringEasing>, value: f64| e.set_damping(value)
        },
        InitialVelocity: f64 {
            get: |e: &Rc<SpringEasing>| e.initial_velocity(),
            set: |e: &Rc<SpringEasing>, value: f64| e.set_initial_velocity(value)
        },
    ],
});

// FerroUI.Animation

ferro_markup_type!(class CrossFade {
    namespace: "FerroUI.Animation",
    handles: [Rc<CrossFade>, Option<Rc<CrossFade>>],
    this: Rc<CrossFade>,
    interfaces: [Rc<dyn IPageTransition>],
    constructors: [
        () => || Rc::new(CrossFade::new()),
        (SignedDuration) => |duration: SignedDuration| Rc::new(CrossFade::with_duration(duration)),
    ],
    properties: [
        Duration: SignedDuration {
            get: |t: &Rc<CrossFade>| t.duration(),
            set: |t: &Rc<CrossFade>, value: SignedDuration| t.set_duration(value)
        },
        FadeInEasing: Easing {
            get: |t: &Rc<CrossFade>| t.fade_in_easing(),
            set: |t: &Rc<CrossFade>, value: Easing| t.set_fade_in_easing(value)
        },
        FadeOutEasing: Easing {
            get: |t: &Rc<CrossFade>| t.fade_out_easing(),
            set: |t: &Rc<CrossFade>, value: Easing| t.set_fade_out_easing(value)
        },
        FillMode: FillMode {
            get: |t: &Rc<CrossFade>| t.fill_mode(),
            set: |t: &Rc<CrossFade>, value: FillMode| t.set_fill_mode(value)
        },
    ],
});

ferro_markup_type!(class PageSlide {
    namespace: "FerroUI.Animation",
    handles: [Rc<PageSlide>, Option<Rc<PageSlide>>],
    this: Rc<PageSlide>,
    interfaces: [Rc<dyn IPageTransition>],
    constructors: [
        () => || Rc::new(PageSlide::new()),
        (SignedDuration, SlideAxis) => |duration: SignedDuration, orientation: SlideAxis| {
            Rc::new(PageSlide::with_duration(duration, orientation))
        },
    ],
    properties: [
        Duration: SignedDuration {
            get: |t: &Rc<PageSlide>| t.duration(),
            set: |t: &Rc<PageSlide>, value: SignedDuration| t.set_duration(value)
        },
        Orientation: SlideAxis {
            get: |t: &Rc<PageSlide>| t.orientation(),
            set: |t: &Rc<PageSlide>, value: SlideAxis| t.set_orientation(value)
        },
        SlideInEasing: Easing {
            get: |t: &Rc<PageSlide>| t.slide_in_easing(),
            set: |t: &Rc<PageSlide>, value: Easing| t.set_slide_in_easing(value)
        },
        SlideOutEasing: Easing {
            get: |t: &Rc<PageSlide>| t.slide_out_easing(),
            set: |t: &Rc<PageSlide>, value: Easing| t.set_slide_out_easing(value)
        },
        FillMode: FillMode {
            get: |t: &Rc<PageSlide>| t.fill_mode(),
            set: |t: &Rc<PageSlide>, value: FillMode| t.set_fill_mode(value)
        },
    ],
});

// The managed class derives from `PageSlide`; here it holds one, so the members of the
// slide are declared on it.
ferro_markup_type!(class Rotate3DTransition {
    namespace: "FerroUI.Animation",
    handles: [Rc<Rotate3DTransition>, Option<Rc<Rotate3DTransition>>],
    this: Rc<Rotate3DTransition>,
    interfaces: [Rc<dyn IPageTransition>],
    constructors: [
        () => || Rc::new(Rotate3DTransition::new()),
        (SignedDuration, SlideAxis, Option<f64>) => |duration: SignedDuration, orientation: SlideAxis, depth: Option<f64>| {
            Rc::new(Rotate3DTransition::with_duration(duration, orientation, depth))
        },
    ],
    properties: [
        Depth: Option<f64> {
            get: |t: &Rc<Rotate3DTransition>| t.depth(),
            set: |t: &Rc<Rotate3DTransition>, value: Option<f64>| t.set_depth(value)
        },
        Duration: SignedDuration {
            get: |t: &Rc<Rotate3DTransition>| t.duration(),
            set: |t: &Rc<Rotate3DTransition>, value: SignedDuration| t.set_duration(value)
        },
        Orientation: SlideAxis {
            get: |t: &Rc<Rotate3DTransition>| t.orientation(),
            set: |t: &Rc<Rotate3DTransition>, value: SlideAxis| t.set_orientation(value)
        },
        SlideInEasing: Easing {
            get: |t: &Rc<Rotate3DTransition>| t.slide_in_easing(),
            set: |t: &Rc<Rotate3DTransition>, value: Easing| t.set_slide_in_easing(value)
        },
        SlideOutEasing: Easing {
            get: |t: &Rc<Rotate3DTransition>| t.slide_out_easing(),
            set: |t: &Rc<Rotate3DTransition>, value: Easing| t.set_slide_out_easing(value)
        },
        FillMode: FillMode {
            get: |t: &Rc<Rotate3DTransition>| t.fill_mode(),
            set: |t: &Rc<Rotate3DTransition>, value: FillMode| t.set_fill_mode(value)
        },
    ],
});

// `PageTransitions` (the content property of the managed class, a plain list) is not
// declared: the port keeps the transitions in a plain vector, which is not a collection
// handle markup can add to. The transitions are added with `Add`.
ferro_markup_type!(class CompositePageTransition {
    namespace: "FerroUI.Animation",
    handles: [Rc<CompositePageTransition>, Option<Rc<CompositePageTransition>>],
    this: Rc<CompositePageTransition>,
    interfaces: [Rc<dyn IPageTransition>],
    constructors: [() => || Rc::new(CompositePageTransition::new())],
    methods: [
        fn Add(Rc<dyn IPageTransition>) =>
            |t: &Rc<CompositePageTransition>, transition: Rc<dyn IPageTransition>| t.add(transition),
    ],
});

/// The types declared in this file.
pub(super) const TYPES: &[&MarkupType] = &[
    <BackEaseIn as MarkupTyped>::MARKUP,
    <BackEaseInOut as MarkupTyped>::MARKUP,
    <BackEaseOut as MarkupTyped>::MARKUP,
    <BounceEaseIn as MarkupTyped>::MARKUP,
    <BounceEaseInOut as MarkupTyped>::MARKUP,
    <BounceEaseOut as MarkupTyped>::MARKUP,
    <CircularEaseIn as MarkupTyped>::MARKUP,
    <CircularEaseInOut as MarkupTyped>::MARKUP,
    <CircularEaseOut as MarkupTyped>::MARKUP,
    <CubicEaseIn as MarkupTyped>::MARKUP,
    <CubicEaseInOut as MarkupTyped>::MARKUP,
    <CubicEaseOut as MarkupTyped>::MARKUP,
    <ElasticEaseIn as MarkupTyped>::MARKUP,
    <ElasticEaseInOut as MarkupTyped>::MARKUP,
    <ElasticEaseOut as MarkupTyped>::MARKUP,
    <ExponentialEaseIn as MarkupTyped>::MARKUP,
    <ExponentialEaseInOut as MarkupTyped>::MARKUP,
    <ExponentialEaseOut as MarkupTyped>::MARKUP,
    <LinearEasing as MarkupTyped>::MARKUP,
    <QuadraticEaseIn as MarkupTyped>::MARKUP,
    <QuadraticEaseInOut as MarkupTyped>::MARKUP,
    <QuadraticEaseOut as MarkupTyped>::MARKUP,
    <QuarticEaseIn as MarkupTyped>::MARKUP,
    <QuarticEaseInOut as MarkupTyped>::MARKUP,
    <QuarticEaseOut as MarkupTyped>::MARKUP,
    <QuinticEaseIn as MarkupTyped>::MARKUP,
    <QuinticEaseInOut as MarkupTyped>::MARKUP,
    <QuinticEaseOut as MarkupTyped>::MARKUP,
    <SineEaseIn as MarkupTyped>::MARKUP,
    <SineEaseInOut as MarkupTyped>::MARKUP,
    <SineEaseOut as MarkupTyped>::MARKUP,
    <SplineEasing as MarkupTyped>::MARKUP,
    <SpringEasing as MarkupTyped>::MARKUP,
    <CrossFade as MarkupTyped>::MARKUP,
    <PageSlide as MarkupTyped>::MARKUP,
    <Rotate3DTransition as MarkupTyped>::MARKUP,
    <CompositePageTransition as MarkupTyped>::MARKUP,
];

/// Registers what the untyped value conversions need of the types of this file that
/// no first use shows: the nullable forms and the casts of the types that members of
/// markup types hold (compiled markup converts a value for such a member before it
/// sets it, and setting it registers nothing). Every other type registers when it is
/// first used ([`ValueTypeRegistration`](crate::data::core::ValueTypeRegistration)).
pub(super) fn register_value_types() {
    ValueTypes::register_cast::<Rc<dyn IEasing>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<BackEaseIn>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<BackEaseInOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<BackEaseOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<BounceEaseIn>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<BounceEaseInOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<BounceEaseOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<CircularEaseIn>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<CircularEaseInOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<CircularEaseOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<CubicEaseIn>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<CubicEaseInOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<CubicEaseOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<ElasticEaseIn>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<ElasticEaseInOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<ElasticEaseOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<ExponentialEaseIn>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<ExponentialEaseInOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<ExponentialEaseOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<LinearEasing>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<QuadraticEaseIn>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<QuadraticEaseInOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<QuadraticEaseOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<QuarticEaseIn>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<QuarticEaseInOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<QuarticEaseOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<QuinticEaseIn>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<QuinticEaseInOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<QuinticEaseOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<SineEaseIn>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<SineEaseInOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<SineEaseOut>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<SplineEasing>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<SpringEasing>, Easing>(|easing| Easing::from_rc(easing.clone()));
    ValueTypes::register_cast::<Rc<CrossFade>, Rc<dyn IPageTransition>>(|transition| transition.clone());
    ValueTypes::register_cast::<Rc<PageSlide>, Rc<dyn IPageTransition>>(|transition| transition.clone());
    ValueTypes::register_cast::<Rc<Rotate3DTransition>, Rc<dyn IPageTransition>>(|transition| transition.clone());
    ValueTypes::register_cast::<Rc<CompositePageTransition>, Rc<dyn IPageTransition>>(|transition| transition.clone());
}

/// Registers every type of this file that registers when it is first used: what the
/// tables of types (`type-tables`) register at start-up.
pub(super) fn register_value_type_table() {
    ValueTypes::register_used::<dyn IEasing>();
    ValueTypes::register_used::<BackEaseIn>();
    ValueTypes::register_used::<BackEaseInOut>();
    ValueTypes::register_used::<BackEaseOut>();
    ValueTypes::register_used::<BounceEaseIn>();
    ValueTypes::register_used::<BounceEaseInOut>();
    ValueTypes::register_used::<BounceEaseOut>();
    ValueTypes::register_used::<CircularEaseIn>();
    ValueTypes::register_used::<CircularEaseInOut>();
    ValueTypes::register_used::<CircularEaseOut>();
    ValueTypes::register_used::<CubicEaseIn>();
    ValueTypes::register_used::<CubicEaseInOut>();
    ValueTypes::register_used::<CubicEaseOut>();
    ValueTypes::register_used::<ElasticEaseIn>();
    ValueTypes::register_used::<ElasticEaseInOut>();
    ValueTypes::register_used::<ElasticEaseOut>();
    ValueTypes::register_used::<ExponentialEaseIn>();
    ValueTypes::register_used::<ExponentialEaseInOut>();
    ValueTypes::register_used::<ExponentialEaseOut>();
    ValueTypes::register_used::<LinearEasing>();
    ValueTypes::register_used::<QuadraticEaseIn>();
    ValueTypes::register_used::<QuadraticEaseInOut>();
    ValueTypes::register_used::<QuadraticEaseOut>();
    ValueTypes::register_used::<QuarticEaseIn>();
    ValueTypes::register_used::<QuarticEaseInOut>();
    ValueTypes::register_used::<QuarticEaseOut>();
    ValueTypes::register_used::<QuinticEaseIn>();
    ValueTypes::register_used::<QuinticEaseInOut>();
    ValueTypes::register_used::<QuinticEaseOut>();
    ValueTypes::register_used::<SineEaseIn>();
    ValueTypes::register_used::<SineEaseInOut>();
    ValueTypes::register_used::<SineEaseOut>();
    ValueTypes::register_used::<SplineEasing>();
    ValueTypes::register_used::<SpringEasing>();
    ValueTypes::register_used::<CrossFade>();
    ValueTypes::register_used::<PageSlide>();
    ValueTypes::register_used::<Rotate3DTransition>();
    ValueTypes::register_used::<CompositePageTransition>();
}

impl crate::data::core::ValueTypeRegistration for dyn IEasing {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<dyn IEasing>>();
        // The easing class is the handle of an easing contract.
        ValueTypes::register_cast::<Easing, Rc<dyn IEasing>>(|easing| easing.as_rc().clone());
        ValueTypes::register_cast::<Rc<BackEaseIn>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<BackEaseInOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<BackEaseOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<BounceEaseIn>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<BounceEaseInOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<BounceEaseOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<CircularEaseIn>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<CircularEaseInOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<CircularEaseOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<CubicEaseIn>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<CubicEaseInOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<CubicEaseOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<ElasticEaseIn>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<ElasticEaseInOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<ElasticEaseOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<ExponentialEaseIn>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<ExponentialEaseInOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<ExponentialEaseOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<LinearEasing>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<QuadraticEaseIn>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<QuadraticEaseInOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<QuadraticEaseOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<QuarticEaseIn>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<QuarticEaseInOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<QuarticEaseOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<QuinticEaseIn>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<QuinticEaseInOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<QuinticEaseOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<SineEaseIn>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<SineEaseInOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<SineEaseOut>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<SplineEasing>, Rc<dyn IEasing>>(|easing| easing.clone());
        ValueTypes::register_cast::<Rc<SpringEasing>, Rc<dyn IEasing>>(|easing| easing.clone());
    }
}

impl crate::data::core::ValueTypeRegistration for BackEaseIn {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<BackEaseIn>>();
    }
}

impl crate::data::core::ValueTypeRegistration for BackEaseInOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<BackEaseInOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for BackEaseOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<BackEaseOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for BounceEaseIn {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<BounceEaseIn>>();
    }
}

impl crate::data::core::ValueTypeRegistration for BounceEaseInOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<BounceEaseInOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for BounceEaseOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<BounceEaseOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for CircularEaseIn {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<CircularEaseIn>>();
    }
}

impl crate::data::core::ValueTypeRegistration for CircularEaseInOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<CircularEaseInOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for CircularEaseOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<CircularEaseOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for CubicEaseIn {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<CubicEaseIn>>();
    }
}

impl crate::data::core::ValueTypeRegistration for CubicEaseInOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<CubicEaseInOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for CubicEaseOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<CubicEaseOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for ElasticEaseIn {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<ElasticEaseIn>>();
    }
}

impl crate::data::core::ValueTypeRegistration for ElasticEaseInOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<ElasticEaseInOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for ElasticEaseOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<ElasticEaseOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for ExponentialEaseIn {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<ExponentialEaseIn>>();
    }
}

impl crate::data::core::ValueTypeRegistration for ExponentialEaseInOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<ExponentialEaseInOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for ExponentialEaseOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<ExponentialEaseOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for LinearEasing {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<LinearEasing>>();
    }
}

impl crate::data::core::ValueTypeRegistration for QuadraticEaseIn {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<QuadraticEaseIn>>();
    }
}

impl crate::data::core::ValueTypeRegistration for QuadraticEaseInOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<QuadraticEaseInOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for QuadraticEaseOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<QuadraticEaseOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for QuarticEaseIn {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<QuarticEaseIn>>();
    }
}

impl crate::data::core::ValueTypeRegistration for QuarticEaseInOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<QuarticEaseInOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for QuarticEaseOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<QuarticEaseOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for QuinticEaseIn {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<QuinticEaseIn>>();
    }
}

impl crate::data::core::ValueTypeRegistration for QuinticEaseInOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<QuinticEaseInOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for QuinticEaseOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<QuinticEaseOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for SineEaseIn {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<SineEaseIn>>();
    }
}

impl crate::data::core::ValueTypeRegistration for SineEaseInOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<SineEaseInOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for SineEaseOut {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<SineEaseOut>>();
    }
}

impl crate::data::core::ValueTypeRegistration for SplineEasing {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<SplineEasing>>();
    }
}

impl crate::data::core::ValueTypeRegistration for SpringEasing {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<SpringEasing>>();
    }
}

impl crate::data::core::ValueTypeRegistration for CrossFade {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<CrossFade>>();
    }
}

impl crate::data::core::ValueTypeRegistration for PageSlide {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<PageSlide>>();
    }
}

impl crate::data::core::ValueTypeRegistration for Rotate3DTransition {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<Rotate3DTransition>>();
    }
}

impl crate::data::core::ValueTypeRegistration for CompositePageTransition {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<CompositePageTransition>>();
    }
}

