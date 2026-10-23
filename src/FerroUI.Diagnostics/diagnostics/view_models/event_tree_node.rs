//! Port of `Diagnostics/ViewModels/EventTreeNode.cs`.

use super::{EventTreeNodeBase, EventTreeNodeKind, EventsPageViewModel, FiredEvent};
use crate::diagnostics::models::EventChainLink;
use crate::diagnostics::DevTools;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEvent, RoutedEventArgs, RoutingStrategies};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::DateTime;
use ferroui_base::{BoxedValue, FerroObject, Ref, Visual};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The most recorded events the events page keeps; the oldest are removed.
const MAX_RECORDED_EVENTS: usize = 100;

/// The node of one routed event: enabling it records the event, with the
/// chain of the elements it visits, in the events page.
///
/// The part of the deriving class of a node (see [`EventTreeNodeKind`]).
pub struct EventTreeNode {
    event: RoutedEvent,
    parent_view_model: Weak<EventsPageViewModel>,
    is_registered: Cell<bool>,
    current_event: RefCell<Option<Rc<FiredEvent>>>,
}

impl EventTreeNode {
    /// C# `new EventTreeNode(EventOwnerTreeNode parent, RoutedEvent @event, EventsPageViewModel vm)`.
    ///
    /// The view model of the page is held weakly: the page holds the nodes.
    pub fn new(
        parent: &Rc<EventTreeNodeBase>,
        event: RoutedEvent,
        vm: Weak<EventsPageViewModel>,
    ) -> Rc<EventTreeNodeBase> {
        EventTreeNodeBase::new(Some(parent), event.name().to_string(), |_| {
            EventTreeNodeKind::Event(EventTreeNode {
                event,
                parent_view_model: vm,
                is_registered: Cell::new(false),
                current_event: RefCell::new(None),
            })
        })
    }

    /// The routed event of the node.
    pub fn event(&self) -> RoutedEvent {
        self.event
    }

    /// The override of the setter of `IsEnabled`: starts tracking the event
    /// when it is enabled, and updates the check box of the owner unless the
    /// change comes from the owner.
    pub(crate) fn set_is_enabled(this: &EventTreeNodeBase, value: Option<bool>) {
        if this.is_enabled() != value {
            this.base_set_is_enabled(value);
            Self::update_tracker(this);
            if let Some(parent) = this.parent() {
                if this.update_parent.get() {
                    // `try { .. } finally { .. }` upstream.
                    parent.update_children.set(false);
                    parent.update_checked();
                    parent.update_children.set(true);
                }
            }
        }
    }

    fn part(this: &EventTreeNodeBase) -> &EventTreeNode {
        this.as_event_tree_node().expect("a node of an event")
    }

    fn update_tracker(this: &EventTreeNodeBase) {
        let node = Self::part(this);
        if this.is_enabled().unwrap_or_default() && !node.is_registered.get() {
            let all_routes = RoutingStrategies::DIRECT | RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE;

            // FIXME: This leaks event handlers.
            // (As upstream, the handlers are never removed; they reach the node through a weak
            // reference, so they do not keep it alive.)
            let weak = Rc::downgrade(&this.to_rc());
            let _ = node.event.add_class_handler_untyped(
                FerroObject::TYPE,
                move |sender, e| {
                    if let Some(this) = weak.upgrade() {
                        Self::handle_event(&this, sender, e);
                    }
                },
                all_routes,
                true,
            );
            let weak = Rc::downgrade(&this.to_rc());
            let _ = node.event.route_finished().subscribe(move |e| {
                if let Some(this) = weak.upgrade() {
                    Self::handle_route_finished(&this, e);
                }
            });

            node.is_registered.set(true);
        }
    }

    /// The link of the chain of an event for an element on its route.
    fn chain_link(handler: &Ref<Interactive>, handled: bool, route: RoutingStrategies) -> Rc<EventChainLink> {
        // The sender as an untyped value (`object`): the handle of the object.
        let handler: BoxedValue = Rc::new(handler.clone().upcast::<FerroObject>());
        EventChainLink::new(handler, handled, route)
    }

    /// Runs `handler` on the UI thread: now when this is the UI thread, else
    /// posted to it.
    ///
    /// Routed events, their class handlers and their route notifications
    /// belong to the thread that raises them, which is the UI thread in an
    /// application; the post is what the original does for any other.
    fn run_on_ui_thread(handler: impl FnOnce() + 'static) {
        let dispatcher = Dispatcher::ui_thread();
        if !dispatcher.check_access() {
            dispatcher.post_local(handler, DispatcherPriority::default());
        } else {
            handler();
        }
    }

    fn handle_event(this: &Rc<EventTreeNodeBase>, sender: &Interactive, e: &dyn IRoutedEventArgs) {
        let node = Self::part(this);
        if !node.is_registered.get() || this.is_enabled() == Some(false) {
            return;
        }
        if let Some(v) = sender.downcast_ref::<Visual>() {
            if DevTools::does_belong_to_dev_tool(v) {
                return;
            }
        }

        let s = sender.to_ref();
        let handled = e.handled();
        let route = e.route();
        let trigger_time = DateTime::now();
        let args: RoutedEventArgs = e.as_routed_event_args().clone();
        let weak = Rc::downgrade(this);

        let handler = move || {
            let Some(this) = weak.upgrade() else {
                return;
            };
            let node = Self::part(&this);

            let current_event = node.current_event.borrow().clone();
            match current_event {
                Some(current_event) if current_event.is_part_of_same_event_chain(&args) => {
                    current_event.add_to_chain(Self::chain_link(&s, handled, route));
                }
                _ => {
                    let current_event = FiredEvent::new(&args, Self::chain_link(&s, handled, route), trigger_time);
                    *node.current_event.borrow_mut() = Some(current_event.clone());

                    if let Some(parent_view_model) = node.parent_view_model.upgrade() {
                        let recorded_events = parent_view_model.recorded_events();
                        recorded_events.add(current_event);

                        while recorded_events.count() > MAX_RECORDED_EVENTS {
                            recorded_events.remove_at(0);
                        }
                    }
                }
            }
        };

        Self::run_on_ui_thread(handler);
    }

    fn handle_route_finished(this: &Rc<EventTreeNodeBase>, e: &dyn IRoutedEventArgs) {
        let node = Self::part(this);
        if !node.is_registered.get() || this.is_enabled() == Some(false) {
            return;
        }
        if let Some(v) = e.source().and_then(|source| source.cast::<Visual>()) {
            if DevTools::does_belong_to_dev_tool(&v) {
                return;
            }
        }

        let handled = e.handled();
        let weak = Rc::downgrade(this);

        let handler = move || {
            let Some(this) = weak.upgrade() else {
                return;
            };
            let node = Self::part(&this);

            let current_event = node.current_event.borrow().clone();
            if let Some(current_event) = current_event {
                if handled {
                    let event_chain = current_event.event_chain();
                    let link_index = event_chain.count() - 1;
                    let link = event_chain.get(link_index);

                    link.set_handled(true);
                    if current_event.handled_by().is_none() {
                        current_event.set_handled_by(Some(link));
                    }
                }
            }
        };

        Self::run_on_ui_thread(handler);
    }
}
