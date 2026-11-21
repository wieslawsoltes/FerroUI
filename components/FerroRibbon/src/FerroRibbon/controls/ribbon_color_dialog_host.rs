//! Existing public color-dialog host, now sharing native chrome with customization.
//! Copyright Wiesław Šoltés, MIT; source dialog chrome retains Microsoft MIT notices.
use super::RibbonDialogHost;
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{ContentControlImpl, ControlImpl, primitives::TemplatedControlImpl};
#[repr(C)]
pub struct RibbonColorDialogHost {
    base: RibbonDialogHost,
}
ferro_class!(RibbonColorDialogHost:RibbonDialogHost);
ferro_class_info!(RibbonColorDialogHost {new:RibbonColorDialogHost::new,markup:{namespace:"FerroRibbon.Controls",content:Content}});
ferro_impl_classes!(RibbonColorDialogHost:FerroObjectImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,ContentControlImpl,TemplatedControlImpl);
impl StyledElementImpl for RibbonColorDialogHost {
    fn style_key_override(_: &Self) -> &'static ferroui_base::TypeInfo {
        Self::TYPE
    }
}
impl RibbonColorDialogHost {
    pub fn construct() -> Self {
        Self {
            base: RibbonDialogHost::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
