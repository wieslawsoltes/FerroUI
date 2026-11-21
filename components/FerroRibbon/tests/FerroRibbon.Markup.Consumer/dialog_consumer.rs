//! An independent subclass exercises native inheritance and original part names.
use ferroribbon::controls::RibbonColorDialogHost;
use ferroui_base::{
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, VisualImpl,
    ferro_class, ferro_class_info, ferro_impl_classes, input::InputElementImpl, instantiate,
    interactivity::InteractiveImpl, layout::LayoutableImpl,
};
use ferroui_controls::{ContentControlImpl, ControlImpl, primitives::TemplatedControlImpl};
#[repr(C)]
pub struct ConsumerDialogHost {
    base: RibbonColorDialogHost,
}
ferro_class!(ConsumerDialogHost:RibbonColorDialogHost);
ferro_class_info!(ConsumerDialogHost {new:ConsumerDialogHost::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerDialogHost:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,ContentControlImpl,TemplatedControlImpl);
ferroui_base::ferro_properties! {impl ConsumerDialogHost {
    pub fn optional_default_button_property()->StyledProperty<Option<ferroribbon::compat::ContentDialogButton>> {
        FerroProperty::register::<Self,_>("OptionalDefaultButton",None)
    }
}}
impl ConsumerDialogHost {
    pub fn optional_default_button(&self) -> Option<ferroribbon::compat::ContentDialogButton> {
        self.get_value(Self::optional_default_button_property())
    }
    pub fn set_optional_default_button(
        &self,
        value: Option<ferroribbon::compat::ContentDialogButton>,
    ) {
        self.set_value(Self::optional_default_button_property(), value)
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonColorDialogHost::construct(),
        })
    }
}
