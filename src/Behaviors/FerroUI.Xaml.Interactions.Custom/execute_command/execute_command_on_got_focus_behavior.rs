//! Port of `ExecuteCommand/ExecuteCommandOnGotFocusBehavior.cs`.

use crate::{AttachedToVisualTreeBehaviorImpl, ExecuteCommandBehaviorBaseImpl, ExecuteCommandRoutedEventBehaviorBase};
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RouteHandler, RoutingStrategies};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl, WeakRef};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// A behavior that executes its command when the [`InputElement::got_focus_event`] of its
/// source control (the [`source_control`](crate::ExecuteCommandBehaviorBase::source_control), or
/// the associated control) is raised and not yet handled.
#[repr(C)]
pub struct ExecuteCommandOnGotFocusBehavior {
    base: ExecuteCommandRoutedEventBehaviorBase,
}

ferro_class!(ExecuteCommandOnGotFocusBehavior: ExecuteCommandRoutedEventBehaviorBase);
ferro_class_info!(ExecuteCommandOnGotFocusBehavior { new: ExecuteCommandOnGotFocusBehavior::new });
ferroui_base::ferro_impl_classes!(
    ExecuteCommandOnGotFocusBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
    ExecuteCommandBehaviorBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for ExecuteCommandOnGotFocusBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let control = this.source_control().or_else(|| this.associated_object_as::<Control>());
            let Some(control) = control else {
                return DisposableAction::empty();
            };

            let behavior: WeakRef<ExecuteCommandOnGotFocusBehavior> = this.to_ref().downgrade();
            let handler: RouteHandler = Rc::new(move |sender: &Interactive, e: &dyn IRoutedEventArgs| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.on_got_focus(sender, e);
                }
            });
            let token = control.add_route_handler(
                InputElement::got_focus_event(),
                handler,
                this.event_routing_strategy(),
                false,
            );

            let control = control.downgrade();
            DisposableAction::create(move || {
                if let Some(control) = control.upgrade() {
                    control.remove_handler(InputElement::got_focus_event(), token);
                }
            })
        }
    }
}

impl ExecuteCommandOnGotFocusBehavior {
    /// The static constructor: the handler listens to the tunnel and bubble routes by default.
    fn static_constructor() {
        ExecuteCommandRoutedEventBehaviorBase::event_routing_strategy_property()
            .override_default_value::<ExecuteCommandOnGotFocusBehavior>(RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ExecuteCommandRoutedEventBehaviorBase::construct() }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn on_got_focus(&self, _sender: &Interactive, e: &dyn IRoutedEventArgs) {
        if e.handled() {
            return;
        }

        if self.execute_command() {
            e.set_handled(self.mark_as_handled());
        }
    }
}
