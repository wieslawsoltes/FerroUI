//! WinUI's `Storyboard` (`Microsoft.UI.Xaml.Media.Animation.Storyboard`;
//! `dxaml/xcp/core/animation/Storyboard.cpp`, `TimeManager.cpp`, `Timeline.cpp` and
//! `animation.cpp` of microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT,
//! Copyright (c) Microsoft Corporation, see NOTICE.md).
//!
//! A storyboard runs on the global clock of the framework. Its animations write their values at
//! `BindingPriority::Animation`, the layer of WinUI's animated values: above local values and
//! styles, which they do not change, and removed when the storyboard stops (or, with
//! `FillBehavior.Stop`, when it ends).

use super::key_frames::ease_of;
use super::{
    property_path, ColorAnimation, ColorAnimationUsingKeyFrames, DoubleAnimation, DoubleAnimationUsingKeyFrames,
    EasingFunctionBase, ObjectAnimationUsingKeyFrames, PointAnimation, PointAnimationUsingKeyFrames, Timeline,
};
use crate::visual_state::Names;
use crate::Control;
use ferroui_base::animation::{
    interpolate, key_frame_segment, ClockState, IGlobalClock, KeyFrameEase, SignedDuration, TimelineDuration,
    TimelineTiming,
};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingPriority;
use ferroui_base::media::Color;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, AnyValue, AttachedProperty,
    BoxedValue, FerroLocator, FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroProperty, LocatorExtensions,
    Point, Ref, WeakRef,
};
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// The natural duration of an animation from, to or by a value (WinUI's one second).
const FROM_TO_BY_NATURAL: Duration = Duration::from_secs(1);

/// A container of timelines that runs them together and gives the animations their targets
/// (WinUI's `Storyboard`).
///
/// The children are animations and storyboards. Each animation animates the property its
/// `Storyboard.TargetProperty` names on the object its `Storyboard.TargetName` names (both
/// attached properties, inherited from the storyboards around it), or on the object
/// [`set_target`](Self::set_target) gave it. Names are looked up when the storyboard begins: in
/// the elements of the template, or the name scope of the document, of the element given to
/// [`set_target_scope`](Self::set_target_scope); the storyboard of a visual state looks them up
/// where the setters of the state do.
///
/// [`begin`](Self::begin) starts it on the global clock (without a global clock, its time stays
/// at zero until [`seek`](Self::seek) moves it); [`stop`](Self::stop) removes its values.
#[repr(C)]
pub struct Storyboard {
    base: Timeline,
    children: FerroList<Ref<Timeline>>,
    run: RefCell<Option<Rc<Run>>>,
    scope: RefCell<Option<WeakRef<Control>>>,
}

ferro_class!(Storyboard: Timeline);
ferro_class_info!(Storyboard {
    new: Storyboard::new,
    markup: {
        namespace: "FerroUI.Animation",
        content: Children,
        properties: [
            Children: FerroList<Ref<Timeline>> { get: Storyboard::children },
        ],
        methods: [
            static fn GetTargetName(Ref<Timeline>) -> String =>
                |timeline: Ref<Timeline>| Storyboard::get_target_name(&timeline),
            static fn SetTargetName(Ref<Timeline>, String) =>
                |timeline: Ref<Timeline>, value: String| Storyboard::set_target_name(&timeline, value),
            static fn GetTargetProperty(Ref<Timeline>) -> String =>
                |timeline: Ref<Timeline>| Storyboard::get_target_property(&timeline),
            static fn SetTargetProperty(Ref<Timeline>, String) =>
                |timeline: Ref<Timeline>, value: String| Storyboard::set_target_property(&timeline, value),
        ],
    },
});
ferro_impl_classes!(Storyboard: FerroObjectImpl);

ferro_properties! { impl Storyboard {
    /// Defines the attached `TargetName` property: the name of the object the animation (or the
    /// animations of a storyboard) animates.
    pub fn target_name_property() -> AttachedProperty<String> {
        FerroProperty::register_attached::<Storyboard, Timeline, String>("TargetName", String::new())
    }

    /// Defines the attached `TargetProperty` property: the path of the animated property
    /// (`Opacity`, `(Grid.Row)`, `(Visual.RenderTransform).(ScaleTransform.ScaleX)`).
    pub fn target_property_property() -> AttachedProperty<String> {
        FerroProperty::register_attached::<Storyboard, Timeline, String>("TargetProperty", String::new())
    }
} }

impl Storyboard {
    pub fn construct() -> Self {
        Self {
            base: Timeline::construct(),
            children: FerroList::new(),
            run: RefCell::new(None),
            scope: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The timelines of the storyboard.
    pub fn children(&self) -> FerroList<Ref<Timeline>> {
        self.children.clone()
    }

    /// The name of the target of `timeline` (`Storyboard.TargetName`).
    pub fn get_target_name(timeline: &Timeline) -> String {
        timeline.get_value(Self::target_name_property())
    }

    pub fn set_target_name(timeline: &Timeline, value: impl Into<String>) {
        timeline.set_value(Self::target_name_property(), value.into());
    }

    /// The path of the property `timeline` animates (`Storyboard.TargetProperty`).
    pub fn get_target_property(timeline: &Timeline) -> String {
        timeline.get_value(Self::target_property_property())
    }

    pub fn set_target_property(timeline: &Timeline, value: impl Into<String>) {
        timeline.set_value(Self::target_property_property(), value.into());
    }

    /// Sets the object `timeline` animates (WinUI's `Storyboard.SetTarget`); it wins over a
    /// `TargetName`. The storyboard does not keep the object alive.
    pub fn set_target(timeline: &Timeline, target: &FerroObject) {
        timeline.set_explicit_target(Some(target));
    }

    /// The element whose name scope the `TargetName`s are looked up in: the elements of its
    /// template when it is the root of a template, else the name scope of its document (a
    /// FerroUI addition: WinUI looks names up in the name scope the storyboard was loaded in).
    pub fn set_target_scope(&self, element: Option<&Control>) {
        *self.scope.borrow_mut() = element.map(|element| element.to_ref().downgrade());
    }

    /// Starts the storyboard from its beginning (again, if it runs).
    pub fn begin(&self) {
        let scope = self.scope.borrow().as_ref().and_then(WeakRef::upgrade);
        let names = scope.as_ref().map(Names::of);
        self.begin_with(names.as_ref());
    }

    /// Starts the storyboard with the names of a visual state.
    pub(crate) fn begin_with(&self, names: Option<&Names>) {
        self.stop();
        let mut leaves = Vec::new();
        let root = self.to_ref();
        for child in self.children.snapshot().iter() {
            collect(child, &[], &Inherited::default(), names, &mut leaves);
        }
        let run = Rc::new(Run {
            storyboard: root.downgrade(),
            leaves,
            elapsed: Cell::new(Duration::ZERO),
            last_tick: Cell::new(None),
            paused: Cell::new(false),
            finished: Cell::new(false),
            stopped: Cell::new(false),
            state: Cell::new(ClockState::Active),
            clock: RefCell::new(None),
        });
        *self.run.borrow_mut() = Some(run.clone());
        run.evaluate();
        if !run.finished.get() {
            run.subscribe();
        }
    }

    /// Stops the storyboard and removes the values of its animations.
    pub fn stop(&self) {
        let run = self.run.borrow_mut().take();
        if let Some(run) = run {
            run.end();
        }
    }

    /// Pauses the storyboard: its time stands still and its values stay.
    pub fn pause(&self) {
        if let Some(run) = self.current_run() {
            run.paused.set(true);
        }
    }

    /// Resumes a paused storyboard.
    pub fn resume(&self) {
        if let Some(run) = self.current_run() {
            run.paused.set(false);
        }
    }

    /// Moves the storyboard to `offset` from its beginning and applies the values of that time.
    pub fn seek(&self, offset: SignedDuration) {
        let Some(run) = self.current_run() else {
            return;
        };
        run.elapsed.set(offset.to_duration().unwrap_or(Duration::ZERO));
        run.finished.set(false);
        run.evaluate();
        if !run.finished.get() {
            run.subscribe();
        }
    }

    /// [`seek`](Self::seek), applied at once (WinUI's `SeekAlignedToLastTick`; `seek` already
    /// applies at once).
    pub fn seek_aligned_to_last_tick(&self, offset: SignedDuration) {
        self.seek(offset);
    }

    /// Moves the storyboard to the end of its active period, which completes it.
    pub fn skip_to_fill(&self) {
        let Some(run) = self.current_run() else {
            return;
        };
        let (timing, natural) = timing_of(self);
        if let Some(end) = timing.expiration(natural) {
            run.elapsed.set(end);
            run.evaluate();
        }
    }

    /// The state of the clock of the storyboard: [`ClockState::Stopped`] when it has not begun or
    /// was stopped.
    pub fn get_current_state(&self) -> ClockState {
        self.current_run().map_or(ClockState::Stopped, |run| run.state.get())
    }

    /// The time of the storyboard since it began (zero when it has not begun).
    pub fn get_current_time(&self) -> SignedDuration {
        self.current_run().map_or(SignedDuration::ZERO, |run| {
            SignedDuration::from_seconds(run.elapsed.get().as_secs_f64())
        })
    }

    /// The animations of the storyboard with their targets (the names looked up in `names`).
    pub(crate) fn resolved_animations(&self, names: Option<&Names>) -> Vec<ResolvedAnimation> {
        let mut resolved = Vec::new();
        for child in self.children.snapshot().iter() {
            resolve(child, &[], &Inherited::default(), names, &mut resolved);
        }
        resolved
    }

    /// The length of the active period of the storyboard: `None` for ever.
    pub(crate) fn active_duration(&self) -> Option<Duration> {
        let (timing, natural) = timing_of(self);
        timing.active_duration(natural)
    }

    /// Whether the storyboard has begun and was not stopped (it may be filling).
    pub(crate) fn is_running(&self) -> bool {
        self.run.borrow().is_some()
    }

    fn current_run(&self) -> Option<Rc<Run>> {
        self.run.borrow().clone()
    }
}

impl Drop for Storyboard {
    fn drop(&mut self) {
        if let Some(run) = self.run.get_mut().take() {
            run.end();
        }
    }
}

/// The timing of a timeline with the natural duration of its kind; a storyboard whose children
/// last for ever lasts for ever.
fn timing_of(timeline: &Timeline) -> (TimelineTiming, Duration) {
    let mut timing = timeline.timing();
    match natural_of(timeline) {
        Some(natural) => (timing, natural),
        None => {
            if timing.duration == TimelineDuration::Automatic {
                timing.duration = TimelineDuration::Forever;
            }
            (timing, Duration::ZERO)
        }
    }
}

/// The natural duration of a timeline (what `Duration="Automatic"` is): `None` for ever.
fn natural_of(timeline: &Timeline) -> Option<Duration> {
    if let Some(storyboard) = timeline.downcast_ref::<Storyboard>() {
        let mut end = Duration::ZERO;
        for child in storyboard.children.snapshot().iter() {
            let (timing, natural) = timing_of(child);
            if timing.begin_time.is_none() {
                continue;
            }
            end = end.max(timing.expiration(natural)?);
        }
        return Some(end);
    }
    let last_key_time = |times: Vec<SignedDuration>| {
        times
            .into_iter()
            .filter_map(SignedDuration::to_duration)
            .max()
            .unwrap_or(Duration::ZERO)
    };
    if let Some(animation) = timeline.downcast_ref::<DoubleAnimationUsingKeyFrames>() {
        return Some(last_key_time(
            animation
                .key_frames()
                .snapshot()
                .iter()
                .map(|frame| frame.key_time())
                .collect(),
        ));
    }
    if let Some(animation) = timeline.downcast_ref::<ColorAnimationUsingKeyFrames>() {
        return Some(last_key_time(
            animation
                .key_frames()
                .snapshot()
                .iter()
                .map(|frame| frame.key_time())
                .collect(),
        ));
    }
    if let Some(animation) = timeline.downcast_ref::<PointAnimationUsingKeyFrames>() {
        return Some(last_key_time(
            animation
                .key_frames()
                .snapshot()
                .iter()
                .map(|frame| frame.key_time())
                .collect(),
        ));
    }
    if let Some(animation) = timeline.downcast_ref::<ObjectAnimationUsingKeyFrames>() {
        return Some(last_key_time(
            animation
                .key_frames()
                .snapshot()
                .iter()
                .map(|frame| frame.key_time())
                .collect(),
        ));
    }
    Some(FROM_TO_BY_NATURAL)
}

/// The target a timeline inherits from the storyboards around it.
#[derive(Clone, Default)]
struct Inherited {
    target: Option<Ref<FerroObject>>,
    name: String,
    property: String,
}

impl Inherited {
    fn of(&self, timeline: &Timeline) -> Self {
        let name = Storyboard::get_target_name(timeline);
        let property = Storyboard::get_target_property(timeline);
        let explicit = timeline.explicit_target();
        Self {
            // A name of its own replaces an inherited target; a target of its own wins over both.
            target: explicit.or_else(|| if name.is_empty() { self.target.clone() } else { None }),
            name: if name.is_empty() { self.name.clone() } else { name },
            property: if property.is_empty() {
                self.property.clone()
            } else {
                property
            },
        }
    }
}

/// An animation of a storyboard with its target resolved.
pub(crate) struct ResolvedAnimation {
    pub(crate) animation: Ref<Timeline>,
    // The storyboards between the root and the animation, outermost first.
    chain: Vec<Ref<Timeline>>,
    pub(crate) object: Ref<FerroObject>,
    pub(crate) property: &'static FerroProperty,
}

/// Resolves the targets of the animations under `timeline`.
fn resolve(
    timeline: &Ref<Timeline>,
    chain: &[Ref<Timeline>],
    inherited: &Inherited,
    names: Option<&Names>,
    resolved: &mut Vec<ResolvedAnimation>,
) {
    let inherited = inherited.of(timeline);
    if let Some(storyboard) = timeline.downcast_ref::<Storyboard>() {
        let mut chain = chain.to_vec();
        chain.push(timeline.clone());
        for child in storyboard.children.snapshot().iter() {
            resolve(child, &chain, &inherited, names, resolved);
        }
        return;
    }
    let target = inherited.target.clone().or_else(|| {
        if inherited.name.is_empty() {
            return None;
        }
        let found = names.and_then(|names| names.find(&inherited.name));
        if found.is_none() {
            warn(&format!("Storyboard: no element named '{}' for an animation.", inherited.name));
        }
        found
    });
    let Some(target) = target else {
        if inherited.name.is_empty() {
            warn("Storyboard: an animation has no target (Storyboard.TargetName or Storyboard.SetTarget).");
        }
        return;
    };
    // A property given in code (the generated animations of a visual transition) wins over the
    // path.
    let (object, property) = match timeline.explicit_property() {
        Some(property) => (target, property),
        None => match property_path::resolve(&target, &inherited.property) {
            Ok(resolved) => resolved,
            Err(error) => {
                warn(&format!("Storyboard: {error}."));
                return;
            }
        },
    };
    if property.is_read_only() {
        warn(&format!("Storyboard: the property '{property}' is read-only."));
        return;
    }
    resolved.push(ResolvedAnimation { animation: timeline.clone(), chain: chain.to_vec(), object, property });
}

/// Collects the animations under `timeline`, with their targets resolved.
fn collect(
    timeline: &Ref<Timeline>,
    chain: &[Ref<Timeline>],
    inherited: &Inherited,
    names: Option<&Names>,
    leaves: &mut Vec<Leaf>,
) {
    let mut resolved = Vec::new();
    resolve(timeline, chain, inherited, names, &mut resolved);
    for ResolvedAnimation { animation, chain, object, property } in resolved {
        let Some(curve) = Curve::of(&animation, &object, property) else {
            continue;
        };
        let natural = natural_of(&animation).unwrap_or(Duration::ZERO);
        leaves.push(Leaf {
            animation,
            chain,
            natural,
            target: object.downgrade(),
            property,
            curve,
            source: Rc::new(AnimatedSource::default()),
            handle: RefCell::new(None),
            direct_old: RefCell::new(None),
            completed: Cell::new(false),
        });
    }
}

/// A value of a double, colour or point animation.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Animated {
    Double(f64),
    Color(Color),
    Point(Point),
}

impl Animated {
    fn boxed(&self) -> BoxedValue {
        match *self {
            Animated::Double(value) => Rc::new(value),
            Animated::Color(value) => Rc::new(value),
            Animated::Point(value) => Rc::new(value),
        }
    }

    /// `self + by` (WinUI adds colours channel by channel).
    fn add(&self, by: &Animated) -> Animated {
        match (*self, *by) {
            (Animated::Double(a), Animated::Double(b)) => Animated::Double(a + b),
            (Animated::Color(a), Animated::Color(b)) => Animated::Color(Color::new(
                a.a.saturating_add(b.a),
                a.r.saturating_add(b.r),
                a.g.saturating_add(b.g),
                a.b.saturating_add(b.b),
            )),
            (Animated::Point(a), Animated::Point(b)) => Animated::Point(Point::new(a.x + b.x, a.y + b.y)),
            _ => *self,
        }
    }

    /// The value at `progress` from `self` to `to` (colours channel by channel, as WinUI's
    /// `CColorAnimation` interpolates them).
    fn lerp(&self, to: &Animated, progress: f64) -> Animated {
        let channel = |a: u8, b: u8| interpolate(a as f64, b as f64, progress).round().clamp(0.0, 255.0) as u8;
        match (*self, *to) {
            (Animated::Double(a), Animated::Double(b)) => Animated::Double(interpolate(a, b, progress)),
            (Animated::Color(a), Animated::Color(b)) => Animated::Color(Color::new(
                channel(a.a, b.a),
                channel(a.r, b.r),
                channel(a.g, b.g),
                channel(a.b, b.b),
            )),
            (Animated::Point(a), Animated::Point(b)) => Animated::Point(Point::new(
                interpolate(a.x, b.x, progress),
                interpolate(a.y, b.y, progress),
            )),
            _ => *to,
        }
    }
}

/// What an animation interpolates.
enum Curve {
    FromTo {
        from: Animated,
        to: Animated,
        easing: Option<Ref<EasingFunctionBase>>,
    },
    KeyFrames {
        base: Animated,
        frames: Vec<(SignedDuration, KeyFrameEase)>,
        values: Vec<Animated>,
    },
    Objects {
        base: BoxedValue,
        frames: Vec<(SignedDuration, KeyFrameEase)>,
        values: Vec<BoxedValue>,
    },
}

/// The current and the non-animated value of a property, as animation values of `wrap`.
fn base_values(
    object: &FerroObject,
    property: &'static FerroProperty,
    wrap: impl Fn(&BoxedValue) -> Option<Animated>,
) -> Option<(Animated, Animated)> {
    let current = object.get_value_untyped(property);
    let current = wrap(&current)?;
    // The value without animations: the base value, else the default of the property for the
    // type of the object (an inherited value is not told apart from the default here).
    let base = object.get_base_value_untyped(property);
    let base = wrap(&base).or_else(|| wrap(&property.default_value_boxed(object.get_type())));
    Some((current, base.unwrap_or(current)))
}

fn sorted<T>(mut frames: Vec<(SignedDuration, KeyFrameEase, T)>) -> (Vec<(SignedDuration, KeyFrameEase)>, Vec<T>) {
    // WinUI sorts the key frames by their key times (stable for equal times).
    frames.sort_by_key(|frame| frame.0);
    frames
        .into_iter()
        .map(|(time, ease, value)| ((time, ease), value))
        .unzip()
}

impl Curve {
    fn of(timeline: &Timeline, object: &FerroObject, property: &'static FerroProperty) -> Option<Curve> {
        let type_ = property.property_type();
        let double = |value: &BoxedValue| value.downcast_ref::<f64>().map(|value| Animated::Double(*value));
        let color = |value: &BoxedValue| value.downcast_ref::<Color>().map(|value| Animated::Color(*value));
        let point = |value: &BoxedValue| value.downcast_ref::<Point>().map(|value| Animated::Point(*value));
        let mismatch = |kind: &str| {
            warn(&format!(
                "Storyboard: a {kind} animation cannot animate the property '{property}' of another type."
            ));
            None
        };
        let from_to = |from: Option<Animated>,
                       to: Option<Animated>,
                       by: Option<Animated>,
                       wrap: &dyn Fn(&BoxedValue) -> Option<Animated>,
                       easing| {
            let (current, non_animated) = base_values(object, property, wrap)?;
            // WinUI's `CAnimation::OnBegin`: `From` or the current value; `To`, or the start
            // plus `By`, or the value without animations.
            let start = from.unwrap_or(current);
            let end = match (to, by) {
                (Some(to), _) => to,
                (None, Some(by)) => start.add(&by),
                (None, None) => non_animated,
            };
            Some(Curve::FromTo {
                from: start,
                to: end,
                easing,
            })
        };
        if let Some(animation) = timeline.downcast_ref::<DoubleAnimation>() {
            if type_ != TypeId::of::<f64>() {
                return mismatch("double");
            }
            let wrap = |value: Option<f64>| value.map(Animated::Double);
            return from_to(
                wrap(animation.from()),
                wrap(animation.to()),
                wrap(animation.by()),
                &double,
                animation.easing_function(),
            );
        }
        if let Some(animation) = timeline.downcast_ref::<ColorAnimation>() {
            if type_ != TypeId::of::<Color>() {
                return mismatch("colour");
            }
            let wrap = |value: Option<Color>| value.map(Animated::Color);
            return from_to(
                wrap(animation.from()),
                wrap(animation.to()),
                wrap(animation.by()),
                &color,
                animation.easing_function(),
            );
        }
        if let Some(animation) = timeline.downcast_ref::<PointAnimation>() {
            if type_ != TypeId::of::<Point>() {
                return mismatch("point");
            }
            let wrap = |value: Option<Point>| value.map(Animated::Point);
            return from_to(
                wrap(animation.from()),
                wrap(animation.to()),
                wrap(animation.by()),
                &point,
                animation.easing_function(),
            );
        }
        macro_rules! key_frames {
            ($animation:expr, $ty:ty, $kind:literal, $wrap:expr, $variant:ident) => {{
                if type_ != TypeId::of::<$ty>() {
                    return mismatch($kind);
                }
                let (current, _) = base_values(object, property, $wrap)?;
                let frames = $animation
                    .key_frames()
                    .snapshot()
                    .iter()
                    .map(|frame| {
                        (
                            frame.key_time(),
                            ease_of(frame),
                            Animated::$variant(frame.value()),
                        )
                    })
                    .collect();
                let (frames, values) = sorted(frames);
                return Some(Curve::KeyFrames {
                    base: current,
                    frames,
                    values,
                });
            }};
        }
        if let Some(animation) = timeline.downcast_ref::<DoubleAnimationUsingKeyFrames>() {
            key_frames!(animation, f64, "double", double, Double);
        }
        if let Some(animation) = timeline.downcast_ref::<ColorAnimationUsingKeyFrames>() {
            key_frames!(animation, Color, "colour", color, Color);
        }
        if let Some(animation) = timeline.downcast_ref::<PointAnimationUsingKeyFrames>() {
            key_frames!(animation, Point, "point", point, Point);
        }
        if let Some(animation) = timeline.downcast_ref::<ObjectAnimationUsingKeyFrames>() {
            let value_type = ValueType::new(property.property_type(), property.property_type_name());
            let mut frames = Vec::new();
            for frame in animation.key_frames().snapshot().iter() {
                match convert(frame.value(), value_type, property) {
                    Some(value) => frames.push((frame.key_time(), KeyFrameEase::Discrete, value)),
                    None => warn(&format!(
                        "Storyboard: the key frame value {:?} is not a value of the property '{property}'.",
                        frame.value()
                    )),
                }
            }
            let (frames, values) = sorted(frames);
            return Some(Curve::Objects {
                base: object.get_value_untyped(property),
                frames,
                values,
            });
        }
        warn("Storyboard: a timeline that is not an animation of the storyboard family is skipped.");
        None
    }

    /// The value at the progress `progress` of the current iteration of an animation whose
    /// iteration lasts `duration`.
    fn value_at(&self, progress: f64, duration: Duration) -> BoxedValue {
        let fractions = |frames: &[(SignedDuration, KeyFrameEase)]| -> Vec<(f64, KeyFrameEase)> {
            let duration = duration.as_secs_f64();
            frames
                .iter()
                .map(|(time, ease)| {
                    let fraction = if duration > 0.0 {
                        time.total_seconds().max(0.0) / duration
                    } else {
                        0.0
                    };
                    (fraction, ease.clone())
                })
                .collect()
        };
        match self {
            Curve::FromTo { from, to, easing } => {
                let progress = easing.as_ref().map_or(progress, |easing| easing.ease(progress));
                from.lerp(to, progress).boxed()
            }
            Curve::KeyFrames { base, frames, values } => {
                let Some(segment) = key_frame_segment(&fractions(frames), progress) else {
                    return base.boxed();
                };
                let from = segment.from.map_or(*base, |index| values[index]);
                from.lerp(&values[segment.to], segment.progress).boxed()
            }
            Curve::Objects { base, frames, values } => {
                let Some(segment) = key_frame_segment(&fractions(frames), progress) else {
                    return base.clone();
                };
                // Discrete: the value of the last key frame passed, or the value before them.
                segment.from.map_or_else(|| base.clone(), |index| values[index].clone())
            }
        }
    }
}

/// A key frame value converted to the type of the property; text is parsed.
fn convert(value: Option<BoxedValue>, value_type: ValueType, property: &'static FerroProperty) -> Option<BoxedValue> {
    let converted = match &value {
        Some(value) if ValueType::of_value(&**value) == value_type => Some(value.clone()),
        Some(value) => match ValueTypes::try_convert(Some(value), value_type) {
            Some(Some(converted)) => Some(converted),
            Some(None) => ValueTypes::null_value(value_type),
            None => ValueTypes::try_parse_text(value, value_type),
        },
        None => ValueTypes::null_value(value_type),
    }?;
    let inner: &dyn AnyValue = &*converted;
    property.is_valid_value(inner.as_any()).then_some(converted)
}

/// An animation of a running storyboard, with its resolved target.
struct Leaf {
    animation: Ref<Timeline>,
    // The storyboards between the root and the animation, outermost first.
    chain: Vec<Ref<Timeline>>,
    natural: Duration,
    target: WeakRef<FerroObject>,
    property: &'static FerroProperty,
    curve: Curve,
    source: Rc<AnimatedSource>,
    handle: RefCell<Option<Rc<dyn IDisposable>>>,
    // The value a direct property had (direct properties have no animation layer).
    direct_old: RefCell<Option<BoxedValue>>,
    completed: Cell<bool>,
}

impl Leaf {
    /// Applies the animation at the time `root_local` of the root storyboard (`None`: the root
    /// has no effect). Returns whether the animation completed now.
    fn evaluate(&self, root_local: Option<Duration>) -> bool {
        let mut time = root_local;
        for timeline in &self.chain {
            let (timing, natural) = timing_of(timeline);
            time = time.and_then(|time| timing.local_time(time, natural));
        }
        let Some(time) = time else {
            self.deactivate();
            return false;
        };
        let timing = self.animation.timing();
        let sample = timing.sample(time, self.natural);
        let completed_now =
            matches!(sample.state, ClockState::Filling | ClockState::Stopped) && !self.completed.replace(true);
        match sample.state {
            ClockState::NotStarted | ClockState::Stopped => self.deactivate(),
            ClockState::Active | ClockState::Filling => {
                let duration = match timing.duration {
                    TimelineDuration::Time(duration) => duration,
                    _ => self.natural,
                };
                self.activate(self.curve.value_at(sample.progress, duration));
            }
        }
        completed_now
    }

    fn activate(&self, value: BoxedValue) {
        let Some(target) = self.target.upgrade() else {
            return;
        };
        if self.property.is_direct() {
            if self.direct_old.borrow().is_none() {
                *self.direct_old.borrow_mut() = Some(target.get_value_untyped(self.property));
            }
            let inner: &dyn AnyValue = &*value;
            target.set_value_untyped(self.property, inner.as_any(), BindingPriority::LocalValue);
            return;
        }
        self.source.set(value);
        if self.handle.borrow().is_none() {
            let source: Rc<dyn IObservable<BoxedValue>> = self.source.clone();
            let handle = target.bind_property_untyped(self.property, source, BindingPriority::Animation);
            *self.handle.borrow_mut() = Some(handle);
        }
    }

    fn deactivate(&self) {
        let handle = self.handle.borrow_mut().take();
        if let Some(handle) = handle {
            handle.dispose();
        }
        let old = self.direct_old.borrow_mut().take();
        if let (Some(old), Some(target)) = (old, self.target.upgrade()) {
            let inner: &dyn AnyValue = &*old;
            target.set_value_untyped(self.property, inner.as_any(), BindingPriority::LocalValue);
        }
    }
}

/// A running storyboard.
struct Run {
    storyboard: WeakRef<Storyboard>,
    leaves: Vec<Leaf>,
    elapsed: Cell<Duration>,
    last_tick: Cell<Option<SignedDuration>>,
    paused: Cell<bool>,
    // The active period is over (the clock is not followed any more).
    finished: Cell<bool>,
    // Stopped: nothing is applied any more.
    stopped: Cell<bool>,
    state: Cell<ClockState>,
    clock: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl Run {
    /// Applies the values of the time of the run, and raises `Completed` of what completed.
    fn evaluate(&self) {
        let Some(storyboard) = self.storyboard.upgrade() else {
            return;
        };
        if self.stopped.get() {
            return;
        }
        let (timing, natural) = timing_of(&storyboard);
        let time = self.elapsed.get();
        let state = timing.sample(time, natural).state;
        self.state.set(state);
        let local = timing.local_time(time, natural);
        let mut completed = Vec::new();
        for leaf in &self.leaves {
            if self.stopped.get() {
                return;
            }
            if leaf.evaluate(local) {
                completed.push(leaf.animation.clone());
            }
        }
        let finished = matches!(state, ClockState::Filling | ClockState::Stopped);
        let finished_now = finished && !self.finished.replace(true);
        if finished {
            self.unsubscribe();
        }
        for animation in completed {
            animation.raise_completed();
        }
        if finished_now {
            storyboard.raise_completed();
        }
    }

    fn tick(&self, time: SignedDuration) {
        if let Some(last) = self.last_tick.replace(Some(time)) {
            if !self.paused.get() {
                let delta = (time - last).to_duration().unwrap_or(Duration::ZERO);
                self.elapsed.set(self.elapsed.get() + delta);
            }
        }
        self.evaluate();
    }

    /// Follows the global clock, if there is one.
    fn subscribe(self: &Rc<Self>) {
        if self.clock.borrow().is_some() {
            return;
        }
        let Some(clock) = FerroLocator::current().get_service::<dyn IGlobalClock>() else {
            return;
        };
        self.last_tick.set(None);
        let observer: Rc<dyn IObserver<SignedDuration>> = Rc::new(ClockObserver(Rc::downgrade(self)));
        let subscription = clock.subscribe(observer);
        *self.clock.borrow_mut() = Some(subscription);
    }

    fn unsubscribe(&self) {
        let subscription = self.clock.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    /// Stops the run: the values are removed.
    fn end(&self) {
        self.stopped.set(true);
        self.unsubscribe();
        for leaf in &self.leaves {
            leaf.deactivate();
        }
    }
}

struct ClockObserver(std::rc::Weak<Run>);

impl IObserver<SignedDuration> for ClockObserver {
    fn on_next(&self, value: SignedDuration) {
        if let Some(run) = self.0.upgrade() {
            run.tick(value);
        }
    }
}

/// The values of one animation: the current value for a new subscriber, then the changes.
#[derive(Default)]
struct AnimatedSource {
    value: RefCell<Option<BoxedValue>>,
    subject: LightweightSubject<BoxedValue>,
}

impl AnimatedSource {
    fn set(&self, value: BoxedValue) {
        let changed = self.value.borrow().as_ref().is_none_or(|old| **old != *value);
        if changed {
            *self.value.borrow_mut() = Some(value.clone());
            self.subject.on_next(value);
        }
    }
}

impl IObservable<BoxedValue> for AnimatedSource {
    fn subscribe(&self, observer: Rc<dyn IObserver<BoxedValue>>) -> Rc<dyn IDisposable> {
        let current = self.value.borrow().clone();
        if let Some(current) = current {
            observer.on_next(current);
        }
        self.subject.subscribe(observer)
    }
}

/// Logs a warning of the storyboards (area `Animations`).
fn warn(message: &str) {
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::ANIMATIONS) {
        logger.log(None, message);
    }
}
