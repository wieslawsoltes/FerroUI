use crate::platform::{DefaultMenuInteractionHandler, IMenuInteractionHandler};
use crate::primitives::{Popup, SelectingItemsControlImpl, TemplatedControlImpl};
use crate::{ControlImpl, ItemsControlImpl, MenuBase, MenuBaseImpl, MenuItem};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElement,
    StyledElementImpl, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::rc::Rc;

/// The control that hosts the items of a menu flyout.
#[repr(C)]
pub struct MenuFlyoutPresenter {
    base: MenuBase,
}

ferro_class!(MenuFlyoutPresenter: MenuBase);
ferro_class_info!(MenuFlyoutPresenter { new: MenuFlyoutPresenter::new });

ferro_impl_classes!(
    MenuFlyoutPresenter: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ItemsControlImpl,
    SelectingItemsControlImpl
);

impl VisualImpl for MenuFlyoutPresenter {
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        let children = StyledElement::logical_children(this).to_vec();
        for i in children {
            if let Some(menu_item) = i.cast::<MenuItem>() {
                menu_item.set_is_sub_menu_open(false);
            }
        }
    }
}

impl MenuBaseImpl for MenuFlyoutPresenter {
    fn close(this: &Self) {
        // The default menu interaction handler calls this
        let host = this.find_logical_ancestor_of_type::<Popup>(false);
        if let Some(host) = host {
            this.set_selected_index(-1);
            host.set_is_open(false);
        }
    }

    fn open(_this: &Self) {
        panic!("Use MenuFlyout.ShowAt(Control) instead");
    }
}

impl MenuFlyoutPresenter {
    /// Creates the class data of a presenter that uses the default menu
    /// interaction handler of context menus; see
    /// [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self::construct_with(DefaultMenuInteractionHandler::new(true))
    }

    /// Creates the class data of a presenter that uses
    /// `menu_interaction_handler`.
    pub fn construct_with(menu_interaction_handler: Rc<dyn IMenuInteractionHandler>) -> Self {
        Self { base: MenuBase::construct_with(menu_interaction_handler) }
    }

    /// Creates a menu flyout presenter.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a menu flyout presenter that uses `menu_interaction_handler`.
    pub fn with_interaction_handler(menu_interaction_handler: Rc<dyn IMenuInteractionHandler>) -> Ref<Self> {
        instantiate(Self::construct_with(menu_interaction_handler))
    }
}
