//! Source IRibbonItemOwner: native identity, metrics, catalog and invocation.
//! Owner backedges use native ElementRef; source GC rooting remains a separate limit.
use crate::primitives::RibbonMetrics;
use ferroribbon_core::{RibbonString, commands::RibbonCommandCatalog};
use ferroui_base::{BoxedValue, FerroObject, Ref};
use ferroui_controls::Control;
use std::{rc::Rc, sync::Arc};
pub trait IRibbonItemOwner {
    fn as_object(&self) -> Ref<FerroObject>;
    fn metrics(&self) -> Option<Arc<RibbonMetrics>>;
    fn owner_ribbon(&self) -> Option<Ref<super::Ribbon>>;
    fn command_catalog(&self) -> Option<Rc<RibbonCommandCatalog>>;
    fn on_item_invoked(
        &self,
        item: Ref<Control>,
        command_id: Option<RibbonString>,
        parameter: Option<BoxedValue>,
    );
}
impl PartialEq for dyn IRibbonItemOwner {
    fn eq(&self, other: &Self) -> bool {
        self.as_object() == other.as_object()
    }
}
ferroui_base::ferro_markup_type!(interface dyn IRibbonItemOwner as "IRibbonItemOwner" {
 namespace:"FerroRibbon.Controls",handles:[Rc<dyn IRibbonItemOwner>,Option<Rc<dyn IRibbonItemOwner>>],this:Rc<dyn IRibbonItemOwner>,
 properties:[OwnerRibbon:Option<Ref<super::Ribbon>>{get:|this:&Rc<dyn IRibbonItemOwner>|this.owner_ribbon()},CommandCatalog:Option<Rc<RibbonCommandCatalog>>{get:|this:&Rc<dyn IRibbonItemOwner>|this.command_catalog()},Metrics:Option<Arc<RibbonMetrics>>{get:|this:&Rc<dyn IRibbonItemOwner>|this.metrics()}],
});
