//! Markup configuration of the actual control-owned touch processor.
use ferroboard_core::input::KeyboardTouchProcessor;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;

/// Native identity handle used by FerroUI property values; it owns no gesture state.
#[derive(Clone)]
pub struct KeyboardTouchProcessorRef(Rc<KeyboardTouchProcessor>);
impl PartialEq for KeyboardTouchProcessorRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for KeyboardTouchProcessorRef {}
impl KeyboardTouchProcessorRef {
    pub fn from_original(value: Rc<KeyboardTouchProcessor>) -> Self {
        Self(value)
    }
    pub fn original(&self) -> Rc<KeyboardTouchProcessor> {
        self.0.clone()
    }
}

#[doc(hidden)]
pub struct KeyboardTouchProcessorMetadata;
ferroui_base::ferro_markup_type!(class KeyboardTouchProcessorMetadata as "KeyboardTouchProcessor" {
 namespace:"FerroBoard.Input",this:KeyboardTouchProcessorRef,
 handles:[KeyboardTouchProcessorRef,Option<KeyboardTouchProcessorRef>],
 properties:[
  PopupMaxColumns:i32{get:|t:&KeyboardTouchProcessorRef|t.0.popup_max_columns(),set:|t:&KeyboardTouchProcessorRef,value:i32|t.0.set_popup_max_columns(value)},
  PopupMinY:f64{get:|t:&KeyboardTouchProcessorRef|t.0.popup_min_y(),set:|t:&KeyboardTouchProcessorRef,value:f64|t.0.set_popup_min_y(value)},
  CursorStepDistance:f64{get:|t:&KeyboardTouchProcessorRef|t.0.cursor_step_distance(),set:|t:&KeyboardTouchProcessorRef,value:f64|t.0.set_cursor_step_distance(value)},
  DeleteStepDistance:f64{get:|t:&KeyboardTouchProcessorRef|t.0.delete_step_distance(),set:|t:&KeyboardTouchProcessorRef,value:f64|t.0.set_delete_step_distance(value)}
 ]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::KeyboardTouchProcessorMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register_all(&[KeyboardTouchProcessorMetadata::MARKUP]);
    ValueTypes::register_global(register_values);
}
fn register_values() {
    ValueTypes::register_nullable::<KeyboardTouchProcessorRef>();
    ferroui_base::__ferro_compiler_metadata! {
        ferroui_base::metadata::register_type_rust_paths(&[
            (||std::any::TypeId::of::<KeyboardTouchProcessorRef>(),
             "::ferroboard_core_ferro::KeyboardTouchProcessorRef")
        ]);
    }
}
