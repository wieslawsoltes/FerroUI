//! Source checked-state contract for command descriptor and item synchronization.
use ferroui_base::{FerroObject, Ref};
use std::rc::Rc;
pub trait IRibbonCheckable {
    fn as_object(&self) -> Ref<FerroObject>;
    fn is_checked(&self) -> Option<bool>;
    fn set_is_checked(&self, value: Option<bool>);
}
impl PartialEq for dyn IRibbonCheckable {
    fn eq(&self, other: &Self) -> bool {
        self.as_object() == other.as_object()
    }
}
ferroui_base::ferro_markup_type!(interface dyn IRibbonCheckable as "IRibbonCheckable" {
 namespace:"FerroRibbon.Controls",handles:[Rc<dyn IRibbonCheckable>,Option<Rc<dyn IRibbonCheckable>>],this:Rc<dyn IRibbonCheckable>,
 properties:[IsChecked:Option<bool>{get:|this:&Rc<dyn IRibbonCheckable>|this.is_checked(),set:|this:&Rc<dyn IRibbonCheckable>,value|this.set_is_checked(value)}],
});
