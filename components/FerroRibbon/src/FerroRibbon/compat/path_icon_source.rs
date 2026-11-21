// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.
//! Native shared geometry input for ribbon and menu snapshots.
use super::IconSource;
use ferroui_base::media::Geometry;
use ferroui_base::{
    FerroObjectImpl, FerroProperty, Ref, StyledProperty, ferro_class, ferro_class_info,
    ferro_impl_classes, ferro_properties, instantiate,
};
#[repr(C)]
pub struct PathIconSource {
    base: IconSource,
}
ferro_class!(PathIconSource:IconSource);
ferro_impl_classes!(PathIconSource:FerroObjectImpl);
ferro_class_info!(PathIconSource {new:PathIconSource::new,markup:{namespace:"FerroRibbon.Compatibility",attributes:[UsableDuringInitialization(false)]}});
ferro_properties! {impl PathIconSource {
    pub fn data_property()->StyledProperty<Option<Ref<Geometry>>> {FerroProperty::register::<Self,_>("Data",None)}
}}
impl PathIconSource {
    pub fn construct() -> Self {
        Self {
            base: IconSource::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn data(&self) -> Option<Ref<Geometry>> {
        self.get_value(Self::data_property())
    }
    pub fn set_data(&self, value: Option<Ref<Geometry>>) {
        self.set_value(Self::data_property(), value)
    }
}
