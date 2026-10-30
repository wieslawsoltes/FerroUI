use ferroui_base::metadata::MarkupAssembly;

pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "ChartsDemo",
    crate_name: "charts_demo",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        ferroui_controls_charts::register_types();
        MarkupAssembly::register(&ASSEMBLY);
        crate::compiled_markup::register();
    });
}
