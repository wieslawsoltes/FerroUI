// Copyright 2021 The Android Open Source Project (reference behavior and tokens).
// Copyright 2026 FerroUI contributors (Rust control and lifecycle integration).
// SPDX-License-Identifier: Apache-2.0
//! Queued Material snackbars with cancellable results and independent display/animation clocks.
//!
//! Reference: AndroidX Material3 1.5.0-beta02, revision
//! `e4d933c98df7baa7ea2b83eea6e086915961e0ff`, `SnackbarHost.kt` and `Snackbar.kt`:
//! <https://github.com/androidx/androidx/blob/e4d933c98df7baa7ea2b83eea6e086915961e0ff/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/SnackbarHost.kt>.
//! Hidden/detached FerroUI hosts pause the readable lifetime and stop their timers. Dropping
//! the host cancels its requests. Dropping a handle alone does not cancel a fire-and-forget
//! message; dropping its pending [`MaterialSnackbarFuture`] does cancel it.

use crate::motion::{
    animations_enabled, MaterialMotionRole, MaterialMotionScheme, MaterialSpringState,
};
use ferroui_base::animation::{IGlobalClock, TimeSpan};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::ScaleTransform;
use ferroui_base::reactive::{AnonymousObserver, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    DirectProperty, FerroLocator, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, LocatorExtensions, Ref, RelativePoint, StyledElementImpl,
    StyledProperty, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef,
};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, ControlAutomationPeer,
};
use ferroui_controls::automation::{AutomationLiveSetting, AutomationProperties};
use ferroui_controls::primitives::{
    TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use ferroui_controls::{Button, ControlImpl, Panel};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

/// Native Material display durations. These describe readable lifetime, not animation duration.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MaterialSnackbarDuration {
    #[default]
    Short,
    Long,
    Indefinite,
}

impl MaterialSnackbarDuration {
    pub const fn timeout(self) -> Option<Duration> {
        match self {
            Self::Short => Some(Duration::from_secs(4)),
            Self::Long => Some(Duration::from_secs(10)),
            Self::Indefinite => None,
        }
    }
}

/// The terminal outcome of one request. Cancellation is distinct from user dismissal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaterialSnackbarResult {
    Dismissed,
    ActionPerformed,
    Cancelled,
}

/// Content/options for a queued snackbar. `duration: None` selects the native default:
/// short without an action, indefinite with an action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterialSnackbarMessage {
    pub message: String,
    pub action_label: Option<String>,
    pub with_dismiss_action: bool,
    pub action_on_new_line: bool,
    pub duration: Option<MaterialSnackbarDuration>,
}

impl MaterialSnackbarMessage {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            action_label: None,
            with_dismiss_action: false,
            action_on_new_line: false,
            duration: None,
        }
    }

    pub fn with_action(mut self, label: impl Into<String>) -> Self {
        self.action_label = Some(label.into());
        self
    }

    pub fn with_duration(mut self, duration: MaterialSnackbarDuration) -> Self {
        self.duration = Some(duration);
        self
    }

    pub fn with_dismiss_action(mut self, visible: bool) -> Self {
        self.with_dismiss_action = visible;
        self
    }

    pub fn with_action_on_new_line(mut self, value: bool) -> Self {
        self.action_on_new_line = value;
        self
    }

    pub fn effective_duration(&self) -> MaterialSnackbarDuration {
        self.duration.unwrap_or(if self.action_label.is_some() {
            MaterialSnackbarDuration::Indefinite
        } else {
            MaterialSnackbarDuration::Short
        })
    }
}

impl From<&str> for MaterialSnackbarMessage {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}
impl From<String> for MaterialSnackbarMessage {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Adapter for a host's recommended accessibility timeout. Text and icons are always present;
/// `contains_controls` means an action label is present, matching the reference host.
/// Return `None` to require explicit dismissal. Recommendations cannot shorten the base duration.
pub trait MaterialSnackbarTimeoutPolicy {
    fn recommended_timeout(&self, original: Duration, contains_controls: bool) -> Option<Duration>;
}

#[derive(Default)]
struct Completion {
    result: Cell<Option<MaterialSnackbarResult>>,
    wakers: RefCell<Vec<Waker>>,
}

impl Completion {
    fn complete(&self, result: MaterialSnackbarResult) {
        if self.result.get().is_some() {
            return;
        }
        self.result.set(Some(result));
        let wakers = std::mem::take(&mut *self.wakers.borrow_mut());
        for waker in wakers {
            waker.wake();
        }
    }
}

/// Identity-stable access to one queued request. Its operations never address a successor.
#[derive(Clone)]
pub struct MaterialSnackbarHandle {
    id: u64,
    host: WeakRef<MaterialSnackbarHost>,
    completion: Rc<Completion>,
}

impl MaterialSnackbarHandle {
    pub fn id(&self) -> u64 {
        self.id
    }
    pub fn result(&self) -> Option<MaterialSnackbarResult> {
        self.completion.result.get()
    }
    pub fn cancel(&self) -> bool {
        self.host.upgrade().is_some_and(|host| host.cancel(self.id))
    }
    pub fn dismiss(&self) -> bool {
        self.host
            .upgrade()
            .is_some_and(|host| host.finish_current(self.id, MaterialSnackbarResult::Dismissed))
    }
    pub fn perform_action(&self) -> bool {
        self.host
            .upgrade()
            .is_some_and(|host| host.perform_action(self.id))
    }
}

impl IntoFuture for MaterialSnackbarHandle {
    type Output = MaterialSnackbarResult;
    type IntoFuture = MaterialSnackbarFuture;
    fn into_future(self) -> Self::IntoFuture {
        MaterialSnackbarFuture { handle: self }
    }
}

/// Awaitable request. Cancelling a pending future removes its active or queued message.
pub struct MaterialSnackbarFuture {
    handle: MaterialSnackbarHandle,
}

impl Future for MaterialSnackbarFuture {
    type Output = MaterialSnackbarResult;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(result) = self.handle.result() {
            return Poll::Ready(result);
        }
        let mut wakers = self.handle.completion.wakers.borrow_mut();
        if !wakers.iter().any(|waker| waker.will_wake(cx.waker())) {
            wakers.push(cx.waker().clone());
        }
        Poll::Pending
    }
}

impl Drop for MaterialSnackbarFuture {
    fn drop(&mut self) {
        if self.handle.result().is_none() {
            self.handle.cancel();
        }
    }
}

struct Request {
    id: u64,
    message: MaterialSnackbarMessage,
    completion: Rc<Completion>,
}

struct AnimatedSnackbar {
    id: u64,
    control: Ref<MaterialSnackbar>,
    transform: Ref<ScaleTransform>,
    alpha: MaterialSpringState,
    scale: MaterialSpringState,
    visible: bool,
}

/// A XAML-friendly host for a FIFO queue of Material snackbars.
/// Place it in the bottom row or overlay of the application's existing layout.
/// At most one request is actionable; outgoing visuals may overlap during their exit fade.
#[repr(C)]
pub struct MaterialSnackbarHost {
    base: TemplatedControl,
    next_id: Cell<u64>,
    is_open: Cell<bool>,
    queue: RefCell<VecDeque<Request>>,
    current: RefCell<Option<Request>>,
    visuals: RefCell<Vec<AnimatedSnackbar>>,
    items: RefCell<Option<Ref<Panel>>>,
    timeout_policy: RefCell<Option<Rc<dyn MaterialSnackbarTimeoutPolicy>>>,
    lifetime_timer: RefCell<Option<Rc<dyn IDisposable>>>,
    lifetime_remaining: Cell<Option<Duration>>,
    lifetime_started: Cell<Option<Instant>>,
    timer_generation: Cell<u64>,
    clock_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    visibility_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    last_frame: Cell<Option<f64>>,
}

ferro_class!(MaterialSnackbarHost: TemplatedControl);
ferro_class_info!(MaterialSnackbarHost {
    new: MaterialSnackbarHost::new,
    markup: { attributes: [TemplatePart("PART_Items", type(Ref<Panel>))] },
});
ferro_impl_classes!(MaterialSnackbarHost: StyledElementImpl, InteractiveImpl, InputElementImpl, LayoutableImpl, ControlImpl);

impl FerroObjectImpl for MaterialSnackbarHost {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::reduced_motion_property().as_property()
            || change.property() == Self::motion_scheme_property().as_property()
        {
            let scheme = this.motion_scheme();
            let reduced = !animations_enabled(this.reduced_motion());
            {
                let mut visuals = this.visuals.borrow_mut();
                for visual in visuals.iter_mut() {
                    visual
                        .alpha
                        .set_spec(scheme.spring(MaterialMotionRole::FastEffects));
                    visual
                        .scale
                        .set_spec(scheme.spring(MaterialMotionRole::FastSpatial));
                    visual.alpha.set_reduced_motion(reduced);
                    visual.scale.set_reduced_motion(reduced);
                }
            }
            this.advance_motion(0.0);
            this.update_clock();
        }
    }
}

impl VisualImpl for MaterialSnackbarHost {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let weak = this.to_ref().downgrade();
        let subscription = this.is_effectively_visible_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.update_activity();
            }
        });
        if let Some(old) = this.visibility_subscription.replace(Some(subscription)) {
            old.dispose();
        }
        this.update_activity();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.pause_lifetime();
        this.stop_clock();
        if let Some(subscription) = this.visibility_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

impl TemplatedControlImpl for MaterialSnackbarHost {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        if let Some(previous) = this.items.borrow_mut().take() {
            previous.children().clear();
        }
        let panel = e.name_scope().find_as::<Panel>("PART_Items");
        *this.items.borrow_mut() = panel.clone();
        if let Some(panel) = panel {
            let controls: Vec<_> = this
                .visuals
                .borrow()
                .iter()
                .map(|v| v.control.clone())
                .collect();
            for control in controls {
                panel.children().add(control);
            }
        }
        this.update_activity();
    }
}

ferro_properties! { impl MaterialSnackbarHost {
    pub fn reduced_motion_property() -> StyledProperty<bool> {
        FerroProperty::register::<MaterialSnackbarHost, _>("ReducedMotion", false)
    }
    pub fn motion_scheme_property() -> StyledProperty<MaterialMotionScheme> {
        FerroProperty::register::<MaterialSnackbarHost, _>("MotionScheme", MaterialMotionScheme::Expressive)
    }
    pub fn is_open_property() -> DirectProperty<MaterialSnackbarHost, bool> {
        FerroProperty::register_direct::<MaterialSnackbarHost, _>("IsOpen", |o| o.is_open(), None, false)
    }
}}

impl MaterialSnackbarHost {
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            next_id: Cell::new(0),
            is_open: Cell::new(false),
            queue: RefCell::new(VecDeque::new()),
            current: RefCell::new(None),
            visuals: RefCell::new(Vec::new()),
            items: RefCell::new(None),
            timeout_policy: RefCell::new(None),
            lifetime_timer: RefCell::new(None),
            lifetime_remaining: Cell::new(None),
            lifetime_started: Cell::new(None),
            timer_generation: Cell::new(0),
            clock_subscription: RefCell::new(None),
            visibility_subscription: RefCell::new(None),
            last_frame: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn reduced_motion(&self) -> bool {
        self.get_value(Self::reduced_motion_property())
    }
    pub fn set_reduced_motion(&self, value: bool) {
        self.set_value(Self::reduced_motion_property(), value);
    }
    pub fn motion_scheme(&self) -> MaterialMotionScheme {
        self.get_value(Self::motion_scheme_property())
    }
    pub fn set_motion_scheme(&self, value: MaterialMotionScheme) {
        self.set_value(Self::motion_scheme_property(), value);
    }
    pub fn is_open(&self) -> bool {
        self.is_open.get()
    }
    pub fn queued_count(&self) -> usize {
        self.queue.borrow().len()
    }
    pub fn current_id(&self) -> Option<u64> {
        self.current.borrow().as_ref().map(|request| request.id)
    }
    pub fn current_snackbar(&self) -> Option<Ref<MaterialSnackbar>> {
        let id = self.current_id()?;
        self.visuals
            .borrow()
            .iter()
            .find(|visual| visual.id == id)
            .map(|visual| visual.control.clone())
    }

    /// Set the platform accessibility timeout adapter for subsequently shown requests.
    pub fn set_timeout_policy(&self, policy: Option<Rc<dyn MaterialSnackbarTimeoutPolicy>>) {
        *self.timeout_policy.borrow_mut() = policy;
    }

    /// Queue a message. A discarded handle leaves the request queued; use `.cancel()` explicitly
    /// or `.await` the handle for cancellation tied to the asynchronous operation's lifetime.
    pub fn show(&self, message: impl Into<MaterialSnackbarMessage>) -> MaterialSnackbarHandle {
        Dispatcher::ui_thread().verify_access();
        let id = self
            .next_id
            .get()
            .checked_add(1)
            .expect("snackbar request identity exhausted");
        self.next_id.set(id);
        let completion = Rc::new(Completion::default());
        self.queue.borrow_mut().push_back(Request {
            id,
            message: message.into(),
            completion: completion.clone(),
        });
        let handle = MaterialSnackbarHandle {
            id,
            host: self.to_ref().downgrade(),
            completion,
        };
        self.present_next();
        handle
    }

    pub fn dismiss_current(&self) -> bool {
        self.current_id()
            .is_some_and(|id| self.finish_current(id, MaterialSnackbarResult::Dismissed))
    }

    /// Cancel one active or queued request. A stale identity cannot affect another message.
    pub fn cancel(&self, id: u64) -> bool {
        Dispatcher::ui_thread().verify_access();
        if self.current_id() == Some(id) {
            return self.finish_current(id, MaterialSnackbarResult::Cancelled);
        }
        let request = {
            let mut queue = self.queue.borrow_mut();
            queue
                .iter()
                .position(|request| request.id == id)
                .and_then(|index| queue.remove(index))
        };
        if let Some(request) = request {
            request
                .completion
                .complete(MaterialSnackbarResult::Cancelled);
            true
        } else {
            false
        }
    }

    /// Cancel the current request and every queued request, without displaying intermediate items.
    pub fn clear(&self) {
        let queued = std::mem::take(&mut *self.queue.borrow_mut());
        if let Some(id) = self.current_id() {
            self.finish_current(id, MaterialSnackbarResult::Cancelled);
        }
        for request in queued {
            request
                .completion
                .complete(MaterialSnackbarResult::Cancelled);
        }
    }

    fn present_next(&self) {
        if self.current.borrow().is_some() {
            return;
        }
        let request = self.queue.borrow_mut().pop_front();
        let Some(request) = request else {
            self.set_and_raise_cell(Self::is_open_property(), &self.is_open, false);
            self.update_clock();
            return;
        };
        let id = request.id;
        let message = request.message.clone();
        *self.current.borrow_mut() = Some(request);
        let policy = self.timeout_policy.borrow().clone();
        let timeout = message.effective_duration().timeout().and_then(|original| {
            policy.as_ref().map_or(Some(original), |policy| {
                policy
                    .recommended_timeout(original, message.action_label.is_some())
                    .map(|recommended| recommended.max(original))
            })
        });
        // A platform adapter may synchronously cancel or enqueue work. It runs without
        // borrowing our state, and cannot replace the current request accidentally.
        if self.current_id() != Some(id) {
            return;
        }
        let control = MaterialSnackbar::new();
        control.set_message(message.message);
        control.set_action_label(message.action_label);
        control.set_with_dismiss_action(message.with_dismiss_action);
        control.set_action_on_new_line(message.action_on_new_line);
        control.request_id.set(id);
        *control.host.borrow_mut() = Some(self.to_ref().downgrade());
        control.set_render_transform_origin(RelativePoint::CENTER);
        let transform = ScaleTransform::with_scale(0.8, 0.8);
        control.set_render_transform(Some(transform.clone().into()));
        AutomationProperties::set_live_setting(&control, AutomationLiveSetting::Polite);
        let scheme = self.motion_scheme();
        let reduced = !animations_enabled(self.reduced_motion());
        let mut alpha =
            MaterialSpringState::new(scheme.spring(MaterialMotionRole::FastEffects), 0.0);
        let mut scale =
            MaterialSpringState::new(scheme.spring(MaterialMotionRole::FastSpatial), 0.8);
        alpha.set_reduced_motion(reduced);
        scale.set_reduced_motion(reduced);
        alpha.retarget(1.0);
        scale.retarget(1.0);
        self.visuals.borrow_mut().push(AnimatedSnackbar {
            id,
            control: control.clone(),
            transform,
            alpha,
            scale,
            visible: true,
        });
        self.lifetime_remaining.set(timeout);
        let panel = self.items.borrow().clone();
        if let Some(panel) = panel {
            panel.children().add(control);
        }
        self.set_and_raise_cell(Self::is_open_property(), &self.is_open, true);
        self.advance_motion(0.0);
        self.update_activity();
    }

    fn perform_action(&self, id: u64) -> bool {
        let has_action = self
            .current
            .borrow()
            .as_ref()
            .is_some_and(|request| request.id == id && request.message.action_label.is_some());
        has_action && self.finish_current(id, MaterialSnackbarResult::ActionPerformed)
    }

    fn finish_current(&self, id: u64, result: MaterialSnackbarResult) -> bool {
        Dispatcher::ui_thread().verify_access();
        if self.current_id() != Some(id) {
            return false;
        }
        let request = self
            .current
            .borrow_mut()
            .take()
            .expect("checked current request");
        self.pause_lifetime();
        self.lifetime_remaining.set(None);
        let outgoing = {
            let mut visuals = self.visuals.borrow_mut();
            visuals
                .iter_mut()
                .find(|visual| visual.id == id)
                .map(|visual| {
                    visual.visible = false;
                    visual.alpha.retarget(0.0);
                    visual.scale.retarget(0.8);
                    visual.control.clone()
                })
        };
        if let Some(outgoing) = outgoing {
            outgoing.set_is_hit_test_visible(false);
            outgoing.set_is_enabled(false);
            AutomationProperties::set_live_setting(&outgoing, AutomationLiveSetting::Off);
        }
        self.advance_motion(0.0);
        self.present_next();
        request.completion.complete(result);
        true
    }

    fn active(&self) -> bool {
        self.is_attached_to_visual_tree() && self.is_effectively_visible()
    }

    fn update_activity(&self) {
        if self.active() {
            self.start_lifetime();
            self.update_clock();
        } else {
            self.pause_lifetime();
            self.stop_clock();
        }
    }

    fn start_lifetime(&self) {
        if self.lifetime_timer.borrow().is_some() {
            return;
        }
        let (Some(id), Some(remaining)) = (self.current_id(), self.lifetime_remaining.get()) else {
            return;
        };
        // Dispatcher timers have a signed 32-bit millisecond interval. Long accessibility
        // recommendations are scheduled in chunks, without clipping their total lifetime.
        let interval = remaining.min(Duration::from_millis(i32::MAX as u64));
        self.lifetime_started.set(Some(Instant::now()));
        let generation = self.timer_generation.get().wrapping_add(1);
        self.timer_generation.set(generation);
        let weak = self.to_ref().downgrade();
        let timer = DispatcherTimer::run_once(
            move || {
                if let Some(this) = weak.upgrade() {
                    if this.timer_generation.get() != generation || this.current_id() != Some(id) {
                        return;
                    }
                    if remaining > interval {
                        if let Some(timer) = this.lifetime_timer.borrow_mut().take() {
                            timer.dispose();
                        }
                        this.lifetime_started.set(None);
                        this.lifetime_remaining.set(Some(remaining - interval));
                        this.start_lifetime();
                    } else {
                        this.finish_current(id, MaterialSnackbarResult::Dismissed);
                    }
                }
            },
            interval,
            DispatcherPriority::NORMAL,
        );
        *self.lifetime_timer.borrow_mut() = Some(timer);
    }

    fn pause_lifetime(&self) {
        self.timer_generation
            .set(self.timer_generation.get().wrapping_add(1));
        if let Some(timer) = self.lifetime_timer.borrow_mut().take() {
            timer.dispose();
        }
        if let Some(started) = self.lifetime_started.take() {
            if let Some(remaining) = self.lifetime_remaining.get() {
                self.lifetime_remaining
                    .set(Some(remaining.saturating_sub(started.elapsed())));
            }
        }
    }

    fn advance_motion(&self, seconds: f64) {
        let reduced = !animations_enabled(self.reduced_motion());
        let (frames, removed) = {
            let mut visuals = self.visuals.borrow_mut();
            let mut frames = Vec::with_capacity(visuals.len());
            let mut removed = Vec::new();
            visuals.retain_mut(|visual| {
                visual.alpha.set_reduced_motion(reduced);
                visual.scale.set_reduced_motion(reduced);
                visual.alpha.advance(seconds);
                visual.scale.advance(seconds);
                // Float animations use a .01 value threshold and its 60 Hz velocity
                // counterpart, matching the reference animation-core defaults.
                if visual.alpha.is_at_rest(0.01, 0.625) {
                    visual.alpha.finish();
                }
                if visual.scale.is_at_rest(0.01, 0.625) {
                    visual.scale.finish();
                }
                if !visual.visible && visual.alpha.current().value == 0.0 {
                    removed.push(visual.control.clone());
                    false
                } else {
                    frames.push((
                        visual.control.clone(),
                        visual.transform.clone(),
                        visual.alpha.current().value,
                        visual.scale.current().value,
                    ));
                    true
                }
            });
            (frames, removed)
        };
        // Property notification, visual attachment and user code must run outside state borrows.
        for (control, transform, alpha, scale) in frames {
            control.set_opacity(alpha.clamp(0.0, 1.0));
            transform.set_scale_x(scale);
            transform.set_scale_y(scale);
        }
        let panel = self.items.borrow().clone();
        if let Some(panel) = panel {
            for control in removed {
                panel.children().remove(control);
            }
        }
    }

    fn motion_pending(&self) -> bool {
        self.visuals.borrow().iter().any(|visual| {
            !visual.alpha.is_at_rest(0.01, 0.625) || !visual.scale.is_at_rest(0.01, 0.625)
        })
    }

    fn update_clock(&self) {
        if !self.active() || !self.motion_pending() || !animations_enabled(self.reduced_motion()) {
            self.stop_clock();
            return;
        }
        if self.clock_subscription.borrow().is_some() {
            return;
        }
        let Some(clock) = FerroLocator::current().get_service::<dyn IGlobalClock>() else {
            self.advance_motion(f64::INFINITY);
            return;
        };
        let weak = self.to_ref().downgrade();
        self.last_frame.set(None);
        let subscription =
            clock.subscribe(Rc::new(AnonymousObserver::new(move |time: TimeSpan| {
                if let Some(this) = weak.upgrade() {
                    if !this.active() {
                        this.stop_clock();
                        return;
                    }
                    let time = time.total_seconds();
                    if !time.is_finite() {
                        return;
                    }
                    let seconds = this
                        .last_frame
                        .replace(Some(time))
                        .map_or(0.0, |previous| (time - previous).max(0.0));
                    this.advance_motion(seconds);
                    if !this.motion_pending() {
                        this.stop_clock();
                    }
                }
            })));
        *self.clock_subscription.borrow_mut() = Some(subscription);
    }

    fn stop_clock(&self) {
        if let Some(subscription) = self.clock_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        self.last_frame.set(None);
    }
}

impl Drop for MaterialSnackbarHost {
    fn drop(&mut self) {
        if let Some(timer) = self.lifetime_timer.get_mut().take() {
            timer.dispose();
        }
        if let Some(subscription) = self.clock_subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(subscription) = self.visibility_subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(request) = self.current.get_mut().take() {
            request
                .completion
                .complete(MaterialSnackbarResult::Cancelled);
        }
        for request in self.queue.get_mut().drain(..) {
            request
                .completion
                .complete(MaterialSnackbarResult::Cancelled);
        }
    }
}

/// The lookless visual for a snackbar. The host owns request lifetime; its template owns
/// message wrapping, optional action/dismiss buttons, and single/two-row composition.
#[repr(C)]
pub struct MaterialSnackbar {
    base: TemplatedControl,
    request_id: Cell<u64>,
    host: RefCell<Option<WeakRef<MaterialSnackbarHost>>>,
    action_button: RefCell<Option<(WeakRef<Button>, RoutedEventHandlerToken)>>,
    dismiss_button: RefCell<Option<(WeakRef<Button>, RoutedEventHandlerToken)>>,
}
ferro_class!(MaterialSnackbar: TemplatedControl);
ferro_class_info!(MaterialSnackbar {
    new: MaterialSnackbar::new,
    markup: { attributes: [
        TemplatePart("PART_ActionButton", type(Ref<Button>)),
        TemplatePart("PART_DismissButton", type(Ref<Button>)),
        PseudoClasses(":action", ":dismiss", ":actionnewline"),
    ] },
});
ferro_impl_classes!(MaterialSnackbar: StyledElementImpl, InteractiveImpl, InputElementImpl, LayoutableImpl, VisualImpl);

impl FerroObjectImpl for MaterialSnackbar {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::action_label_property().as_property()
            || property == Self::with_dismiss_action_property().as_property()
            || property == Self::action_on_new_line_property().as_property()
        {
            this.pseudo_classes()
                .set(":action", this.action_label().is_some());
            this.pseudo_classes()
                .set(":dismiss", this.with_dismiss_action());
            this.pseudo_classes().set(
                ":actionnewline",
                this.action_on_new_line() && this.action_label().is_some(),
            );
        }
        if property == Self::message_property().as_property() {
            AutomationProperties::set_name(this, Some(&this.message()));
        }
    }
}

impl ControlImpl for MaterialSnackbar {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        ControlAutomationPeer::new(this).upcast()
    }
}

impl TemplatedControlImpl for MaterialSnackbar {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        remove_button_handler(this.action_button.borrow_mut().take());
        remove_button_handler(this.dismiss_button.borrow_mut().take());
        if let Some(button) = e.name_scope().find_as::<Button>("PART_ActionButton") {
            let weak = this.to_ref().downgrade();
            let token = button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let host = this.host.borrow().as_ref().and_then(WeakRef::upgrade);
                    if let Some(host) = host {
                        host.perform_action(this.request_id.get());
                    }
                }
            });
            *this.action_button.borrow_mut() = Some((button.downgrade(), token));
        }
        if let Some(button) = e.name_scope().find_as::<Button>("PART_DismissButton") {
            let weak = this.to_ref().downgrade();
            let token = button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let host = this.host.borrow().as_ref().and_then(WeakRef::upgrade);
                    if let Some(host) = host {
                        host.finish_current(
                            this.request_id.get(),
                            MaterialSnackbarResult::Dismissed,
                        );
                    }
                }
            });
            *this.dismiss_button.borrow_mut() = Some((button.downgrade(), token));
        }
    }
}

ferro_properties! { impl MaterialSnackbar {
    pub fn message_property() -> StyledProperty<String> { FerroProperty::register::<MaterialSnackbar, _>("Message", String::new()) }
    pub fn action_label_property() -> StyledProperty<Option<String>> { FerroProperty::register::<MaterialSnackbar, _>("ActionLabel", None) }
    pub fn with_dismiss_action_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialSnackbar, _>("WithDismissAction", false) }
    pub fn action_on_new_line_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialSnackbar, _>("ActionOnNewLine", false) }
}}

impl MaterialSnackbar {
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            request_id: Cell::new(0),
            host: RefCell::new(None),
            action_button: RefCell::new(None),
            dismiss_button: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        AutomationProperties::set_control_type_override(
            &this,
            Some(AutomationControlType::StatusBar),
        );
        this
    }
    pub fn message(&self) -> String {
        self.get_value(Self::message_property())
    }
    pub fn set_message(&self, value: String) {
        self.set_value(Self::message_property(), value);
    }
    pub fn action_label(&self) -> Option<String> {
        self.get_value(Self::action_label_property())
    }
    pub fn set_action_label(&self, value: Option<String>) {
        self.set_value(Self::action_label_property(), value);
    }
    pub fn with_dismiss_action(&self) -> bool {
        self.get_value(Self::with_dismiss_action_property())
    }
    pub fn set_with_dismiss_action(&self, value: bool) {
        self.set_value(Self::with_dismiss_action_property(), value);
    }
    pub fn action_on_new_line(&self) -> bool {
        self.get_value(Self::action_on_new_line_property())
    }
    pub fn set_action_on_new_line(&self, value: bool) {
        self.set_value(Self::action_on_new_line_property(), value);
    }
}

fn remove_button_handler(handler: Option<(WeakRef<Button>, RoutedEventHandlerToken)>) {
    if let Some((button, token)) = handler {
        if let Some(button) = button.upgrade() {
            button.remove_handler(Button::click_event(), token);
        }
    }
}

impl Drop for MaterialSnackbar {
    fn drop(&mut self) {
        remove_button_handler(self.action_button.get_mut().take());
        remove_button_handler(self.dismiss_button.get_mut().take());
    }
}
