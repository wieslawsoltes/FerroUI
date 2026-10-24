//! The type table of this crate: its namespaces, its classes, what it states about itself for
//! markup, its embedded assets and the loader table of its compiled markup.

use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::TypeInfo;

/// The dotted namespaces of the modules of this crate (the namespaces of the upstream project,
/// whose root namespace is the root of the framework).
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_diagnostics", "FerroUI"),
    ("ferroui_diagnostics::diagnostics", "FerroUI.Diagnostics"),
    ("ferroui_diagnostics::diagnostics::behaviors", "FerroUI.Diagnostics.Behaviors"),
    ("ferroui_diagnostics::diagnostics::controls", "FerroUI.Diagnostics.Controls"),
    ("ferroui_diagnostics::diagnostics::converters", "FerroUI.Diagnostics.Converters"),
    ("ferroui_diagnostics::diagnostics::models", "FerroUI.Diagnostics.Models"),
    ("ferroui_diagnostics::diagnostics::screenshots", "FerroUI.Diagnostics.Screenshots"),
    ("ferroui_diagnostics::diagnostics::view_models", "FerroUI.Diagnostics.ViewModels"),
    ("ferroui_diagnostics::diagnostics::views", "FerroUI.Diagnostics.Views"),
];

/// What this crate states about itself for markup: its assembly name.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Diagnostics",
    crate_name: "ferroui_diagnostics",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    // Upstream builds its documents with compiled bindings by default.
    metadata: &[(MarkupAssembly::USE_COMPILED_BINDINGS_BY_DEFAULT, "True")],
};

use crate::diagnostics::behaviors::ColumnDefinition;
use crate::diagnostics::controls::{
    Application, BrushEditor, CommitTextBox, ControlHighlightAdorner, FilterTextBox, ThicknessEditor, TopLevelGroup,
};
use crate::diagnostics::converters::{
    BoolToImageConverter, BoolToOpacityConverter, BrushSelectorConveter, EnumToCheckedConverter, GetTypeNameConverter,
};
use crate::diagnostics::models::EventChainLink;
use crate::diagnostics::view_models::{
    BindingSetterViewModel, ClrPropertyViewModel, ControlDetailsViewModel, ControlLayoutViewModel, EventChainLinkList,
    EventTreeNodeBase, EventTreeNodeList, EventsPageViewModel, FerroPropertyViewModel, FilterViewModel, FiredEvent,
    FiredEventList, HotKeyDescription, HotKeyDescriptionList, HotKeyPageViewModel, MainViewModel, PropertiesView,
    PropertiesViewItem, PseudoClassViewModel, ResourceSetterViewModel, SetterViewModel, TreeNode, TreeNodeCollection,
    TreeNodeList, TreePageViewModel, ValueFrameViewModel,
};
use crate::diagnostics::views::{
    ControlDetailsView, EventsPageView, HotKeyPageView, LayoutExplorerView, MainView, MainWindow, PropertyValueEditorView,
    TreePageView,
};
use crate::diagnostics::ViewLocator;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::StaticType;
use ferroui_controls::templates::IDataTemplate;
use std::rc::Rc;

/// The classes of the crate (`X::TYPE`).
const TYPES: &[&TypeInfo] = &[
    // FerroUI.Diagnostics.Behaviors
    <ColumnDefinition as StaticType>::TYPE,
    // FerroUI.Diagnostics.Controls
    Application::TYPE,
    BrushEditor::TYPE,
    CommitTextBox::TYPE,
    ControlHighlightAdorner::TYPE,
    FilterTextBox::TYPE,
    ThicknessEditor::TYPE,
    TopLevelGroup::TYPE,
    // FerroUI.Diagnostics.Converters
    BrushSelectorConveter::TYPE,
    // FerroUI.Diagnostics.Views
    ControlDetailsView::TYPE,
    EventsPageView::TYPE,
    HotKeyPageView::TYPE,
    LayoutExplorerView::TYPE,
    MainView::TYPE,
    MainWindow::TYPE,
    PropertyValueEditorView::TYPE,
    TreePageView::TYPE,
];

/// The types of the crate declared with `ferro_markup_type!`.
const MARKUP_TYPES: &[&MarkupType] = &[
    <ViewLocator as MarkupTyped>::MARKUP,
    <BoolToImageConverter as MarkupTyped>::MARKUP,
    <BoolToOpacityConverter as MarkupTyped>::MARKUP,
    <EnumToCheckedConverter as MarkupTyped>::MARKUP,
    <GetTypeNameConverter as MarkupTyped>::MARKUP,
    <EventChainLink as MarkupTyped>::MARKUP,
    <TreeNode as MarkupTyped>::MARKUP,
    <TreeNodeCollection as MarkupTyped>::MARKUP,
    <TreePageViewModel as MarkupTyped>::MARKUP,
    <FilterViewModel as MarkupTyped>::MARKUP,
    <ControlDetailsViewModel as MarkupTyped>::MARKUP,
    <PropertiesView as MarkupTyped>::MARKUP,
    <PropertiesViewItem as MarkupTyped>::MARKUP,
    <FerroPropertyViewModel as MarkupTyped>::MARKUP,
    <ClrPropertyViewModel as MarkupTyped>::MARKUP,
    <ValueFrameViewModel as MarkupTyped>::MARKUP,
    <SetterViewModel as MarkupTyped>::MARKUP,
    <BindingSetterViewModel as MarkupTyped>::MARKUP,
    <ResourceSetterViewModel as MarkupTyped>::MARKUP,
    <PseudoClassViewModel as MarkupTyped>::MARKUP,
    <ControlLayoutViewModel as MarkupTyped>::MARKUP,
    <EventsPageViewModel as MarkupTyped>::MARKUP,
    <EventTreeNodeBase as MarkupTyped>::MARKUP,
    <FiredEvent as MarkupTyped>::MARKUP,
    <HotKeyDescription as MarkupTyped>::MARKUP,
    <HotKeyPageViewModel as MarkupTyped>::MARKUP,
    <MainViewModel as MarkupTyped>::MARKUP,
];

/// Registers the namespaces, the types, the assembly, the embedded assets and the compiled
/// markup of this crate (and of the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        // Deviation (DEVIATIONS.md, Developer tools): the WinUI 3 theme, which the window includes
        // in place of the Simple theme of upstream.
        ferroui_themes_winui::register_types();
        ferroui_controls_color_picker::register_types();
        ferroui_controls_treedatagrid::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        crate::rust_paths::register_rust_paths();
        MarkupType::register_all(MARKUP_TYPES);
        TreeNodeList::register();
        crate::diagnostics::view_models::ValueFrameViewModelList::register();
        crate::diagnostics::view_models::PseudoClassViewModelList::register();
        EventTreeNodeList::register();
        EventChainLinkList::register();
        FiredEventList::register();
        HotKeyDescriptionList::register();
        ValueTypes::register_global(register_value_types);
        ValueTypes::register_global(crate::diagnostics::view_models::register_tree_value_types);
        ValueTypes::register_global(crate::diagnostics::view_models::register_main_value_types);
        ValueTypes::register_global(crate::diagnostics::view_models::register_details_value_types);
        ferroui_base::data::model::ModelTypes::register::<FilterViewModel>(|b| b.notify_property_changed().notify_data_error_info());
        MarkupAssembly::register(&ASSEMBLY);
        // The assets (`Assets/`) and the loader table of the compiled markup, which the build of
        // the crate generates: a load of the document of a class by its URI creates an instance
        // of the class, whose constructor populates it.
        crate::compiled_markup::register();
    });
}

/// What the untyped value conversions must know about the types of the core of the crate: the
/// view locator is a data template, the converters are value converters, the links of event
/// chains are reference types.
fn register_value_types() {
    ValueTypes::register_reference::<ViewLocator>();
    ValueTypes::register_cast::<ViewLocator, Rc<dyn IDataTemplate>>(ViewLocator::as_data_template);
    macro_rules! converter {
        ($($t:ty),*) => {$(
            ValueTypes::register_nullable::<Rc<$t>>();
            ValueTypes::register_cast::<Rc<$t>, Rc<dyn IValueConverter>>(|c| c.clone());
        )*};
    }
    converter!(BoolToImageConverter, BoolToOpacityConverter, EnumToCheckedConverter, GetTypeNameConverter);
    ValueTypes::register_nullable::<Rc<EventChainLink>>();
}
