//! WinUI's `TitleBarTemplateSettings` (`controls/dev/TitleBar/TitleBarTemplateSettings.cpp`, `.h`
//! and `TitleBar.idl` of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`;
//! MIT, Copyright (c) Microsoft Corporation, see NOTICE.md).

use crate::IconElement;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};

/// The values the template of a [`TitleBar`](crate::TitleBar) binds to: the icon element created
/// from its `IconSource`. Each title bar has its own.
#[repr(C)]
pub struct TitleBarTemplateSettings {
    base: FerroObject,
}

ferro_class!(TitleBarTemplateSettings: FerroObject);
ferro_class_info!(TitleBarTemplateSettings {
    new: TitleBarTemplateSettings::new,
    markup: { namespace: "FerroUI.Controls.Primitives" },
});
ferro_impl_classes!(TitleBarTemplateSettings: FerroObjectImpl);

ferro_properties! { impl TitleBarTemplateSettings {
    ferro_property!(
        /// Defines the `IconElement` property: the icon of the title bar, created from its
        /// `IconSource`.
        pub fn icon_element_property() -> StyledProperty<Option<Ref<IconElement>>> {
            FerroProperty::register::<TitleBarTemplateSettings, _>("IconElement", None)
        }
    );
} }

impl TitleBarTemplateSettings {
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
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
