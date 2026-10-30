//! Compiles the actual source view classes and downstream binding fixtures.
fn main() {
    let views = [
        (
            "views/views/DashboardView.xaml",
            include_str!("views/views/DashboardView.xaml"),
        ),
        (
            "views/views/HomeView.xaml",
            include_str!("views/views/HomeView.xaml"),
        ),
        (
            "views/documents/DocumentView.xaml",
            include_str!("views/documents/DocumentView.xaml"),
        ),
        (
            "views/tools/Tool1View.xaml",
            include_str!("views/tools/Tool1View.xaml"),
        ),
        (
            "views/tools/Tool2View.xaml",
            include_str!("views/tools/Tool2View.xaml"),
        ),
        (
            "views/tools/Tool3View.xaml",
            include_str!("views/tools/Tool3View.xaml"),
        ),
        (
            "views/tools/Tool4View.xaml",
            include_str!("views/tools/Tool4View.xaml"),
        ),
        (
            "views/tools/Tool5View.xaml",
            include_str!("views/tools/Tool5View.xaml"),
        ),
        (
            "views/tools/Tool6View.xaml",
            include_str!("views/tools/Tool6View.xaml"),
        ),
        (
            "views/tools/Tool7View.xaml",
            include_str!("views/tools/Tool7View.xaml"),
        ),
        (
            "views/tools/Tool8View.xaml",
            include_str!("views/tools/Tool8View.xaml"),
        ),
        (
            "views/DockableOptionsView.xaml",
            include_str!("views/DockableOptionsView.xaml"),
        ),
        ("views/MainView.xaml", include_str!("views/MainView.xaml")),
        (
            "views/MainWindow.xaml",
            include_str!("views/MainWindow.xaml"),
        ),
        ("App.xaml", include_str!("App.xaml")),
    ];
    let mut build = ferroui_build::Build::from_env()
        .type_system(ferroui_build::TypeSystem::Model)
        .default_compile_bindings(true);
    for (module, path) in [
        ("compiled_dashboard_view", "views/views/DashboardView.xaml"),
        ("compiled_home_view", "views/views/HomeView.xaml"),
        (
            "compiled_document_view",
            "views/documents/DocumentView.xaml",
        ),
        ("compiled_tool1_view", "views/tools/Tool1View.xaml"),
        ("compiled_tool2_view", "views/tools/Tool2View.xaml"),
        ("compiled_tool3_view", "views/tools/Tool3View.xaml"),
        ("compiled_tool4_view", "views/tools/Tool4View.xaml"),
        ("compiled_tool5_view", "views/tools/Tool5View.xaml"),
        ("compiled_tool6_view", "views/tools/Tool6View.xaml"),
        ("compiled_tool7_view", "views/tools/Tool7View.xaml"),
        ("compiled_tool8_view", "views/tools/Tool8View.xaml"),
        (
            "compiled_dockable_options_view",
            "views/DockableOptionsView.xaml",
        ),
        ("compiled_main_view", "views/MainView.xaml"),
        ("compiled_main_window", "views/MainWindow.xaml"),
        ("compiled_app", "App.xaml"),
    ] {
        build = build.input(path).compile_group(
            ferroui_build::XamlGroup::new(module)
                .documents(&views)
                .class_document(path),
        );
    }
    build
        .input("tests/InheritedBindingConsumer.xaml")
        .input("tests/MainWindowBindingConsumer.xaml")
        .compile_group(
            ferroui_build::XamlGroup::new("compiled_xaml")
                .internal()
                .documents(&[
                    (
                        "tests/InheritedBindingConsumer.xaml",
                        include_str!("tests/InheritedBindingConsumer.xaml"),
                    ),
                    (
                        "tests/MainWindowBindingConsumer.xaml",
                        include_str!("tests/MainWindowBindingConsumer.xaml"),
                    ),
                ]),
        )
        .run();
}
