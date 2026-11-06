use ferroui_base::{
    TypeInfo,
    metadata::{MarkupAssembly, MarkupType, MarkupTyped},
};
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroBoard.Sample",
    crate_name: "ferroboard_sample",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};
const NAMESPACES: &[(&str, &str)] = &[("ferroboard_sample", "FerroBoard.Sample")];
ferroui_base::ferro_rust_paths! {classes:[crate::App,crate::MainWindow,crate::MainView,crate::settings_panel::SettingsPanel,crate::language_check_box::LanguageCheckBox],types:[crate::bindings::SampleBindings,crate::composition::SampleComposition,crate::language_bindings::LanguageRowBindings,crate::language_bindings::LanguagePanelBindings],contracts:[],generics:[],generic_contracts:[]}
pub fn register_types() {
    ferroboard_controls::register_types();
    ferroui_markup_xaml::register_types();
    ferroui_themes_winui::register_types();
    TypeInfo::register_namespaces(NAMESPACES);
    TypeInfo::register_all(&[
        crate::App::TYPE,
        crate::MainWindow::TYPE,
        crate::MainView::TYPE,
        crate::settings_panel::SettingsPanel::TYPE,
        crate::language_check_box::LanguageCheckBox::TYPE,
    ]);
    ferroui_base::metadata::MarkupAssembly::register(&ASSEMBLY);
    register_rust_paths();
    crate::bindings::register();
    crate::language_bindings::register();
    MarkupType::register_all(&[crate::composition::SampleComposition::MARKUP]);
    crate::compiled_markup::register();
}
