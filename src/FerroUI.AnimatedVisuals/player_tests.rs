//! The player and the icon in a tree: loading, playing, the fallback, states and colours.

use super::*;
use crate::{AnimatedBackVisualSource, LottieVisualSource};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_controls::testing::{MockWindowingPlatform, TestServices, UnitTestApplication};
use ferroui_controls::Window;

const DOCUMENT: &str = r#"{"fr":30,"ip":0,"op":30,"w":40,"h":40,"layers":[],
    "markers":[{"cm":"Normal","tm":6,"dr":0},{"cm":"Pressed","tm":24,"dr":0}]}"#;
const LONGER: &str = r#"{"fr":30,"ip":0,"op":60,"w":40,"h":40,"layers":[]}"#;

fn attach(content: Ref<Control>) -> Ref<Window> {
    let window = Window::with_impl(MockWindowingPlatform::create_window_mock_with_size(200.0, 200.0));
    window.set_content(Some(Control::boxed(&content)));
    window.show();
    window.layout_manager().execute_initial_layout_pass();
    window
}

fn source(json: &str) -> Ref<AnimatedVisualSource> {
    LottieVisualSource::create_from_string(json).upcast()
}

#[test]
fn a_player_in_a_tree_loads_its_source_and_plays() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let player = AnimatedVisualPlayer::new();
    player.set_source(Some(source(DOCUMENT)));
    assert!(!player.is_animated_visual_loaded(), "a source is loaded in a tree");
    let _window = attach(player.clone().upcast());
    assert!(player.is_animated_visual_loaded());
    assert!(player.is_playing(), "AutoPlay");
    assert_eq!(player.duration(), Duration::from_secs(1));
}

#[test]
fn going_to_a_progress_ends_the_play() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let player = AnimatedVisualPlayer::new();
    player.set_auto_play(false);
    player.set_source(Some(source(DOCUMENT)));
    let _window = attach(player.clone().upcast());
    assert!(!player.is_playing());
    let play = player.play_async(0.0, 1.0, false);
    assert!(player.is_playing());
    player.set_progress(0.5);
    assert!(play.is_completed());
    assert!(!player.is_playing());
    assert_eq!(player.progress(), 0.5);
}

#[test]
fn a_bad_source_shows_the_fallback_until_a_good_one_comes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let player = AnimatedVisualPlayer::new();
    let fallback = Control::new();
    player.set_fallback_content(Some(fallback.clone()));
    player.set_source(Some(source("not json")));
    let _window = attach(player.clone().upcast());
    assert!(player.load_error().is_some());
    assert!(player.visual_children().contains(&fallback.clone().upcast::<Visual>()));
    player.set_source(Some(source(DOCUMENT)));
    assert!(player.load_error().is_none());
    assert!(!player.visual_children().contains(&fallback.upcast::<Visual>()));
}

#[test]
fn a_new_document_of_the_source_reaches_the_player_and_a_colour_keeps_the_play() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let player = AnimatedVisualPlayer::new();
    let lottie = LottieVisualSource::create_from_string(DOCUMENT);
    player.set_source(Some(lottie.clone().upcast()));
    let _window = attach(player.clone().upcast());
    player.set_progress(0.5);
    lottie.set_color_property("Foreground", Color::from_rgb(1, 2, 3));
    assert_eq!(player.progress(), 0.5, "a colour does not start the play again");
    assert!(!player.is_playing());
    lottie.set_source(LONGER);
    assert_eq!(player.duration(), Duration::from_secs(2));
}

#[test]
fn a_state_is_taken_after_the_next_layout() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let icon = AnimatedIcon::new();
    icon.set_source(Some(source(DOCUMENT)));
    let window = attach(icon.clone().upcast());
    AnimatedIcon::set_state(&icon, Some("Pressed".to_owned()));
    assert_eq!(icon.progress(), 0.0, "not before the layout");
    window.layout_manager().execute_layout_pass();
    assert!((icon.progress() - 0.8).abs() < 1e-9, "{}", icon.progress());
}

#[test]
fn an_icon_in_a_toggle_follows_it_until_the_application_sets_a_state() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let icon = AnimatedIcon::new();
    let toggle = ToggleButton::new();
    toggle.set_is_checked(Some(false));
    toggle.set_content(Some(Control::boxed(&icon)));
    let _window = attach(toggle.clone().upcast());
    assert_eq!(AnimatedIcon::get_state(&icon).as_deref(), Some("NormalOff"));
    toggle.set_is_checked(Some(true));
    assert_eq!(AnimatedIcon::get_state(&icon).as_deref(), Some("NormalOn"));
    toggle.set_is_checked(None);
    assert_eq!(AnimatedIcon::get_state(&icon).as_deref(), Some("NormalIndeterminate"));
    AnimatedIcon::set_state(&icon, Some("Custom".to_owned()));
    toggle.set_is_checked(Some(false));
    assert_eq!(AnimatedIcon::get_state(&icon).as_deref(), Some("Custom"));
}

#[test]
fn an_icon_leaves_its_owner_when_it_is_detached() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let icon = AnimatedIcon::new();
    let button = Button::new();
    button.set_content(Some(Control::boxed(&icon)));
    let window = attach(button.clone().upcast());
    let subscribed = button.property_changed_subscriber_count();
    assert_eq!(AnimatedIcon::get_state(&icon).as_deref(), Some("Normal"));
    window.set_content(None);
    assert_eq!(button.property_changed_subscriber_count(), subscribed - 1);
}

#[test]
fn the_foreground_of_an_icon_reaches_its_renderer() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let icon = AnimatedIcon::new();
    icon.set_source(Some(AnimatedBackVisualSource::new().upcast::<LottieVisualSource>().upcast()));
    let _window = attach(icon.clone().upcast());
    assert!(icon.presenter.borrow().visual.is_some());
    icon.set_foreground(Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(200, 10, 20)))));
    assert_eq!(icon.presenter.borrow().colors, vec![("Foreground".to_owned(), Color::from_rgb(200, 10, 20))]);
    icon.set_foreground(None);
}

#[test]
fn an_icon_source_makes_an_icon_with_its_values_and_follows_them() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = crate::AnimatedIconSource::new();
    let visual = self::source(DOCUMENT);
    source.set_source(Some(visual.clone()));
    source.set_mirrored_when_right_to_left(true);
    let element = source.create_icon_element().expect("an icon");
    let icon = element.cast::<AnimatedIcon>().expect("an animated icon");
    assert!(icon.source().is_some_and(|shown| shown.ptr_eq(&visual)));
    assert!(icon.mirrored_when_right_to_left());
    source.set_mirrored_when_right_to_left(false);
    assert!(!icon.mirrored_when_right_to_left());
}

#[test]
fn an_icon_in_a_check_box_a_button_or_a_navigation_item_takes_their_states() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let icon = AnimatedIcon::new();
    let check = ferroui_controls::CheckBox::new();
    check.set_is_checked(Some(true));
    check.set_content(Some(Control::boxed(&icon)));
    let _window = attach(check.clone().upcast());
    assert_eq!(AnimatedIcon::get_state(&icon).as_deref(), Some("NormalOn"));

    let icon = AnimatedIcon::new();
    let back = Button::new();
    back.set_content(Some(Control::boxed(&icon)));
    let _window = attach(back.clone().upcast());
    assert_eq!(AnimatedIcon::get_state(&icon).as_deref(), Some("Normal"));

    let icon = AnimatedIcon::new();
    let item = NavigationViewItem::new();
    let child = icon.clone();
    item.set_template(Some(ferroui_controls::templates::FuncControlTemplate::new(move |_, _| {
        let border = ferroui_controls::Border::new();
        border.set_child(Some(child.clone().upcast()));
        border.upcast()
    })));
    let _window = attach(item.clone().upcast());
    assert_eq!(AnimatedIcon::get_state(&icon).as_deref(), Some("Normal"));
}

#[test]
fn the_names_of_the_states_are_those_of_the_winui_templates() {
    assert_eq!(state_name(false, false, None), "Normal");
    assert_eq!(state_name(false, true, None), "PointerOver");
    assert_eq!(state_name(true, true, None), "Pressed");
    assert_eq!(state_name(false, true, Some(Some(false))), "PointerOverOff");
    assert_eq!(state_name(true, false, Some(Some(true))), "PressedOn");
    assert_eq!(state_name(false, false, Some(None)), "NormalIndeterminate");
}

#[test]
fn the_catalog_pages_build_and_show_their_controls() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let player_page = crate::catalog_pages::player_page();
    let _window = attach(player_page.clone().upcast());
    let icon_page = crate::catalog_pages::icon_page();
    let _other = attach(icon_page.clone().upcast());
}

#[test]
fn the_gallery_handlers_play_and_set_states() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    ferroui_markup_xaml::register_types();
    crate::register_types();
    let load = |page: &str| {
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(page, None)
            .unwrap_or_else(|error| panic!("{error}"))
    };
    let root = load(include_str!("../../samples/WinUIGallery/Pages/AnimatedVisualPlayer.xaml"));
    crate::gallery_samples::wire(&root);
    let player = root.get_control::<AnimatedVisualPlayer>("Player");
    let click = RoutedEventArgs::with_event(&Button::click_event());
    root.get_control::<Button>("PlayButton").raise_event(&click);
    assert!(player.is_playing());
    root.get_control::<Button>("StopButton").raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    assert!(!player.is_playing());
    let root = load(include_str!("../../samples/WinUIGallery/Pages/AnimatedIcon.xaml"));
    crate::gallery_samples::wire(&root);
    let icon = root.get_control::<AnimatedIcon>("SearchAnimatedIcon");
    let first = icon.source().unwrap();
    root.get_control::<ComboBox>("AnimatedVisualSourceKind").set_selected_index(1);
    assert!(!first.ptr_eq(&icon.source().unwrap()), "the choice of visual changes the source");
}

/// The pages of the gallery, read by the runtime loader: the classes of this crate in markup.
#[test]
fn the_gallery_pages_load_with_the_classes_of_this_crate() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    ferroui_markup_xaml::register_types();
    crate::register_types();
    let load = |page: &str| {
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(page, None)
            .unwrap_or_else(|error| panic!("{error}"))
    };
    let root = load(include_str!("../../samples/WinUIGallery/Pages/AnimatedVisualPlayer.xaml"));
    let player = root.find_control::<Control>("Player").and_then(|control| control.cast::<AnimatedVisualPlayer>());
    assert!(player.expect("the player").source().is_some(), "the content of a player is its source");
    let root = load(include_str!("../../samples/WinUIGallery/Pages/AnimatedIcon.xaml"));
    for name in ["SearchAnimatedIcon", "NavigationAnimatedIcon"] {
        let icon = root.find_control::<Control>(name).and_then(|control| control.cast::<AnimatedIcon>());
        let icon = icon.unwrap_or_else(|| panic!("{name}"));
        assert!(icon.source().is_some(), "{name}: the content of an icon is its source");
        assert!(icon.fallback_icon_source().is_some(), "{name}: the fallback");
    }
}
