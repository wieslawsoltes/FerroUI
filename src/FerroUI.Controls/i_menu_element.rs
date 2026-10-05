//! `IMenuElement`: a menu or a menu item, as the menu interaction handler
//! sees it.
//!
//! A class cannot implement the trait itself (its methods would shadow the
//! inherent ones of the class): it provides a small adapter handle that
//! does, and makes it known with `register_menu` / `register_menu_item`,
//! normally from its static constructor. [`as_menu_element`] then views an
//! object of that class (or of a class derived from it) as a menu element.

use crate::i_menu::{as_menu, IMenu};
use crate::i_menu_item::{as_menu_item, IMenuItem};
use ferroui_base::input::{IMainMenu, InputElement, KeyModifiers, NavigationDirection, NavigationMethod};
use ferroui_base::{FerroObject, Ref};
use std::rc::Rc;

/// Represents an [`IMenu`] or [`IMenuItem`].
pub(crate) trait IMenuElement {
    /// The element behind the handle: the input element and logical tree
    /// node the interface is implemented by.
    fn element(&self) -> Ref<InputElement>;

    /// Gets the currently selected submenu item.
    fn selected_item(&self) -> Option<Rc<dyn IMenuItem>>;

    /// Sets the currently selected submenu item.
    fn set_selected_item(&self, value: Option<Rc<dyn IMenuItem>>);

    /// Gets the submenu items.
    fn sub_items(&self) -> Vec<Rc<dyn IMenuItem>>;

    /// Opens the menu or menu item.
    fn open(&self);

    /// Closes the menu or menu item.
    fn close(&self);

    /// Moves the submenu selection in the specified direction.
    ///
    /// `wrap` is whether to wrap after the first or last item. Returns true
    /// if the selection was moved; otherwise false.
    fn move_selection(&self, direction: NavigationDirection, wrap: bool) -> bool;

    /// Focuses the element: the `Focus` member of the input element
    /// contract the interface extends. Returns whether it was focused.
    fn focus_with(&self, method: NavigationMethod, key_modifiers: KeyModifiers) -> bool {
        self.element().focus_with(method, key_modifiers)
    }

    /// The element viewed as a menu, if it is one.
    fn as_menu(&self) -> Option<Rc<dyn IMenu>> {
        None
    }

    /// The element viewed as a menu item, if it is one.
    fn as_menu_item(&self) -> Option<Rc<dyn IMenuItem>> {
        None
    }

    /// The element viewed as a main menu, if it is one.
    fn as_main_menu(&self) -> Option<Rc<dyn IMainMenu>> {
        None
    }
}

/// The object viewed as a menu element, if its class implements [`IMenu`]
/// or [`IMenuItem`].
#[allow(dead_code)]
pub(crate) fn as_menu_element(object: &FerroObject) -> Option<Rc<dyn IMenuElement>> {
    if let Some(item) = as_menu_item(object) {
        let element: Rc<dyn IMenuElement> = item;
        return Some(element);
    }

    as_menu(object).map(|menu| {
        let element: Rc<dyn IMenuElement> = menu;
        element
    })
}

/// Whether two handles are views of the same element (reference equality of
/// the managed interface references).
pub(crate) fn same_menu_element(a: &dyn IMenuElement, b: &dyn IMenuElement) -> bool {
    a.element() == b.element()
}

/// Whether two optional handles are views of the same element, or both
/// absent.
pub(crate) fn same_menu_item(a: Option<&Rc<dyn IMenuItem>>, b: Option<&Rc<dyn IMenuItem>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.element() == b.element(),
        (None, None) => true,
        _ => false,
    }
}
