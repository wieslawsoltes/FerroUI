use ferroui_animated_visuals::{
    AnimatedAcceptVisualSource, AnimatedBackVisualSource, AnimatedChevronDownSmallVisualSource,
    AnimatedChevronRightDownSmallVisualSource, AnimatedChevronUpDownSmallVisualSource, AnimatedFindVisualSource,
    AnimatedGlobalNavigationButtonVisualSource, AnimatedIcon, AnimatedSettingsVisualSource,
    AnimatedVisualPlayer, LottieVisualSource,
};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupAssembly, FERRO_XML_NAMESPACE};
use ferroui_base::{FerroObject, Ref};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Control, StackPanel};

fn resource(root: &Ref<StackPanel>, key: &str) -> Ref<FerroObject> {
    let resources = root.resources();
    let value = resources.try_get_resource(&key.into(), None).flatten().unwrap_or_else(|| panic!("the resource {key}"));
    ValueTypes::as_object(&*value).unwrap_or_else(|| panic!("the object of {key}"))
}

fn assert_every_type(root: &Ref<StackPanel>) {
    assert!(resource(root, "Accept").cast::<AnimatedAcceptVisualSource>().is_some());
    assert!(resource(root, "Back").cast::<AnimatedBackVisualSource>().is_some());
    assert!(resource(root, "ChevronDown").cast::<AnimatedChevronDownSmallVisualSource>().is_some());
    assert!(resource(root, "ChevronRightDown").cast::<AnimatedChevronRightDownSmallVisualSource>().is_some());
    assert!(resource(root, "ChevronUpDown").cast::<AnimatedChevronUpDownSmallVisualSource>().is_some());
    assert!(resource(root, "Find").cast::<AnimatedFindVisualSource>().is_some());
    assert!(resource(root, "GlobalNavigation").cast::<AnimatedGlobalNavigationButtonVisualSource>().is_some());
    assert!(resource(root, "Settings").cast::<AnimatedSettingsVisualSource>().is_some());
    assert!(resource(root, "Lottie").cast::<LottieVisualSource>().is_some());
    let player = root.find_control::<Control>("Player").and_then(|control| control.cast::<AnimatedVisualPlayer>());
    assert!(player.expect("the player").source().is_some());
    let icon = root.find_control::<Control>("Icon").and_then(|control| control.cast::<AnimatedIcon>());
    assert!(icon.expect("the icon").source().is_some());
}

#[test]
fn the_crate_maps_its_namespace_to_the_default_xml_namespace() {
    let definitions: &[_] = ferroui_animated_visuals::ASSEMBLY.xmlns_definitions;
    assert!(definitions.iter().all(|definition| definition.xml_namespace == FERRO_XML_NAMESPACE));
    assert!(definitions.iter().any(|definition| definition.namespace == "FerroUI.AnimatedVisuals"));
    let _: &MarkupAssembly = &ferroui_animated_visuals::ASSEMBLY;
}

#[test]
fn the_compiled_loader_resolves_the_types_without_a_prefix() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    ferroui_animated_visuals::register_types();
    let value = crate::compiled_xaml::try_load(None, "ferres://ferroui_animated_visuals_markup_tests/DefaultNamespace.xaml")
        .unwrap()
        .expect("the compiled page");
    assert_every_type(value.downcast_ref::<Ref<StackPanel>>().expect("a stack panel"));
}

#[test]
fn the_run_time_loader_resolves_the_types_without_a_prefix() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    ferroui_markup_xaml::register_types();
    ferroui_animated_visuals::register_types();
    let root = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(
        include_str!("DefaultNamespace.xaml"),
        None,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_every_type(&root.cast::<StackPanel>().expect("a stack panel"));
}
