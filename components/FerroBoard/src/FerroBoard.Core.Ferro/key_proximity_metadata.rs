//! Native XAML carrier for a Rust callback. Closures are supplied at the composition root.
use ferroboard_core::text::KeyProximity;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;

#[derive(Clone)]
pub struct KeyProximityRef(KeyProximity);
impl KeyProximityRef {
    pub fn new(callback: KeyProximity) -> Self {
        Self(callback)
    }
    pub fn callback(&self) -> KeyProximity {
        self.0.clone()
    }
}
impl PartialEq for KeyProximityRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for KeyProximityRef {}
#[doc(hidden)]
pub struct KeyProximityMetadata;
ferroui_base::ferro_markup_type!(class KeyProximityMetadata as "KeyProximity" {
    namespace:"FerroBoard.Text",this:KeyProximityRef,
    handles:[KeyProximityRef,Option<KeyProximityRef>]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::KeyProximityMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(KeyProximityMetadata::MARKUP);
    ValueTypes::register_global(register_values);
}
fn register_values() {
    ValueTypes::register_nullable::<KeyProximityRef>();
    ferroui_base::__ferro_compiler_metadata! {
        ferroui_base::metadata::register_type_rust_paths(&[
            (||std::any::TypeId::of::<KeyProximityRef>(),"::ferroboard_core_ferro::KeyProximityRef")
        ]);
    }
}
