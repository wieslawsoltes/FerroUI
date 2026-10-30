//! The type table of this crate: its namespaces, its classes, what it
//! states about itself for markup, its embedded documents and the loader
//! of the documents with a class.

use crate::accents::SystemAccentColors;
use crate::converters::{CornerRadiusToThicknessConverter, CornerRadiusToThicknessConverterKind};
use crate::primitives::{InfoBarPanel, TabViewEqualWidthPanel};
use crate::material_theme::{DensityStyle, MaterialTheme};
use crate::{MaterialBottomSheetHost, MaterialSheetState, MaterialSnackbar, MaterialSnackbarHost};
use crate::{MaterialTextField, MaterialTextFieldLayout, MaterialTextFieldSlot};
use crate::MaterialSearchView;
use crate::{MaterialChip, MaterialChipKind, MaterialChipDismissButton, MaterialChipLayout};
use crate::{MaterialDatePicker, MaterialDatePickerAutomationPeer, MaterialDateCell, MaterialDateCellAutomationPeer, MaterialDateMonthView, MaterialDateMonthsPanel, MaterialDatePickerMode, MaterialDateSelectionMode};
use crate::{MaterialTimePicker, MaterialTimePickerField, MaterialTimePickerLayout, MaterialTimePickerAutomationPeer, MaterialTimePickerDial, MaterialTimePickerNumber, MaterialTimePickerNumberAutomationPeer, MaterialTimeSelection, MaterialTimeDisplayMode};
use crate::MaterialBottomSheetAutomationPeer;
use crate::{MaterialAppBar, MaterialAppBarLayout, MaterialAppBarSize, MaterialAppBarScrollBehavior};
use crate::{MaterialSegmentedContent, MaterialSegmentedGroup, MaterialSegmentedGroupAutomationPeer};
use crate::{MaterialExtendedFab, MaterialExtendedFabLayout, MaterialExtendedFabSize};
use crate::{MaterialRangeSlider, MaterialRangeSliderThumb, MaterialRangeSliderLayout, MaterialRangeSliderThumbAutomationPeer};
use crate::{MaterialDoubleTransition, MaterialCornerRadiusTransition, MaterialThicknessTransition, MaterialMotionScheme, MaterialRipple, MaterialProgressIndicator, MaterialLoadingIndicator, MaterialSelectionMark, MaterialTextFieldBorder};
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::TypeInfo;
use ferroui_markup_xaml::FerroXamlLoader;
use std::rc::Rc;

/// The dotted namespaces of the modules of this crate.
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_themes_material::chip", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::chip_layout", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::date_picker", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::date_picker_calendar", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::date_picker_state", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::time_picker", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::time_picker_dial", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::time_picker_state", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::bottom_sheet_automation", "FerroUI.Themes.Material"),

    ("ferroui_themes_material", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::ripple", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::progress_indicator", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::loading_indicator", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::selection_mark", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::text_field_border", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::text_field", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::text_field_layout", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::motion", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::app_bar", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::app_bar_layout", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::app_bar_state", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::segmented_content", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::segmented_group", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::spring_transitions", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::sheet_state", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::bottom_sheet", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::search_view", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::extended_fab", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::range_slider", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::snackbar", "FerroUI.Themes.Material"),
    ("ferroui_themes_material::accents", "FerroUI.Themes.Material.Accents"),
    ("ferroui_themes_material::converters", "FerroUI.Themes.Material.Converters"),
    ("ferroui_themes_material::primitives", "FerroUI.Themes.Material.Primitives"),
];

/// What this crate states about itself for markup: its assembly name and
/// the namespace the XML namespace of the framework maps to.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Themes.Material",
    crate_name: "ferroui_themes_material",
    xmlns_definitions: &[XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Themes.Material" }],
    xmlns_prefixes: &[],
    metadata: &[],
};

const TYPES: &[&TypeInfo] =
    &[MaterialChip::TYPE, MaterialChipDismissButton::TYPE, MaterialChipLayout::TYPE, MaterialDatePicker::TYPE, MaterialDatePickerAutomationPeer::TYPE, MaterialDateCell::TYPE, MaterialDateCellAutomationPeer::TYPE, MaterialDateMonthView::TYPE, MaterialDateMonthsPanel::TYPE,
    MaterialTimePicker::TYPE, MaterialTimePickerField::TYPE, MaterialTimePickerLayout::TYPE, MaterialTimePickerAutomationPeer::TYPE, MaterialTimePickerDial::TYPE, MaterialTimePickerNumber::TYPE, MaterialTimePickerNumberAutomationPeer::TYPE,
    MaterialBottomSheetAutomationPeer::TYPE, MaterialAppBar::TYPE, MaterialAppBarLayout::TYPE,
    MaterialSegmentedContent::TYPE, MaterialSegmentedGroup::TYPE, MaterialSegmentedGroupAutomationPeer::TYPE,
    MaterialRangeSlider::TYPE, MaterialRangeSliderThumb::TYPE, MaterialRangeSliderLayout::TYPE, MaterialRangeSliderThumbAutomationPeer::TYPE,
    MaterialExtendedFab::TYPE, MaterialExtendedFabLayout::TYPE,
    MaterialSearchView::TYPE, MaterialTextField::TYPE, MaterialTextFieldLayout::TYPE,
    MaterialBottomSheetHost::TYPE, MaterialSnackbar::TYPE, MaterialSnackbarHost::TYPE,
    MaterialDoubleTransition::TYPE, MaterialCornerRadiusTransition::TYPE, MaterialThicknessTransition::TYPE, MaterialTheme::TYPE, MaterialRipple::TYPE, MaterialProgressIndicator::TYPE, MaterialLoadingIndicator::TYPE, MaterialSelectionMark::TYPE, MaterialTextFieldBorder::TYPE, SystemAccentColors::TYPE, InfoBarPanel::TYPE, TabViewEqualWidthPanel::TYPE];

const MARKUP_TYPES: &[&MarkupType] = &[
    <MaterialChipKind as MarkupTyped>::MARKUP,
    <MaterialDatePickerMode as MarkupTyped>::MARKUP,
    <MaterialDateSelectionMode as MarkupTyped>::MARKUP,
    <MaterialTimeSelection as MarkupTyped>::MARKUP,
    <MaterialTimeDisplayMode as MarkupTyped>::MARKUP,

    <DensityStyle as MarkupTyped>::MARKUP,
    <MaterialMotionScheme as MarkupTyped>::MARKUP,
    <MaterialSheetState as MarkupTyped>::MARKUP,
    <MaterialTextFieldSlot as MarkupTyped>::MARKUP,
    <MaterialExtendedFabSize as MarkupTyped>::MARKUP,
    <MaterialAppBarSize as MarkupTyped>::MARKUP,
    <MaterialAppBarScrollBehavior as MarkupTyped>::MARKUP,
    <CornerRadiusToThicknessConverter as MarkupTyped>::MARKUP,
    <CornerRadiusToThicknessConverterKind as MarkupTyped>::MARKUP,
];

/// Registers the namespaces, the types, the assembly, the embedded
/// documents and the document loader of this crate (and of the crates it is
/// built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        ferroui_dialogs::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        crate::rust_paths::register_rust_paths();
        MarkupType::register_all(MARKUP_TYPES);
        ValueTypes::register_global(register_value_types);
        MarkupAssembly::register(&ASSEMBLY);
        crate::assets::register();
        crate::fonts::register();
        FerroXamlLoader::register_compiled_xaml(ASSEMBLY.name, crate::compiled_xaml::try_load);
    });
}

/// What the untyped value conversions must know about the types of this crate: the nullable
/// forms of the enumerations and of the converter, and the converter as its contract.
fn register_value_types() {
    ValueTypes::register_nullable::<MaterialChipKind>();
    ValueTypes::register_nullable::<MaterialDatePickerMode>();
    ValueTypes::register_nullable::<MaterialDateSelectionMode>();
    ValueTypes::register_nullable::<MaterialTimeSelection>();
    ValueTypes::register_nullable::<MaterialTimeDisplayMode>();

    ValueTypes::register_nullable::<DensityStyle>();
    ValueTypes::register_nullable::<MaterialMotionScheme>();
    ValueTypes::register_nullable::<MaterialSheetState>();
    ValueTypes::register_nullable::<MaterialTextFieldSlot>();
    ValueTypes::register_nullable::<MaterialExtendedFabSize>();
    ValueTypes::register_nullable::<MaterialAppBarSize>();
    ValueTypes::register_nullable::<MaterialAppBarScrollBehavior>();
    ValueTypes::register_nullable::<CornerRadiusToThicknessConverterKind>();
    ValueTypes::register_nullable::<Rc<CornerRadiusToThicknessConverter>>();
    ValueTypes::register_cast::<Rc<CornerRadiusToThicknessConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
}
