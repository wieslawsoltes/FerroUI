//! A list: it creates an item as the container of each value, a spacer by a path with its
//! module, a separator through a qualified type, and a class of another crate.

use ferroui_base::{ferro_class, Control, Ref};

use crate::item::Item;
use crate::parts::Separator;

#[repr(C)]
pub struct List {
    base: Control,
}

ferro_class!(List: Control);

impl List {
    pub fn create_container(&self) -> Ref<Control> {
        Item::new().upcast()
    }

    pub fn spacer(&self) -> Ref<crate::parts::Spacer> {
        crate::parts::Spacer::new_with_width(2.0)
    }

    pub fn separator(&self) -> Ref<Separator> {
        <Separator>::default()
    }

    /// The class itself is no edge.
    pub fn same(&self) -> Ref<List> {
        List::new()
    }

    /// A class of the crate the fixture is scanned with, when it is.
    pub fn decorator(&self) -> Ref<fixture::Decorator> {
        fixture::Decorator::new()
    }
}
