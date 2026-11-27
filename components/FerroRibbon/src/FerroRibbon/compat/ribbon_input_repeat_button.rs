//! Source Uno RepeatButton defaults; native repeat/capture/input implementation.
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, StyledPropertyMetadata, TypeInfo, VisualImpl,
    ferro_class, ferro_class_info, ferro_impl_classes, input::InputElementImpl, instantiate,
    interactivity::InteractiveImpl, layout::LayoutableImpl,
};
use ferroui_controls::{
    Button, ButtonImpl, ClickMode, ContentControlImpl, ControlImpl, RepeatButton,
    primitives::TemplatedControlImpl,
};
#[repr(C)]
pub struct RibbonInputRepeatButton {
    base: RepeatButton,
}
ferro_class!(RibbonInputRepeatButton:RepeatButton);
ferro_class_info!(RibbonInputRepeatButton{new:RibbonInputRepeatButton::new,markup:{namespace:"FerroRibbon.Internal"}});
ferro_impl_classes!(RibbonInputRepeatButton:FerroObjectImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl);
impl RibbonInputRepeatButton {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RepeatButton::construct(),
        })
    }
    fn static_constructor() {
        RepeatButton::delay_property()
            .override_metadata::<Self>(StyledPropertyMetadata::new(Some(250)));
        RepeatButton::interval_property()
            .override_metadata::<Self>(StyledPropertyMetadata::new(Some(250)));
        Button::click_mode_property()
            .override_metadata::<Self>(StyledPropertyMetadata::new(Some(ClickMode::Press)));
    }
}
impl StyledElementImpl for RibbonInputRepeatButton {
    fn style_key_override(_c: &Self) -> &'static TypeInfo {
        RepeatButton::TYPE
    }
}
