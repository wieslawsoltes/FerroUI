//! Native badge from shared RibbonKeyTip.cs; navigation remains in the Ribbon port.
use ferroui_base::input::{InputElementImpl, KeyboardNavigation};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    AttachedProperty, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref,
    StaticType, StyledElementImpl, StyledProperty, TypeInfo, VisualImpl, ferro_class,
    ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_controls::{
    Control, ControlImpl,
    primitives::{TemplatedControl, TemplatedControlImpl},
};
use std::cell::RefCell;
#[repr(C)]
pub struct RibbonKeyTip {
    base: TemplatedControl,
    pub(crate) target: RefCell<Option<Ref<Control>>>,
}
ferro_class!(RibbonKeyTip:TemplatedControl);
ferro_impl_classes!(RibbonKeyTip:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
ferro_class_info!(RibbonKeyTip {new:RibbonKeyTip::new,markup:{namespace:"FerroRibbon.Controls",properties:[Target:Option<Ref<Control>>{get:RibbonKeyTip::target}],methods:[
static fn GetKeyTip(Ref<FerroObject>)->Option<String> => |element:Ref<FerroObject>|RibbonKeyTip::get_key_tip(&element),
static fn SetKeyTip(Ref<FerroObject>,Option<String>) => |element:Ref<FerroObject>,value:Option<String>|RibbonKeyTip::set_key_tip(&element,value),
]}});
ferro_properties! {impl RibbonKeyTip {
    pub fn text_property()->StyledProperty<Option<String>> {FerroProperty::register::<RibbonKeyTip,_>("Text",Some(String::new()))}
    pub fn key_tip_property()->AttachedProperty<Option<String>> {FerroProperty::register_attached::<RibbonKeyTip,FerroObject,_>("KeyTip",None)}
}}
impl RibbonKeyTip {
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            target: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }
    pub fn set_text(&self, value: Option<String>) {
        self.set_value(Self::text_property(), value)
    }
    pub fn target(&self) -> Option<Ref<Control>> {
        self.target.borrow().clone()
    }
    pub fn get_key_tip(element: &FerroObject) -> Option<String> {
        element.get_value(Self::key_tip_property())
    }
    pub fn set_key_tip(element: &FerroObject, value: Option<String>) {
        element.set_value(Self::key_tip_property(), value)
    }
}
impl FerroObjectImpl for RibbonKeyTip {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_is_hit_test_visible(false);
        KeyboardNavigation::set_is_tab_stop(this, false);
    }
}
impl StyledElementImpl for RibbonKeyTip {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <Self as StaticType>::TYPE
    }
}
