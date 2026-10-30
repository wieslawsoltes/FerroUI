//! Port of `GotFocusEventBehavior.cs`.

use ferroui_base::input::{FocusChangedEventArgs, InputElement};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::{ferro_class, FerroObjectImpl, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveBehaviorBase, StyledElementBehaviorImpl};
use std::cell::Cell;

/// Behavior that handles the [`InputElement::got_focus_event`].
///
/// The class is abstract: a deriving class overrides [`GotFocusEventBehaviorImpl::on_got_focus`].
#[repr(C)]
pub struct GotFocusEventBehavior {
    base: InteractiveBehaviorBase,
    // The handler added to the associated object (the reference removes its delegate).
    got_focus_handler: Cell<Option<RoutedEventHandlerToken>>,
}

// Deviation (DEVIATIONS.md, Behaviors): the `object? sender` of the hooks is the element the handler
// is attached to (`&Interactive`), as the routed event handlers of the framework receive it.
ferro_class! {
    GotFocusEventBehavior: InteractiveBehaviorBase, virtuals GotFocusEventBehaviorImpl: StyledElementBehaviorImpl {
        /// Called when the associated control receives focus.
        fn on_got_focus(this, sender: &Interactive, e: &FocusChangedEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(GotFocusEventBehavior: FerroObjectImpl, StyledElementImpl);

impl GotFocusEventBehaviorImpl for GotFocusEventBehavior {
    fn on_got_focus(_this: &Self, _sender: &Interactive, _e: &FocusChangedEventArgs) {}
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for GotFocusEventBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::got_focus_event(),
                move |sender: &Interactive, e: &FocusChangedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.got_focus(sender, e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.got_focus_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            if let Some(handler) = this.got_focus_handler.take() {
                associated_object.remove_handler(InputElement::got_focus_event(), handler);
            }
        }
    }
}

impl GotFocusEventBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveBehaviorBase::construct(), got_focus_handler: Cell::new(None) }
    }

    fn got_focus(&self, sender: &Interactive, e: &FocusChangedEventArgs) {
        self.on_got_focus(sender, e);
    }
}
