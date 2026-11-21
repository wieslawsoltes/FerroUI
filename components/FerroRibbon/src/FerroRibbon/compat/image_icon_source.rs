// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.
//! Native shared image input for ribbon and menu snapshots.
use super::IconSource;
use ferroui_base::media::IImage;
use ferroui_base::{
    FerroObjectImpl, FerroProperty, Ref, StyledProperty, ferro_class, ferro_class_info,
    ferro_impl_classes, ferro_properties, instantiate,
};
use std::rc::Rc;
#[repr(C)]
pub struct ImageIconSource {
    base: IconSource,
}
ferro_class!(ImageIconSource:IconSource);
ferro_impl_classes!(ImageIconSource:FerroObjectImpl);
ferro_class_info!(ImageIconSource {new:ImageIconSource::new,markup:{namespace:"FerroRibbon.Compatibility",attributes:[UsableDuringInitialization(false)]}});
ferro_properties! {impl ImageIconSource {
    pub fn image_source_property()->StyledProperty<Option<Rc<dyn IImage>>> {FerroProperty::register::<Self,_>("ImageSource",None)}
}}
impl ImageIconSource {
    pub fn construct() -> Self {
        Self {
            base: IconSource::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn image_source(&self) -> Option<Rc<dyn IImage>> {
        self.get_value(Self::image_source_property())
    }
    pub fn set_image_source(&self, value: Option<Rc<dyn IImage>>) {
        self.set_value(Self::image_source_property(), value)
    }
}
