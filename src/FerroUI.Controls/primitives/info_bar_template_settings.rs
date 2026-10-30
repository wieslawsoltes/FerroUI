//! WinUI's `InfoBarTemplateSettings` (`controls/dev/InfoBar/InfoBarTemplateSettings.cpp`, `.h` and
//! `InfoBar.idl` of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT,
//! Copyright (c) Microsoft Corporation, see NOTICE.md).

use crate::IconElement;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};

/// The values the template of an [`InfoBar`](crate::InfoBar) binds to: the icon element created
/// from its `IconSource` (`TemplateSettings.IconElement`, the child of the `UserIconBox` of the
/// template). Each bar has its own.
#[repr(C)]
pub struct InfoBarTemplateSettings {
    base: FerroObject,
}

ferro_class!(InfoBarTemplateSettings: FerroObject);
ferro_class_info!(InfoBarTemplateSettings {
    new: InfoBarTemplateSettings::new,
    markup: { namespace: "FerroUI.Controls.Primitives" },
});
ferro_impl_classes!(InfoBarTemplateSettings: FerroObjectImpl);

ferro_properties! { impl InfoBarTemplateSettings {
    ferro_property!(
        /// Defines the `IconElement` property: the icon element of the icon source of the bar.
        pub fn icon_element_property() -> StyledProperty<Option<Ref<IconElement>>> {
            FerroProperty::register::<InfoBarTemplateSettings, _>("IconElement", None)
        }
    );
} }

impl InfoBarTemplateSettings {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates the settings of a bar.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the icon element of the icon source of the bar.
    pub fn icon_element(&self) -> Option<Ref<IconElement>> {
        self.get_value(Self::icon_element_property())
    }

    /// Sets the icon element of the icon source of the bar.
    pub fn set_icon_element(&self, value: Option<Ref<IconElement>>) {
        self.set_value(Self::icon_element_property(), value);
    }
}
