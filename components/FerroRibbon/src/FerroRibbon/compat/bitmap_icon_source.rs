// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.
//! Bitmap inputs for the ribbon's snapshots; SDK live factory remains absent.
use super::IconSource;
use ferroui_base::{
    FerroObjectImpl, FerroProperty, Ref, StyledProperty, ferro_class, ferro_class_info,
    ferro_impl_classes, ferro_properties, instantiate, utilities::Uri,
};
#[repr(C)]
pub struct BitmapIconSource {
    base: IconSource,
}
ferro_class!(BitmapIconSource:IconSource);
ferro_class_info!(BitmapIconSource {new:BitmapIconSource::new,markup:{namespace:"FerroRibbon.Compatibility",attributes:[UsableDuringInitialization(false)]}});
ferro_impl_classes!(BitmapIconSource:FerroObjectImpl);
ferro_properties! {impl BitmapIconSource {
    pub fn uri_source_property()->StyledProperty<Option<Uri>> {FerroProperty::register::<Self,_>("UriSource",None)}
    pub fn show_as_monochrome_property()->StyledProperty<bool> {FerroProperty::register::<Self,_>("ShowAsMonochrome",true)}
}}
impl BitmapIconSource {
    pub fn construct() -> Self {
        Self {
            base: IconSource::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn uri_source(&self) -> Option<Uri> {
        self.get_value(Self::uri_source_property())
    }
    pub fn set_uri_source(&self, value: Option<Uri>) {
        self.set_value(Self::uri_source_property(), value);
    }
    pub fn show_as_monochrome(&self) -> bool {
        self.get_value(Self::show_as_monochrome_property())
    }
    pub fn set_show_as_monochrome(&self, value: bool) {
        self.set_value(Self::show_as_monochrome_property(), value);
    }
}
