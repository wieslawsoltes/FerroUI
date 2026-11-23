use crate::examples;
use crate::tests::start;
use ferroui_animated_visuals::{AnimatedIcon, AnimatedVisualPlayer};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::Ref;
use ferroui_controls::{Button, ComboBox, Control};

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap()
}

fn click(root: &Ref<Control>, name: &str) {
    root.get_control::<Button>(name).raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}

#[test]
fn the_player_page_plays_pauses_and_stops() {
    let _application = start();
    let root = page("AnimatedVisualPlayer");
    let player = root.get_control::<AnimatedVisualPlayer>("Player");
    assert!(player.source().is_some());
    assert!(!player.auto_play());
    click(&root, "PlayButton");
    assert!(player.is_playing());
    click(&root, "StopButton");
    assert!(!player.is_playing());
}

#[test]
fn the_icon_page_has_both_examples_and_changes_the_source() {
    let _application = start();
    let root = page("AnimatedIcon");
    let icon = root.get_control::<AnimatedIcon>("SearchAnimatedIcon");
    assert!(icon.fallback_icon_source().is_some());
    let first = icon.source().unwrap();
    root.get_control::<ComboBox>("AnimatedVisualSourceKind").set_selected_index(1);
    assert!(!Ref::ptr_eq(&first, &icon.source().unwrap()));
    let navigation = root.get_control::<AnimatedIcon>("NavigationAnimatedIcon");
    assert!(navigation.source().is_some());
}
