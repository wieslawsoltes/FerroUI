// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.
// Native FerroUI property owner for RibbonSpace's snapshot icon-source consumers.
//! Source CreateIconElement/live forwarding is a separate platform API, not used
//! by the ribbon presenter or menu helper.
use ferroui_base::media::IBrush;
use ferroui_base::{
    FerroObject, FerroObjectImpl, FerroProperty, StyledProperty, ferro_class, ferro_class_info,
    ferro_impl_classes, ferro_properties,
};
use std::rc::Rc;

#[repr(C)]
pub struct IconSource {
    base: FerroObject,
}
ferro_class!(IconSource:FerroObject);
ferro_impl_classes!(IconSource:FerroObjectImpl);
ferro_class_info!(IconSource {markup:{namespace:"FerroRibbon.Compatibility",attributes:[UsableDuringInitialization(false)]}});
ferro_properties! {impl IconSource {
    pub fn foreground_property()->StyledProperty<Option<Rc<dyn IBrush>>> {FerroProperty::register::<Self,_>("Foreground",None)}
}}
impl IconSource {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }
    pub fn foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::foreground_property())
    }
    pub fn set_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::foreground_property(), value)
    }
}
