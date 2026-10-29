//! Port of `Button/ButtonHidePopupOnClickBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StaticType, StyledElementImpl};
use ferroui_controls::primitives::Popup;
use ferroui_controls::Button;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Hides the popup when the button is clicked.
#[repr(C)]
pub struct ButtonHidePopupOnClickBehavior {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(ButtonHidePopupOnClickBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(ButtonHidePopupOnClickBehavior { new: ButtonHidePopupOnClickBehavior::new });
ferroui_base::ferro_impl_classes!(
    ButtonHidePopupOnClickBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for ButtonHidePopupOnClickBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(button) = this.associated_object_as::<Button>() else {
                return DisposableAction::empty();
            };

            let Some(popup) = button.find_logical_ancestor_of_type::<Popup>(false) else {
                return DisposableAction::empty();
            };

            // The handler holds the button and the popup weakly: it lives in the button.
            let weak_button = button.downgrade();
            let weak_popup = popup.downgrade();
            let token = button.click(move |_sender: &Interactive, _e: &RoutedEventArgs| {
                let (Some(button), Some(popup)) = (weak_button.upgrade(), weak_popup.upgrade()) else {
                    return;
                };

                if let Some(command) = button.command().filter(|_| button.is_enabled()) {
                    command.execute(button.command_parameter().as_ref());
                }

                popup.close();
            });

            let button = button.downgrade();
            DisposableAction::create(move || {
                if let Some(button) = button.upgrade() {
                    button.remove_handler(Button::click_event(), token);
                }
            })
        }
    }
}

impl ButtonHidePopupOnClickBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<Button as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
