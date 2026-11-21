//! Compiles the actual source view classes and downstream binding fixtures.
use ferroui_build::{xaml_document_file, Build, TypeSystem, XamlGroup};

fn main() {
    let views = [
        (
            "compiled_dashboard_view",
            xaml_document_file!(
                "views/views/DashboardView.xaml",
                "views/views/DashboardView.xaml",
            ),
        ),
        (
            "compiled_home_view",
            xaml_document_file!("views/views/HomeView.xaml", "views/views/HomeView.xaml"),
        ),
        (
            "compiled_document_view",
            xaml_document_file!(
                "views/documents/DocumentView.xaml",
                "views/documents/DocumentView.xaml",
            ),
        ),
        (
            "compiled_tool1_view",
            xaml_document_file!("views/tools/Tool1View.xaml", "views/tools/Tool1View.xaml"),
        ),
        (
            "compiled_tool2_view",
            xaml_document_file!("views/tools/Tool2View.xaml", "views/tools/Tool2View.xaml"),
        ),
        (
            "compiled_tool3_view",
            xaml_document_file!("views/tools/Tool3View.xaml", "views/tools/Tool3View.xaml"),
        ),
        (
            "compiled_tool4_view",
            xaml_document_file!("views/tools/Tool4View.xaml", "views/tools/Tool4View.xaml"),
        ),
        (
            "compiled_tool5_view",
            xaml_document_file!("views/tools/Tool5View.xaml", "views/tools/Tool5View.xaml"),
        ),
        (
            "compiled_tool6_view",
            xaml_document_file!("views/tools/Tool6View.xaml", "views/tools/Tool6View.xaml"),
        ),
        (
            "compiled_tool7_view",
            xaml_document_file!("views/tools/Tool7View.xaml", "views/tools/Tool7View.xaml"),
        ),
        (
            "compiled_tool8_view",
            xaml_document_file!("views/tools/Tool8View.xaml", "views/tools/Tool8View.xaml"),
        ),
        (
            "compiled_dockable_options_view",
            xaml_document_file!(
                "views/DockableOptionsView.xaml",
                "views/DockableOptionsView.xaml",
            ),
        ),
        (
            "compiled_main_view",
            xaml_document_file!("views/MainView.xaml", "views/MainView.xaml"),
        ),
        (
            "compiled_main_window",
            xaml_document_file!("views/MainWindow.xaml", "views/MainWindow.xaml"),
        ),
        ("compiled_app", xaml_document_file!("App.xaml", "App.xaml")),
    ];
    let documents = views.map(|(_, file)| file);
    let mut build = Build::from_env().type_system(TypeSystem::Model);
    for (module, file) in views {
        build = build.compile_group(
            XamlGroup::new(module)
                .document_files(&documents)
                .class_document(file.key()),
        );
    }
    build
        .input("tests/InheritedBindingConsumer.xaml")
        .input("tests/MainWindowBindingConsumer.xaml")
        .compile_group(XamlGroup::new("compiled_xaml").internal().documents(&[
            (
                "tests/InheritedBindingConsumer.xaml",
                include_str!("tests/InheritedBindingConsumer.xaml"),
            ),
            (
                "tests/MainWindowBindingConsumer.xaml",
                include_str!("tests/MainWindowBindingConsumer.xaml"),
            ),
        ]))
        .run();
}
