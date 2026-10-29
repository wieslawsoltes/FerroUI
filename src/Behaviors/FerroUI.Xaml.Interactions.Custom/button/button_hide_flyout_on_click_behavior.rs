//! Port of `Button/ButtonHideFlyoutOnClickBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StaticType, StyledElementImpl};
use ferroui_controls::primitives::Popup;
use ferroui_controls::{Button, FlyoutPresenter};
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Hides the flyout when the button is clicked.
#[repr(C)]
pub struct ButtonHideFlyoutOnClickBehavior {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(ButtonHideFlyoutOnClickBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(ButtonHideFlyoutOnClickBehavior { new: ButtonHideFlyoutOnClickBehavior::new });
ferroui_base::ferro_impl_classes!(
    ButtonHideFlyoutOnClickBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for ButtonHideFlyoutOnClickBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(button) = this.associated_object_as::<Button>() else {
                return DisposableAction::empty();
            };

            let flyout_presenter = button.find_ancestor_of_type::<FlyoutPresenter>(false);
            let Some(popup) = flyout_presenter
                .and_then(|flyout_presenter| flyout_presenter.parent())
                .and_then(|parent| parent.cast::<Popup>())
            else {
                return DisposableAction::empty();
            };

            // The handler holds the button and the popup weakly: it lives in the button.
            let weak_button = button.downgrade();
            let weak_popup = popup.downgrade();
            let token = button.click(move |_sender: &Interactive, _e: &RoutedEventArgs| {
                let (Some(button), Some(popup)) = (weak_button.upgrade(), weak_popup.upgrade()) else {
                    return;
                };
                // Execute Command if any before closing. Otherwise, it won't execute because Close
                // will destroy the associated object before Click can execute it.
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

impl ButtonHideFlyoutOnClickBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<Button as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
