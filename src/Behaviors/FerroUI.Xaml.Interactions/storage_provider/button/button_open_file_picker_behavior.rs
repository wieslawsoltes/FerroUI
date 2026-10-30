//! Port of `StorageProvider/Button/ButtonOpenFilePickerBehavior.cs`.

use crate::action_task::ActionTask;
use crate::{OpenFilePickerBehaviorBase, OpenFilePickerBehaviorBaseImpl};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_controls::Button;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt};
use std::cell::Cell;
use std::rc::Rc;

/// Open file picker behavior for [`Button`]: the picker opens when the button is clicked.
#[repr(C)]
pub struct ButtonOpenFilePickerBehavior {
    base: OpenFilePickerBehaviorBase,
    click_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(ButtonOpenFilePickerBehavior: OpenFilePickerBehaviorBase);
ferro_class_info!(ButtonOpenFilePickerBehavior { new: ButtonOpenFilePickerBehavior::new });
ferroui_base::ferro_impl_classes!(ButtonOpenFilePickerBehavior: FerroObjectImpl, StyledElementImpl, OpenFilePickerBehaviorBaseImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ButtonOpenFilePickerBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);

            if let Some(button) = this.associated_object().and_then(|associated_object| associated_object.cast::<Button>()) {
                let behavior = this.to_ref().downgrade();
                let token = button.click(move |sender: &Interactive, e: &RoutedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_click(sender, e);
                    }
                });
                if let Some(previous) = this.click_handler.replace(Some(token)) {
                    button.remove_handler(Button::click_event(), previous);
                }
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            Self::parent_on_detached_from_visual_tree(this);

            if let Some(button) = this.associated_object().and_then(|associated_object| associated_object.cast::<Button>()) {
                if let Some(token) = this.click_handler.take() {
                    button.remove_handler(Button::click_event(), token);
                }
            }
        }
    }
}

impl ButtonOpenFilePickerBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: OpenFilePickerBehaviorBase::construct(), click_handler: Cell::new(None) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Opens the picker (an `async void` handler in the reference): the operation runs in a job of
    /// the dispatcher.
    fn on_click(&self, sender: &Interactive, e: &RoutedEventArgs) {
        let behavior: Ref<ButtonOpenFilePickerBehavior> = self.to_ref();
        let sender: BoxedValue = Rc::new(sender.to_ref());
        let event_args: BoxedValue = Rc::new(e.share());
        drop(ActionTask::spawn(async move {
            behavior.execute(Some(&sender), Some(&event_args)).await;
        }));
    }
}
