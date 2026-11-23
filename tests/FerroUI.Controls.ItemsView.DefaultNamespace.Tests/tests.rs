use ferroui_base::metadata::from_markup_value;
use ferroui_base::Ref;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::StackPanel;
use ferroui_controls_items_view::{ItemContainer, ItemsView, ItemsViewSelectionMode};
use ferroui_markup_xaml::RuntimeXamlLoaderDocument;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

const DOCUMENT: &str = include_str!("DefaultNamespaceRoot.xaml");

fn register() {
    ferroui_markup_xaml::register_types();
    ferroui_controls::register_types();
    ferroui_controls_items_view::register_types();
}

fn assert_resolved(root: &Ref<StackPanel>) {
    let children = root.children().snapshot();
    assert_eq!(children.len(), 2);
    let view = children[0].clone().cast::<ItemsView>().expect("an ItemsView");
    assert_eq!(view.selection_mode(), ItemsViewSelectionMode::Multiple);
    assert!(children[1].clone().cast::<ItemContainer>().is_some());
}

#[test]
fn the_compiled_loader_resolves_the_controls_without_a_prefix() {
    let _scope = UnitTestApplication::start(TestServices::styled_window());
    register();
    crate::compiled_markup::register();
    let value = crate::compiled_xaml::try_load(None, "ferres://ferroui_controls_itemsview_default_namespace_tests/DefaultNamespaceRoot.xaml")
        .unwrap()
        .unwrap();
    assert_resolved(value.downcast_ref::<Ref<StackPanel>>().unwrap());
}

#[test]
fn the_run_time_loader_resolves_the_controls_without_a_prefix() {
    let _scope = UnitTestApplication::start(TestServices::styled_window());
    register();
    FerroRuntimeXamlLoader::register();
    let value = FerroRuntimeXamlLoader::load_document(RuntimeXamlLoaderDocument::new(DOCUMENT), None)
        .unwrap_or_else(|error| panic!("the document failed to load: {}", error.message()));
    assert_resolved(&from_markup_value::<Ref<StackPanel>>(&Some(value)).expect("a StackPanel"));
}
