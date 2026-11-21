//! Ribbon.cs sealed RibbonHandledEventArgs. Copyright Wiesław Šoltés, MIT.
use std::{cell::Cell, rc::Rc};
#[derive(Default)]
pub struct RibbonHandledEventArgs {
    handled: Cell<bool>,
}
impl RibbonHandledEventArgs {
    pub fn new() -> Rc<Self> {
        ferroui_base::data::core::ValueTypes::register_reference::<Self>();
        Rc::new(Self::default())
    }
    pub fn handled(&self) -> bool {
        self.handled.get()
    }
    pub fn set_handled(&self, value: bool) {
        self.handled.set(value);
    }
}
impl PartialEq for RibbonHandledEventArgs {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class RibbonHandledEventArgs { namespace:"FerroRibbon.Controls",this:Rc<RibbonHandledEventArgs>,handles:[RibbonHandledEventArgs,Rc<RibbonHandledEventArgs>],constructors:[()=>RibbonHandledEventArgs::new],properties:[Handled:bool{get:|a:&Rc<RibbonHandledEventArgs>|a.handled(),set:|a:&Rc<RibbonHandledEventArgs>,v|a.set_handled(v)}]});
