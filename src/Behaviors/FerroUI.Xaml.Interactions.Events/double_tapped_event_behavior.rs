//! Port of `DoubleTappedEventBehavior.cs`.

use ferroui_base::input::{InputElement, TappedEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventHandlerToken};
use ferroui_base::{ferro_class, FerroObjectImpl, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveBehaviorBase, StyledElementBehaviorImpl};
use std::cell::Cell;

/// Behavior that listens for [`InputElement::double_tapped_event`].
///
/// The class is abstract: a deriving class overrides [`DoubleTappedEventBehaviorImpl::on_double_tapped`].
#[repr(C)]
pub struct DoubleTappedEventBehavior {
    base: InteractiveBehaviorBase,
    // The handler added to the associated object (the reference removes its delegate).
    double_tapped_handler: Cell<Option<RoutedEventHandlerToken>>,
}

// Deviation (DEVIATIONS.md, Behaviors): the `object? sender` of the hooks is the element the handler
// is attached to (`&Interactive`), as the routed event handlers of the framework receive it.
// The `RoutedEventArgs` of the reference are `&dyn IRoutedEventArgs`, so that a deriving class
// can cast them back to the args of the event (`TappedEventArgs`), as it can in the reference.
ferro_class! {
    DoubleTappedEventBehavior: InteractiveBehaviorBase, virtuals DoubleTappedEventBehaviorImpl: StyledElementBehaviorImpl {
        /// Called when a double tap gesture is raised on the associated object.
        fn on_double_tapped(this, sender: &Interactive, e: &dyn IRoutedEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(DoubleTappedEventBehavior: FerroObjectImpl, StyledElementImpl);

impl DoubleTappedEventBehaviorImpl for DoubleTappedEventBehavior {
    fn on_double_tapped(_this: &Self, _sender: &Interactive, _e: &dyn IRoutedEventArgs) {}
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for DoubleTappedEventBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::double_tapped_event(),
                move |sender: &Interactive, e: &TappedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.double_tapped(sender, e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.double_tapped_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            if let Some(handler) = this.double_tapped_handler.take() {
                associated_object.remove_handler(InputElement::double_tapped_event(), handler);
            }
        }
    }
}

impl DoubleTappedEventBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveBehaviorBase::construct(), double_tapped_handler: Cell::new(None) }
    }

    fn double_tapped(&self, sender: &Interactive, e: &dyn IRoutedEventArgs) {
        self.on_double_tapped(sender, e);
    }
}
