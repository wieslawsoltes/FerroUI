//! Source sample assembly and exact concrete-model markup registration.
use ferroui_base::metadata::MarkupAssembly;
const NAMESPACES: &[(&str, &str)] = &[
    (
        "ferrodock_reactiveui_sample::app",
        "FerroDockReactiveUISample",
    ),
    (
        "ferrodock_reactiveui_sample::behaviors",
        "FerroDockReactiveUISample.Behaviors",
    ),
    (
        "ferrodock_reactiveui_sample::views",
        "FerroDockReactiveUISample.Views",
    ),
    (
        "ferrodock_reactiveui_sample::views::documents",
        "FerroDockReactiveUISample.Views.Documents",
    ),
    (
        "ferrodock_reactiveui_sample::views::tools",
        "FerroDockReactiveUISample.Views.Tools",
    ),
    (
        "ferrodock_reactiveui_sample::views::views",
        "FerroDockReactiveUISample.Views.Views",
    ),
];
/// Native sample assembly identity.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroDockReactiveUISample",
    crate_name: "ferrodock_reactiveui_sample",
    xmlns_definitions: &[ferroui_base::metadata::XmlnsDefinition {
        xml_namespace: "https://github.com/ferroui",
        namespace: "FerroUI.Media",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};
ferroui_base::ferro_rust_paths! {classes:[crate::App,crate::views::MainView,crate::views::MainWindow,crate::views::documents::DocumentView,crate::views::tools::Tool1View,crate::views::tools::Tool2View,crate::views::tools::Tool3View,crate::views::tools::Tool4View,crate::views::tools::Tool5View,crate::views::tools::Tool6View,crate::views::tools::Tool7View,crate::views::tools::Tool8View,crate::views::DockableOptionsView,crate::views::views::DashboardView,crate::views::views::HomeView],types:[crate::behaviors::MainViewThemeBehavior,crate::ViewLocator,crate::RenderOptionsMetadata,crate::models::DemoData,crate::models::documents::DemoDocument,crate::models::tools::Tool1,crate::models::tools::Tool2,crate::models::tools::Tool3,crate::models::tools::Tool4,crate::models::tools::Tool5,crate::models::tools::Tool6,crate::models::tools::Tool7,crate::models::tools::Tool8,crate::view_models::documents::DocumentViewModel,crate::view_models::tools::Tool1ViewModel,crate::view_models::tools::Tool2ViewModel,crate::view_models::tools::Tool3ViewModel,crate::view_models::tools::Tool4ViewModel,crate::view_models::tools::Tool5ViewModel,crate::view_models::tools::Tool6ViewModel,crate::view_models::tools::Tool7ViewModel,crate::view_models::tools::Tool8ViewModel,crate::view_models::views::DashboardViewModel,crate::view_models::views::HomeViewModel,crate::view_models::docks::CustomDocumentDock,crate::view_models::MainWindowViewModel,crate::view_models::LayoutFileViewModel],contracts:[],generics:[],generic_contracts:[]}
/// Registers original sample model metadata and its compiled resources.
pub fn register_types() {
    ferrodock_ferro::register_types();
    ferrodock_model_reactiveui_ferro::register_types();
    ferrodock_ferro_diagnostics::register_types();
    ferroui_themes_fluent::register_types();
    ferrodock_ferro_themes_fluent::register_types();
    ferrodock_ferro_themes_browser::register_types();
    <crate::behaviors::MainViewThemeBehavior as ferroui_base::StaticType>::TYPE.ensure_class_init();
    ferroui_base::metadata::MarkupType::register(
        <crate::behaviors::MainViewThemeBehavior as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_markup_xaml::register_types();
    ferroui_base::metadata::MarkupType::register(
        <crate::ViewLocator as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<crate::ViewLocator>();
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::ViewLocator>,
        std::rc::Rc<dyn ferroui_controls::templates::IDataTemplate>,
    >(|locator| locator.clone());
    ferroui_base::metadata::MarkupType::register(<crate::markup::render_options_metadata::RenderOptionsMetadata as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::TypeInfo::register_namespaces(NAMESPACES);
    ferroui_base::TypeInfo::register_all(&[
        crate::App::TYPE,
        crate::views::views::DashboardView::TYPE,
        crate::views::MainView::TYPE,
        crate::views::MainWindow::TYPE,
        crate::views::documents::DocumentView::TYPE,
        crate::views::tools::Tool1View::TYPE,
        crate::views::tools::Tool2View::TYPE,
        crate::views::tools::Tool3View::TYPE,
        crate::views::tools::Tool4View::TYPE,
        crate::views::tools::Tool5View::TYPE,
        crate::views::tools::Tool6View::TYPE,
        crate::views::tools::Tool7View::TYPE,
        crate::views::tools::Tool8View::TYPE,
        crate::views::DockableOptionsView::TYPE,
        crate::views::views::HomeView::TYPE,
    ]);
    register_rust_paths();
    ferroui_base::metadata::MarkupAssembly::register(&ASSEMBLY);
    crate::markup::document_view_model_metadata::register();
    crate::markup::tool1_view_model_metadata::register();
    crate::markup::tool2_view_model_metadata::register();
    crate::markup::tool3_view_model_metadata::register();
    crate::markup::tool4_view_model_metadata::register();
    crate::markup::tool5_view_model_metadata::register();
    crate::markup::tool6_view_model_metadata::register();
    crate::markup::tool7_view_model_metadata::register();
    crate::markup::tool8_view_model_metadata::register();
    crate::markup::dashboard_view_model_metadata::register();
    crate::markup::home_view_model_metadata::register();
    crate::markup::custom_document_dock_metadata::register();
    crate::markup::main_window_view_model_metadata::register();
    crate::markup::layout_file_view_model_metadata::register();
    crate::markup::demo_data_metadata::register();
    crate::markup::demo_document_metadata::register();
    crate::markup::tool1_context_metadata::register();
    crate::markup::tool2_context_metadata::register();
    crate::markup::tool3_context_metadata::register();
    crate::markup::tool4_context_metadata::register();
    crate::markup::tool5_context_metadata::register();
    crate::markup::tool6_context_metadata::register();
    crate::markup::tool7_context_metadata::register();
    crate::markup::tool8_context_metadata::register();
    crate::compiled_markup::register();
}
