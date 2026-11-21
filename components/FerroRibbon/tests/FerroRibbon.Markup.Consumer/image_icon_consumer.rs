//! A downstream subclass and typed image binding use only the public dependency API.
use ferroribbon::compat::ImageIcon;
use ferroui_base::{
    FerroObject, FerroObjectImpl, FerroProperty, Ref, Size, StyledElementImpl, StyledProperty,
    VisualImpl, ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{LayoutableImpl, LayoutableImplExt},
    media::IImage,
};
use ferroui_controls::{ControlImpl, primitives::TemplatedControlImpl};
use std::{cell::Cell, rc::Rc};
#[repr(C)]
pub struct ConsumerImageIcon {
    base: ImageIcon,
    measures: Cell<usize>,
}
ferro_class!(ConsumerImageIcon:ImageIcon);
ferro_class_info!(ConsumerImageIcon {new:ConsumerImageIcon::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerImageIcon:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl ConsumerImageIcon {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: ImageIcon::construct(),
            measures: Cell::new(0),
        })
    }
    pub fn measures(&self) -> usize {
        self.measures.get()
    }
}
impl LayoutableImpl for ConsumerImageIcon {
    fn measure_override(this: &Self, available: Size) -> Size {
        this.measures.set(this.measures.get() + 1);
        Self::parent_measure_override(this, available)
    }
}
#[repr(C)]
pub struct ImageViewModel {
    base: FerroObject,
}
ferro_class!(ImageViewModel:FerroObject);
ferro_class_info!(ImageViewModel {new:ImageViewModel::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ImageViewModel:FerroObjectImpl);
ferro_properties! {impl ImageViewModel {
    pub fn image_property()->StyledProperty<Option<Rc<dyn IImage>>> {
        FerroProperty::register::<Self,_>("Image",None)
    }
}}
impl ImageViewModel {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: FerroObject::construct(),
        })
    }
    pub fn image(&self) -> Option<Rc<dyn IImage>> {
        self.get_value(Self::image_property())
    }
    pub fn set_image(&self, value: Option<Rc<dyn IImage>>) {
        self.set_value(Self::image_property(), value);
    }
}
