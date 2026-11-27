//! Selected Uno single-line/null TextBox mapping, retaining native edit/focus/input services.
use ferroui_base::{
    FerroObjectImpl, Ref, StaticType, StyledElementImpl, StyledPropertyMetadata, TypeInfo,
    VisualImpl, ferro_class, ferro_class_info, ferro_impl_classes, input::InputElementImpl,
    instantiate, interactivity::InteractiveImpl, layout::LayoutableImpl,
};
use ferroui_controls::{
    ControlImpl, TextBox, TextBoxImpl, TextBoxImplExt, primitives::TemplatedControlImpl,
};
#[repr(C)]
pub struct RibbonInputTextBox {
    base: TextBox,
}
ferro_class!(RibbonInputTextBox:TextBox);
ferro_class_info!(RibbonInputTextBox{new:RibbonInputTextBox::new,markup:{namespace:"FerroRibbon.Internal"}});
ferro_impl_classes!(RibbonInputTextBox:FerroObjectImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl RibbonInputTextBox {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: TextBox::construct(),
        })
    }
    fn static_constructor() {
        TextBox::text_property()
            .override_metadata::<Self>(StyledPropertyMetadata::new(Some(Some(String::new()))));
        TextBox::accepts_return_property()
            .changed()
            .add_class_handler::<Self>(|this, _| {
                if !this.accepts_return() {
                    this.set_text(this.text().as_deref());
                }
            })
            .detach();
    }
}
impl StyledElementImpl for RibbonInputTextBox {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <TextBox as StaticType>::TYPE
    }
}
impl TextBoxImpl for RibbonInputTextBox {
    fn coerce_text(this: &Self, value: Option<String>) -> Option<String> {
        // Original Text setter normalizes null to empty. GetFirstLine scans UTF16
        // for CR/LF; the returned prefix is identical for valid native UTF8 text.
        let mut text = value.unwrap_or_default();
        if !this.accepts_return()
            && let Some(i) = text.find(['\r', '\n'])
        {
            text.truncate(i);
        }
        Self::parent_coerce_text(this, Some(text))
    }
}
