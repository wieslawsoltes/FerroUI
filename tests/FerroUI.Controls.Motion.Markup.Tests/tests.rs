use ferroui_base::numerics::Vector3;
use ferroui_base::{Ref, Vector3D};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::primitives::Popup;
use ferroui_controls::{Control, ItemsControl, StackPanel};
use ferroui_controls_motion::{
    AddDeleteThemeTransition, ContentThemeTransition, EntranceThemeTransition, ImplicitTransitions,
    ParallaxSourceOffsetKind, ParallaxView, PopupThemeTransition, RepositionThemeTransition, ThemeTransitions,
};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::time::Duration;

fn named(root: &Ref<StackPanel>, name: &str) -> Ref<Control> {
    root.find_control::<Control>(name).expect(name)
}

fn assert_page(root: &Ref<StackPanel>) {
    let opacity = named(root, "Opacity");
    assert_eq!(opacity.opacity_transition().map(|value| value.duration()), Some(Duration::from_millis(500)));
    let rotation = named(root, "Rotation");
    assert!(rotation.rotation_transition().is_some());
    assert_eq!(rotation.rotation(), 30.0);
    let scale = named(root, "Scale");
    assert!(scale.scale_transition().is_some());
    assert_eq!(scale.scale(), Vector3::new(2.0, 2.0, 1.0));
    let translation = named(root, "Translation");
    let components = translation.translation_transition().map(|value| value.components());
    assert_eq!(components, Some(ferroui_base::Vector3TransitionComponents::X | ferroui_base::Vector3TransitionComponents::Y));
    assert_eq!(translation.translation(), Vector3D::new(4.0, 5.0, 0.0));
    for name in ["Presenter", "Panel"] {
        let element = named(root, name);
        assert!(ImplicitTransitions::get_background_transition(&element).is_some(), "{name}");
        assert_eq!(element.transitions().map(|list| list.count()), Some(1), "{name}");
    }
    let parallax = named(root, "Parallax").cast::<ParallaxView>().expect("a parallax view");
    assert_eq!(parallax.vertical_shift(), 100.0);
    assert_eq!(parallax.horizontal_source_offset_kind(), ParallaxSourceOffsetKind::Absolute);
    assert!(parallax.child().is_some());

    let children = named(root, "ThemeChildren").cast::<StackPanel>().expect("a stack panel");
    let transitions = ThemeTransitions::get_children_transitions(&children).expect("the children transitions");
    assert_eq!(transitions.count(), 2);
    let entrance = transitions.get(0).cast::<EntranceThemeTransition>().expect("an entrance transition");
    assert!(entrance.is_staggering_enabled());
    assert_eq!(entrance.from_vertical_offset(), 12.0);
    assert!(transitions.get(1).cast::<AddDeleteThemeTransition>().is_some());

    let own = ThemeTransitions::get_transitions(&named(root, "ThemeOwn")).expect("the transitions of the element");
    let reposition = own.get(0).cast::<RepositionThemeTransition>().expect("a reposition transition");
    assert!(!reposition.is_staggering_enabled());

    let items = named(root, "ThemeItems").cast::<ItemsControl>().expect("an items control");
    let containers = ThemeTransitions::get_item_container_transitions(&items).expect("the container transitions");
    let content = containers.get(0).cast::<ContentThemeTransition>().expect("a content transition");
    assert_eq!(content.vertical_offset(), 10.0);

    let popup = named(root, "ThemePopup").cast::<Popup>().expect("a popup");
    let child = ThemeTransitions::get_child_transitions(&popup).expect("the child transitions");
    let opening = child.get(0).cast::<PopupThemeTransition>().expect("a popup transition");
    assert_eq!(opening.from_vertical_offset(), 16.0);
}

#[test]
fn the_compiled_loader_resolves_the_winui_names_without_a_prefix() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    ferroui_controls_motion::register_types();
    let value = crate::compiled_xaml::try_load(None, "ferres://ferroui_controls_motion_markup_tests/DefaultNamespace.xaml")
        .unwrap()
        .expect("the compiled page");
    assert_page(value.downcast_ref::<Ref<StackPanel>>().expect("a stack panel"));
}

#[test]
fn the_run_time_loader_resolves_the_winui_names_without_a_prefix() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    ferroui_markup_xaml::register_types();
    ferroui_controls_motion::register_types();
    let root = FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(include_str!("DefaultNamespace.xaml"), None)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_page(&root.cast::<StackPanel>().expect("a stack panel"));
}
