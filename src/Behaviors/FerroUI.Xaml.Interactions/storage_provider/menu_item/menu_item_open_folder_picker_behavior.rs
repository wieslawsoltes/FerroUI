//! Port of `StorageProvider/MenuItem/MenuItemOpenFolderPickerBehavior.cs`.

use crate::action_task::ActionTask;
use crate::{OpenFolderPickerBehaviorBase, OpenFolderPickerBehaviorBaseImpl};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_controls::MenuItem;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt};
use std::cell::Cell;
use std::rc::Rc;

/// Open folder picker behavior for [`MenuItem`]: the picker opens when the menu item is clicked.
#[repr(C)]
pub struct MenuItemOpenFolderPickerBehavior {
    base: OpenFolderPickerBehaviorBase,
    click_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(MenuItemOpenFolderPickerBehavior: OpenFolderPickerBehaviorBase);
ferro_class_info!(MenuItemOpenFolderPickerBehavior { new: MenuItemOpenFolderPickerBehavior::new });
ferroui_base::ferro_impl_classes!(MenuItemOpenFolderPickerBehavior: FerroObjectImpl, StyledElementImpl, OpenFolderPickerBehaviorBaseImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for MenuItemOpenFolderPickerBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);

            if let Some(menu_item) = this.associated_object().and_then(|associated_object| associated_object.cast::<MenuItem>()) {
                let behavior = this.to_ref().downgrade();
                let token = menu_item.click(move |sender: &Interactive, e: &RoutedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_click(sender, e);
                    }
                });
                if let Some(previous) = this.click_handler.replace(Some(token)) {
                    menu_item.remove_handler(MenuItem::click_event(), previous);
                }
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            Self::parent_on_detached_from_visual_tree(this);

            if let Some(menu_item) = this.associated_object().and_then(|associated_object| associated_object.cast::<MenuItem>()) {
                if let Some(token) = this.click_handler.take() {
                    menu_item.remove_handler(MenuItem::click_event(), token);
                }
            }
        }
    }
}

impl MenuItemOpenFolderPickerBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: OpenFolderPickerBehaviorBase::construct(), click_handler: Cell::new(None) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Opens the picker (an `async void` handler in the reference): the operation runs in a job of
    /// the dispatcher.
    fn on_click(&self, sender: &Interactive, e: &RoutedEventArgs) {
        let behavior: Ref<MenuItemOpenFolderPickerBehavior> = self.to_ref();
        let sender: BoxedValue = Rc::new(sender.to_ref());
        let event_args: BoxedValue = Rc::new(e.share());
        drop(ActionTask::spawn(async move {
            behavior.execute(Some(&sender), Some(&event_args)).await;
        }));
    }
}
