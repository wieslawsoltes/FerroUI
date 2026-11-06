//! Native source LayoutComputer API over the original Core object.
use ferroboard_core::{
    keys::KeyboardMode,
    layouts::{
        ComponentName, ComputedKeyboard, KeyboardResources, LayoutComputer, ResourceError, Subtype,
    },
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
};
use std::{rc::Rc, sync::Arc};

fn failure(value: ResourceError) -> MarkupInvokeError {
    crate::resource_error_metadata::failure(value)
}

#[doc(hidden)]
pub struct LayoutComputerMetadata;
ferroui_base::ferro_markup_type!(class LayoutComputerMetadata as "LayoutComputer" {
 namespace:"FerroBoard.Layouts",this:Rc<LayoutComputer>,handles:[LayoutComputer,Rc<LayoutComputer>,Option<Rc<LayoutComputer>>],attributes:[Sealed],
 constructors:[(resources:Option<Rc<KeyboardResources>>)=>(|resources|Rc::new(LayoutComputer::new_nullable(resources)))],
 properties:[
 Resources:Option<Rc<KeyboardResources>>{get:|v:&Rc<LayoutComputer>|v.resources_nullable()},
 DefaultModifier:ComponentName{get:|v:&Rc<LayoutComputer>|v.default_modifier(),set:|v:&Rc<LayoutComputer>,value|v.set_default_modifier(value)}
 ],methods:[
 try fn Compute(KeyboardMode,Option<Arc<Subtype>>)->Rc<ComputedKeyboard> => (|v:&Rc<LayoutComputer>,mode,subtype|{let number_row=false;v.compute_nullable(mode,subtype,number_row).map(Rc::new).map_err(failure)}),
 try fn Compute(KeyboardMode,Option<Arc<Subtype>>,bool)->Rc<ComputedKeyboard> =>
 (|v:&Rc<LayoutComputer>,mode,subtype,number_row|v.compute_nullable(mode,subtype,number_row).map(Rc::new).map_err(failure))
 ]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::LayoutComputerMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(LayoutComputerMetadata::MARKUP);
    ValueTypes::register_global(ValueTypes::register_reference::<LayoutComputer>);
}
