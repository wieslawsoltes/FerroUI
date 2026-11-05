//! WinUI's `PagerControlTemplateSettings` (`controls/dev/PagerControl/PagerControlTemplateSettings.cpp`,
//! `.h` and `PagerControl.idl` of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`;
//! MIT, Copyright (c) Microsoft Corporation, see NOTICE.md).

use crate::Control;
use ferroui_base::collections::FerroList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, DirectProperty,
    DirectPropertyMetadata, FerroObject, FerroObjectImpl, FerroProperty, Ref,
};

/// The values the template of a [`PagerControl`](crate::PagerControl) binds to: the page
/// numbers of its combo box (`Pages`, 1 to the number of pages) and the buttons and ellipses
/// of its number panel (`NumberPanelItems`). Each pager has its own; the pager fills them.
#[repr(C)]
pub struct PagerControlTemplateSettings {
    base: FerroObject,
    pages: FerroList<i32>,
    number_panel_items: FerroList<Ref<Control>>,
}

ferro_class!(PagerControlTemplateSettings: FerroObject);
ferro_class_info!(PagerControlTemplateSettings {
    new: PagerControlTemplateSettings::new,
    markup: { namespace: "FerroUI.Controls.Primitives" },
});
ferro_impl_classes!(PagerControlTemplateSettings: FerroObjectImpl);

ferro_properties! { impl PagerControlTemplateSettings {
    /// Defines the `Pages` property: the page numbers of the combo box.
    pub fn pages_property() -> DirectProperty<PagerControlTemplateSettings, FerroList<i32>> {
        FerroProperty::register_direct_with::<PagerControlTemplateSettings, _>(
            "Pages",
            |o| o.pages(),
            None,
            DirectPropertyMetadata::new(None),
        )
    }

    /// Defines the `NumberPanelItems` property: the buttons and ellipses of the number panel.
    pub fn number_panel_items_property() -> DirectProperty<PagerControlTemplateSettings, FerroList<Ref<Control>>> {
        FerroProperty::register_direct_with::<PagerControlTemplateSettings, _>(
            "NumberPanelItems",
            |o| o.number_panel_items(),
            None,
            DirectPropertyMetadata::new(None),
        )
    }
} }

impl PagerControlTemplateSettings {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), pages: FerroList::new(), number_panel_items: FerroList::new() }
    }

    /// Creates the settings of a pager.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the page numbers of the combo box: 1 to the number of pages, or the first hundreds
    /// while the number of pages is not known.
    pub fn pages(&self) -> FerroList<i32> {
        self.pages.clone()
    }

    /// Gets the buttons and ellipses of the number panel.
    pub fn number_panel_items(&self) -> FerroList<Ref<Control>> {
        self.number_panel_items.clone()
    }
}
