//! Tests for routed events and [`Interactive`].

use super::*;
use crate::input::{InputElement, KeyEventArgs, PointerEventArgs, PointerPressedEventArgs, TextInputEventArgs};
use crate::layout::LayoutableImpl;
use crate::reactive::ObservableExt;
use crate::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
struct TestInteractive {
    base: Interactive,
    label: &'static str,
    class_handler_invoked: Cell<bool>,
}

ferro_class!(TestInteractive: Interactive);
ferro_impl_classes!(
    TestInteractive: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl
);

impl TestInteractive {
    fn new(label: &'static str) -> Ref<Self> {
        instantiate(Self { base: Interactive::construct(), label, class_handler_invoked: Cell::new(false) })
    }

    fn with_children(label: &'static str, children: Vec<Ref<TestInteractive>>) -> Ref<Self> {
        let result = Self::new(label);
        result.set_children(children);
        result
    }

    fn set_children(&self, children: Vec<Ref<TestInteractive>>) {
        self.visual_children().clear();
        self.visual_children().add_range(children.into_iter().map(Ref::upcast::<Visual>));
    }

    fn class_handler(&self, _e: &RoutedEventArgs) {
        self.class_handler_invoked.set(true);
    }

    fn mark_event_as_handled(&self, e: &RoutedEventArgs) {
        e.set_handled(true);
    }
}

fn label_of(sender: &Interactive) -> &'static str {
    sender.downcast_ref::<TestInteractive>().expect("sender is a TestInteractive").label
}

type Handler = Rc<dyn Fn(&Interactive, &RoutedEventArgs)>;

fn untyped_event(strategies: RoutingStrategies) -> RoutedEvent {
    RoutedEvent::new_untyped::<RoutedEventArgs>("test", strategies, TestInteractive::TYPE)
}

/// Keeps the root of a test tree alive: parents are only weakly referenced
/// by their children.
struct Tree {
    _root: Ref<TestInteractive>,
    target: Ref<TestInteractive>,
}

impl std::ops::Deref for Tree {
    type Target = Ref<TestInteractive>;

    fn deref(&self) -> &Ref<TestInteractive> {
        &self.target
    }
}

/// Builds the tree `1 -> (2a, 2b -> 3)`, attaches `handler` to every
/// element and returns `2b`.
fn create_tree(
    ev: RoutedEvent,
    handler: Option<Handler>,
    handler_routes: RoutingStrategies,
    handled_events_too: bool,
) -> Tree {
    let target = TestInteractive::with_children("2b", vec![TestInteractive::new("3")]);
    let tree = TestInteractive::with_children("1", vec![TestInteractive::new("2a"), target.clone()]);

    if let Some(handler) = handler {
        for visual in tree.get_self_and_visual_descendants() {
            let i = visual.cast::<Interactive>().unwrap();
            let handler = handler.clone();
            i.add_handler_as::<RoutedEventArgs, _>(&ev, move |s, e| handler(s, e), handler_routes, handled_events_too);
        }
    }

    Tree { _root: tree, target }
}

fn recording_handler(invoked: &Rc<RefCell<Vec<&'static str>>>) -> Handler {
    let invoked = invoked.clone();
    Rc::new(move |s, _| invoked.borrow_mut().push(label_of(s)))
}

const BUBBLE_TUNNEL: RoutingStrategies = RoutingStrategies::BUBBLE.union(RoutingStrategies::TUNNEL);

#[test]
fn direct_event_should_go_straight_to_source() {
    let ev = untyped_event(RoutingStrategies::DIRECT);
    let invoked = Rc::new(RefCell::new(Vec::new()));
    let target = create_tree(ev, Some(recording_handler(&invoked)), RoutingStrategies::DIRECT, false);

    let args = RoutedEventArgs::with_event_and_source(&ev, &*target);
    target.raise_event(&args);

    assert_eq!(*invoked.borrow(), ["2b"]);
}

#[test]
fn direct_event_should_have_route_set_to_direct() {
    let ev = untyped_event(RoutingStrategies::DIRECT);
    let called = Rc::new(Cell::new(false));
    let handler: Handler = {
        let called = called.clone();
        Rc::new(move |_, e| {
            assert_eq!(e.route(), RoutingStrategies::DIRECT);
            called.set(true);
        })
    };
    let target = create_tree(ev, Some(handler), RoutingStrategies::DIRECT, false);

    let args = RoutedEventArgs::with_event_and_source(&ev, &*target);
    target.raise_event(&args);

    assert!(called.get());
}

#[test]
fn bubbling_event_should_bubble_up() {
    let ev = untyped_event(RoutingStrategies::BUBBLE);
    let invoked = Rc::new(RefCell::new(Vec::new()));
    let target = create_tree(ev, Some(recording_handler(&invoked)), BUBBLE_TUNNEL, false);

    let args = RoutedEventArgs::with_event_and_source(&ev, &*target);
    target.raise_event(&args);

    assert_eq!(*invoked.borrow(), ["2b", "1"]);
}

#[test]
fn tunneling_event_should_tunnel() {
    let ev = untyped_event(RoutingStrategies::TUNNEL);
    let invoked = Rc::new(RefCell::new(Vec::new()));
    let target = create_tree(ev, Some(recording_handler(&invoked)), BUBBLE_TUNNEL, false);

    let args = RoutedEventArgs::with_event_and_source(&ev, &*target);
    target.raise_event(&args);

    assert_eq!(*invoked.borrow(), ["1", "2b"]);
}

#[test]
fn tunneling_bubbling_event_should_tunnel_then_bubble_up() {
    let ev = untyped_event(BUBBLE_TUNNEL);
    let invoked = Rc::new(RefCell::new(Vec::new()));
    let target = create_tree(ev, Some(recording_handler(&invoked)), BUBBLE_TUNNEL, false);

    let args = RoutedEventArgs::with_event_and_source(&ev, &*target);
    target.raise_event(&args);

    assert_eq!(*invoked.borrow(), ["1", "2b", "2b", "1"]);
}

#[test]
fn events_should_have_route_set() {
    let ev = untyped_event(BUBBLE_TUNNEL);
    let invoked = Rc::new(RefCell::new(Vec::new()));
    let handler: Handler = {
        let invoked = invoked.clone();
        Rc::new(move |_, e| invoked.borrow_mut().push(e.route()))
    };
    let target = create_tree(ev, Some(handler), BUBBLE_TUNNEL, false);

    let args = RoutedEventArgs::with_event_and_source(&ev, &*target);
    target.raise_event(&args);

    assert_eq!(
        *invoked.borrow(),
        [RoutingStrategies::TUNNEL, RoutingStrategies::TUNNEL, RoutingStrategies::BUBBLE, RoutingStrategies::BUBBLE]
    );
}

#[test]
fn handled_bubbled_event_should_not_propogate_further() {
    let ev = untyped_event(RoutingStrategies::BUBBLE);
    let invoked = Rc::new(RefCell::new(Vec::new()));
    let handler: Handler = {
        let invoked = invoked.clone();
        Rc::new(move |s, e| {
            let label = label_of(s);
            invoked.borrow_mut().push(label);
            e.set_handled(label == "2b");
        })
    };
    let target = create_tree(ev, Some(handler), RoutingStrategies::BUBBLE, false);

    let args = RoutedEventArgs::with_event_and_source(&ev, &*target);
    target.raise_event(&args);

    assert_eq!(*invoked.borrow(), ["2b"]);
}

#[test]
fn handled_tunnelled_event_should_not_propogate_further() {
    let ev = untyped_event(BUBBLE_TUNNEL);
    let invoked = Rc::new(RefCell::new(Vec::new()));
    let handler: Handler = {
        let invoked = invoked.clone();
        Rc::new(move |s, e| {
            let label = label_of(s);
            invoked.borrow_mut().push(label);
            e.set_handled(label == "2b");
        })
    };
    let target = create_tree(ev, Some(handler), BUBBLE_TUNNEL, false);

    let args = RoutedEventArgs::with_event_and_source(&ev, &*target);
    target.raise_event(&args);

    assert_eq!(*invoked.borrow(), ["1", "2b"]);
}

#[test]
fn direct_subscription_should_not_catch_tunneling_or_bubbling() {
    let ev = untyped_event(BUBBLE_TUNNEL);
    let count = Rc::new(Cell::new(0));
    let handler: Handler = {
        let count = count.clone();
        Rc::new(move |_, _| count.set(count.get() + 1))
    };
    let target = create_tree(ev, Some(handler), RoutingStrategies::DIRECT, false);

    let args = RoutedEventArgs::with_event_and_source(&ev, &*target);
    target.raise_event(&args);

    assert_eq!(count.get(), 0);
}

#[test]
fn bubbling_subscription_should_not_catch_tunneling() {
    let ev = untyped_event(BUBBLE_TUNNEL);
    let count = Rc::new(Cell::new(0));
    let handler: Handler = {
        let count = count.clone();
        Rc::new(move |_, e| {
            assert_eq!(e.route(), RoutingStrategies::BUBBLE);
            count.set(count.get() + 1);
        })
    };
    let target = create_tree(ev, Some(handler), RoutingStrategies::BUBBLE, false);

    let args = RoutedEventArgs::with_event_and_source(&ev, &*target);
    target.raise_event(&args);

    assert_eq!(count.get(), 2);
}

#[test]
fn tunneling_subscription_should_not_catch_bubbling() {
    let ev = untyped_event(BUBBLE_TUNNEL);
    let count = Rc::new(Cell::new(0));
    let handler: Handler = {
        let count = count.clone();
        Rc::new(move |_, e| {
            assert_eq!(e.route(), RoutingStrategies::TUNNEL);
            count.set(count.get() + 1);
        })
    };
    let target = create_tree(ev, Some(handler), RoutingStrategies::TUNNEL, false);

    let args = RoutedEventArgs::with_event_and_source(&ev, &*target);
    target.raise_event(&args);

    assert_eq!(count.get(), 2);
}

#[test]
fn event_should_should_keep_propogating_to_handed_events_too_handlers() {
    let ev = untyped_event(BUBBLE_TUNNEL);
    let invoked = Rc::new(RefCell::new(Vec::new()));
    let handler: Handler = {
        let invoked = invoked.clone();
        Rc::new(move |s, e| {
            invoked.borrow_mut().push(label_of(s));
            e.set_handled(true);
        })
    };
    let target = create_tree(ev, Some(handler), BUBBLE_TUNNEL, true);

    let args = RoutedEventArgs::with_event_and_source(&ev, &*target);
    target.raise_event(&args);

    assert_eq!(*invoked.borrow(), ["1", "2b", "2b", "1"]);
}

fn class_handler_test(
    event_strategies: RoutingStrategies,
    handler_routes: RoutingStrategies,
) -> Vec<&'static str> {
    let ev = untyped_event(event_strategies);
    let invoked = Rc::new(RefCell::new(Vec::new()));
    let target = create_tree(ev, None, RoutingStrategies::empty(), false);

    let recorded = invoked.clone();
    let subscription = ev.add_class_handler_untyped(
        TestInteractive::TYPE,
        move |s, _| recorded.borrow_mut().push(label_of(s)),
        handler_routes,
        false,
    );

    let args = RoutedEventArgs::with_event_and_source(&ev, &*target);
    target.raise_event(&args);
    subscription.dispose();

    let result = invoked.borrow().clone();
    result
}

#[test]
fn direct_class_handlers_should_be_called() {
    assert_eq!(class_handler_test(RoutingStrategies::DIRECT, RoutingStrategies::DIRECT), ["2b"]);
}

#[test]
fn tunneling_class_handlers_should_be_called() {
    assert_eq!(class_handler_test(BUBBLE_TUNNEL, RoutingStrategies::TUNNEL), ["1", "2b"]);
}

#[test]
fn bubbling_class_handlers_should_be_called() {
    assert_eq!(class_handler_test(BUBBLE_TUNNEL, RoutingStrategies::BUBBLE), ["2b", "1"]);
}

#[test]
fn typed_class_handlers_should_be_called() {
    let ev = RoutedEvent::<RoutedEventArgs>::new("test", BUBBLE_TUNNEL, TestInteractive::TYPE);
    let target = create_tree(ev.as_routed_event(), None, RoutingStrategies::empty(), false);

    ev.add_class_handler_with::<TestInteractive>(|x, e| x.class_handler(e), RoutingStrategies::BUBBLE, false);

    let args = RoutedEventArgs::with_event_and_source(&ev, &*target);
    target.raise_event(&args);

    assert!(target.class_handler_invoked.get());
    let interactive = target.get_visual_parent_of_type::<TestInteractive>().unwrap();
    assert!(interactive.class_handler_invoked.get());
}

#[test]
fn typed_class_handlers_should_be_called_for_handled_events() {
    let ev = RoutedEvent::<RoutedEventArgs>::new("test", BUBBLE_TUNNEL, TestInteractive::TYPE);
    let target = create_tree(ev.as_routed_event(), None, RoutingStrategies::empty(), false);

    ev.add_class_handler_with::<TestInteractive>(|x, e| x.mark_event_as_handled(e), RoutingStrategies::BUBBLE, false);
    ev.add_class_handler_with::<TestInteractive>(|x, e| x.class_handler(e), RoutingStrategies::BUBBLE, true);

    let args = RoutedEventArgs::with_event_and_source(&ev, &*target);
    target.raise_event(&args);

    assert!(args.handled());
    assert!(target.class_handler_invoked.get());
    let interactive = target.get_visual_parent_of_type::<TestInteractive>().unwrap();
    assert!(interactive.class_handler_invoked.get());
}

#[test]
fn get_observable_should_listen_to_event() {
    let ev = RoutedEvent::<RoutedEventArgs>::new("test", RoutingStrategies::DIRECT, TestInteractive::TYPE);
    let target = TestInteractive::new("target");
    let called = Rc::new(Cell::new(0));
    let counter = called.clone();
    let subscription = target.get_observable(&ev).subscribe_fn(move |_| counter.set(counter.get() + 1));

    let args = RoutedEventArgs::with_event_and_source(&ev, &target);
    target.raise_event(&args);
    subscription.dispose();
    target.raise_event(&args);

    assert_eq!(called.get(), 1);
}

#[test]
fn removing_control_in_handler_should_not_stop_event() {
    let ev = untyped_event(RoutingStrategies::BUBBLE);
    let invoked = Rc::new(RefCell::new(Vec::new()));
    let parent = create_tree(ev, Some(recording_handler(&invoked)), BUBBLE_TUNNEL, false);
    let target = parent.get_visual_children()[0].cast::<Interactive>().unwrap();

    let removed_from = parent.target.clone();
    target.add_handler_as::<RoutedEventArgs, _>(
        &ev,
        move |_, _| removed_from.set_children(Vec::new()),
        Interactive::DEFAULT_ROUTES,
        false,
    );

    let args = RoutedEventArgs::with_event_and_source(&ev, &target);
    target.raise_event(&args);

    assert_eq!(*invoked.borrow(), ["3", "2b", "1"]);
}

// --- additional tests for the port ------------------------------------------

#[test]
fn remove_handler_removes_only_that_handler() {
    let ev = RoutedEvent::<RoutedEventArgs>::new("test", RoutingStrategies::BUBBLE, TestInteractive::TYPE);
    let target = TestInteractive::new("target");
    let count = Rc::new(Cell::new(0));
    let before = Interactive::total_handlers_count();

    let first = {
        let count = count.clone();
        target.add_handler(&ev, move |_, _| count.set(count.get() + 1))
    };
    let second = {
        let count = count.clone();
        target.add_handler(&ev, move |_, _| count.set(count.get() + 10))
    };
    assert_eq!(Interactive::total_handlers_count(), before + 2);

    target.raise_event(&RoutedEventArgs::with_event(&ev));
    assert_eq!(count.get(), 11);

    assert!(target.remove_handler(&ev, first));
    assert!(!target.remove_handler(&ev, first));
    assert_eq!(Interactive::total_handlers_count(), before + 1);

    target.raise_event(&RoutedEventArgs::with_event(&ev));
    assert_eq!(count.get(), 21);

    assert!(target.remove_handler(&ev, second));
    target.raise_event(&RoutedEventArgs::with_event(&ev));
    assert_eq!(count.get(), 21);
}

#[test]
fn handlers_can_add_and_remove_handlers_during_raise() {
    let ev = RoutedEvent::<RoutedEventArgs>::new("test", RoutingStrategies::BUBBLE, TestInteractive::TYPE);
    let target = TestInteractive::new("target");
    let log = Rc::new(RefCell::new(Vec::new()));

    let token = Rc::new(Cell::new(None));
    {
        let log = log.clone();
        let token_cell = token.clone();
        let target_weak = target.downgrade();
        let ev_copy = ev;
        let added = target.add_handler(&ev, move |_, _| {
            log.borrow_mut().push("first");
            let target = target_weak.upgrade().unwrap();
            // Remove ourselves and add another handler: neither affects the
            // raise in progress.
            target.remove_handler(&ev_copy, token_cell.get().unwrap());
            let log = log.clone();
            target.add_handler(&ev_copy, move |_, _| log.borrow_mut().push("added"));
        });
        token.set(Some(added));
    }
    {
        let log = log.clone();
        target.add_handler(&ev, move |_, _| log.borrow_mut().push("second"));
    }

    target.raise_event(&RoutedEventArgs::with_event(&ev));
    assert_eq!(*log.borrow(), ["first", "second"]);

    log.borrow_mut().clear();
    target.raise_event(&RoutedEventArgs::with_event(&ev));
    assert_eq!(*log.borrow(), ["second", "added"]);
}

#[test]
fn untyped_handler_can_be_attached_to_event_with_derived_args() {
    let ev = RoutedEvent::<TextInputEventArgs>::new("test", RoutingStrategies::BUBBLE, TestInteractive::TYPE);
    let target = TestInteractive::new("target");
    let base_called = Rc::new(Cell::new(false));
    let typed_text = Rc::new(RefCell::new(None));

    {
        let base_called = base_called.clone();
        target.add_handler_as::<RoutedEventArgs, _>(
            &ev,
            move |_, e: &RoutedEventArgs| base_called.set(e.routed_event().is_some()),
            Interactive::DEFAULT_ROUTES,
            false,
        );
    }
    {
        let typed_text = typed_text.clone();
        target.add_handler(&ev, move |_, e| *typed_text.borrow_mut() = e.text.clone());
    }

    let mut args = TextInputEventArgs::new();
    args.text = Some("hello".to_string());
    args.set_routed_event(Some(&ev));
    target.raise_event(&args);

    assert!(base_called.get());
    assert_eq!(typed_text.borrow().as_deref(), Some("hello"));
}

#[test]
#[should_panic(expected = "cannot be attached")]
fn handler_with_unrelated_args_cannot_be_attached() {
    let ev = RoutedEvent::<TextInputEventArgs>::new("test", RoutingStrategies::BUBBLE, TestInteractive::TYPE);
    let target = TestInteractive::new("target");
    target.add_handler_as::<KeyEventArgs, _>(&ev, |_, _| {}, Interactive::DEFAULT_ROUTES, false);
}

#[test]
fn args_hierarchy_supports_downcasting_to_every_base() {
    assert!(InputElement::pointer_pressed_event().event_args_is::<PointerPressedEventArgs>());
    assert!(InputElement::pointer_pressed_event().event_args_is::<PointerEventArgs>());
    assert!(InputElement::pointer_pressed_event().event_args_is::<RoutedEventArgs>());
    assert!(!InputElement::pointer_moved_event().event_args_is::<PointerPressedEventArgs>());
    assert!(!InputElement::pointer_pressed_event().event_args_is::<KeyEventArgs>());
}

#[test]
#[should_panic(expected = "Cannot raise an event whose RoutedEvent is null.")]
fn raising_args_without_event_panics() {
    let target = TestInteractive::new("target");
    target.raise_event(&RoutedEventArgs::new());
}

#[test]
fn route_finished_is_raised_per_route() {
    let ev = RoutedEvent::<RoutedEventArgs>::new("test", BUBBLE_TUNNEL, TestInteractive::TYPE);
    let target = TestInteractive::new("target");
    let routes = Rc::new(RefCell::new(Vec::new()));
    let recorded = routes.clone();
    let subscription = ev.route_finished().subscribe(move |e| recorded.borrow_mut().push(e.route()));

    target.raise_event(&RoutedEventArgs::with_event(&ev));
    subscription.dispose();
    target.raise_event(&RoutedEventArgs::with_event(&ev));

    assert_eq!(*routes.borrow(), [RoutingStrategies::TUNNEL, RoutingStrategies::BUBBLE]);
}

#[test]
fn routed_event_display_and_equality() {
    let ev = RoutedEvent::<RoutedEventArgs>::new("Click", RoutingStrategies::BUBBLE, TestInteractive::TYPE);
    assert_eq!(ev.to_string(), "TestInteractive.Click");
    assert_eq!(ev.as_routed_event(), ev);
    assert_eq!(ev.name(), "Click");
    assert!(std::ptr::eq(ev.owner_type(), TestInteractive::TYPE));

    let other = RoutedEvent::<RoutedEventArgs>::new("Click", RoutingStrategies::BUBBLE, TestInteractive::TYPE);
    assert_ne!(ev, other);
}

#[test]
fn source_is_set_to_raising_element() {
    let ev = RoutedEvent::<RoutedEventArgs>::new("test", RoutingStrategies::BUBBLE, TestInteractive::TYPE);
    let child = TestInteractive::new("child");
    let parent = TestInteractive::with_children("parent", vec![child.clone()]);
    let seen = Rc::new(Cell::new(false));

    {
        let seen = seen.clone();
        let child = child.clone();
        parent.add_handler(&ev, move |sender, e| {
            assert_eq!(label_of(sender), "parent");
            seen.set(e.source().is_some_and(|source| source == child) && e.is_source(&child));
        });
    }

    child.raise_event(&RoutedEventArgs::with_event(&ev));
    assert!(seen.get());
}

// --- routed event registry ----------------------------------------------------

struct Owner1;

impl StaticType for Owner1 {
    const TYPE: &'static TypeInfo = {
        static TYPE: TypeInfo = TypeInfo::new("Owner1", None);
        &TYPE
    };
}

#[test]
fn pointer_events_should_be_registered() {
    // Routed events are registered on first use.
    let expected = [
        InputElement::pointer_pressed_event().as_routed_event(),
        InputElement::pointer_released_event().as_routed_event(),
    ];
    let registered = RoutedEventRegistry::instance().get_registered_for::<InputElement>();

    assert!(expected.iter().all(|e| registered.contains(e)));
    assert!(expected.iter().all(|e| RoutedEventRegistry::instance().get_all_registered().contains(e)));
}

#[test]
fn events_should_be_registered_on_their_owner_only() {
    let ev = RoutedEvent::register::<Owner1, RoutedEventArgs>("Click", RoutingStrategies::BUBBLE);
    let registry = RoutedEventRegistry::instance();

    assert!(registry.get_registered_for::<Owner1>().contains(&ev.as_routed_event()));
    assert!(!registry.get_registered_for::<InputElement>().contains(&ev.as_routed_event()));
    assert!(!registry
        .get_registered_for::<Owner1>()
        .contains(&InputElement::pointer_pressed_event().as_routed_event()));
    assert!(registry.get_registered(TestInteractive::TYPE).is_empty());
}

// --- untyped handlers ---------------------------------------------------------

/// The args handle an untyped handler was given.
fn untyped_args(arguments: &[crate::metadata::MarkupValue]) -> Rc<dyn IRoutedEventArgs> {
    let args = arguments[1].as_ref().expect("the args");
    args.downcast_ref::<Rc<dyn IRoutedEventArgs>>().expect("an args handle").clone()
}

#[test]
fn untyped_handler_receives_the_sender_and_the_live_args() {
    let ev = RoutedEvent::<RoutedEventArgs>::new("test", RoutingStrategies::BUBBLE, TestInteractive::TYPE);
    let child = TestInteractive::new("child");
    let parent = TestInteractive::with_children("parent", vec![child.clone()]);

    // The untyped handler on the source marks the event as handled.
    let senders = Rc::new(RefCell::new(Vec::new()));
    let recorded = senders.clone();
    let handler = crate::metadata::MarkupDelegate::new(move |arguments| {
        assert_eq!(arguments.len(), 2);
        let sender = arguments[0].as_ref().expect("the sender");
        let sender = sender.downcast_ref::<Ref<Interactive>>().expect("an interactive");
        recorded.borrow_mut().push(label_of(sender));
        untyped_args(arguments).set_handled(true);
        None
    });
    child.add_handler_untyped(&ev, handler, RoutingStrategies::BUBBLE, false);

    // Typed handlers further along the route observe it.
    let unhandled_only = Rc::new(Cell::new(0));
    let count = unhandled_only.clone();
    parent.add_handler(&ev, move |_, _| count.set(count.get() + 1));
    let observed = Rc::new(Cell::new(None));
    let seen = observed.clone();
    parent.add_handler_with(&ev, move |_, e: &RoutedEventArgs| seen.set(Some(e.handled())), RoutingStrategies::BUBBLE, true);

    let args = RoutedEventArgs::with_event(&ev);
    child.raise_event(&args);

    assert_eq!(*senders.borrow(), vec!["child"]);
    assert!(args.handled());
    assert_eq!(unhandled_only.get(), 0);
    assert_eq!(observed.get(), Some(true));
}

#[test]
fn untyped_handler_is_removed_by_its_token() {
    let ev = RoutedEvent::<RoutedEventArgs>::new("test", RoutingStrategies::BUBBLE, TestInteractive::TYPE);
    let target = TestInteractive::new("target");
    let count = Rc::new(Cell::new(0));
    let before = Interactive::total_handlers_count();

    let calls = count.clone();
    let token = target.add_handler_untyped(
        &ev,
        crate::metadata::MarkupDelegate::new(move |_| {
            calls.set(calls.get() + 1);
            None
        }),
        RoutingStrategies::BUBBLE,
        false,
    );
    assert_eq!(Interactive::total_handlers_count(), before + 1);

    target.raise_event(&RoutedEventArgs::with_event(&ev));
    assert_eq!(count.get(), 1);

    assert!(target.remove_handler(&ev, token));
    assert!(!target.remove_handler(&ev, token));
    assert_eq!(Interactive::total_handlers_count(), before);

    target.raise_event(&RoutedEventArgs::with_event(&ev));
    assert_eq!(count.get(), 1);
}

#[test]
fn untyped_handler_shares_the_state_of_derived_args() {
    let ev = RoutedEvent::<CancelRoutedEventArgs>::new("test", RoutingStrategies::BUBBLE, TestInteractive::TYPE);
    let target = TestInteractive::new("target");
    let handles = Rc::new(RefCell::new(Vec::new()));

    let kept = handles.clone();
    target.add_handler_untyped(
        &ev,
        crate::metadata::MarkupDelegate::new(move |arguments| {
            let args = untyped_args(arguments);
            args.downcast_ref::<CancelRoutedEventArgs>().expect("the args type of the event").set_cancel(true);
            kept.borrow_mut().push(args);
            None
        }),
        RoutingStrategies::BUBBLE,
        false,
    );
    let kept = handles.clone();
    target.add_handler_untyped(
        &ev,
        crate::metadata::MarkupDelegate::new(move |arguments| {
            kept.borrow_mut().push(untyped_args(arguments));
            None
        }),
        RoutingStrategies::BUBBLE,
        false,
    );

    let args = CancelRoutedEventArgs::with_event(&ev);
    target.raise_event(&args);

    assert!(args.cancel());
    // Every handle refers to the same args.
    let handles = handles.borrow();
    assert_eq!(handles.len(), 2);
    assert!(*handles[0] == *handles[1]);
    assert!(*handles[0] == *args.share());
    let other = CancelRoutedEventArgs::with_event(&ev);
    assert!(*handles[0] != *other.share());
}

#[test]
fn a_copy_of_routed_event_args_shares_the_routing_state() {
    let args = RoutedEventArgs::new();
    let copy = args.clone();
    copy.set_handled(true);
    copy.set_route(RoutingStrategies::TUNNEL);
    assert!(args.handled());
    assert_eq!(args.route(), RoutingStrategies::TUNNEL);
    assert!(args == copy);
    assert!(args != RoutedEventArgs::new());
}
