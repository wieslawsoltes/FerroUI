// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See ../NOTICE.md.

use crate::IconElement;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    CornerRadius, FerroObject, FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};

/// Live values supplied to an InfoBadge template. Each badge owns its own settings.
#[repr(C)]
pub struct InfoBadgeTemplateSettings {
    base: FerroObject,
}
ferro_class!(InfoBadgeTemplateSettings: FerroObject);
ferro_class_info!(InfoBadgeTemplateSettings {
    new: InfoBadgeTemplateSettings::new,
    markup: { namespace: "FerroUI.Controls.Primitives" },
});
ferro_impl_classes!(InfoBadgeTemplateSettings: FerroObjectImpl);
ferro_properties! { impl InfoBadgeTemplateSettings {
    pub fn info_badge_corner_radius_property() -> StyledProperty<CornerRadius> {
        FerroProperty::register::<InfoBadgeTemplateSettings, _>("InfoBadgeCornerRadius", CornerRadius::default())
    }
    pub fn icon_element_property() -> StyledProperty<Option<Ref<IconElement>>> {
        FerroProperty::register::<InfoBadgeTemplateSettings, _>("IconElement", None)
    }
} }
impl InfoBadgeTemplateSettings {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn info_badge_corner_radius(&self) -> CornerRadius {
        self.get_value(Self::info_badge_corner_radius_property())
    }
    pub fn set_info_badge_corner_radius(&self, value: CornerRadius) {
        self.set_value(Self::info_badge_corner_radius_property(), value);
    }
    pub fn icon_element(&self) -> Option<Ref<IconElement>> {
        self.get_value(Self::icon_element_property())
    }
    pub fn set_icon_element(&self, value: Option<Ref<IconElement>>) {
        self.set_value(Self::icon_element_property(), value);
    }
}
