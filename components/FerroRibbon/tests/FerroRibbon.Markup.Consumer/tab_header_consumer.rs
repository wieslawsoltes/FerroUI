//! Downstream Header(Tab) construction and native virtual initialization.
use ferroribbon::controls::{RibbonTab, RibbonTabHeader};
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl, VisualImpl, ferro_class,
    ferro_class_info, ferro_impl_classes, input::InputElementImpl, instantiate,
    interactivity::InteractiveImpl, layout::LayoutableImpl,
};
use ferroui_controls::{
    ButtonImpl, ContentControlImpl, ControlImpl, primitives::TemplatedControlImpl,
};
use std::cell::Cell;

#[repr(C)]
pub struct ConsumerTabHeader {
    base: RibbonTabHeader,
    base_initialized: Cell<bool>,
}
ferro_class!(ConsumerTabHeader:RibbonTabHeader);
ferro_class_info!(ConsumerTabHeader {markup:{namespace:"FerroRibbon.Consumer",constructors:[(Ref<RibbonTab>)=>ConsumerTabHeader::new]}});
ferro_impl_classes!(ConsumerTabHeader:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl);
impl ConsumerTabHeader {
    pub fn new(tab: Ref<RibbonTab>) -> Ref<Self> {
        instantiate(Self {
            base: RibbonTabHeader::construct(tab),
            base_initialized: Cell::new(false),
        })
    }
    pub fn base_initialized(&self) -> bool {
        self.base_initialized.get()
    }
}
impl FerroObjectImpl for ConsumerTabHeader {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.base_initialized.set(
            this.content()
                .as_ref()
                .and_then(|v| v.downcast_ref::<String>())
                == this.tab().header().as_ref(),
        );
    }
}
