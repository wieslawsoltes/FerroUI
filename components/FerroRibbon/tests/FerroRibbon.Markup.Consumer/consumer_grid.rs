use ferroribbon::primitives::RibbonUniformGridPanel;
use ferroui_base::{
    FerroObjectImpl, Ref, Size, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, instantiate,
};
use ferroui_base::{
    input::InputElementImpl,
    interactivity::InteractiveImpl,
    layout::{LayoutableImpl, LayoutableImplExt},
};
use ferroui_controls::{ControlImpl, PanelImpl};
#[repr(C)]
pub struct ConsumerGrid {
    base: RibbonUniformGridPanel,
}
ferro_class!(ConsumerGrid:RibbonUniformGridPanel);
ferro_impl_classes!(ConsumerGrid:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_class_info!(ConsumerGrid {new:ConsumerGrid::new,markup:{namespace:"FerroRibbon.Consumer",content:Children,},});
impl ConsumerGrid {
    pub fn construct() -> Self {
        Self {
            base: RibbonUniformGridPanel::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
impl LayoutableImpl for ConsumerGrid {
    fn measure_override(this: &Self, available: Size) -> Size {
        let size = Self::parent_measure_override(this, available);
        Size::new(size.width + 1., size.height)
    }
}
