use ferroribbon::primitives::RibbonIconPresenter;
use ferroui_base::{
    FerroObject, FerroObjectImpl, FerroProperty, Ref, Size, StyledElementImpl, StyledProperty,
    VisualImpl, ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_base::{
    input::InputElementImpl,
    interactivity::InteractiveImpl,
    layout::{LayoutableImpl, LayoutableImplExt},
};
use ferroui_controls::{ControlImpl, PanelImpl};
use std::cell::Cell;

#[repr(C)]
pub struct ConsumerIcon {
    base: RibbonIconPresenter,
    measures: Cell<usize>,
}
ferro_class!(ConsumerIcon:RibbonIconPresenter);
ferro_class_info!(ConsumerIcon {new:ConsumerIcon::new,markup:{namespace:"FerroRibbon.Consumer",content:Children}});
ferro_impl_classes!(ConsumerIcon:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
impl ConsumerIcon {
    pub fn construct() -> Self {
        Self {
            base: RibbonIconPresenter::construct(),
            measures: Cell::new(0),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn measures(&self) -> usize {
        self.measures.get()
    }
}
impl LayoutableImpl for ConsumerIcon {
    fn measure_override(this: &Self, available: Size) -> Size {
        this.measures.set(this.measures.get() + 1);
        Self::parent_measure_override(this, available)
    }
}

#[repr(C)]
pub struct IconViewModel {
    base: FerroObject,
}
ferro_class!(IconViewModel:FerroObject);
ferro_class_info!(IconViewModel {new:IconViewModel::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(IconViewModel:FerroObjectImpl);
ferro_properties! {impl IconViewModel {
    pub fn icon_text_property()->StyledProperty<String> {FerroProperty::register::<IconViewModel,_>("IconText","AB".to_owned())}
    pub fn icon_size_property()->StyledProperty<f64> {FerroProperty::register::<IconViewModel,_>("IconSize",18.)}
}}
impl IconViewModel {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn icon_text(&self) -> String {
        self.get_value(Self::icon_text_property())
    }
    pub fn set_icon_text(&self, value: String) {
        self.set_value(Self::icon_text_property(), value)
    }
    pub fn icon_size(&self) -> f64 {
        self.get_value(Self::icon_size_property())
    }
    pub fn set_icon_size(&self, value: f64) {
        self.set_value(Self::icon_size_property(), value)
    }
}
