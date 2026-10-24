//! `VisualInteractionSource`: the input of an element, redirected to the
//! interaction trackers it is added to.

use super::interaction_tracker::InteractionTracker;
use crate::input::{
    IPointer, InputElement, KeyModifiers, PointerCaptureLostEventArgs, PointerEventArgs, PointerPoint,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerWheelEventArgs,
};
use crate::interactivity::Interactive;
use crate::numerics::Vector3;
use crate::reactive::IDisposable;
use crate::rendering::composition::{CompositionDrawListVisual, CompositionVisual, Compositor};
use crate::{Point, Ref, WeakRef};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

/// The pixels a notch of the mouse wheel scrolls, as the scroll content
/// presenter scrolls without a scroll unit.
const WHEEL_PIXELS_PER_NOTCH: f64 = 50.0;

/// The scale factor of a notch of the mouse wheel with Ctrl.
const WHEEL_SCALE_PER_NOTCH: f32 = 1.1;

/// The distance a drag moves before the rails decide its axis.
const RAILS_DECISION_DISTANCE: f64 = 4.0;

/// `tan(22.5 degrees)`: a drag within this slope of an axis is railed to it.
const RAILS_SLOPE: f64 = 0.414_213_57;

/// The window of the last moves the release velocity is measured over, in
/// milliseconds.
const VELOCITY_WINDOW_MS: u64 = 100;

/// How the input of a source drives an axis (`InteractionSourceMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InteractionSourceMode {
    /// The input does not drive the axis.
    #[default]
    Disabled,
    /// The input drives the axis and the release coasts in inertia.
    EnabledWithInertia,
    /// The input drives the axis and the release does not coast.
    EnabledWithoutInertia,
}

impl InteractionSourceMode {
    fn is_enabled(self) -> bool {
        self != Self::Disabled
    }
}

/// How the manipulations of nested sources chain
/// (`InteractionChainingMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InteractionChainingMode {
    #[default]
    Auto,
    Always,
    Never,
}

/// Which input other than touch is redirected to a source without
/// `TryRedirectForManipulation` (`VisualInteractionSourceRedirectionMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VisualInteractionSourceRedirectionMode {
    #[default]
    Off,
    CapableTouchpadOnly,
    PointerWheelOnly,
    CapableTouchpadAndPointerWheel,
}

impl VisualInteractionSourceRedirectionMode {
    fn includes_pointer_wheel(self) -> bool {
        matches!(self, Self::PointerWheelOnly | Self::CapableTouchpadAndPointerWheel)
    }
}

/// Whether a kind of redirected input drives an axis
/// (`InteractionSourceRedirectionMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InteractionSourceRedirectionMode {
    Disabled,
    #[default]
    Enabled,
}

/// Which axes the redirected mouse wheel drives
/// (`InteractionSourceConfiguration`, the `PointerWheelConfig` of a
/// source).
#[derive(Default)]
pub struct InteractionSourceConfiguration {
    position_x_source_mode: Cell<InteractionSourceRedirectionMode>,
    position_y_source_mode: Cell<InteractionSourceRedirectionMode>,
    scale_source_mode: Cell<InteractionSourceRedirectionMode>,
}

impl InteractionSourceConfiguration {
    pub fn position_x_source_mode(&self) -> InteractionSourceRedirectionMode {
        self.position_x_source_mode.get()
    }

    pub fn set_position_x_source_mode(&self, value: InteractionSourceRedirectionMode) {
        self.position_x_source_mode.set(value)
    }

    pub fn position_y_source_mode(&self) -> InteractionSourceRedirectionMode {
        self.position_y_source_mode.get()
    }

    pub fn set_position_y_source_mode(&self, value: InteractionSourceRedirectionMode) {
        self.position_y_source_mode.set(value)
    }

    pub fn scale_source_mode(&self) -> InteractionSourceRedirectionMode {
        self.scale_source_mode.get()
    }

    pub fn set_scale_source_mode(&self, value: InteractionSourceRedirectionMode) {
        self.scale_source_mode.set(value)
    }
}

/// The axis a drag is railed to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rail {
    X,
    Y,
}

/// A drag in progress, after `TryRedirectForManipulation`.
struct Manipulation {
    pointer: Option<Rc<dyn IPointer>>,
    subscriptions: Vec<Rc<dyn IDisposable>>,
    trackers: Vec<Rc<InteractionTracker>>,
    start: Point,
    last: Point,
    rails_decided: bool,
    rail: Option<Rail>,
    /// The recent moves (timestamp in milliseconds, position).
    samples: VecDeque<(u64, Point)>,
}

/// The input of the element of a visual, redirected to the interaction
/// trackers the source is added to
/// (`Microsoft.UI.Composition.Interactions.VisualInteractionSource`).
///
/// A drag starts with [`try_redirect_for_manipulation`](Self::try_redirect_for_manipulation),
/// called from a pointer pressed handler: the source captures the pointer to
/// the element and follows its moves until it is released. Dragging content
/// up increases `Position.Y` of the trackers, as scrolling does. The mouse
/// wheel is redirected without a call when `ManipulationRedirectionMode`
/// includes it.
///
/// Deviation (winui-composition.md): WinUI receives the input in the
/// compositor once redirected. Here the source listens to the routed pointer
/// events of the element (handled ones too) and drives the trackers on the
/// UI thread; the element is the one of the visual (the element whose
/// composition visual it is, or of its nearest ancestor) or the one given to
/// [`create_for_element`](Self::create_for_element).
pub struct VisualInteractionSource {
    this: Weak<VisualInteractionSource>,
    source: Rc<CompositionVisual>,
    element: RefCell<Option<WeakRef<InputElement>>>,
    position_x_source_mode: Cell<InteractionSourceMode>,
    position_y_source_mode: Cell<InteractionSourceMode>,
    scale_source_mode: Cell<InteractionSourceMode>,
    position_x_chaining_mode: Cell<InteractionChainingMode>,
    position_y_chaining_mode: Cell<InteractionChainingMode>,
    scale_chaining_mode: Cell<InteractionChainingMode>,
    manipulation_redirection_mode: Cell<VisualInteractionSourceRedirectionMode>,
    is_position_x_rails_enabled: Cell<bool>,
    is_position_y_rails_enabled: Cell<bool>,
    pointer_wheel_config: Rc<InteractionSourceConfiguration>,
    delta_position: Cell<Vector3>,
    delta_scale: Cell<f32>,
    position: Cell<Vector3>,
    position_velocity: Cell<Vector3>,
    scale: Cell<f32>,
    scale_velocity: Cell<f32>,
    trackers: RefCell<Vec<Weak<InteractionTracker>>>,
    manipulation: RefCell<Option<Manipulation>>,
    wheel_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl VisualInteractionSource {
    /// `VisualInteractionSource.Create(source)`: a source for the input of
    /// the element of `source`.
    pub fn create(source: &Rc<CompositionVisual>) -> Rc<VisualInteractionSource> {
        let element = Self::find_element(source);
        Self::create_core(source, element)
    }

    /// A source for the input of `element`, shown by `source` (not a WinUI
    /// member): for a visual that is not, or not yet, the composition visual
    /// of an element or below one.
    pub fn create_for_element(source: &Rc<CompositionVisual>, element: &Ref<InputElement>) -> Rc<VisualInteractionSource> {
        Self::create_core(source, Some(element.clone()))
    }

    fn create_core(source: &Rc<CompositionVisual>, element: Option<Ref<InputElement>>) -> Rc<VisualInteractionSource> {
        Rc::new_cyclic(|this: &Weak<VisualInteractionSource>| VisualInteractionSource {
            this: this.clone(),
            source: source.clone(),
            element: RefCell::new(element.map(|element| element.downgrade())),
            position_x_source_mode: Cell::new(InteractionSourceMode::Disabled),
            position_y_source_mode: Cell::new(InteractionSourceMode::Disabled),
            scale_source_mode: Cell::new(InteractionSourceMode::Disabled),
            position_x_chaining_mode: Cell::new(InteractionChainingMode::Auto),
            position_y_chaining_mode: Cell::new(InteractionChainingMode::Auto),
            scale_chaining_mode: Cell::new(InteractionChainingMode::Auto),
            manipulation_redirection_mode: Cell::new(VisualInteractionSourceRedirectionMode::Off),
            is_position_x_rails_enabled: Cell::new(true),
            is_position_y_rails_enabled: Cell::new(true),
            pointer_wheel_config: Rc::new(InteractionSourceConfiguration::default()),
            delta_position: Cell::new(Vector3::ZERO),
            delta_scale: Cell::new(1.0),
            position: Cell::new(Vector3::ZERO),
            position_velocity: Cell::new(Vector3::ZERO),
            scale: Cell::new(1.0),
            scale_velocity: Cell::new(0.0),
            trackers: RefCell::new(Vec::new()),
            manipulation: RefCell::new(None),
            wheel_subscription: RefCell::new(None),
        })
    }

    /// The element of a composition visual: the element it is the visual
    /// of, or the one of its nearest ancestor that has one.
    fn find_element(visual: &Rc<CompositionVisual>) -> Option<Ref<InputElement>> {
        let mut current = Some(visual.clone());
        while let Some(visual) = current {
            let element = CompositionDrawListVisual::from_visual(&visual)
                .and_then(|draw_list| draw_list.visual())
                .and_then(|element| element.downcast::<InputElement>().ok());
            if element.is_some() {
                return element;
            }
            current = visual.parent();
        }
        None
    }

    /// The element whose input the source redirects, if it is known and
    /// alive. Looked up again from the visual while it is not known.
    pub fn element(&self) -> Option<Ref<InputElement>> {
        if let Some(element) = self.element.borrow().as_ref().and_then(WeakRef::upgrade) {
            return Some(element);
        }
        let element = Self::find_element(&self.source)?;
        *self.element.borrow_mut() = Some(element.downgrade());
        Some(element)
    }

    /// The visual the source was created for (`Source`).
    pub fn source(&self) -> &Rc<CompositionVisual> {
        &self.source
    }

    /// The compositor of the visual.
    pub fn compositor(&self) -> &Rc<Compositor> {
        self.source.compositor()
    }

    // --- configuration ------------------------------------------------------

    /// `PositionXSourceMode`.
    pub fn position_x_source_mode(&self) -> InteractionSourceMode {
        self.position_x_source_mode.get()
    }

    pub fn set_position_x_source_mode(&self, value: InteractionSourceMode) {
        self.position_x_source_mode.set(value)
    }

    /// `PositionYSourceMode`.
    pub fn position_y_source_mode(&self) -> InteractionSourceMode {
        self.position_y_source_mode.get()
    }

    pub fn set_position_y_source_mode(&self, value: InteractionSourceMode) {
        self.position_y_source_mode.set(value)
    }

    /// `ScaleSourceMode`. Only the Ctrl + mouse wheel scales: pinch input
    /// is not redirected.
    pub fn scale_source_mode(&self) -> InteractionSourceMode {
        self.scale_source_mode.get()
    }

    pub fn set_scale_source_mode(&self, value: InteractionSourceMode) {
        self.scale_source_mode.set(value)
    }

    /// `PositionXChainingMode`.
    ///
    /// Deviation (winui-composition.md): stored only; a manipulation does
    /// not chain to the sources of ancestors.
    pub fn position_x_chaining_mode(&self) -> InteractionChainingMode {
        self.position_x_chaining_mode.get()
    }

    pub fn set_position_x_chaining_mode(&self, value: InteractionChainingMode) {
        self.position_x_chaining_mode.set(value)
    }

    /// `PositionYChainingMode` (stored only).
    pub fn position_y_chaining_mode(&self) -> InteractionChainingMode {
        self.position_y_chaining_mode.get()
    }

    pub fn set_position_y_chaining_mode(&self, value: InteractionChainingMode) {
        self.position_y_chaining_mode.set(value)
    }

    /// `ScaleChainingMode` (stored only).
    pub fn scale_chaining_mode(&self) -> InteractionChainingMode {
        self.scale_chaining_mode.get()
    }

    pub fn set_scale_chaining_mode(&self, value: InteractionChainingMode) {
        self.scale_chaining_mode.set(value)
    }

    /// `ManipulationRedirectionMode`: the input redirected without
    /// `TryRedirectForManipulation`. The mouse wheel is redirected when it
    /// includes `PointerWheel`; touchpads have no input of their own in
    /// FerroUI (their scrolling arrives as the wheel).
    pub fn manipulation_redirection_mode(&self) -> VisualInteractionSourceRedirectionMode {
        self.manipulation_redirection_mode.get()
    }

    pub fn set_manipulation_redirection_mode(&self, value: VisualInteractionSourceRedirectionMode) {
        self.manipulation_redirection_mode.set(value);
        self.update_wheel_subscription();
    }

    /// `IsPositionXRailsEnabled`: a drag that starts close to the X axis
    /// moves along it only.
    pub fn is_position_x_rails_enabled(&self) -> bool {
        self.is_position_x_rails_enabled.get()
    }

    pub fn set_is_position_x_rails_enabled(&self, value: bool) {
        self.is_position_x_rails_enabled.set(value)
    }

    /// `IsPositionYRailsEnabled`.
    pub fn is_position_y_rails_enabled(&self) -> bool {
        self.is_position_y_rails_enabled.get()
    }

    pub fn set_is_position_y_rails_enabled(&self, value: bool) {
        self.is_position_y_rails_enabled.set(value)
    }

    /// `PointerWheelConfig`: the axes the redirected wheel drives.
    pub fn pointer_wheel_config(&self) -> &Rc<InteractionSourceConfiguration> {
        &self.pointer_wheel_config
    }

    // --- values of the input ------------------------------------------------

    /// The position change of the last move (`DeltaPosition`), as the
    /// trackers see it.
    ///
    /// Deviation (winui-composition.md): the values of the input are
    /// properties of the UI-thread object; expressions cannot reference a
    /// source.
    pub fn delta_position(&self) -> Vector3 {
        self.delta_position.get()
    }

    /// The scale change of the last input (`DeltaScale`), as a factor.
    pub fn delta_scale(&self) -> f32 {
        self.delta_scale.get()
    }

    /// The position of the current drag since it started (`Position`).
    pub fn position(&self) -> Vector3 {
        self.position.get()
    }

    /// The velocity of the current drag (`PositionVelocity`).
    pub fn position_velocity(&self) -> Vector3 {
        self.position_velocity.get()
    }

    /// The scale of the input (`Scale`).
    pub fn scale(&self) -> f32 {
        self.scale.get()
    }

    /// The velocity of the scale of the input (`ScaleVelocity`).
    pub fn scale_velocity(&self) -> f32 {
        self.scale_velocity.get()
    }

    // --- trackers -----------------------------------------------------------

    pub(crate) fn attach_tracker(&self, tracker: &Weak<InteractionTracker>) {
        let mut trackers = self.trackers.borrow_mut();
        trackers.retain(|t| t.strong_count() > 0);
        if !trackers.iter().any(|t| t.ptr_eq(tracker)) {
            trackers.push(tracker.clone());
        }
        drop(trackers);
        self.update_wheel_subscription();
    }

    pub(crate) fn detach_tracker(&self, tracker: &Weak<InteractionTracker>) {
        self.trackers.borrow_mut().retain(|t| t.strong_count() > 0 && !t.ptr_eq(tracker));
    }

    fn live_trackers(&self) -> Vec<Rc<InteractionTracker>> {
        self.trackers.borrow().iter().filter_map(Weak::upgrade).collect()
    }

    // --- redirection --------------------------------------------------------

    /// `TryRedirectForManipulation(pointerPoint)`, from the pressed event:
    /// the drag of the pressed pointer drives the trackers until it is
    /// released. Returns whether the drag was redirected (not a WinUI
    /// result): not when a drag of this source is in progress, when no axis
    /// is enabled, or when the source has no element.
    ///
    /// Deviation (winui-composition.md): any pointer type is redirected,
    /// mouse included.
    pub fn try_redirect_for_manipulation(&self, e: &PointerPressedEventArgs) -> bool {
        let element = self
            .element()
            .or_else(|| e.source().and_then(|source| source.downcast::<InputElement>().ok()));
        let Some(element) = element else { return false };
        let point = e.get_current_point(None);
        self.try_redirect_for_manipulation_point(&point, &element)
    }

    /// `TryRedirectForManipulation(pointerPoint)` with the element that
    /// captures the pointer. The position of `point` must be in the
    /// coordinates of the top level (`get_current_point(None)`), as the
    /// moves are followed there.
    pub fn try_redirect_for_manipulation_point(&self, point: &PointerPoint, element: &Ref<InputElement>) -> bool {
        if self.manipulation.borrow().is_some() {
            return false;
        }
        if !self.position_x_source_mode.get().is_enabled() && !self.position_y_source_mode.get().is_enabled() {
            return false;
        }
        let pointer = point.pointer.clone();
        let pointer_id = pointer.id();
        pointer.capture(Some(element));

        let mut subscriptions = Vec::new();
        let weak = self.this.clone();
        subscriptions.push(element.add_disposable_handler_with(
            InputElement::pointer_moved_event(),
            move |_, e: &PointerEventArgs| {
                if e.pointer().id() != pointer_id {
                    return;
                }
                if let Some(this) = weak.upgrade() {
                    this.manipulation_moved(e.get_current_point(None).position, e.timestamp());
                }
            },
            Interactive::DEFAULT_ROUTES,
            true,
        ));
        let weak = self.this.clone();
        subscriptions.push(element.add_disposable_handler_with(
            InputElement::pointer_released_event(),
            move |_, e: &PointerReleasedEventArgs| {
                if e.pointer().id() != pointer_id {
                    return;
                }
                if let Some(this) = weak.upgrade() {
                    this.manipulation_moved(e.get_current_point(None).position, e.timestamp());
                    this.manipulation_ended(Some(e.timestamp()));
                }
            },
            Interactive::DEFAULT_ROUTES,
            true,
        ));
        let weak = self.this.clone();
        subscriptions.push(element.add_disposable_handler_with(
            InputElement::pointer_capture_lost_event(),
            move |_, e: &PointerCaptureLostEventArgs| {
                if e.pointer().id() != pointer_id {
                    return;
                }
                if let Some(this) = weak.upgrade() {
                    // Released already, or taken by another element: a
                    // drag that is still in progress is cancelled.
                    this.manipulation_ended(None);
                }
            },
            Interactive::DEFAULT_ROUTES,
            true,
        ));

        self.begin_manipulation(point.position, Some(pointer), subscriptions);
        true
    }

    /// Starts a drag at `position` (top-level coordinates): the trackers
    /// enter the interacting state.
    pub(crate) fn begin_manipulation(
        &self,
        position: Point,
        pointer: Option<Rc<dyn IPointer>>,
        subscriptions: Vec<Rc<dyn IDisposable>>,
    ) {
        let trackers = self.live_trackers();
        *self.manipulation.borrow_mut() = Some(Manipulation {
            pointer,
            subscriptions,
            trackers: trackers.clone(),
            start: position,
            last: position,
            rails_decided: false,
            rail: None,
            samples: VecDeque::new(),
        });
        self.position.set(Vector3::ZERO);
        self.delta_position.set(Vector3::ZERO);
        self.position_velocity.set(Vector3::ZERO);
        for tracker in trackers {
            tracker.begin_interaction();
        }
    }

    /// The drag moved to `position` at `timestamp` (milliseconds).
    pub(crate) fn manipulation_moved(&self, position: Point, timestamp: u64) {
        let (delta, trackers) = {
            let mut manipulation = self.manipulation.borrow_mut();
            let Some(m) = manipulation.as_mut() else { return };
            let total = position - m.start;
            if !m.rails_decided && (total.x * total.x + total.y * total.y).sqrt() >= RAILS_DECISION_DISTANCE {
                m.rails_decided = true;
                if self.is_position_x_rails_enabled.get() && total.y.abs() <= total.x.abs() * RAILS_SLOPE {
                    m.rail = Some(Rail::X);
                } else if self.is_position_y_rails_enabled.get() && total.x.abs() <= total.y.abs() * RAILS_SLOPE {
                    m.rail = Some(Rail::Y);
                }
            }
            let mut step = position - m.last;
            m.last = position;
            match m.rail {
                Some(Rail::X) => step = Point::new(step.x, 0.0),
                Some(Rail::Y) => step = Point::new(0.0, step.y),
                None => {}
            }
            m.samples.push_back((timestamp, position));
            while m.samples.len() > 2 && m.samples.front().is_some_and(|(t, _)| timestamp.saturating_sub(*t) > VELOCITY_WINDOW_MS) {
                m.samples.pop_front();
            }
            // Dragging content up scrolls it down: the position moves
            // against the pointer.
            let delta = self.mask(Vector3::new(-step.x as f32, -step.y as f32, 0.0));
            self.position.set(self.position.get() + delta);
            (delta, m.trackers.clone())
        };
        self.delta_position.set(delta);
        let velocity = self.release_velocity(timestamp, false);
        self.position_velocity.set(velocity);
        if delta == Vector3::ZERO {
            return;
        }
        for tracker in trackers {
            tracker.interaction_delta(delta, velocity);
        }
    }

    /// The drag ended: released at `timestamp`, or cancelled (`None`). The
    /// trackers coast with the velocity of the last moves, on the axes
    /// whose mode has inertia.
    pub(crate) fn manipulation_ended(&self, timestamp: Option<u64>) {
        let velocity = match timestamp {
            Some(timestamp) => self.release_velocity(timestamp, true),
            None => Vector3::ZERO,
        };
        let Some(manipulation) = self.manipulation.borrow_mut().take() else { return };
        for subscription in &manipulation.subscriptions {
            subscription.dispose();
        }
        drop(manipulation.pointer);
        self.position_velocity.set(Vector3::ZERO);
        for tracker in manipulation.trackers {
            tracker.end_interaction(velocity);
        }
    }

    /// Zeroes the axes whose mode is disabled.
    fn mask(&self, v: Vector3) -> Vector3 {
        Vector3::new(
            if self.position_x_source_mode.get().is_enabled() { v.x } else { 0.0 },
            if self.position_y_source_mode.get().is_enabled() { v.y } else { 0.0 },
            0.0,
        )
    }

    /// The velocity of the drag over the last moves, in the direction of
    /// the trackers. For a release, only the axes with inertia keep it, and
    /// a pointer held still for longer than the window releases without
    /// velocity.
    fn release_velocity(&self, now: u64, release: bool) -> Vector3 {
        let manipulation = self.manipulation.borrow();
        let Some(m) = manipulation.as_ref() else { return Vector3::ZERO };
        let (Some(&(t0, p0)), Some(&(t1, p1))) = (m.samples.front(), m.samples.back()) else { return Vector3::ZERO };
        if t1 <= t0 || now.saturating_sub(t1) > VELOCITY_WINDOW_MS {
            return Vector3::ZERO;
        }
        let seconds = (t1 - t0) as f64 / 1000.0;
        let mut v = (p1 - p0) / seconds;
        match m.rail {
            Some(Rail::X) => v = Point::new(v.x, 0.0),
            Some(Rail::Y) => v = Point::new(0.0, v.y),
            None => {}
        }
        let v = self.mask(Vector3::new(-v.x as f32, -v.y as f32, 0.0));
        if !release {
            return v;
        }
        let inertia = |mode: InteractionSourceMode| mode == InteractionSourceMode::EnabledWithInertia;
        Vector3::new(
            if inertia(self.position_x_source_mode.get()) { v.x } else { 0.0 },
            if inertia(self.position_y_source_mode.get()) { v.y } else { 0.0 },
            0.0,
        )
    }

    // --- mouse wheel --------------------------------------------------------

    fn update_wheel_subscription(&self) {
        let wanted = self.manipulation_redirection_mode.get().includes_pointer_wheel();
        if !wanted {
            if let Some(subscription) = self.wheel_subscription.borrow_mut().take() {
                subscription.dispose();
            }
            return;
        }
        if self.wheel_subscription.borrow().is_some() {
            return;
        }
        let Some(element) = self.element() else { return };
        let weak = self.this.clone();
        let subscription = element.add_disposable_handler_with(
            InputElement::pointer_wheel_changed_event(),
            move |_, e: &PointerWheelEventArgs| {
                if e.handled() {
                    return;
                }
                if let Some(this) = weak.upgrade() {
                    if this.on_wheel(e) {
                        e.set_handled(true);
                    }
                }
            },
            Interactive::DEFAULT_ROUTES,
            false,
        );
        *self.wheel_subscription.borrow_mut() = Some(subscription);
    }

    fn on_wheel(&self, e: &PointerWheelEventArgs) -> bool {
        if !self.manipulation_redirection_mode.get().includes_pointer_wheel() {
            return false;
        }
        let delta = e.delta();
        let config = &self.pointer_wheel_config;
        if e.key_modifiers().contains(KeyModifiers::CONTROL) {
            if !self.scale_source_mode.get().is_enabled()
                || config.scale_source_mode() != InteractionSourceRedirectionMode::Enabled
            {
                return false;
            }
            let center = match self.element() {
                Some(element) => e.get_position(Some(&element)),
                None => Point::default(),
            };
            return self.wheel_scale(delta.y as f32, Vector3::new(center.x as f32, center.y as f32, 0.0));
        }
        let x = if self.position_x_source_mode.get().is_enabled()
            && config.position_x_source_mode() == InteractionSourceRedirectionMode::Enabled
        {
            -delta.x * WHEEL_PIXELS_PER_NOTCH
        } else {
            0.0
        };
        let y = if self.position_y_source_mode.get().is_enabled()
            && config.position_y_source_mode() == InteractionSourceRedirectionMode::Enabled
        {
            -delta.y * WHEEL_PIXELS_PER_NOTCH
        } else {
            0.0
        };
        self.wheel_position(Vector3::new(x as f32, y as f32, 0.0))
    }

    /// The redirected wheel scrolled the trackers by `delta` (pixels, in
    /// the direction of the trackers). Returns whether it drove a tracker.
    pub(crate) fn wheel_position(&self, delta: Vector3) -> bool {
        if delta == Vector3::ZERO {
            return false;
        }
        let trackers = self.live_trackers();
        self.delta_position.set(delta);
        for tracker in &trackers {
            tracker.wheel_delta(delta);
        }
        !trackers.is_empty()
    }

    /// The redirected wheel with Ctrl scaled by `notches` around `center`.
    fn wheel_scale(&self, notches: f32, center: Vector3) -> bool {
        if notches == 0.0 {
            return false;
        }
        let factor = WHEEL_SCALE_PER_NOTCH.powf(notches);
        let trackers = self.live_trackers();
        self.delta_scale.set(factor);
        self.scale.set(self.scale.get() * factor);
        for tracker in &trackers {
            tracker.wheel_scale(factor, center);
        }
        !trackers.is_empty()
    }
}

impl Drop for VisualInteractionSource {
    fn drop(&mut self) {
        if let Some(subscription) = self.wheel_subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(manipulation) = self.manipulation.get_mut().take() {
            for subscription in &manipulation.subscriptions {
                subscription.dispose();
            }
        }
    }
}
