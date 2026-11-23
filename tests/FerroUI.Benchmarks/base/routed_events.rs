//! Routed events on a chain of elements (#1040): raising an event that nothing listens to, one
//! that a handler of every element sees, one that a class handler sees on every element, one
//! whose class handler is for another class and so filters every element out, and adding and
//! removing a handler.

use crate::harness::Registry;
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::Subscription;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StaticType, StyledElementImpl, Visual,
    VisualImpl,
};
use std::cell::Cell;
use std::hint::black_box;
use std::rc::Rc;

/// The elements of the chain.
const DEPTH: usize = 16;

#[repr(C)]
struct Node {
    base: Interactive,
}

ferro_class!(Node: Interactive);
ferro_impl_classes!(Node: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

/// A class no element of the chain is.
#[repr(C)]
struct OtherNode {
    base: Interactive,
}

ferro_class!(OtherNode: Interactive);
ferro_impl_classes!(OtherNode: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

fn node() -> Ref<Node> {
    instantiate(Node { base: Interactive::construct() })
}

pub struct RoutedEvents {
    target: Ref<Node>,
    event: RoutedEvent<RoutedEventArgs>,
    args: RoutedEventArgs,
    seen: Rc<Cell<u64>>,
    /// An element holds its parent weakly, so the chain is held here.
    chain: Vec<Ref<Node>>,
    _class_handler: Option<Subscription>,
}

impl RoutedEvents {
    fn new() -> Self {
        let chain: Vec<_> = (0..DEPTH).map(|_| node()).collect();
        for pair in chain.windows(2) {
            pair[0].visual_children().add(pair[1].clone().upcast::<Visual>());
        }
        let strategies = RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE;
        let event = RoutedEvent::<RoutedEventArgs>::new("BenchmarkEvent", strategies, Node::TYPE);
        let args = RoutedEventArgs::with_event(&event);
        let target = chain[DEPTH - 1].clone();
        Self { target, event, args, seen: Rc::new(Cell::new(0)), chain, _class_handler: None }
    }

    fn with_instance_handlers() -> Self {
        let result = Self::new();
        for element in &result.chain {
            let seen = result.seen.clone();
            element.add_handler(&result.event, move |_, _: &RoutedEventArgs| seen.set(seen.get() + 1));
        }
        result
    }

    fn with_class_handler() -> Self {
        let mut result = Self::new();
        let seen = result.seen.clone();
        result._class_handler = Some(
            result.event.add_class_handler::<Node>(move |_, _: &RoutedEventArgs| seen.set(seen.get() + 1)),
        );
        result
    }

    fn with_class_handler_of_another_class() -> Self {
        let mut result = Self::new();
        let seen = result.seen.clone();
        result._class_handler = Some(
            result.event.add_class_handler::<OtherNode>(move |_, _: &RoutedEventArgs| seen.set(seen.get() + 1)),
        );
        result
    }

    /// Raises the event on the last element, and returns what the handlers saw.
    fn raise(&self) -> u64 {
        black_box(&self.target).raise_event(black_box(&self.args));
        self.seen.get()
    }

    /// Adds a handler to the last element and removes it.
    fn add_and_remove(&self) -> bool {
        let token = self.target.add_handler(&self.event, |_, _: &RoutedEventArgs| {});
        self.target.remove_handler(&self.event, black_box(token))
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("base", "RoutedEvents");
    class.benchmark("raise_no_listeners", "", RoutedEvents::new, |b| b.raise());
    class.benchmark("raise_instance_handlers", "", RoutedEvents::with_instance_handlers, |b| b.raise());
    class.benchmark("raise_class_handler", "", RoutedEvents::with_class_handler, |b| b.raise());
    class.benchmark("raise_class_handler_filtered", "", RoutedEvents::with_class_handler_of_another_class, |b| b.raise());
    class.benchmark("add_remove_handler", "", RoutedEvents::new, |b| b.add_and_remove());
}
