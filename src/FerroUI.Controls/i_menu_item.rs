//! `IMenuItem` and the registry of the classes that implement it; see
//! [`crate::i_menu_element`].

use crate::i_menu_element::IMenuElement;
use crate::MenuItemToggleType;
use ferroui_base::{FerroObject, ObjectType, Ref, TypeInfo};
use std::cell::RefCell;
use std::rc::Rc;

/// Represents a menu item.
#[allow(dead_code)]
pub(crate) trait IMenuItem: IMenuElement {
    /// Whether the menu item has a submenu.
    fn has_sub_menu(&self) -> bool;

    /// Gets a value indicating whether the mouse is currently over the menu
    /// item's submenu.
    fn is_pointer_over_sub_menu(&self) -> bool;

    /// Whether the submenu of the menu item is open.
    fn is_sub_menu_open(&self) -> bool;

    /// Opens or closes the submenu of the menu item.
    fn set_is_sub_menu_open(&self, value: bool);

    /// Whether the submenu the menu item is within should not close when
    /// the item is clicked.
    fn stays_open_on_click(&self) -> bool;

    /// Sets whether the submenu the menu item is within should not close
    /// when the item is clicked.
    fn set_stays_open_on_click(&self, value: bool);

    /// Whether the menu item is a top-level main menu item.
    fn is_top_level(&self) -> bool;

    /// Gets the parent [`IMenuElement`].
    fn parent(&self) -> Option<Rc<dyn IMenuElement>>;

    /// The toggle type of the menu item.
    fn toggle_type(&self) -> MenuItemToggleType;

    /// The radio group of the menu item.
    fn group_name(&self) -> Option<String>;

    /// Whether the menu item is checked.
    fn is_checked(&self) -> bool;

    /// Checks or unchecks the menu item.
    fn set_is_checked(&self, value: bool);

    /// Raises a click event on the menu item.
    fn raise_click(&self);
}

type Adapter = Rc<dyn Fn(&FerroObject) -> Option<Rc<dyn IMenuItem>>>;

thread_local! {
    static MENU_ITEM_TYPES: RefCell<Vec<(&'static TypeInfo, Adapter)>> = const { RefCell::new(Vec::new()) };
}

/// Declares that class `T` (and the classes derived from it) implements
/// [`IMenuItem`] through the handle created by `adapter`.
pub(crate) fn register_menu_item<T: ObjectType>(adapter: fn(Ref<T>) -> Rc<dyn IMenuItem>) {
    MENU_ITEM_TYPES.with(|types| {
        let mut types = types.borrow_mut();
        if !types.iter().any(|(type_, _)| *type_ == T::TYPE) {
            let adapter: Adapter =
                Rc::new(move |object| object.downcast_ref::<T>().map(|target| adapter(FerroObject::ref_of(target))));
            types.push((T::TYPE, adapter));
        }
    });
}

/// The object viewed as a menu item, if its class implements [`IMenuItem`].
pub(crate) fn as_menu_item(object: &FerroObject) -> Option<Rc<dyn IMenuItem>> {
    let adapter = MENU_ITEM_TYPES.with(|types| {
        let types = types.borrow();
        let mut current = Some(object.get_type());
        while let Some(type_) = current {
            if let Some((_, adapter)) = types.iter().find(|(candidate, _)| *candidate == type_) {
                return Some(adapter.clone());
            }
            current = type_.base_type();
        }
        None
    });
    adapter.and_then(|adapter| adapter(object))
}
