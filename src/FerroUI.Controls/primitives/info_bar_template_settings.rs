// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See ../NOTICE.md.

use crate::IconElement;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObject, FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};

/// The actual icon element used by the current template.
#[repr(C)]
pub struct InfoBarTemplateSettings {
    base: FerroObject,
}
ferro_class!(InfoBarTemplateSettings: FerroObject);
ferro_class_info!(InfoBarTemplateSettings { new: InfoBarTemplateSettings::new, markup: { namespace: "FerroUI.Controls.Primitives" } });
ferro_impl_classes!(InfoBarTemplateSettings: FerroObjectImpl);
ferro_properties! { impl InfoBarTemplateSettings {
    pub fn icon_element_property() -> StyledProperty<Option<Ref<IconElement>>> {
        FerroProperty::register::<InfoBarTemplateSettings, _>("IconElement", None)
    }
} }
impl InfoBarTemplateSettings {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn icon_element(&self) -> Option<Ref<IconElement>> {
        self.get_value(Self::icon_element_property())
    }
    pub fn set_icon_element(&self, value: Option<Ref<IconElement>>) {
        self.set_value(Self::icon_element_property(), value);
    }
}
