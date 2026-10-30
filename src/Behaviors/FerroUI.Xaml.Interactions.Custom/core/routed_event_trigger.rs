//! Port of `Core/RoutedEventTrigger.cs`.

use crate::{AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBase};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RouteHandler, RoutedEvent, RoutingStrategies};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, BoxedValue, FerroObjectImpl, StyledElementImpl};
use ferroui_xaml_interactivity::{DisposableAction, DisposingTriggerImpl, Interaction, StyledElementBehaviorImpl};
use std::rc::Rc;

/// A trigger of a routed event of the associated object, given untyped
/// ([`routed_event`](RoutedEventTriggerImpl::routed_event)): while the associated object is in
/// the visual tree, the event runs the actions with its args as the parameter.
///
/// The class is abstract: a deriving class implements
/// [`RoutedEventTriggerImpl::routed_event`].
#[repr(C)]
pub struct RoutedEventTrigger {
    base: RoutedEventTriggerBase,
}

// Deviation (DEVIATIONS.md, Behaviors): the `object? sender` of the handler is the element the
// handler is attached to (`&Interactive`), as the routed event handlers of the framework receive
// it.
ferro_class! {
    RoutedEventTrigger: RoutedEventTriggerBase, virtuals RoutedEventTriggerImpl: AttachedToVisualTreeTriggerBaseImpl {
        /// The routed event the trigger listens to (an abstract property of the reference).
        fn routed_event(this) -> RoutedEvent;
        /// Handles the event: runs the actions unless the trigger is disabled.
        fn handler(this, sender: &Interactive, e: &dyn IRoutedEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(RoutedEventTrigger: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl, DisposingTriggerImpl);

impl RoutedEventTriggerImpl for RoutedEventTrigger {
    fn routed_event(_this: &Self) -> RoutedEvent {
        panic!("RoutedEventTrigger is abstract: 'routed_event' must be implemented by the deriving class")
    }

    fn handler(this: &Self, _sender: &Interactive, e: &dyn IRoutedEventArgs) {
        if !this.is_enabled() {
            return;
        }

        this.execute(e);
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeTriggerBaseImpl for RoutedEventTrigger {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            if let Some(interactive) = this.associated_object_as::<Interactive>() {
                let disposable = Self::add_disposable_handler(
                    &interactive,
                    this.routed_event(),
                    this.to_ref().downgrade(),
                    this.event_routing_strategy(),
                    false,
                );
                return disposable;
            }

            DisposableAction::empty()
        }
    }
}

impl RoutedEventTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: RoutedEventTriggerBase::construct() }
    }

    /// Marks the event as handled (or not, see
    /// [`mark_as_handled`](RoutedEventTriggerBase::mark_as_handled)) and runs the actions with
    /// its args as the parameter.
    pub fn execute(&self, e: &dyn IRoutedEventArgs) {
        e.set_handled(self.mark_as_handled());
        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        // The args are the parameter of the actions (a shared handle to the live args).
        let parameter: BoxedValue = Rc::new(e.share());
        Interaction::execute_actions(sender.as_ref(), self.actions().as_ref(), Some(&parameter));
    }

    /// Adds the handler of the trigger to `o` and returns a disposable that removes it.
    fn add_disposable_handler(
        o: &Interactive,
        routed_event: RoutedEvent,
        trigger: ferroui_base::WeakRef<RoutedEventTrigger>,
        routes: RoutingStrategies,
        handled_events_too: bool,
    ) -> Rc<dyn IDisposable> {
        let handler: RouteHandler = Rc::new(move |sender: &Interactive, e: &dyn IRoutedEventArgs| {
            if let Some(trigger) = trigger.upgrade() {
                trigger.handler(sender, e);
            }
        });
        let token = o.add_route_handler(&routed_event, handler, routes, handled_events_too);

        let o = o.to_ref().downgrade();
        DisposableAction::create(move || {
            if let Some(o) = o.upgrade() {
                o.remove_handler(&routed_event, token);
            }
        })
    }
}
