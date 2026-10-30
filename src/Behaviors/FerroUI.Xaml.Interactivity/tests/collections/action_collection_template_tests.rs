//! Ported from
//! `tests/Xaml.Behaviors.Interactivity.UnitTests/Collections/ActionCollectionTemplateTests.cs` of
//! Xaml.Behaviors (with its window `ActionCollectionTemplate001`, `ActionCollectionTemplate001.xaml`
//! here, loaded with the run-time loader).
//!
//! A module of the integration test target `collection_template_tests`
//! (`collection_template_tests.rs`), not of the unit tests of the library: see there.

use crate::start_app;
use ferroui_xaml_interactivity::{Interaction, ASSEMBLY};
use ferroui_base::controls::{NameScope, ResourceKey};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::Ref;
use ferroui_controls::{Button, Window};
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use ferroui_xaml_interactions::{ChangePropertyAction, EventTriggerBehavior};
use std::rc::Rc;

fn load_window() -> Ref<Window> {
    FerroRuntimeXamlLoader::register();
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);
    let loaded = FerroRuntimeXamlLoader::load_document(
        RuntimeXamlLoaderDocument::new(include_str!("ActionCollectionTemplate001.xaml")),
        Some(configuration),
    )
    .unwrap_or_else(|error| panic!("ActionCollectionTemplate001.xaml failed to load: {}", error.message()));
    from_markup_value::<Ref<Window>>(&Some(loaded)).expect("a window")
}

fn find_button(window: &Window, name: &str) -> Ref<Button> {
    NameScope::get_name_scope(window)
        .and_then(|scope| scope.find(name))
        .and_then(|object| object.cast::<Button>())
        .expect("the named button")
}

#[test]
fn action_collection_template_001() {
    let _app = start_app();
    let window = load_window();

    window.show();

    let buttons = [
        find_button(&window, "TargetButton1"),
        find_button(&window, "TargetButton2"),
        find_button(&window, "TargetButton3"),
    ];

    for button in &buttons {
        let behaviors = button.get_value(Interaction::behaviors_property());
        assert!(behaviors.is_some());
        let behaviors = behaviors.unwrap();
        assert_eq!(1, behaviors.count());
        let trigger = behaviors.get(0).cast::<EventTriggerBehavior>().expect("an EventTriggerBehavior");
        let actions = trigger.actions();
        assert!(actions.is_some());
        let actions = actions.unwrap();
        assert_eq!(1, actions.count());
        assert!(actions.get(0).is::<ChangePropertyAction>());
    }

    for button in &buttons {
        assert_eq!(button.background(), Some(Brushes::transparent() as Rc<dyn IBrush>));
    }

    let red_brush = from_markup_value::<Rc<dyn IBrush>>(&window.resources().get(&ResourceKey::from("RedBrush")));
    assert!(red_brush.is_some());
    for button in &buttons {
        button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));

        assert_eq!(red_brush, button.background());
    }
}
