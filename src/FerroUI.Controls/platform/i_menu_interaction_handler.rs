use super::DefaultMenuInteractionHandler;
use crate::MenuBase;
use ferroui_base::Ref;

/// Handles user interaction for menus.
pub trait IMenuInteractionHandler {
    /// Attaches the interaction handler to a menu.
    fn attach(&self, menu: &Ref<MenuBase>);

    /// Detaches the interaction handler from the attached menu.
    fn detach(&self, menu: &Ref<MenuBase>);

    /// The handler viewed as the default menu interaction handler, if it is
    /// one.
    fn as_default_menu_interaction_handler(&self) -> Option<&DefaultMenuInteractionHandler> {
        None
    }
}
