//! `IMenu` and the registry of the classes that implement it; see
//! [`crate::i_menu_element`].

use crate::i_menu_element::IMenuElement;
use crate::platform::IMenuInteractionHandler;
use crate::TopLevel;
use ferroui_base::{FerroObject, ObjectType, Ref, TypeInfo};
use std::cell::RefCell;
use std::rc::Rc;

/// Represents a menu or a context menu.
#[allow(dead_code)]
pub(crate) trait IMenu: IMenuElement {
    /// Gets the menu interaction handler.
    fn interaction_handler(&self) -> Rc<dyn IMenuInteractionHandler>;

    /// Gets a value indicating whether the menu is open.
    fn is_open(&self) -> bool;

    /// Gets the root of the visual tree, if the control is attached to a
    /// visual tree.
    fn top_level(&self) -> Option<Ref<TopLevel>>;
}

type Adapter = Rc<dyn Fn(&FerroObject) -> Option<Rc<dyn IMenu>>>;

thread_local! {
    static MENU_TYPES: RefCell<Vec<(&'static TypeInfo, Adapter)>> = const { RefCell::new(Vec::new()) };
}

/// Declares that class `T` (and the classes derived from it) implements
/// [`IMenu`] through the handle created by `adapter`.
pub(crate) fn register_menu<T: ObjectType>(adapter: fn(Ref<T>) -> Rc<dyn IMenu>) {
    MENU_TYPES.with(|types| {
        let mut types = types.borrow_mut();
        if !types.iter().any(|(type_, _)| *type_ == T::TYPE) {
            let adapter: Adapter =
                Rc::new(move |object| object.downcast_ref::<T>().map(|target| adapter(FerroObject::ref_of(target))));
            types.push((T::TYPE, adapter));
        }
    });
}

/// The object viewed as a menu, if its class implements [`IMenu`].
pub(crate) fn as_menu(object: &FerroObject) -> Option<Rc<dyn IMenu>> {
    let adapter = MENU_TYPES.with(|types| {
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
