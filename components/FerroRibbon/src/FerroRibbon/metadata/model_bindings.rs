//! Reusable scope lifecycle metadata.
use crate::mvvm::{RibbonAction, RibbonModelBindings};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupDelegate, MarkupType, MarkupTyped},
};
use std::rc::Rc;
pub struct RibbonModelBindingsMetadata;
fn action(value: Option<MarkupDelegate>) -> Option<RibbonAction> {
    value.map(|d| {
        Rc::new(move || {
            d.try_invoke(&[]).expect("RibbonModelBindings callback");
        }) as RibbonAction
    })
}
ferroui_base::ferro_markup_type!(class RibbonModelBindingsMetadata as "RibbonModelBindings"{
 namespace:"FerroRibbon.Controls.Mvvm",this:Rc<RibbonModelBindings>,handles:[RibbonModelBindings,Rc<RibbonModelBindings>,Option<Rc<RibbonModelBindings>>],
 constructors:[()=>RibbonModelBindings::new],
 properties:[IsDisposed:bool{get:|v:&Rc<RibbonModelBindings>|v.is_disposed()},IsSuspended:bool{get:|v:&Rc<RibbonModelBindings>|v.is_suspended()}],
 methods:[fn Add(Option<MarkupDelegate>)=>|v:&Rc<RibbonModelBindings>,f|v.add(action(f)),fn Child()->Rc<RibbonModelBindings> =>RibbonModelBindings::child,fn RequestRecreate()=>|v:&Rc<RibbonModelBindings>|v.request_recreate(),fn Suspend()=>|v:&Rc<RibbonModelBindings>|v.suspend(),fn Resume()=>|v:&Rc<RibbonModelBindings>|v.resume(),fn Dispose()=>|v:&Rc<RibbonModelBindings>|v.close()]
});
pub(crate) fn register() {
    ValueTypes::register_reference::<RibbonModelBindings>();
    MarkupType::register_all(&[RibbonModelBindingsMetadata::MARKUP]);
}
