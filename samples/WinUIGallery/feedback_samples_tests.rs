//! Acceptance of the original page handlers, separate from control contract tests.

use crate::{examples, tests::{press, start}};
use ferroui_base::input::Key;
use ferroui_base::controls::ResourceKey;
use ferroui_base::media::{Brushes, Color, IBrush};
use ferroui_base::styling::ControlTheme;
use ferroui_base::Ref;
use ferroui_controls::{
    CheckBox, ComboBox, Control, FontIconSource, InfoBadge, NumberBox, ProgressRing,
    RatingControl, Slider, TextBlock, ToggleSwitch,
};

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap()
}

#[test]
fn progress_ring_page_preserves_both_source_modes_and_active_option() {
    let _application = start();
    let root = page("ProgressRing");
    let spinner = root.get_control::<ProgressRing>("ProgressRing1");
    let determinate = root.get_control::<ProgressRing>("ProgressRing2");
    assert!(spinner.is_active() && spinner.is_indeterminate());
    assert!(!determinate.is_indeterminate());
    let toggle = root.get_control::<ToggleSwitch>("ProgressToggle");
    toggle.set_is_checked(Some(false));
    assert!(!spinner.is_active());
    toggle.set_is_checked(Some(true));
    assert!(spinner.is_active());
    for (option, ring) in [
        ("BackgroundComboBox1", spinner),
        ("BackgroundComboBox2", determinate),
    ] {
        root.get_control::<ComboBox>(option).set_selected_index(1);
        assert_eq!(
            ring.background()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            Color::from_rgb(211, 211, 211)
        );
        root.get_control::<ComboBox>(option).set_selected_index(0);
        assert_eq!(
            ring.background()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            Brushes::transparent()
                .as_solid_color_brush()
                .unwrap()
                .color()
        );
    }
}

#[test]
fn progress_ring_page_restores_empty_numeric_input_and_updates_the_native_range() {
    let _application = start();
    let root = page("ProgressRing");
    let number = root.get_control::<NumberBox>("ProgressValue");
    let ring = root.get_control::<ProgressRing>("ProgressRing2");
    number.set_numeric_value(67.5);
    assert_eq!(ring.value(), 67.5);
    number.set_numeric_value(f64::NAN);
    assert_eq!(number.value(), 0.0);
    assert_eq!(ring.value(), 0.0);
    number.set_numeric_value(100.0);
    assert_eq!(ring.value(), 100.0);
}

#[test]
fn rating_page_changes_caption_only_for_user_changes_and_obeys_clear_and_readonly() {
    let _application = start();
    let root = page("RatingControl");
    let rating = root.get_control::<RatingControl>("RatingControl1");
    assert_eq!(rating.caption(), "312 ratings");
    assert!(!rating.is_clear_enabled());
    rating.set_rating_value(2.0);
    assert_eq!(rating.caption(), "312 ratings");
    assert_eq!(
        root.get_control::<TextBlock>("RatingOutput")
            .text()
            .as_deref(),
        Some("2")
    );
    // The user moves the rating from 2 to 4 with the keyboard.
    press(&rating, Key::Right, false);
    press(&rating, Key::Right, false);
    assert_eq!(rating.caption(), "Your rating");
    assert_eq!(
        root.get_control::<TextBlock>("RatingOutput")
            .text()
            .as_deref(),
        Some("4")
    );
    root.get_control::<CheckBox>("ClearEnabled")
        .set_is_checked(Some(true));
    // With IsClearEnabled, Home (a rating of 0) clears the rating.
    press(&rating, Key::Home, false);
    assert_eq!(rating.value(), -1.0);
    root.get_control::<CheckBox>("ReadOnly")
        .set_is_checked(Some(true));
    // Read only: the keys change nothing.
    press(&rating, Key::End, false);
    assert_eq!(rating.value(), -1.0);
}

#[test]
fn rating_page_tracks_fractional_placeholder_in_both_directions() {
    let _application = start();
    let root = page("RatingControl");
    let rating = root.get_control::<RatingControl>("RatingControl2");
    let slider = root.get_control::<Slider>("PlaceholderSlider");
    slider.set_range_value(3.5);
    assert_eq!(rating.placeholder_value(), 3.5);
    assert_eq!(rating.value(), -1.0);
    rating.set_placeholder_value(2.5);
    assert_eq!(slider.value(), 2.5);
    press(&rating, Key::End, false);
    assert_eq!(rating.placeholder_value(), 2.5);
}

#[test]
fn info_badge_page_switches_actual_source_themes_and_keeps_original_icon() {
    let _application = start();
    let root = page("InfoBadge");
    for (index, severity) in ["Attention", "Informational", "Success", "Critical"]
        .iter()
        .enumerate()
    {
        root.get_control::<ComboBox>("InfoBadgeStyleComboBox")
            .set_selected_index(index as i32);
        for (name, kind) in [
            ("StyleIconBadge", "Icon"),
            ("StyleValueBadge", "Value"),
            ("StyleDotBadge", "Dot"),
        ] {
            let value = root
                .try_find_resource(
                    &ResourceKey::from(format!("{severity}{kind}InfoBadgeStyle")),
                    None,
                )
                .flatten()
                .unwrap();
            let expected = value.downcast_ref::<Ref<ControlTheme>>().unwrap();
            assert_eq!(
                root.get_control::<InfoBadge>(name).theme().unwrap(),
                *expected
            );
        }
    }
    assert_eq!(root.get_control::<InfoBadge>("StyleValueBadge").value(), 10);
    let badge = root.get_control::<InfoBadge>("EmbeddedInfoBadge");
    let icon = badge
        .icon_source()
        .unwrap()
        .cast::<FontIconSource>()
        .unwrap();
    assert_eq!(icon.glyph(), "\u{f13c}");
}

#[test]
fn info_badge_page_preserves_source_integer_guard_for_dynamic_values() {
    let _application = start();
    let root = page("InfoBadge");
    let badge = root.get_control::<InfoBadge>("DynamicInfoBadge");
    let value = root.get_control::<NumberBox>("ValueNumberBox");
    assert_eq!(badge.value(), 1);
    value.set_numeric_value(27.9);
    assert_eq!(badge.value(), 27);
    value.set_numeric_value(f64::NAN);
    assert_eq!(badge.value(), 27);
    value.set_numeric_value(i32::MAX as f64 + 1.0);
    assert_eq!(badge.value(), 27);
    value.set_numeric_value(-1.0);
    assert_eq!(badge.value(), -1);
    value.set_numeric_value(0.0);
    assert_eq!(badge.value(), 0);
}

#[test]
fn info_badge_navigation_composition_keeps_original_badge_and_opacity_handler() {
    use ferroui_controls::{NavigationView, NavigationViewItem};
    let _application = start();
    let root = page("InfoBadge");
    let nav = root.get_control::<NavigationView>("nvSample1");
    assert_eq!(nav.menu_items().count(), 3);
    let inbox = root.get_control::<NavigationViewItem>("InboxPage");
    let badge = root.get_control::<InfoBadge>("infoBadge1");
    assert_eq!(inbox.info_badge(), Some(badge.clone()));
    assert_eq!(badge.value(), 5);
    assert_eq!(badge.opacity(), 1.0);
    let toggle = root.get_control::<ToggleSwitch>("ToggleInfoBadgeOpacity");
    toggle.set_is_checked(Some(false));
    assert_eq!(badge.opacity(), 0.0);
    assert_eq!(badge.value(), 5);
    toggle.set_is_checked(Some(true));
    assert_eq!(badge.opacity(), 1.0);
}

#[test]
fn info_badge_navigation_modes_preserve_the_same_notification_and_selection() {
    use ferroui_controls::{Frame, NavigationView, NavigationViewPaneDisplayMode};
    let _application = start();
    let root = page("InfoBadge");
    let nav = root.get_control::<NavigationView>("nvSample1");
    let badge = root.get_control::<InfoBadge>("infoBadge1");
    let inbox = nav.menu_items().get_at(2).unwrap();
    nav.set_selected_item(Some(inbox.clone()));
    let modes = root.get_control::<ComboBox>("NavigationViewDisplayMode");
    for (index, expected, open) in [
        (1, NavigationViewPaneDisplayMode::LeftCompact, false),
        (2, NavigationViewPaneDisplayMode::Top, true),
        (0, NavigationViewPaneDisplayMode::Left, true),
    ] {
        modes.set_selected_index(index);
        assert_eq!(nav.pane_display_mode(), expected);
        assert_eq!(nav.is_pane_open(), open);
        assert!(std::rc::Rc::ptr_eq(&nav.selected_item().unwrap(), &inbox));
        assert_eq!(root.get_control::<InfoBadge>("infoBadge1"), badge);
        assert_eq!(badge.value(), 5);
    }
    // This source example has an empty Frame and no navigation handler.
    assert!(root
        .get_control::<Frame>("contentFrame")
        .current_page()
        .is_none());
}

#[test]
fn feedback_pages_and_callbacks_release_their_native_controls() {
    let _application = start();
    for id in ["InfoBadge", "ProgressRing", "RatingControl"] {
        let root = page(id);
        let weak = root.downgrade();
        drop(root);
        assert!(weak.upgrade().is_none(), "{id} retained its page");
    }
}

#[test]
fn compact_sizing_copies_every_field_across_distinct_page_types() {
    use ferroui_base::animation::TimeSpan;
    use ferroui_base::utilities::DateTimeOffset;
    use ferroui_controls::{Frame, RadioButtons};
    use ferroui_controls::{DatePicker, TextBox};
    let _application = start();
    let root = page("CompactSizing");
    let frame = root.get_control::<Frame>("SizingFrame");
    let options = root.get_control::<RadioButtons>("SizingMode");
    let initial_type = frame.current_source_page_type();
    let initial = crate::sizing_sample::form(&frame).unwrap();
    for (name, value) in [
        ("FirstName", "Ada"),
        ("LastName", "Lovelace"),
        ("Password", "example only"),
        ("ConfirmPassword", "example only"),
    ] {
        initial.get_control::<TextBox>(name).set_text(Some(value));
    }
    let selected = DateTimeOffset::new(2026, 10, 22, 0, 0, 0, TimeSpan::ZERO);
    initial
        .get_control::<DatePicker>("ChosenDate")
        .set_selected_date(Some(selected));
    options.set_selected_index(1);
    assert_ne!(frame.current_source_page_type(), initial_type);
    let compact = crate::sizing_sample::form(&frame).unwrap();
    for name in ["FirstName", "LastName", "Password", "ConfirmPassword"] {
        assert_eq!(
            compact.get_control::<TextBox>(name).text(),
            initial.get_control::<TextBox>(name).text()
        );
    }
    assert_eq!(
        compact
            .get_control::<DatePicker>("ChosenDate")
            .selected_date(),
        Some(selected)
    );
    compact
        .get_control::<TextBox>("LastName")
        .set_text(Some("Byron"));
    compact
        .get_control::<DatePicker>("ChosenDate")
        .set_selected_date(None);
    options.set_selected_index(0);
    assert_eq!(frame.current_source_page_type(), initial_type);
    let standard = crate::sizing_sample::form(&frame).unwrap();
    assert_eq!(
        standard
            .get_control::<TextBox>("LastName")
            .text()
            .as_deref(),
        Some("Byron")
    );
    assert_eq!(
        standard
            .get_control::<DatePicker>("ChosenDate")
            .selected_date(),
        None
    );
    assert!(!root.get_control::<TextBlock>("SizingError").is_visible());
}

#[test]
fn compact_sizing_resources_remain_page_scoped_and_release_with_the_gallery_page() {
    use ferroui_controls::{Frame, RadioButtons};
    use ferroui_controls::Application;
    let _application = start();
    let key = ResourceKey::from("TextControlThemeMinHeight");
    let app = Application::current().unwrap();
    assert!(!app.resources().contains_key(&key));
    let root = page("CompactSizing");
    let frame = root.get_control::<Frame>("SizingFrame");
    assert!(!frame.current_page().unwrap().resources().contains_key(&key));
    root.get_control::<RadioButtons>("SizingMode")
        .set_selected_index(1);
    let compact = frame.current_page().unwrap();
    let height = compact.resources().try_get_value(&key).flatten().unwrap();
    assert_eq!(height.downcast_ref::<f64>(), Some(&24.0));
    assert!(!app.resources().contains_key(&key));
    root.get_control::<RadioButtons>("SizingMode")
        .set_selected_index(0);
    assert!(!frame.current_page().unwrap().resources().contains_key(&key));
    let weak_root = root.downgrade();
    let weak_frame = frame.downgrade();
    let weak_compact = compact.downgrade();
    drop(compact);
    drop(frame);
    drop(root);
    assert!(weak_root.upgrade().is_none());
    assert!(weak_frame.upgrade().is_none());
    assert!(weak_compact.upgrade().is_none());
}
