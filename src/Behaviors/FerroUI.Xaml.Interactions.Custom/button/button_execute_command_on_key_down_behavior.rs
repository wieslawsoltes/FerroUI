//! Port of `Button/ButtonExecuteCommandOnKeyDownBehavior.cs`.

use crate::{AttachedToVisualTreeBehaviorImpl, ExecuteCommandBehaviorBaseImpl, ExecuteCommandOnKeyBehaviorBase};
use ferroui_base::input::{InputElement, KeyEventArgs};
use ferroui_base::interactivity::Interactive;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_controls::{Button, Control, TopLevel};
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Executes a command when a key is pressed while the button has focus.
#[repr(C)]
pub struct ButtonExecuteCommandOnKeyDownBehavior {
    base: ExecuteCommandOnKeyBehaviorBase,
}

ferro_class!(ButtonExecuteCommandOnKeyDownBehavior: ExecuteCommandOnKeyBehaviorBase);
ferro_class_info!(ButtonExecuteCommandOnKeyDownBehavior { new: ButtonExecuteCommandOnKeyDownBehavior::new });
ferroui_base::ferro_impl_classes!(
    ButtonExecuteCommandOnKeyDownBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
    ExecuteCommandBehaviorBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for ButtonExecuteCommandOnKeyDownBehavior {
        /// Called when the behavior is attached to the visual tree.
        ///
        /// Returns a disposable used to detach the key handler.
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let associated_object = this.associated_object_as::<Control>();
            let input_root = TopLevel::get_top_level(associated_object.as_deref().map(crate::part_c::control_as_visual));
            if let Some(input_root) = input_root {
                let behavior = this.to_ref().downgrade();
                return input_root.add_disposable_handler(
                    InputElement::key_down_event(),
                    move |_sender: &Interactive, e: &KeyEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.root_default_key_down(e);
                        }
                    },
                );
            }

            DisposableAction::empty()
        }
    }
}

impl ButtonExecuteCommandOnKeyDownBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ExecuteCommandOnKeyBehaviorBase::construct() }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn root_default_key_down(&self, e: &KeyEventArgs) {
        let have_key = self.key().is_some_and(|key| e.key == key);
        let have_gesture = self.gesture().is_some_and(|gesture| gesture.matches(Some(e)));
        if !have_key && !have_gesture {
            return;
        }

        if let Some(button) = self.associated_object_as::<Button>() {
            self.execute_button_command(&button);
        }
    }

    // The `ExecuteCommand(Button)` overload of the reference.
    fn execute_button_command(&self, button: &Ref<Button>) -> bool {
        if !self.is_enabled() {
            return false;
        }

        if !button.is_visible() || !button.is_enabled() {
            return false;
        }

        let command_parameter = button.command_parameter();
        let Some(command) = button.command().filter(|command| command.can_execute(command_parameter.as_ref())) else {
            return false;
        };

        if self.focus_top_level() {
            let behavior: Ref<ButtonExecuteCommandOnKeyDownBehavior> = self.to_ref();
            Dispatcher::ui_thread().post_local(
                move || {
                    let top_level = behavior.top_level().or_else(|| {
                        behavior
                            .associated_object_as::<Control>()
                            .and_then(|associated_object| associated_object.get_self_and_logical_ancestors().last())
                            .and_then(|root| root.cast::<TopLevel>())
                    });
                    if let Some(top_level) = top_level {
                        top_level.focus();
                    }
                },
                DispatcherPriority::DEFAULT,
            );
        }

        if let Some(focus_control) = self.focus_control() {
            Dispatcher::ui_thread().post_local(
                move || {
                    focus_control.focus();
                },
                DispatcherPriority::DEFAULT,
            );
        }

        command.execute(command_parameter.as_ref());
        true
    }
}
