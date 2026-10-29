//! Port of `Core/RoutedEventTriggerBaseOfT.cs` (`RoutedEventTriggerBase<T>`).

use crate::{AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBase};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RouteHandler, RoutedEvent};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, BoxedValue, FerroObjectImpl, StyledElementImpl, WeakRef};
use ferroui_xaml_interactivity::{DisposableAction, DisposingTriggerImpl, Interaction, StyledElementBehaviorImpl};
use std::rc::Rc;

/// A trigger of a routed event of the associated object whose args are of type `T`: while the
/// associated object is in the visual tree, the event runs the actions with its args as the
/// parameter.
///
/// The class is abstract: a deriving class implements
/// [`RoutedEventTriggerBaseOfTImpl::routed_event`], and can override
/// [`RoutedEventTriggerBaseOfTImpl::handler`] to filter the events (calling
/// [`execute`](Self::execute) to run the actions).
#[repr(C)]
pub struct RoutedEventTriggerBaseOfT {
    base: RoutedEventTriggerBase,
}

// Deviation (DEVIATIONS.md, Behaviors): classes of the class model are not generic. The routed
// event is the untyped handle of the event (`InputElement::tapped_event().as_routed_event()`), and
// the handler receives the args as `&dyn IRoutedEventArgs`, which a deriving class casts back to
// the args of the event (`e.downcast_ref::<KeyEventArgs>()`), the `T` of the reference. The
// `object? sender` of the handler is the element the handler is attached to (`&Interactive`).
ferro_class! {
    RoutedEventTriggerBaseOfT: RoutedEventTriggerBase,
    virtuals RoutedEventTriggerBaseOfTImpl: AttachedToVisualTreeTriggerBaseImpl {
        /// The routed event the trigger listens to (an abstract property of the reference).
        fn routed_event(this) -> RoutedEvent;
        /// Handles the event: runs the actions unless the trigger is disabled.
        fn handler(this, sender: &Interactive, e: &dyn IRoutedEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(
    RoutedEventTriggerBaseOfT: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
);

impl RoutedEventTriggerBaseOfTImpl for RoutedEventTriggerBaseOfT {
    fn routed_event(_this: &Self) -> RoutedEvent {
        panic!("RoutedEventTriggerBaseOfT is abstract: 'routed_event' must be implemented by the deriving class")
    }

    fn handler(this: &Self, _sender: &Interactive, e: &dyn IRoutedEventArgs) {
        if !this.is_enabled() {
            return;
        }

        this.execute(e);
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeTriggerBaseImpl for RoutedEventTriggerBaseOfT {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            if let Some(interactive) = this.associated_object_as::<Interactive>() {
                let routed_event = this.routed_event();
                let trigger: WeakRef<RoutedEventTriggerBaseOfT> = this.to_ref().downgrade();
                let handler: RouteHandler = Rc::new(move |sender: &Interactive, e: &dyn IRoutedEventArgs| {
                    if let Some(trigger) = trigger.upgrade() {
                        trigger.handler(sender, e);
                    }
                });
                let token = interactive.add_route_handler(&routed_event, handler, this.event_routing_strategy(), false);

                let interactive = interactive.downgrade();
                return DisposableAction::create(move || {
                    if let Some(interactive) = interactive.upgrade() {
                        interactive.remove_handler(&routed_event, token);
                    }
                });
            }

            DisposableAction::empty()
        }
    }
}

impl RoutedEventTriggerBaseOfT {
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
}
