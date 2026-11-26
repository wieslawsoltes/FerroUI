//! The ScrollView page: the source's three examples with their code behind
//! (`scroll_view_samples.rs`).

use crate::scroll_view_samples::{constant_velocity, scroll_bar_visibility, scroll_mode, target_vertical_offset};
use crate::{examples, tests::start};
use ferroui_base::Ref;
use ferroui_controls::{
    ComboBox, Control, Image, ScrollView, ScrollingScrollBarVisibility, ScrollingScrollMode, ScrollingZoomMode, StackPanel,
};

fn page() -> Ref<Control> {
    examples::create(examples::find("ScrollView").unwrap()).unwrap_or_else(|error| panic!("ScrollView: {error}"))
}

#[test]
fn scroll_view_options_set_the_modes_and_visibilities_of_the_first_view() {
    let _application = start();
    let root = page();
    let view = root.get_control::<ScrollView>("scrollView1");
    // The source's attributes.
    assert_eq!(view.zoom_mode(), ScrollingZoomMode::Enabled);
    assert_eq!(view.content_orientation(), ferroui_controls::ScrollingContentOrientation::None);
    assert!(root.find_control::<Image>("cliffImage").is_some());
    root.get_control::<ComboBox>("cmbZoomMode").set_selected_index(1);
    assert_eq!(view.zoom_mode(), ScrollingZoomMode::Disabled);
    root.get_control::<ComboBox>("cmbHorizontalScrollMode").set_selected_index(1);
    assert_eq!(view.horizontal_scroll_mode(), ScrollingScrollMode::Disabled);
    root.get_control::<ComboBox>("cmbVerticalScrollMode").set_selected_index(0);
    assert_eq!(view.vertical_scroll_mode(), ScrollingScrollMode::Enabled);
    root.get_control::<ComboBox>("cmbHorizontalScrollBarVisibility").set_selected_index(2);
    assert_eq!(view.horizontal_scroll_bar_visibility(), ScrollingScrollBarVisibility::Hidden);
    root.get_control::<ComboBox>("cmbVerticalScrollBarVisibility").set_selected_index(1);
    assert_eq!(view.vertical_scroll_bar_visibility(), ScrollingScrollBarVisibility::Visible);
    // The images of the other two examples, named as in the source.
    assert_eq!(root.get_control::<StackPanel>("scrollView2Images").children().count(), 6);
    assert_eq!(root.get_control::<StackPanel>("scrollView3Images").children().count(), 8);
}

#[test]
fn scroll_view_code_behind_keeps_the_source_s_arithmetic() {
    // The casts of the combo boxes' indices.
    assert_eq!(scroll_mode(0), ScrollingScrollMode::Enabled);
    assert_eq!(scroll_mode(2), ScrollingScrollMode::Auto);
    assert_eq!(scroll_bar_visibility(1), ScrollingScrollBarVisibility::Visible);
    // NbVerticalVelocity_ValueChanged: outside the dead range the value is the velocity...
    assert_eq!(constant_velocity(60.0, 30.0, 10.0, 100.0), 60.0);
    // ...within it, toward the end the view is not at...
    assert_eq!(constant_velocity(20.0, 30.0, 0.0, 100.0), 30.0);
    assert_eq!(constant_velocity(20.0, 30.0, 50.0, 100.0), -30.0);
    assert_eq!(constant_velocity(-20.0, -30.0, 100.0, 100.0), -30.0);
    // ...and away from an end the view is at.
    assert_eq!(constant_velocity(-60.0, -30.0, 0.0, 100.0), 30.0);
    assert_eq!(constant_velocity(60.0, 30.0, 100.0, 100.0), -30.0);
    // GetTargetVerticalOffset.
    assert_eq!(target_vertical_offset(10.0, 100.0), 80.0);
    assert_eq!(target_vertical_offset(90.0, 100.0), 20.0);
}

/// With the WinUI theme the views get their presenters and bars, the velocity box starts a
/// constant velocity scroll of the second view and the button scrolls the third one.
#[cfg(feature = "theme")]
#[test]
fn scroll_view_examples_scroll_with_the_winui_theme() {
    use ferroui_controls::{Button, NumberBox, ScrollingInteractionState, Window};
    let _application = crate::tests::start_winui();
    let root = page();
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    for name in ["scrollView1", "scrollView2", "scrollView3"] {
        assert!(root.get_control::<ScrollView>(name).scroll_presenter().is_some(), "{name} has its presenter");
    }
    let second = root.get_control::<ScrollView>("scrollView2");
    root.get_control::<NumberBox>("nbVerticalVelocity").set_numeric_value(60.0);
    assert_eq!(second.state(), ScrollingInteractionState::Inertia);
    let third = root.get_control::<ScrollView>("scrollView3");
    let target = target_vertical_offset(third.vertical_offset(), third.scrollable_height());
    root.get_control::<Button>("btnScrollWithAnimation")
        .raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(&Button::click_event()));
    window.update_layout();
    assert!(
        third.state() == ScrollingInteractionState::Animation || (third.vertical_offset() - target).abs() < 0.5,
        "the button scrolls the third view"
    );
}
