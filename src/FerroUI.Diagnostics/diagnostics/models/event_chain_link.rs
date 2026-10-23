//! Port of `Diagnostics/Models/EventChainLink.cs`.

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::{ferro_markup_type, BoxedValue, StyledElement};
use std::cell::Cell;
use std::rc::Rc;

/// One link of the chain of a routed event: an object whose handlers saw the event, whether the
/// event was handled there and the route it took.
pub struct EventChainLink {
    handler: BoxedValue,
    begins_new_route: Cell<bool>,
    handled: Cell<bool>,
    route: RoutingStrategies,
}

impl PartialEq for EventChainLink {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl EventChainLink {
    /// C# `new EventChainLink(object handler, bool handled, RoutingStrategies route)`.
    pub fn new(handler: BoxedValue, handled: bool, route: RoutingStrategies) -> Rc<Self> {
        Rc::new(Self { handler, begins_new_route: Cell::new(false), handled: Cell::new(handled), route })
    }

    pub fn handler(&self) -> BoxedValue {
        self.handler.clone()
    }

    pub fn begins_new_route(&self) -> bool {
        self.begins_new_route.get()
    }

    pub fn set_begins_new_route(&self, value: bool) {
        self.begins_new_route.set(value);
    }

    /// The name of the handler: its name and the name of its type (`name (Button)`) when it is
    /// a named element (`INamed`) with a name, else the name of its type.
    pub fn handler_name(&self) -> String {
        let object = ValueTypes::as_object(&*self.handler);
        // `Handler.GetType().Name`.
        let type_name = match &object {
            Some(object) => object.get_type().name().to_string(),
            None => {
                let full_name = ValueTypes::type_full_name(ValueType::of_value(&*self.handler));
                let start = full_name.rfind('.').map_or(0, |index| index + 1);
                full_name[start..].to_string()
            }
        };

        if let Some(name) = object
            .and_then(|object| object.cast::<StyledElement>())
            .and_then(|named| named.name())
            .filter(|name| !name.is_empty())
        {
            return name + " (" + &type_name + ")";
        }

        type_name
    }

    pub fn handled(&self) -> bool {
        self.handled.get()
    }

    pub fn set_handled(&self, value: bool) {
        self.handled.set(value);
    }

    pub fn route(&self) -> RoutingStrategies {
        self.route
    }
}

ferro_markup_type!(class EventChainLink {
    this: Rc<EventChainLink>,
    handles: [Rc<EventChainLink>, Option<Rc<EventChainLink>>],
    properties: [
        Handler: Option<BoxedValue> { get: |l: &Rc<EventChainLink>| Some(l.handler()) },
        BeginsNewRoute: bool {
            get: |l: &Rc<EventChainLink>| l.begins_new_route(),
            set: |l: &Rc<EventChainLink>, value: bool| l.set_begins_new_route(value)
        },
        HandlerName: String { get: |l: &Rc<EventChainLink>| l.handler_name() },
        Handled: bool {
            get: |l: &Rc<EventChainLink>| l.handled(),
            set: |l: &Rc<EventChainLink>, value: bool| l.set_handled(value)
        },
        Route: RoutingStrategies { get: |l: &Rc<EventChainLink>| l.route() },
    ],
});
