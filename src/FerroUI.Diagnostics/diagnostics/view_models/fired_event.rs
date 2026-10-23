//! Port of `Diagnostics/ViewModels/FiredEvent.cs`.

use super::ViewModelBase;
use crate::diagnostics::models::EventChainLink;
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs, RoutingStrategies};
use ferroui_base::utilities::DateTime;
use std::cell::RefCell;
use std::rc::Rc;

/// `Environment.NewLine`.
const NEW_LINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

/// The text of routing strategies as the text of a flags enumeration of
/// .NET: the names of the set flags, by value, separated by `, `.
pub(crate) fn routing_strategies_text(strategies: RoutingStrategies) -> String {
    let names: Vec<&str> = [
        (RoutingStrategies::DIRECT, "Direct"),
        (RoutingStrategies::TUNNEL, "Tunnel"),
        (RoutingStrategies::BUBBLE, "Bubble"),
    ]
    .into_iter()
    .filter(|(flag, _)| strategies.contains(*flag))
    .map(|(_, name)| name)
    .collect();

    if names.is_empty() {
        strategies.bits().to_string()
    } else {
        names.join(", ")
    }
}

/// `a == b` of two references to links that may be null: reference
/// equality, as the class of the original.
fn same_link(a: &Option<Rc<EventChainLink>>, b: &Option<Rc<EventChainLink>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

/// A routed event recorded by the events page: the event, when it was
/// raised, and the chain of the elements it visited.
pub struct FiredEvent {
    base: ViewModelBase,
    event_args: RoutedEventArgs,
    original_event: Option<RoutedEvent>,
    handled_by: RefCell<Option<Rc<EventChainLink>>>,
    trigger_time: DateTime,
    event_chain: FerroList<Rc<EventChainLink>>,
    originator: Rc<EventChainLink>,
}

/// Reference equality, as the class of the original.
impl PartialEq for FiredEvent {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for FiredEvent {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl FiredEvent {
    /// C# `new FiredEvent(RoutedEventArgs eventArgs, EventChainLink originator, DateTime triggerTime)`:
    /// the event of `event_args`, with `originator` as the first link of its
    /// chain.
    pub fn new(event_args: &RoutedEventArgs, originator: Rc<EventChainLink>, trigger_time: DateTime) -> Rc<Self> {
        let this = Rc::new(Self {
            base: ViewModelBase::new(),
            event_args: event_args.clone(),
            original_event: event_args.routed_event(),
            handled_by: RefCell::new(None),
            trigger_time,
            event_chain: FerroList::new(),
            originator: originator.clone(),
        });
        this.add_to_chain(originator);
        this
    }

    /// Whether `e` are the args of the same raise of the same event.
    pub fn is_part_of_same_event_chain(&self, e: &RoutedEventArgs) -> bool {
        // Note, the framework might reuse RoutedEventArgs for different events to avoid extra allocations.
        // Like, PointerEntered and PointerExited will use the same instance of RoutedEventArgs.
        *e == self.event_args && e.routed_event() == self.original_event
    }

    pub fn trigger_time(&self) -> DateTime {
        self.trigger_time
    }

    /// The routed event.
    pub fn event(&self) -> RoutedEvent {
        self.original_event.expect("the args of a raised event name the routed event")
    }

    /// Whether an element on the route handled the event.
    pub fn is_handled(&self) -> bool {
        self.handled_by().is_some_and(|handled_by| handled_by.handled())
    }

    /// The elements the event visited, in order.
    pub fn event_chain(&self) -> FerroList<Rc<EventChainLink>> {
        self.event_chain.clone()
    }

    /// The description of the event: its name, its source, its routing
    /// strategies and what handled it.
    pub fn display_text(&self) -> String {
        let event = self.event();
        if self.is_handled() {
            let handled_by = self.handled_by().expect("a handled event has the link that handled it");
            return format!("{} on {};", event.name(), self.originator.handler_name())
                + NEW_LINE
                + &format!(
                    "strategies: {}; handled by: {}",
                    routing_strategies_text(event.routing_strategies()),
                    handled_by.handler_name()
                );
        }

        format!(
            "{} on {}; strategies: {}",
            event.name(),
            self.originator.handler_name(),
            routing_strategies_text(event.routing_strategies())
        )
    }

    /// The first link of the chain: the source of the event.
    pub fn originator(&self) -> Rc<EventChainLink> {
        self.originator.clone()
    }

    /// The link of the element that handled the event.
    pub fn handled_by(&self) -> Option<Rc<EventChainLink>> {
        self.handled_by.borrow().clone()
    }

    pub fn set_handled_by(&self, value: Option<Rc<EventChainLink>>) {
        let changed = !same_link(&self.handled_by.borrow(), &value);
        if changed {
            *self.handled_by.borrow_mut() = value;
            self.base.raise_property_changed("HandledBy");
            self.base.raise_property_changed("IsHandled");
            self.base.raise_property_changed("DisplayText");
        }
    }

    /// Adds the link of the next element the event visited; a link whose
    /// route differs from the route of the previous link begins a new route.
    pub fn add_to_chain(&self, link: Rc<EventChainLink>) {
        if self.event_chain.count() > 0 {
            let prev_link = self.event_chain.get(self.event_chain.count() - 1);

            if prev_link.route() != link.route() {
                link.set_begins_new_route(true);
            }
        }

        self.event_chain.add(link.clone());

        if self.handled_by().is_none() && link.handled() {
            self.set_handled_by(Some(link));
        }
    }
}

// The list of the links of the chain of an event, which the events page shows.
ferroui_controls::ferro_markup_list!(pub EventChainLinkList: Rc<EventChainLink>);

// The list of the recorded events, which the events page shows.
ferroui_controls::ferro_markup_list!(pub FiredEventList: Rc<FiredEvent>);

ferro_markup_type!(class FiredEvent {
    this: Rc<FiredEvent>,
    handles: [FiredEvent, Rc<FiredEvent>, Option<Rc<FiredEvent>>],
    base: ViewModelBase,
    properties: [
        TriggerTime: DateTime { get: FiredEvent::trigger_time },
        Event: RoutedEvent { get: FiredEvent::event },
        IsHandled: bool { get: FiredEvent::is_handled },
        EventChain: FerroList<Rc<EventChainLink>> { get: FiredEvent::event_chain },
        DisplayText: String { get: FiredEvent::display_text },
        Originator: Rc<EventChainLink> { get: FiredEvent::originator },
        HandledBy: Option<Rc<EventChainLink>> { get: FiredEvent::handled_by, set: FiredEvent::set_handled_by },
    ],
    notify_property_changed: FiredEvent,
});
