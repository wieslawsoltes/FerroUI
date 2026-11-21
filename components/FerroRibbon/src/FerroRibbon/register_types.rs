use crate::controls::{
    IRibbonCommandSource, IRibbonLayoutHost, RibbonControlBase, RibbonItemHelper,
};
use crate::primitives::{
    IRibbonScrollContent, RibbonIconPresenter, RibbonItemContent, RibbonScrollPanel,
    RibbonUniformGridPanel,
};
use ferroui_base::metadata::{
    FERRO_XML_NAMESPACE, MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition, XmlnsPrefix,
};
use ferroui_base::{StaticType, TypeInfo};

const NAMESPACES: &[(&str, &str)] = &[
    ("ferroribbon::mvvm", "FerroRibbon.Controls.Mvvm"),
    (
        "ferroribbon::compat::ribbon_input_slider",
        "FerroRibbon.Internal",
    ),
    (
        "ferroribbon::compat::ribbon_input_repeat_button",
        "FerroRibbon.Internal",
    ),
    (
        "ferroribbon::compat::ribbon_input_text_box",
        "FerroRibbon.Internal",
    ),
    ("ferroribbon::compat", "FerroRibbon.Compatibility"),
    ("ferroribbon::primitives", "FerroRibbon.Controls.Primitives"),
    ("ferroribbon::controls", "FerroRibbon.Controls"),
    ("ferroribbon::theming", "FerroRibbon.Controls"),
];
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroRibbon",
    crate_name: "ferroribbon",
    xmlns_definitions: &[
        XmlnsDefinition {
            xml_namespace: "http://schemas.microsoft.com/winfx/2006/xaml/presentation/global",
            namespace: "FerroRibbon.Controls",
        },
        XmlnsDefinition {
            xml_namespace: "http://schemas.microsoft.com/winfx/2006/xaml/presentation/global",
            namespace: "FerroRibbon.Controls.Primitives",
        },
        XmlnsDefinition {
            xml_namespace: "http://schemas.microsoft.com/winfx/2006/xaml/presentation/global",
            namespace: "FerroRibbon.Controls.Mvvm",
        },
        XmlnsDefinition {
            xml_namespace: "https://github.com/wieslawsoltes/RibbonSpace",
            namespace: "FerroRibbon.Search",
        },
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "FerroRibbon.Search",
        },
        XmlnsDefinition {
            xml_namespace: "https://github.com/wieslawsoltes/RibbonSpace",
            namespace: "FerroRibbon.Controls.Mvvm",
        },
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "FerroRibbon.Controls.Mvvm",
        },
        XmlnsDefinition {
            xml_namespace: "https://github.com/wieslawsoltes/RibbonSpace",
            namespace: "FerroRibbon.State",
        },
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "FerroRibbon.State",
        },
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "FerroRibbon.Compatibility",
        },
        XmlnsDefinition {
            xml_namespace: "https://github.com/wieslawsoltes/RibbonSpace",
            namespace: "FerroRibbon.Compatibility",
        },
        XmlnsDefinition {
            xml_namespace: "https://github.com/wieslawsoltes/RibbonSpace",
            namespace: "FerroRibbon",
        },
        XmlnsDefinition {
            xml_namespace: "https://github.com/wieslawsoltes/RibbonSpace",
            namespace: "FerroRibbon.Controls",
        },
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "FerroRibbon",
        },
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "FerroRibbon.Controls",
        },
        XmlnsDefinition {
            xml_namespace: "https://github.com/wieslawsoltes/RibbonSpace",
            namespace: "FerroRibbon.Controls.Primitives",
        },
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "FerroRibbon.Controls.Primitives",
        },
        XmlnsDefinition {
            xml_namespace: "https://github.com/wieslawsoltes/RibbonSpace",
            namespace: "FerroRibbon.Theming",
        },
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "FerroRibbon.Theming",
        },
        XmlnsDefinition {
            xml_namespace: "https://github.com/wieslawsoltes/RibbonSpace",
            namespace: "FerroRibbon.Layout",
        },
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "FerroRibbon.Layout",
        },
    ],
    xmlns_prefixes: &[XmlnsPrefix {
        xml_namespace: "https://github.com/wieslawsoltes/RibbonSpace",
        prefix: "rs",
    }],
    metadata: &[],
};
ferroui_base::ferro_rust_paths! { classes:[crate::compat::BitmapIcon,crate::compat::BitmapIconSource,crate::compat::SymbolIcon,crate::compat::SymbolIconSource,crate::compat::IconSource,crate::compat::FontIconSource,crate::compat::PathIconSource,crate::compat::ImageIconSource,crate::controls::RibbonCustomizeDialog,crate::controls::RibbonDialogHost,crate::controls::RibbonApplicationMenu,crate::controls::RibbonApplicationMenuItem,crate::controls::RibbonApplicationMenuRecentItem,crate::controls::RibbonTitleBar,crate::controls::RibbonStatusBar,crate::controls::RibbonZoomControl,crate::controls::RibbonMenuBar,crate::controls::RibbonMenuBarItem,crate::controls::RibbonToolBar,crate::controls::RibbonContextualToolBar,crate::primitives::RibbonToolBarPanel,crate::controls::RibbonGallery,crate::controls::RibbonGalleryItem,crate::controls::RibbonColorDialogHost,crate::controls::RibbonColorPicker,crate::controls::RibbonColorPalette,crate::controls::RibbonSegment,crate::controls::RibbonSegmentedControl,crate::controls::RibbonGridPicker,crate::controls::RibbonBackstage,crate::controls::RibbonBackstageItem,crate::controls::RibbonSlider,crate::compat::RibbonInputSlider,crate::compat::RibbonInputRepeatButton,crate::controls::RibbonSpinner,crate::controls::RibbonComboBox,crate::controls::RibbonFontComboBox,crate::controls::RibbonFontSizeComboBox,crate::compat::RibbonInputTextBox,crate::controls::RibbonInputBase,crate::controls::RibbonTextBox,crate::controls::RibbonQuickAccessToolBar,crate::controls::RibbonToolBarAutomationPeer,crate::controls::RibbonAutomationPeer,crate::controls::RibbonGroupAutomationPeer,crate::controls::RibbonTabHeaderAutomationPeer,crate::controls::RibbonSplitButton,crate::controls::RibbonSplitButtonAutomationPeer,crate::compat::MenuFlyoutItem,crate::compat::ToggleMenuFlyoutItem,crate::compat::RadioMenuFlyoutItem,crate::compat::MenuFlyoutSubItem,crate::compat::MenuFlyoutSeparator,crate::controls::RibbonMenu,crate::controls::RibbonMenuCloner,crate::controls::RibbonToggleButton,crate::controls::RibbonCheckBox,crate::controls::Ribbon,crate::controls::RibbonTab,crate::controls::RibbonTabHeader,crate::controls::RibbonContextualTabGroup,crate::primitives::RibbonGroupsPanel,crate::controls::RibbonGroup,crate::controls::RibbonItemsContainer,crate::controls::RibbonButtonGroup,crate::controls::RibbonStackPanel,crate::controls::RibbonButton,crate::controls::RibbonDropDownButton,crate::controls::RibbonSeparator,crate::controls::RibbonLabel,crate::primitives::RibbonGroupItemsPanel,crate::controls::RibbonScreenTip,crate::controls::RibbonKeyTip,crate::theming::RibbonThemeResources,crate::compat::FontIcon,crate::compat::ImageIcon,crate::controls::RibbonItemHelper,crate::controls::RibbonControlBase,crate::primitives::RibbonUniformGridPanel, crate::primitives::RibbonScrollPanel,crate::primitives::RibbonIconPresenter,crate::primitives::RibbonItemContent],types:[crate::metadata::enums::SymbolMetadata,crate::controls::RibbonXmlns,crate::controls::RibbonDialogClosingEventArgs,crate::controls::RibbonDialogClosedEventArgs,crate::controls::RibbonDialogOpenedEventArgs,crate::controls::RibbonDialogButtonClickEventArgs,crate::controls::RibbonContextMenuEventArgs,crate::controls::RibbonCustomizeRequestedEventArgs,crate::controls::RibbonCustomizePage,crate::controls::RibbonDialogOutcome,crate::controls::RibbonApplicationMenuItemEventArgs,crate::metadata::application_menu_items::RibbonApplicationMenuItemsMetadata,crate::metadata::application_menu_items::RibbonApplicationMenuRecentItemsMetadata,crate::metadata::search::RibbonSearchEntryMetadata,crate::metadata::search::RibbonSearchResultMetadata,crate::metadata::search::RibbonSearchEngineMetadata,crate::metadata::search::RibbonSearchEntriesMetadata,crate::metadata::search::RibbonSearchResultsMetadata,crate::metadata::search::RibbonAdditionalSearchEntriesMetadata,crate::metadata::search::RibbonSearchStringsMetadata,crate::metadata::search::RibbonSearchEntryEnumerableMetadata,crate::metadata::search::RibbonSearchStringEnumerableMetadata,crate::metadata::menu_bar_items::RibbonMenuBarItemsMetadata,crate::metadata::orientation::OrientationMetadata,crate::metadata::toolbar_overflow::RibbonToolBarOverflowMetadata,crate::metadata::models::RibbonGalleryModelMetadata,crate::metadata::models::RibbonGalleryItemModelMetadata,crate::metadata::models::ModelGalleryItemsMetadata,crate::controls::RibbonGalleryItemEventArgs,crate::metadata::content_dialog_button::ContentDialogButtonMetadata,crate::metadata::models::RibbonColorPickerModelMetadata,crate::metadata::models::RibbonCustomItemModelMetadata,crate::metadata::models::ModelColorsMetadata,crate::metadata::models::ModelSwatchesMetadata,crate::metadata::colors::RibbonColorSwatchMetadata,crate::controls::RibbonColorDialog,crate::metadata::models::RibbonSegmentModelMetadata,crate::metadata::models::RibbonSegmentedModelMetadata,crate::metadata::models::ModelSegmentsMetadata,crate::metadata::segments::RibbonSegmentsMetadata,crate::metadata::grid_size::RibbonGridSizeMetadata,crate::metadata::models::RibbonGridPickerModelMetadata,crate::metadata::model_icon::RibbonIconLayersMetadata,crate::metadata::model_icon::RibbonIconLayerListMetadata,crate::metadata::model_icon::RibbonIconMetadata,crate::metadata::model_icon::RibbonIconLayerMetadata,crate::metadata::model_screen_tip::RibbonScreenTipMetadata,crate::metadata::models::ObservableObjectMetadata,crate::metadata::models::RibbonNodeModelMetadata,crate::metadata::models::RibbonItemModelMetadata,crate::metadata::models::RibbonButtonModelMetadata,crate::metadata::models::RibbonToggleButtonModelMetadata,
crate::metadata::models::RibbonDropDownButtonModelMetadata,
crate::metadata::models::RibbonSplitButtonModelMetadata,
crate::metadata::models::RibbonCheckBoxModelMetadata,crate::metadata::models::RibbonComboBoxModelMetadata,crate::metadata::models::RibbonSpinnerModelMetadata,crate::metadata::models::RibbonTextBoxModelMetadata,crate::metadata::models::RibbonSliderModelMetadata,
crate::metadata::models::RibbonMenuItemModelMetadata,
crate::metadata::models::RibbonMenuSeparatorModelMetadata,
crate::metadata::models::RibbonMenuHeaderModelMetadata,crate::metadata::models::RibbonGroupModelMetadata,crate::metadata::models::RibbonTabModelMetadata,crate::metadata::models::RibbonContextualGroupModelMetadata,crate::metadata::models::RibbonBackstageItemModelMetadata,crate::metadata::models::RibbonBackstageModelMetadata,crate::metadata::models::RibbonModelMetadata,crate::metadata::models::ModelItemsMetadata,crate::metadata::models::ModelNodesMetadata,crate::metadata::models::ModelTabsMetadata,crate::metadata::models::ModelGroupsMetadata,crate::metadata::models::ModelContextualGroupsMetadata,crate::metadata::models::ModelBackstageItemsMetadata,crate::metadata::models::ModelStringsMetadata,crate::controls::RibbonHandledEventArgs,crate::compat::DataTemplateSelector,crate::metadata::backstage_items::RibbonBackstageItemsMetadata,crate::controls::RibbonValueChangedEventArgs,crate::metadata::combo_items::RibbonObjectItemsMetadata,crate::metadata::combo_items::RibbonObjectEnumerableMetadata,crate::metadata::combo_items::RibbonObjectArrayMetadata,crate::metadata::combo_items::RibbonFontNamesMetadata,crate::metadata::combo_items::RibbonFontSizesMetadata,crate::controls::RibbonComboBoxCommittedEventArgs,crate::metadata::input_template_values::RibbonInputTemplateValues,crate::metadata::model_bindings::RibbonModelBindingsMetadata,crate::metadata::state_types::RibbonStateMetadata,crate::metadata::state_types::RibbonCustomizationMetadata,crate::metadata::state_types::RibbonCustomTabMetadata,crate::metadata::state_types::RibbonCustomGroupMetadata,crate::metadata::state_types::RibbonFloatingGroupStateMetadata,crate::metadata::state_types::RibbonStateStringsMetadata,crate::metadata::state_types::RibbonStateGroupsMetadata,crate::metadata::state_types::RibbonStateTabsMetadata,crate::metadata::state_types::RibbonStateFloatingGroupsMetadata,crate::metadata::state_types::RibbonStateLabelsMetadata,crate::metadata::state_types::RibbonStateGroupOrderMetadata,crate::metadata::state_types::RibbonStateSerializerMetadata,crate::metadata::ribbon_collections::RibbonFloatingGroupsMetadata,crate::metadata::quick_access_collections::RibbonQuickAccessCandidatesMetadata,crate::metadata::quick_access_collections::RibbonQuickAccessItemIdsMetadata,crate::metadata::key_tip_collections::RibbonActiveKeyTipsMetadata,crate::metadata::key_tip_collections::RibbonCurrentKeyTipsMetadata,crate::metadata::key_tip_collections::RibbonKeyTipTupleMetadata,crate::controls::RibbonDisplayGroupsMetadata,crate::metadata::ribbon_collections::RibbonGroupsMetadata,crate::metadata::ribbon_collections::RibbonTabsMetadata,crate::metadata::ribbon_collections::RibbonContextualGroupsMetadata,crate::controls::RibbonTabChangedEventArgs,crate::controls::RibbonItemInvokedEventArgs,crate::primitives::RibbonPopupPlacement,crate::metadata::template_values::RibbonTemplateValues,crate::metadata::control_items::RibbonControlItemsMetadata,crate::controls::RibbonItemLayout,crate::controls::RibbonKeyTipResult,crate::metadata::item_contracts::RibbonSizeDefinitionMembers,crate::controls::RibbonScreenTipService,crate::primitives::RibbonMetrics,crate::theming::RibbonTheme],contracts:[crate::controls::IRibbonCheckable,crate::controls::IRibbonItemOwner,crate::controls::IRibbonItem,crate::controls::IRibbonItemEditable,crate::primitives::IRibbonScrollContent,crate::controls::IRibbonLayoutHost,crate::controls::IRibbonCommandSource],generics:[],generic_contracts:[] }
pub fn register_types() {
    crate::compat::register_template_selector();
    crate::metadata::application_menu_items::register();
    crate::controls::register_display_groups();
    ferroui_base::platform::register_assets(
        "FerroRibbon",
        &[(
            "Assets/Fonts/uno-fluentui-assets.ttf",
            include_bytes!("Assets/Fonts/uno-fluentui-assets.ttf"),
        )],
    );
    ferroui_markup_xaml::register_types();
    crate::commands::install_culture_provider();
    crate::search::install_culture_provider();
    crate::localization::install_culture_provider();
    crate::metadata::enums::register();
    crate::metadata::combo_items::register();
    crate::metadata::item_content_part::register();
    crate::metadata::theme_types::register();
    crate::metadata::item_contracts::register();
    crate::metadata::control_items::register();
    crate::metadata::ribbon_collections::register();
    crate::metadata::key_tip_collections::register();
    crate::metadata::quick_access_collections::register();
    crate::metadata::state_types::register();
    crate::metadata::model_bindings::register();
    crate::metadata::models::register();
    crate::metadata::colors::register();
    crate::metadata::grid_size::register();
    crate::metadata::segments::register();
    crate::metadata::model_icon::register();
    crate::metadata::model_screen_tip::register();
    MarkupType::register_all(&[
        <crate::theming::RibbonSharedThemeInclude as MarkupTyped>::MARKUP,
        crate::controls::RibbonDialogClosingEventArgs::MARKUP,
        crate::controls::RibbonDialogClosedEventArgs::MARKUP,
        crate::controls::RibbonDialogOpenedEventArgs::MARKUP,
        crate::controls::RibbonDialogButtonClickEventArgs::MARKUP,
        crate::controls::RibbonContextMenuEventArgs::MARKUP,
        crate::controls::RibbonCustomizeRequestedEventArgs::MARKUP,
        crate::controls::RibbonCustomizePage::MARKUP,
        crate::controls::RibbonDialogOutcome::MARKUP,
        <crate::controls::RibbonApplicationMenuItemEventArgs as MarkupTyped>::MARKUP,
        <crate::controls::RibbonColorDialog as MarkupTyped>::MARKUP,
        <crate::controls::RibbonDisplayGroupsMetadata as MarkupTyped>::MARKUP,
        crate::controls::RibbonTabChangedEventArgs::MARKUP,
        crate::controls::RibbonComboBoxCommittedEventArgs::MARKUP,
        crate::controls::RibbonGalleryItemEventArgs::MARKUP,
        crate::controls::RibbonValueChangedEventArgs::MARKUP,
        crate::controls::RibbonItemInvokedEventArgs::MARKUP,
        crate::controls::RibbonHandledEventArgs::MARKUP,
    ]);
    ferroui_base::data::core::ValueTypes::register_global(register_group_item_mask);
    crate::metadata::simplified_label::register();
    crate::metadata::content_dialog_button::register();
    crate::metadata::orientation::register();
    crate::metadata::toolbar_overflow::register();
    crate::metadata::menu_bar_items::register();
    crate::metadata::search::register();
    crate::controls::register_search_ui();
    ferroui_base::data::core::ValueTypes::register_global(crate::commands::register_conversions);
    ferroui_base::data::core::ValueTypes::register_global(
        crate::controls::register_contract_values,
    );
    ferroui_base::data::core::ValueTypes::register_global(register_scroll_contract);
    ferroui_base::data::core::ValueTypes::register_nullable::<
        std::sync::Arc<crate::primitives::RibbonMetrics>,
    >();
    MarkupType::register_all(&[
        <crate::primitives::RibbonPopupPlacement as MarkupTyped>::MARKUP,
        <crate::metadata::template_values::RibbonTemplateValues as MarkupTyped>::MARKUP,
        <crate::metadata::input_template_values::RibbonInputTemplateValues as MarkupTyped>::MARKUP,
        <crate::primitives::RibbonMetrics as MarkupTyped>::MARKUP,
        <crate::theming::RibbonTheme as MarkupTyped>::MARKUP,
        <crate::controls::RibbonXmlns as MarkupTyped>::MARKUP,
        <crate::controls::RibbonScreenTipService as MarkupTyped>::MARKUP,
        <RibbonItemHelper as MarkupTyped>::MARKUP,
        <crate::controls::RibbonSearch as MarkupTyped>::MARKUP,
        <crate::controls::RibbonMenu as MarkupTyped>::MARKUP,
        <crate::controls::RibbonMenuCloner as MarkupTyped>::MARKUP,
        <dyn IRibbonCommandSource as MarkupTyped>::MARKUP,
        <dyn crate::controls::IRibbonItemOwner as MarkupTyped>::MARKUP,
        <dyn crate::controls::IRibbonCheckable as MarkupTyped>::MARKUP,
        <dyn IRibbonLayoutHost as MarkupTyped>::MARKUP,
        <dyn IRibbonScrollContent as MarkupTyped>::MARKUP,
    ]);
    ferroui_controls::register_types();
    TypeInfo::register_namespaces(NAMESPACES);
    TypeInfo::register_all(&[
        <crate::controls::RibbonApplicationMenu as StaticType>::TYPE,
        <crate::controls::RibbonApplicationMenuItem as StaticType>::TYPE,
        <crate::controls::RibbonApplicationMenuRecentItem as StaticType>::TYPE,
        <crate::controls::RibbonSearchBox as StaticType>::TYPE,
        <crate::controls::RibbonZoomControl as StaticType>::TYPE,
        <crate::controls::RibbonTitleBar as StaticType>::TYPE,
        <crate::controls::RibbonStatusBar as StaticType>::TYPE,
        <crate::primitives::RibbonToolBarPanel as StaticType>::TYPE,
        <crate::mvvm::RibbonElementFactory as StaticType>::TYPE,
        <crate::controls::RibbonSlider as StaticType>::TYPE,
        <crate::controls::RibbonColorPicker as StaticType>::TYPE,
        <crate::controls::RibbonColorDialogHost as StaticType>::TYPE,
        <crate::controls::RibbonGallery as StaticType>::TYPE,
        <crate::controls::RibbonGalleryItem as StaticType>::TYPE,
        <crate::controls::RibbonColorPalette as StaticType>::TYPE,
        <crate::controls::RibbonSegment as StaticType>::TYPE,
        <crate::controls::RibbonSegmentedControl as StaticType>::TYPE,
        <crate::controls::RibbonGridPicker as StaticType>::TYPE,
        <crate::compat::RibbonInputSlider as StaticType>::TYPE,
        <crate::compat::RibbonInputRepeatButton as StaticType>::TYPE,
        <crate::compat::RibbonInputTextBox as StaticType>::TYPE,
        <crate::controls::RibbonSpinner as StaticType>::TYPE,
        <crate::controls::RibbonComboBox as StaticType>::TYPE,
        <crate::controls::RibbonFontComboBox as StaticType>::TYPE,
        <crate::controls::RibbonFontSizeComboBox as StaticType>::TYPE,
        <crate::controls::RibbonInputBase as StaticType>::TYPE,
        <crate::controls::RibbonTextBox as StaticType>::TYPE,
        <crate::controls::RibbonMenuBar as StaticType>::TYPE,
        <crate::controls::RibbonMenuBarItem as StaticType>::TYPE,
        <crate::controls::RibbonToolBar as StaticType>::TYPE,
        <crate::controls::RibbonContextualToolBar as StaticType>::TYPE,
        <crate::controls::RibbonQuickAccessToolBar as StaticType>::TYPE,
        <crate::controls::RibbonToolBarAutomationPeer as StaticType>::TYPE,
        <crate::controls::RibbonSplitButton as StaticType>::TYPE,
        <crate::controls::RibbonSplitButtonAutomationPeer as StaticType>::TYPE,
        <crate::controls::RibbonAutomationPeer as StaticType>::TYPE,
        <crate::controls::RibbonGroupAutomationPeer as StaticType>::TYPE,
        <crate::controls::RibbonTabHeaderAutomationPeer as StaticType>::TYPE,
        <crate::compat::MenuFlyoutItem as StaticType>::TYPE,
        <crate::compat::ToggleMenuFlyoutItem as StaticType>::TYPE,
        <crate::compat::RadioMenuFlyoutItem as StaticType>::TYPE,
        <crate::compat::MenuFlyoutSubItem as StaticType>::TYPE,
        <crate::compat::MenuFlyoutSeparator as StaticType>::TYPE,
        <crate::controls::RibbonMenu as StaticType>::TYPE,
        <crate::controls::RibbonMenuCloner as StaticType>::TYPE,
        <crate::controls::RibbonToggleButton as StaticType>::TYPE,
        <crate::controls::RibbonCheckBox as StaticType>::TYPE,
        <crate::controls::Ribbon as StaticType>::TYPE,
        <crate::controls::RibbonTab as StaticType>::TYPE,
        <crate::controls::RibbonTabHeader as StaticType>::TYPE,
        <crate::controls::RibbonContextualTabGroup as StaticType>::TYPE,
        <crate::primitives::RibbonGroupsPanel as StaticType>::TYPE,
        <crate::controls::RibbonGroup as StaticType>::TYPE,
        <crate::compat::FontIcon as StaticType>::TYPE,
        <crate::compat::ImageIcon as StaticType>::TYPE,
        <crate::theming::RibbonTheme as StaticType>::TYPE,
        <RibbonUniformGridPanel as StaticType>::TYPE,
        <crate::primitives::RibbonGroupItemsPanel as StaticType>::TYPE,
        <RibbonIconPresenter as StaticType>::TYPE,
        <RibbonItemContent as StaticType>::TYPE,
        <crate::controls::RibbonKeyTip as StaticType>::TYPE,
        <crate::theming::RibbonThemeResources as StaticType>::TYPE,
        <crate::controls::RibbonScreenTip as StaticType>::TYPE,
        <crate::controls::RibbonScreenTipService as StaticType>::TYPE,
        <RibbonControlBase as StaticType>::TYPE,
        <crate::controls::RibbonItemsContainer as StaticType>::TYPE,
        <crate::controls::RibbonButtonGroup as StaticType>::TYPE,
        <crate::controls::RibbonStackPanel as StaticType>::TYPE,
        <crate::controls::RibbonButton as StaticType>::TYPE,
        <crate::controls::RibbonDropDownButton as StaticType>::TYPE,
        <crate::controls::RibbonSeparator as StaticType>::TYPE,
        <crate::controls::RibbonLabel as StaticType>::TYPE,
        <RibbonItemHelper as StaticType>::TYPE,
        <RibbonScrollPanel as StaticType>::TYPE,
    ]);
    crate::metadata::backstage_items::register();
    TypeInfo::register_all(&[
        <crate::controls::RibbonSearchBox as StaticType>::TYPE,
        <crate::primitives::RibbonToolBarPanel as StaticType>::TYPE,
        <crate::controls::RibbonBackstage as StaticType>::TYPE,
        <crate::controls::RibbonBackstageItem as StaticType>::TYPE,
    ]);
    register_rust_paths();
    MarkupAssembly::register(&ASSEMBLY);
    crate::compiled_markup::register();
}

fn register_scroll_contract() {
    use ferroui_base::data::core::ValueTypes;
    use std::rc::Rc;
    ValueTypes::register_nullable::<Rc<dyn IRibbonScrollContent>>();
    ValueTypes::register_interface_object::<Rc<dyn IRibbonScrollContent>>(|value| {
        Some(value.as_object())
    });
}

fn register_group_item_mask() {
    ferroui_base::data::core::ValueTypes::register_nullable::<
        ferroribbon_core::collections::RibbonReadOnlyList<bool>,
    >();
}
