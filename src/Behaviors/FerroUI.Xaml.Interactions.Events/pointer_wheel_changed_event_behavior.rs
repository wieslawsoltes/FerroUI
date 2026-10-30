//! Port of `PointerWheelChangedEventBehavior.cs`.

use ferroui_base::input::{InputElement, PointerWheelEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{ferro_class, FerroObjectImpl, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveBehaviorBase, StyledElementBehaviorImpl};
use std::cell::Cell;

/// Behavior that handles the [`InputElement::pointer_wheel_changed_event`].
///
/// The class is abstract: a deriving class overrides [`PointerWheelChangedEventBehaviorImpl::on_pointer_wheel_changed`].
#[repr(C)]
pub struct PointerWheelChangedEventBehavior {
    base: InteractiveBehaviorBase,
    // The handler added to the associated object (the reference removes its delegate).
    pointer_wheel_changed_handler: Cell<Option<RoutedEventHandlerToken>>,
}

// Deviation (DEVIATIONS.md, Behaviors): the `object? sender` of the hooks is the element the handler
// is attached to (`&Interactive`), as the routed event handlers of the framework receive it.
ferro_class! {
    PointerWheelChangedEventBehavior: InteractiveBehaviorBase, virtuals PointerWheelChangedEventBehaviorImpl: StyledElementBehaviorImpl {
        /// Called when the mouse wheel changes while over the associated control.
        fn on_pointer_wheel_changed(this, sender: &Interactive, e: &PointerWheelEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(PointerWheelChangedEventBehavior: FerroObjectImpl, StyledElementImpl);

impl PointerWheelChangedEventBehaviorImpl for PointerWheelChangedEventBehavior {
    fn on_pointer_wheel_changed(_this: &Self, _sender: &Interactive, _e: &PointerWheelEventArgs) {}
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for PointerWheelChangedEventBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::pointer_wheel_changed_event(),
                move |sender: &Interactive, e: &PointerWheelEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.pointer_wheel_changed(sender, e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.pointer_wheel_changed_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            if let Some(handler) = this.pointer_wheel_changed_handler.take() {
                associated_object.remove_handler(InputElement::pointer_wheel_changed_event(), handler);
            }
        }
    }
}

impl PointerWheelChangedEventBehavior {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        InteractiveBehaviorBase::routing_strategies_property().override_default_value::<PointerWheelChangedEventBehavior>(
            RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveBehaviorBase::construct(), pointer_wheel_changed_handler: Cell::new(None) }
    }

    fn pointer_wheel_changed(&self, sender: &Interactive, e: &PointerWheelEventArgs) {
        self.on_pointer_wheel_changed(sender, e);
    }
}
