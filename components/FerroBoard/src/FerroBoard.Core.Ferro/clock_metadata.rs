//! XAML carrier of an actual Rust clock callback, supplied at the composition root.
use ferroboard_core::editor::EditorClock;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;

#[derive(Clone)]
pub struct ClockRef(EditorClock);
impl ClockRef {
    pub fn new(callback: EditorClock) -> Self {
        Self(callback)
    }
    pub fn callback(&self) -> EditorClock {
        self.0.clone()
    }
}
impl PartialEq for ClockRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for ClockRef {}
#[doc(hidden)]
pub struct EditorClockMetadata;
ferroui_base::ferro_markup_type!(class EditorClockMetadata as "EditorClock" {
    namespace:"FerroBoard.Editor",this:ClockRef,handles:[ClockRef,Option<ClockRef>]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::EditorClockMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(EditorClockMetadata::MARKUP);
    ValueTypes::register_global(ValueTypes::register_nullable::<ClockRef>);
}
