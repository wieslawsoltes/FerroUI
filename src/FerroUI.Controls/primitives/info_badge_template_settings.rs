//! WinUI's `InfoBadgeTemplateSettings` (`controls/dev/InfoBadge/InfoBadgeTemplateSettings.cpp`,
//! `.h` and `InfoBadge.idl` of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`;
//! MIT, Copyright (c) Microsoft Corporation, see NOTICE.md).

use crate::IconElement;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property, instantiate, CornerRadius,
    FerroObject, FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};

/// The values the template of an [`InfoBadge`](crate::InfoBadge) binds to: the corner radius of
/// the badge (half its height unless the badge has a local `CornerRadius`) and the icon element
/// created from its `IconSource`. Each badge has its own.
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
    ferro_property!(
        /// Defines the `InfoBadgeCornerRadius` property: the corner radius of the badge.
        pub fn info_badge_corner_radius_property() -> StyledProperty<CornerRadius> {
            FerroProperty::register::<InfoBadgeTemplateSettings, _>("InfoBadgeCornerRadius", CornerRadius::default())
        }
    );

    ferro_property!(
        /// Defines the `IconElement` property: the icon element of the icon source of the badge.
        pub fn icon_element_property() -> StyledProperty<Option<Ref<IconElement>>> {
            FerroProperty::register::<InfoBadgeTemplateSettings, _>("IconElement", None)
        }
    );
} }

impl InfoBadgeTemplateSettings {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates the settings of a badge.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the corner radius of the badge.
    pub fn info_badge_corner_radius(&self) -> CornerRadius {
        self.get_value(Self::info_badge_corner_radius_property())
    }

    /// Sets the corner radius of the badge.
    pub fn set_info_badge_corner_radius(&self, value: CornerRadius) {
        self.set_value(Self::info_badge_corner_radius_property(), value);
    }

    /// Gets the icon element of the icon source of the badge.
    pub fn icon_element(&self) -> Option<Ref<IconElement>> {
        self.get_value(Self::icon_element_property())
    }

    /// Sets the icon element of the icon source of the badge.
    pub fn set_icon_element(&self, value: Option<Ref<IconElement>>) {
        self.set_value(Self::icon_element_property(), value);
    }
}
