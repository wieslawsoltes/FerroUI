//! A container: it creates its handle with a builder and its markers by their default values,
//! in the body of a macro.

use ferroui_base::{ferro_class, ContentControl, Ref};

use crate::parts::{Handle, Marker};

#[repr(C)]
pub struct Item {
    base: ContentControl,
}

ferro_class!(Item: ContentControl);

impl Item {
    pub fn new() -> Ref<Item> {
        unimplemented!()
    }

    pub fn handle(&self) -> Ref<Handle> {
        Handle::builder().build()
    }

    pub fn markers(&self) -> Vec<Ref<Marker>> {
        vec![Marker::default(), Self::new_marker()]
    }

    fn new_marker() -> Ref<Marker> {
        unimplemented!()
    }
}
