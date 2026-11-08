//! Tests of the routing of pointer input to interactive effects (#455, E7; not from upstream;
//! `input_element_effects.rs`), with the test root of the input tests (a child module of them):
//! an interactive effect is attached and detached with the effect of its host and its handlers
//! with it; the pointer events reach it in the order of the routed events, not when they are
//! handled unless it asks for handled events, wherever the pointer is while the host captures it,
//! and the loss of the capture reaches it; the positions are those of the host, normalized to it
//! and in the coordinates of the program, through a render transform; and the interactive effects
//! of the library follow the pointer and take the presses their hosts handled (the burn-away button
//! the press of its button), and the water ripple makes ripples and stops animating when it is
//! detached.

use super::*;
use crate::media::effects::library::{
    InteractiveBurningFlameEffect, InteractivePointerSpotlightEffect, InteractiveWaterRippleEffect,
};
use crate::media::effects::{
    CustomEffect, CustomEffectClass, Effect, EffectHostContext, EffectParameterValue, IEffect, IEffectInputHandler,
    ImmutableBlurEffect, ImmutableCustomEffect,
};

/// The snapshot of the logging effect: a value with a padding.
#[derive(Clone, Debug, Default, PartialEq)]
struct Probe {
    padding: f64,
}

impl CustomEffect for Probe {
    const NAME: &'static str = "input-test-probe";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("padding", self.padding.into());
    }

    fn set_parameter(&mut self, _: &str, _: EffectParameterValue) -> bool {
        false
    }

    fn padding(&self) -> Thickness {
        Thickness::uniform(self.padding)
    }
}

/// An interactive effect that logs what it receives into a log it shares with the handlers of
/// the tests, and records the positions of the pointer events: in the host, normalized and in the
/// coordinates of the program, with the scale of the layer.
#[repr(C)]
struct LoggingEffect {
    base: Effect,
    log: Rc<RefCell<Vec<String>>>,
    padding: f64,
    capture_on_press: Cell<bool>,
    handled_too: Cell<bool>,
    positions: RefCell<Vec<(Point, Point, Point, f64)>>,
}

ferro_class!(LoggingEffect: Effect);
ferro_impl_classes!(LoggingEffect: FerroObjectImpl);

impl LoggingEffect {
    fn new(log: &Rc<RefCell<Vec<String>>>, padding: f64) -> Ref<Self> {
        instantiate(Self {
            base: Effect::construct_interactive::<Self>(),
            log: log.clone(),
            padding,
            capture_on_press: Cell::new(false),
            handled_too: Cell::new(false),
            positions: RefCell::new(Vec::new()),
        })
    }

    fn record(&self, context: &EffectHostContext<'_>, event: &str) {
        self.log.borrow_mut().push(format!("effect {event} on {}", label_of(context.host())));
    }

    fn record_at(&self, context: &EffectHostContext<'_>, event: &str, e: &PointerEventArgs) {
        self.record(context, event);
        self.positions.borrow_mut().push((
            context.get_position(e),
            context.get_normalized_position(e, false),
            context.get_effect_position(e),
            context.layer_scale(),
        ));
    }

    fn last_position(&self) -> (Point, Point, Point, f64) {
        *self.positions.borrow().last().expect("a pointer event was recorded")
    }
}

impl CustomEffectClass for LoggingEffect {
    fn to_custom_effect(&self) -> ImmutableCustomEffect {
        ImmutableCustomEffect::new(Probe { padding: self.padding })
    }
}

impl IEffectInputHandler for LoggingEffect {
    fn handled_events_too(&self) -> bool {
        self.handled_too.get()
    }

    fn on_attached(&self, context: &EffectHostContext<'_>) {
        self.record(context, "attached");
    }

    fn on_detached(&self, context: &EffectHostContext<'_>) {
        self.record(context, "detached");
    }

    fn on_host_bounds_changed(&self, context: &EffectHostContext<'_>) {
        self.record(context, "bounds");
    }

    fn on_pointer_entered(&self, context: &EffectHostContext<'_>, e: &PointerEventArgs) {
        self.record_at(context, "entered", e);
    }

    fn on_pointer_exited(&self, context: &EffectHostContext<'_>, e: &PointerEventArgs) {
        self.record_at(context, "exited", e);
    }

    fn on_pointer_moved(&self, context: &EffectHostContext<'_>, e: &PointerEventArgs) {
        self.record_at(context, "moved", e);
    }

    fn on_pointer_pressed(&self, context: &EffectHostContext<'_>, e: &PointerPressedEventArgs) {
        self.record_at(context, "pressed", e);
        if self.capture_on_press.get() {
            context.capture_pointer(e);
        }
    }

    fn on_pointer_released(&self, context: &EffectHostContext<'_>, e: &PointerReleasedEventArgs) {
        self.record_at(context, "released", e);
        if self.capture_on_press.get() {
            context.release_pointer_capture(e);
        }
    }

    fn on_pointer_capture_lost(&self, context: &EffectHostContext<'_>, _e: &PointerCaptureLostEventArgs) {
        self.record(context, "capture lost");
    }

    fn on_pointer_wheel_changed(&self, context: &EffectHostContext<'_>, e: &PointerWheelEventArgs) {
        self.record(context, &format!("wheel {}", e.delta().y));
    }
}

/// The entries of `log` that are pointer events of the effect other than entering and leaving.
fn effect_events(log: &Rc<RefCell<Vec<String>>>) -> Vec<String> {
    log.borrow()
        .iter()
        .filter(|entry| entry.starts_with("effect ") && !entry.contains("entered") && !entry.contains("exited"))
        .filter(|entry| !entry.contains("attached") && !entry.contains("bounds"))
        .cloned()
        .collect()
}

/// Sets `effect` as the effect of `host`.
fn set_effect(host: &Ref<TestControl>, effect: &Ref<LoggingEffect>) {
    let effect: Rc<dyn IEffect> = effect.clone().into();
    host.set_effect(Some(effect));
}

#[test]
fn an_interactive_effect_is_attached_and_detached_with_the_effect_of_its_host() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let events = log();
    let handlers = Interactive::total_handlers_count();
    let property_subscribers = tree.border.property_changed_subscriber_count();
    let effect = LoggingEffect::new(&events, 0.0);

    set_effect(&tree.border, &effect);
    assert_eq!(*events.borrow(), ["effect attached on border", "effect bounds on border"]);
    let attached: Ref<Effect> = effect.clone().upcast();
    assert_eq!(Some(attached), tree.border.interactive_effect());
    assert!(Interactive::total_handlers_count() > handlers);

    // The bounds and the render transform of the host reach the effect.
    events.borrow_mut().clear();
    tree.border.set_bounds(Rect::new(100.0, 0.0, 100.0, 150.0));
    tree.border.set_render_transform_origin(RelativePoint::TOP_LEFT);
    assert_eq!(*events.borrow(), ["effect bounds on border", "effect bounds on border"]);

    // Another effect detaches it, and its handlers with it.
    events.borrow_mut().clear();
    tree.border.set_effect(Some(Rc::new(ImmutableBlurEffect::new(2.0))));
    assert_eq!(*events.borrow(), ["effect detached on border"]);
    assert!(tree.border.interactive_effect().is_none());
    assert_eq!(handlers, Interactive::total_handlers_count());
    assert_eq!(property_subscribers, tree.border.property_changed_subscriber_count());
    events.borrow_mut().clear();
    tree.root.host.mouse_move(&device, OVER_DECORATOR);
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_DECORATOR, 0);
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonUp, OVER_DECORATOR, 1);
    tree.border.set_bounds(Rect::new(100.0, 0.0, 100.0, 200.0));
    assert!(events.borrow().is_empty(), "{:?}", events.borrow());

    // Set again, it is attached again; cleared, detached.
    set_effect(&tree.border, &effect);
    tree.border.set_effect(None);
    assert_eq!(
        *events.borrow(),
        ["effect attached on border", "effect bounds on border", "effect detached on border"]
    );
    assert_eq!(handlers, Interactive::total_handlers_count());

    // A custom effect value is no interactive effect: nothing is attached.
    tree.border.set_effect(Some(Rc::new(ImmutableCustomEffect::new(Probe { padding: 1.0 }))));
    assert!(tree.border.interactive_effect().is_none());
    assert_eq!(handlers, Interactive::total_handlers_count());
}

#[test]
fn pointer_input_reaches_the_effect_in_the_order_of_the_routed_events() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let events = log();
    let both = RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE;
    let route = |e: &PointerPressedEventArgs| if e.route() == RoutingStrategies::TUNNEL { "tunnel" } else { "bubble" };

    let recorded = events.clone();
    tree.root.root.add_handler_with(
        InputElement::pointer_pressed_event(),
        move |s, e| recorded.borrow_mut().push(format!("{} {}", label_of(s), route(e))),
        both,
        false,
    );
    let recorded = events.clone();
    tree.border.add_handler_with(
        InputElement::pointer_pressed_event(),
        move |s, e| recorded.borrow_mut().push(format!("{} {}", label_of(s), route(e))),
        RoutingStrategies::TUNNEL,
        false,
    );
    let recorded = events.clone();
    tree.decorator.pointer_pressed(move |s, e| recorded.borrow_mut().push(format!("{} {}", label_of(s), route(e))));

    let effect = LoggingEffect::new(&events, 0.0);
    set_effect(&tree.border, &effect);
    // A handler of the host added after the effect runs after it.
    let recorded = events.clone();
    tree.border.pointer_pressed(move |s, e| recorded.borrow_mut().push(format!("{} {}", label_of(s), route(e))));

    // Entering moves the pointer over the host: entered, then moved.
    events.borrow_mut().clear();
    tree.root.host.mouse_move(&device, OVER_DECORATOR);
    assert_eq!(*events.borrow(), ["effect entered on border", "effect moved on border"]);

    events.borrow_mut().clear();
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_DECORATOR, 0);
    assert_eq!(
        *events.borrow(),
        [
            "root tunnel",
            "border tunnel",
            "decorator bubble",
            "effect pressed on border",
            "border bubble",
            "root bubble",
        ]
    );

    // The release, the wheel and leaving.
    events.borrow_mut().clear();
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonUp, OVER_DECORATOR, 1);
    let wheel = Rc::new(RawMouseWheelEventArgs::new(
        device.clone(),
        2,
        tree.root.host.as_input_root(),
        OVER_DECORATOR,
        Vector::new(0.0, 1.0),
        RawInputModifiers::NONE,
    ));
    tree.root.host.input(wheel);
    tree.root.host.mouse_move(&device, OVER_CANVAS);
    let effect_entries: Vec<String> = events.borrow().iter().filter(|e| e.starts_with("effect ")).cloned().collect();
    assert_eq!(
        effect_entries,
        [
            "effect released on border",
            "effect capture lost on border",
            "effect wheel 1 on border",
            "effect exited on border",
        ]
    );
}

#[test]
fn a_handled_event_reaches_only_an_effect_that_asks_for_handled_events() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let events = log();
    tree.decorator.pointer_pressed(|_, e| e.set_handled(true));
    let effect = LoggingEffect::new(&events, 0.0);
    set_effect(&tree.border, &effect);

    let click = || {
        tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_DECORATOR, 0);
        tree.root.host.pointer(&device, RawPointerEventType::LeftButtonUp, OVER_DECORATOR, 1);
    };
    click();
    let presses = || events.borrow().iter().filter(|entry| entry.contains("pressed")).count();
    assert_eq!(0, presses(), "{:?}", events.borrow());
    // The release is not handled: it is delivered.
    assert!(events.borrow().iter().any(|entry| entry == "effect released on border"));

    effect.handled_too.set(true);
    click();
    assert_eq!(1, presses(), "{:?}", events.borrow());
}

#[test]
fn a_captured_pointer_reaches_the_effect_outside_its_host_and_the_loss_of_the_capture_too() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let events = log();
    let effect = LoggingEffect::new(&events, 0.0);
    effect.capture_on_press.set(true);
    set_effect(&tree.border, &effect);
    let border: Ref<InputElement> = tree.border.clone().upcast();

    tree.root.host.mouse_move(&device, OVER_DECORATOR);
    events.borrow_mut().clear();
    // The press over the decorator: the effect captures the pointer to its host, which takes it
    // from the decorator (the effect does not lose anything).
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_DECORATOR, 0);
    assert_eq!(Some(border.clone()), device.pointer().captured());
    // Moved over the canvas, outside the host: still the host's.
    tree.root.host.mouse_move(&device, OVER_CANVAS);
    let (position, normalized, _, _) = effect.last_position();
    assert_eq!(Point::new(-50.0, 50.0), position);
    assert_eq!(Point::new(-0.5, 0.25), normalized);
    // Released there: the effect releases the capture, and hears of its loss.
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonUp, OVER_CANVAS, 1);
    assert!(device.pointer().captured().is_none());
    assert_eq!(
        effect_events(&events),
        [
            "effect pressed on border",
            "effect moved on border",
            "effect released on border",
            "effect capture lost on border",
        ]
    );

    // Another element taking the capture: the effect loses it, and the release goes to that
    // element.
    events.borrow_mut().clear();
    tree.root.host.mouse_move(&device, OVER_DECORATOR);
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_DECORATOR, 2);
    assert_eq!(Some(border), device.pointer().captured());
    device.pointer().capture(Some(&tree.canvas.clone().upcast()));
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonUp, OVER_CANVAS, 3);
    let moves_and_presses: Vec<String> =
        effect_events(&events).into_iter().filter(|entry| !entry.contains("moved")).collect();
    assert_eq!(moves_and_presses, ["effect pressed on border", "effect capture lost on border"]);
}

#[test]
fn positions_are_those_of_the_host_normalized_and_in_the_coordinates_of_the_program() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let events = log();
    let effect = LoggingEffect::new(&events, 4.0);
    set_effect(&tree.border, &effect);

    // The border is at (100, 0) and 100 x 200; the area of the effect reaches 4 beyond it.
    tree.root.host.mouse_move(&device, OVER_DECORATOR);
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_DECORATOR, 0);
    assert_eq!((Point::new(50.0, 50.0), Point::new(0.5, 0.25), Point::new(54.0, 54.0), 1.0), effect.last_position());
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonUp, OVER_DECORATOR, 1);

    // Scaled by two from its top left corner: the pointer is half as far into the host, and the
    // program counts device pixels of the scaled layer.
    tree.border.set_render_transform_origin(RelativePoint::TOP_LEFT);
    tree.border.set_render_transform(Some(Rc::new(crate::media::immutable::ImmutableTransform::new(
        Matrix::create_scale(2.0, 2.0),
    ))));
    tree.root.host.mouse_move(&device, OVER_DECORATOR);
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_DECORATOR, 2);
    assert_eq!((Point::new(25.0, 25.0), Point::new(0.25, 0.125), Point::new(58.0, 58.0), 2.0), effect.last_position());
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonUp, OVER_DECORATOR, 3);

    // The context directly: normalizing clamps when asked, an empty host counts as one pixel.
    let attached = tree.border.interactive_effect().expect("attached");
    let context = EffectHostContext::new(&attached, &tree.border);
    assert_eq!(Point::new(1.0, 0.0), context.normalize(Point::new(150.0, -10.0), true));
    assert_eq!(Point::new(1.5, -0.05), context.normalize(Point::new(150.0, -10.0), false));
    assert_eq!(Rect::new(-4.0, -4.0, 108.0, 208.0), context.effect_bounds());
    assert_eq!(Point::new(8.0, 8.0), context.to_effect_space(Point::new(0.0, 0.0)));
    let empty = TestControl::new("empty");
    let context = EffectHostContext::new(&attached, &empty);
    assert_eq!(Point::new(3.0, 4.0), context.normalize(Point::new(3.0, 4.0), false));
    assert_eq!(1.0, context.layer_scale(), "not in a visual tree");
}

#[test]
fn the_pointer_spotlight_follows_the_pointer_and_captures_it_while_pressed() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let spotlight = InteractivePointerSpotlightEffect::new();
    tree.border.set_effect(Some(spotlight.clone().into()));
    let border: Ref<InputElement> = tree.border.clone().upcast();

    tree.root.host.mouse_move(&device, Point::new(125.0, 150.0));
    let value = spotlight.value();
    assert!(value.is_pointer_over && !value.is_pressed);
    assert_eq!((0.25, 0.75), (value.pointer_x, value.pointer_y));

    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_DECORATOR, 0);
    assert!(spotlight.value().is_pressed);
    assert_eq!(Some(border), device.pointer().captured());
    // Dragged out of the host, the position is clamped to its side.
    tree.root.host.mouse_move(&device, OVER_CANVAS);
    assert_eq!(0.0, spotlight.value().pointer_x);
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonUp, OVER_CANVAS, 1);
    assert!(!spotlight.value().is_pressed);
    assert!(device.pointer().captured().is_none());

    tree.root.host.pointer(&device, RawPointerEventType::LeaveWindow, Point::new(-1.0, -1.0), 2);
    assert!(!spotlight.value().is_pointer_over);
}

#[test]
fn the_burn_away_button_takes_the_press_its_button_handled() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();
    let device = MouseDevice::new();
    let tree = pointer_tree();
    // The button handles its press, as a button does.
    tree.border.pointer_pressed(|_, e| e.set_handled(true));
    tree.canvas.pointer_pressed(|_, e| e.set_handled(true));
    let flame = InteractiveBurningFlameEffect::new();
    tree.border.set_effect(Some(flame.clone().into()));
    let spotlight = InteractivePointerSpotlightEffect::new();
    tree.canvas.set_effect(Some(spotlight.clone().into()));

    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, Point::new(150.0, 100.0), 0);
    let value = flame.value();
    assert!(value.is_pressed);
    assert_eq!(1.0, value.burn_amount);
    assert_eq!((0.5, 0.5), (value.ignition_x, value.ignition_y));
    assert!(flame.is_animating(), "the flame burns on the frames of the animation clock");
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonUp, Point::new(150.0, 100.0), 1);
    assert!(!flame.value().is_pressed);

    // The spotlight takes handled presses too (#580): every input effect of the library does, as
    // Effector's use `handledEventsToo`.
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_CANVAS, 2);
    assert!(spotlight.value().is_pressed);
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonUp, OVER_CANVAS, 3);
    assert!(!spotlight.value().is_pressed);

    // Detached, the flame stops.
    tree.border.set_effect(None);
    assert!(!flame.is_animating());
}

#[test]
fn the_water_ripple_makes_ripples_where_the_pointer_is_pressed_dragged_and_released() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let ripple = InteractiveWaterRippleEffect::new();
    tree.border.set_effect(Some(ripple.clone().into()));

    tree.root.host.mouse_move(&device, Point::new(150.0, 100.0));
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, Point::new(150.0, 100.0), 0);
    let value = ripple.value();
    assert!(value.is_pressed);
    assert_eq!(Point::new(0.5, 0.5), value.ripples[0].center);
    assert_eq!(1.0, value.ripples[0].strength);
    assert!(ripple.is_animating());

    // A short drag makes no ripple, a longer one does.
    tree.root.host.mouse_move(&device, Point::new(152.0, 100.0));
    assert_eq!(Point::new(0.5, 0.5), ripple.value().ripples[0].center);
    tree.root.host.mouse_move(&device, Point::new(170.0, 100.0));
    let value = ripple.value();
    assert_eq!(Point::new(0.7, 0.5), value.ripples[0].center);
    assert_eq!(0.45, value.ripples[0].strength);
    assert_eq!(Point::new(0.5, 0.5), value.ripples[1].center);

    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonUp, Point::new(170.0, 100.0), 1);
    let value = ripple.value();
    assert!(!value.is_pressed);
    assert_eq!(0.55, value.ripples[0].strength);
    assert_eq!(3, value.ripples.iter().filter(|ripple| ripple.is_active()).count());

    // Detached, it stops animating; attached again, it starts without ripples.
    tree.border.set_effect(None);
    assert!(!ripple.is_animating());
    tree.border.set_effect(Some(ripple.clone().into()));
    assert!(!ripple.value().has_active_ripples());
}
