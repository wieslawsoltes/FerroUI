//! Native action references and the existing shared Vec for actual XAML consumers.
use crate::{SmartbarAction, themes::ThemeList};
use ferroboard_core::{keys::KeyIcon, text::Utf16Text};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;

/// Comparable native UI handle; callbacks and domain data stay in the original action.
#[derive(Clone)]
pub struct SmartbarActionRef(Rc<SmartbarAction>);
impl PartialEq for SmartbarActionRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for SmartbarActionRef {}
impl SmartbarActionRef {
    pub fn from_original(value: Rc<SmartbarAction>) -> Self {
        Self(value)
    }
    pub fn original(&self) -> Rc<SmartbarAction> {
        self.0.clone()
    }
}

#[doc(hidden)]
pub struct SmartbarActionMetadata;
#[doc(hidden)]
pub struct SmartbarActionsMetadata;

// Callbacks are supplied through the public Rust SmartbarAction constructor.
// Metadata reads those same action objects; it does not serialize closures.
ferroui_base::ferro_markup_type!(class SmartbarActionMetadata as "SmartbarAction" {
 namespace:"FerroBoard.Controls.Views",this:SmartbarActionRef,
 handles:[SmartbarActionRef,Option<SmartbarActionRef>],
 properties:[
  Id:Option<String>{try_get:|a:&SmartbarActionRef|a.0.id.as_ref().map(Utf16Text::to_utf8).transpose()},
  Icon:KeyIcon{get:|a:&SmartbarActionRef|a.0.icon},
  Name:Option<String>{try_get:|a:&SmartbarActionRef|a.0.name.as_ref().map(Utf16Text::to_utf8).transpose()}
 ]
});
// Add is the native XAML collection adder over Vec::push; no CLR list facade.
ferroui_base::ferro_markup_type!(class SmartbarActionsMetadata as "SmartbarActions" {
 namespace:"FerroBoard.Controls",this:ThemeList<Option<Rc<SmartbarAction>>>,
 handles:[ThemeList<Option<Rc<SmartbarAction>>>,Option<ThemeList<Option<Rc<SmartbarAction>>>>],
 properties:[Count:i32{try_get:|items:&ThemeList<Option<Rc<SmartbarAction>>>|
    i32::try_from(items.lock().unwrap_or_else(std::sync::PoisonError::into_inner).len())}],
 methods:[fn Add(Option<SmartbarActionRef>)=>(|items:&ThemeList<Option<Rc<SmartbarAction>>>,item:Option<SmartbarActionRef>|
    items.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(item.map(|a|a.original())))]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::SmartbarActionMetadata,crate::SmartbarActionsMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register_all(&[
        SmartbarActionMetadata::MARKUP,
        SmartbarActionsMetadata::MARKUP,
    ]);
    ValueTypes::register_global(register_values);
}
fn register_values() {
    ValueTypes::register_nullable::<SmartbarActionRef>();
    ValueTypes::register_nullable::<ThemeList<Option<Rc<SmartbarAction>>>>();
    ferroui_base::__ferro_compiler_metadata! {
        ferroui_base::metadata::register_type_rust_paths(&[
            (||std::any::TypeId::of::<SmartbarActionRef>(),
             "::ferroboard_controls::SmartbarActionRef"),
            (||std::any::TypeId::of::<ThemeList<Option<Rc<SmartbarAction>>>>(),
             "::ferroboard_controls::themes::ThemeList<Option<::std::rc::Rc<::ferroboard_controls::SmartbarAction>>>")
        ]);
    }
}
