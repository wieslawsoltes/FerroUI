//! Port of `PointerMovedEventTrigger.cs`.

use ferroui_base::input::{InputElement, PointerEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveTriggerBase, StyledElementBehaviorImpl};
use std::cell::Cell;
use std::rc::Rc;

/// Trigger that listens for the [`InputElement::pointer_moved_event`].
#[repr(C)]
pub struct PointerMovedEventTrigger {
    base: InteractiveTriggerBase,
    // The handlers added to the associated object (the reference removes its delegates).
    on_pointer_moved_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(PointerMovedEventTrigger: InteractiveTriggerBase);
ferroui_base::ferro_class_info!(PointerMovedEventTrigger { new: PointerMovedEventTrigger::new });
ferroui_base::ferro_impl_classes!(PointerMovedEventTrigger: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for PointerMovedEventTrigger {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            let trigger = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::pointer_moved_event(),
                move |_sender: &Interactive, e: &PointerEventArgs| {
                    if let Some(trigger) = trigger.upgrade() {
                        trigger.on_pointer_moved(e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.on_pointer_moved_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            if let Some(handler) = this.on_pointer_moved_handler.take() {
                associated_object.remove_handler(InputElement::pointer_moved_event(), handler);
            }
        }
    }
}

impl PointerMovedEventTrigger {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        InteractiveTriggerBase::routing_strategies_property().override_default_value::<PointerMovedEventTrigger>(
            RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveTriggerBase::construct(), on_pointer_moved_handler: Cell::new(None) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn on_pointer_moved(&self, e: &dyn IRoutedEventArgs) {
        // The args are the parameter of the actions (a shared handle to the live args).
        let parameter: BoxedValue = Rc::new(e.share());
        self.execute(Some(&parameter));
    }
}
