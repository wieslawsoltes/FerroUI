//! Port of `ExecuteCommand/ExecuteCommandOnKeyDownBehavior.cs`.

use crate::{AttachedToVisualTreeBehaviorImpl, ExecuteCommandBehaviorBaseImpl, ExecuteCommandOnKeyBehaviorBase};
use ferroui_base::input::{InputElement, KeyEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RouteHandler};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl, WeakRef};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// A behavior that executes its command when the [`InputElement::key_down_event`] of its
/// source control (the [`source_control`](crate::ExecuteCommandBehaviorBase::source_control), or
/// the associated control) is raised and not yet handled with its key or key gesture.
#[repr(C)]
pub struct ExecuteCommandOnKeyDownBehavior {
    base: ExecuteCommandOnKeyBehaviorBase,
}

ferro_class!(ExecuteCommandOnKeyDownBehavior: ExecuteCommandOnKeyBehaviorBase);
ferro_class_info!(ExecuteCommandOnKeyDownBehavior { new: ExecuteCommandOnKeyDownBehavior::new });
ferroui_base::ferro_impl_classes!(
    ExecuteCommandOnKeyDownBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
    ExecuteCommandBehaviorBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for ExecuteCommandOnKeyDownBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let control = this.source_control().or_else(|| this.associated_object_as::<Control>());
            let Some(control) = control else {
                return DisposableAction::empty();
            };

            let behavior: WeakRef<ExecuteCommandOnKeyDownBehavior> = this.to_ref().downgrade();
            let handler: RouteHandler = Rc::new(move |sender: &Interactive, e: &dyn IRoutedEventArgs| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.on_key_down(sender, e);
                }
            });
            let token = control.add_route_handler(
                InputElement::key_down_event(),
                handler,
                this.event_routing_strategy(),
                false,
            );

            let control = control.downgrade();
            DisposableAction::create(move || {
                if let Some(control) = control.upgrade() {
                    control.remove_handler(InputElement::key_down_event(), token);
                }
            })
        }
    }
}

impl ExecuteCommandOnKeyDownBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ExecuteCommandOnKeyBehaviorBase::construct() }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn on_key_down(&self, _sender: &Interactive, e: &dyn IRoutedEventArgs) {
        let Some(key_event) = e.downcast_ref::<KeyEventArgs>() else {
            return;
        };

        let key = self.key();
        let gesture = self.gesture();
        let have_key = key.is_some_and(|key| key_event.key == key);
        let have_gesture = gesture.is_some_and(|gesture| gesture.matches(Some(key_event)));

        if !have_key && !have_gesture {
            return;
        }

        if e.handled() {
            return;
        }

        if self.execute_command() {
            e.set_handled(self.mark_as_handled());
        }
    }
}
