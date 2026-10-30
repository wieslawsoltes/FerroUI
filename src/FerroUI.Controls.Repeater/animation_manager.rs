// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

use super::item_collection_transition::ItemCollectionTransition;
use super::item_collection_transition_progress::ItemCollectionTransitionProgress;
use ferroui_controls::Control;
use ferroui_base::animation::easings::{Easing, SplineEasing};
use ferroui_base::animation::{
    Animatable, Animation, Cue, FillMode, IClock, IGlobalClock, KeyFrame, TimeSpan,
};
use ferroui_base::data::BindingPriority;
use ferroui_base::media::MatrixTransform;
use ferroui_base::reactive::{IDisposable, IObserver, LightweightSubject};
use ferroui_base::styling::Setter;
use ferroui_base::threading::CancellationTokenSource;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroLocator, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, LocatorExtensions, Matrix,
    Point, Ref, RelativePoint, StyledProperty, Visual,
};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) const QUICK_DURATION_MS: f64 = 167.0;
pub(crate) const DEFAULT_DURATION_MS: f64 = 250.0;
const HALF_QUICK_DURATION_MS: f64 = 83.0; // Integral milliseconds in the source.

ferroui_base::ferro_thread_local! {
    static EASE_IN: Easing = Easing::new(SplineEasing::with_points(0.55, 0.0, 0.0, 1.0));
    static EASE_OUT: Easing = Easing::new(SplineEasing::with_points(1.0, 0.0, 1.0, 1.0));
    static DEFAULT_EASE: Easing = Easing::new(SplineEasing::with_points(0.25, 0.1, 0.25, 1.0));
}

/// All time intervals, delays, intermediate positions and easing functions of
/// the lined-flow source animations, expressed on FerroUI's animation clock.
#[derive(Clone, Copy, Debug)]
pub(crate) enum TransitionTimeline {
    SlideIn,
    ScaleIn { delay: f64 },
    ScaleOut,
    Translate { delay: f64, delta: Point },
    ChangeLine { delay: f64, delta: Point },
}
impl TransitionTimeline {
    fn duration(self) -> f64 {
        match self {
            Self::SlideIn => DEFAULT_DURATION_MS,
            Self::ScaleIn { delay } => delay + QUICK_DURATION_MS,
            Self::ScaleOut => QUICK_DURATION_MS,
            Self::Translate { delay, .. } => delay + HALF_QUICK_DURATION_MS + DEFAULT_DURATION_MS,
            Self::ChangeLine { delay, .. } => {
                delay + HALF_QUICK_DURATION_MS + QUICK_DURATION_MS * 2.0
            }
        }
    }
    fn sample(self, elapsed: f64) -> (f64, Point, f64) {
        let unit = |start: f64, duration: f64| ((elapsed - start) / duration).clamp(0.0, 1.0);
        match self {
            Self::SlideIn => {
                let progress = unit(0.0, DEFAULT_DURATION_MS);
                let offset = 100.0 * (1.0 - EASE_IN.with(|easing| easing.ease(progress)));
                (
                    1.0,
                    Point::new(0.0, offset),
                    DEFAULT_EASE.with(|easing| easing.ease(progress)),
                )
            }
            Self::ScaleIn { delay } => (
                EASE_IN.with(|easing| easing.ease(unit(delay, QUICK_DURATION_MS))),
                Point::default(),
                1.0,
            ),
            Self::ScaleOut => (
                1.0 - EASE_OUT.with(|easing| easing.ease(unit(0.0, QUICK_DURATION_MS))),
                Point::default(),
                1.0,
            ),
            Self::Translate { delay, delta } => {
                let remaining = 1.0
                    - EASE_IN.with(|easing| {
                        easing.ease(unit(delay + HALF_QUICK_DURATION_MS, DEFAULT_DURATION_MS))
                    });
                (
                    1.0,
                    Point::new(delta.x * remaining, delta.y * remaining),
                    1.0,
                )
            }
            Self::ChangeLine { delay, delta } => {
                let start = delay + HALF_QUICK_DURATION_MS;
                if elapsed <= start + QUICK_DURATION_MS {
                    (
                        1.0 - EASE_OUT.with(|easing| easing.ease(unit(start, QUICK_DURATION_MS))),
                        delta,
                        1.0,
                    )
                } else {
                    (
                        EASE_IN.with(|easing| {
                            easing.ease(unit(start + QUICK_DURATION_MS, QUICK_DURATION_MS))
                        }),
                        Point::default(),
                        1.0,
                    )
                }
            }
        }
    }
}

/// Keeps one active transition per element. Property values are installed at
/// Animation priority, so finishing or replacing an animation reveals the
/// application's original bindings and styles without rewriting them.
pub(crate) struct AnimationManager {
    next_id: Cell<u64>,
    current: RefCell<HashMap<Ref<Control>, (u64, Rc<RunningTransition>)>>,
}
impl AnimationManager {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            next_id: Cell::new(0),
            current: RefCell::new(HashMap::new()),
        })
    }
    pub(crate) fn active_count(&self) -> usize {
        self.current.borrow().len()
    }
    pub(crate) fn cancel_for_element(&self, element: &Ref<Control>) {
        let state = self.current.borrow_mut().remove(element);
        if let Some((_, state)) = state {
            if let Some(transition) = state.progress.transition() {
                transition.cancel();
            }
            state.finish();
        }
    }
    pub(crate) fn start(
        self: &Rc<Self>,
        transition: Rc<ItemCollectionTransition>,
        timeline: TransitionTimeline,
    ) {
        if transition.is_cancelled() {
            return;
        }
        let element = transition.element();
        let clock = element.clock().or_else(|| {
            FerroLocator::current()
                .get_service::<dyn IGlobalClock>()
                .map(|clock| clock as Rc<dyn IClock>)
        });
        // No rendering clock exists in a tree used solely for offscreen layout.
        // Leaving it unstarted lets the provider report ordinary completion.
        let Some(clock) = clock else {
            return;
        };
        let old = self.current.borrow_mut().remove(&element);
        if let Some((_, old)) = old {
            old.finish();
        }
        if transition.is_cancelled() {
            return;
        }
        let id = self.next_id.get().wrapping_add(1);
        self.next_id.set(id);
        let original_origin = element
            .render_transform_origin()
            .to_pixels(element.bounds().size());
        let original_matrix = element
            .render_transform()
            .map_or(Matrix::IDENTITY, |transform| transform.value());
        let original_matrix = Matrix::create_translation(-original_origin.x, -original_origin.y)
            * original_matrix
            * Matrix::create_translation(original_origin.x, original_origin.y);
        let proxy = RepeaterAnimationProxy::new();
        let state = Rc::new(RunningTransition {
            _proxy: proxy.clone(),
            element: element.clone(),
            timeline,
            center: Point::new(element.bounds().width / 2.0, element.bounds().height / 2.0),
            original_matrix,
            original_opacity: element.opacity(),
            transform: MatrixTransform::new(),
            opacity: Rc::new(LightweightSubject::new()),
            values: RefCell::new(Vec::new()),
            cancellation: CancellationTokenSource::new(),
            progress: transition.start(),
            finished: Cell::new(false),
        });
        self.current
            .borrow_mut()
            .insert(element.clone(), (id, state.clone()));
        let mut values = Vec::with_capacity(3);
        if let Some(value) = element.set_value_with_priority(
            Visual::render_transform_property(),
            Some(state.transform.clone().into()),
            BindingPriority::Animation,
        ) {
            values.push(value);
        }
        if let Some(value) = element.set_value_with_priority(
            Visual::render_transform_origin_property(),
            RelativePoint::TOP_LEFT,
            BindingPriority::Animation,
        ) {
            values.push(value);
        }
        values.push(element.bind(
            Visual::opacity_property(),
            state.opacity.clone(),
            BindingPriority::Animation,
        ));
        *state.values.borrow_mut() = values;
        state.render(0.0);
        let weak_state = Rc::downgrade(&state);
        *proxy.changed.borrow_mut() = Some(Rc::new(move |progress| {
            if let Some(state) = weak_state.upgrade() {
                state.render(progress);
            }
        }));
        let animation = Animation::new();
        animation.set_duration(TimeSpan::from_milliseconds(timeline.duration()));
        animation.set_fill_mode(FillMode::Both);
        animation.children().add(KeyFrame::with_cue(
            Cue::new(0.0),
            [Setter::new(RepeaterAnimationProxy::progress_property(), 0.0) as _],
        ));
        animation.children().add(KeyFrame::with_cue(
            Cue::new(1.0),
            [Setter::new(RepeaterAnimationProxy::progress_property(), 1.0) as _],
        ));
        let task = animation.run_async_with_clock(&proxy, Some(clock), state.cancellation.token());
        let weak = Rc::downgrade(self);
        task.on_completed(move || {
            if let Some(this) = weak.upgrade() {
                let state = {
                    let mut current = this.current.borrow_mut();
                    if current
                        .get(&element)
                        .is_some_and(|(current_id, _)| *current_id == id)
                    {
                        current.remove(&element).map(|(_, state)| state)
                    } else {
                        None
                    }
                };
                if let Some(state) = state {
                    state.finish();
                }
            }
        });
    }
}
impl Drop for AnimationManager {
    fn drop(&mut self) {
        for (_, state) in self.current.get_mut().values() {
            state.finish();
        }
    }
}

struct RunningTransition {
    // Native animations hold their target weakly to avoid binding cycles.
    // The running operation therefore retains its private animation target.
    _proxy: Ref<RepeaterAnimationProxy>,
    element: Ref<Control>,
    timeline: TransitionTimeline,
    center: Point,
    original_matrix: Matrix,
    original_opacity: f64,
    transform: Ref<MatrixTransform>,
    opacity: Rc<LightweightSubject<f64>>,
    values: RefCell<Vec<Rc<dyn IDisposable>>>,
    cancellation: CancellationTokenSource,
    progress: Rc<ItemCollectionTransitionProgress>,
    finished: Cell<bool>,
}
impl RunningTransition {
    fn render(&self, progress: f64) {
        if self.finished.get() {
            return;
        }
        let (scale, delta, opacity) = self
            .timeline
            .sample(progress.clamp(0.0, 1.0) * self.timeline.duration());
        let matrix = self.original_matrix
            * Matrix::create_translation(-self.center.x, -self.center.y)
            * Matrix::create_scale(scale, scale)
            * Matrix::create_translation(self.center.x + delta.x, self.center.y + delta.y);
        self.transform.set_matrix(matrix);
        self.opacity.on_next(self.original_opacity * opacity);
        self.element.invalidate_visual();
    }
    fn finish(&self) {
        if self.finished.replace(true) {
            return;
        }
        self.cancellation.cancel();
        let values = self.values.borrow_mut().drain(..).collect::<Vec<_>>();
        for value in values {
            value.dispose();
        }
        self.progress.complete();
    }
}

/// A native Animatable used to drive the complete transition timeline. This
/// avoids tying animation completion to wall-clock timers or renderer frames.
#[repr(C)]
pub(crate) struct RepeaterAnimationProxy {
    base: Animatable,
    changed: RefCell<Option<Rc<dyn Fn(f64)>>>,
}
ferro_class!(RepeaterAnimationProxy: Animatable);
ferro_class_info!(RepeaterAnimationProxy {
    new: RepeaterAnimationProxy::new
});
impl FerroObjectImpl for RepeaterAnimationProxy {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::progress_property().as_property() {
            let changed = this.changed.borrow().clone();
            if let Some(changed) = changed {
                changed(change.get_new_value::<f64>());
            }
        }
    }
}
ferro_properties! {
    impl RepeaterAnimationProxy {
        pub(crate) fn progress_property() -> StyledProperty<f64> { FerroProperty::register::<RepeaterAnimationProxy, _>("Progress", 0.0) }
    }
}
impl RepeaterAnimationProxy {
    fn construct() -> Self {
        Self {
            base: Animatable::construct(),
            changed: RefCell::new(None),
        }
    }
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub(crate) fn set_changed(&self, callback: Option<Rc<dyn Fn(f64)>>) {
        *self.changed.borrow_mut() = callback;
    }
}
