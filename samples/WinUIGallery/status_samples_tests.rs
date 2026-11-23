//! The ProgressBar, Expander and PipsPager pages: the source's examples with their handlers
//! (`status_samples.rs`) and x:Bind. The InfoBar page is tested in `tests.rs` and
//! `x_bind_tests.rs`.

use crate::{examples, tests::start};
use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_base::Ref;
use ferroui_controls::{
    Carousel, ComboBox, Control, ExpandDirection, Expander, NumberBox, PipsPager, ProgressBar,
    RadioButton, TextBlock,
};

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap_or_else(|error| panic!("{id}: {error}"))
}

#[test]
fn progress_value_sets_the_determinate_bar_and_an_empty_box_goes_back_to_zero() {
    let _application = start();
    let root = page("ProgressBar");
    assert!(root
        .get_control::<ProgressBar>("ProgressBar1")
        .is_indeterminate());
    let bar = root.get_control::<ProgressBar>("ProgressBar2");
    let number_box = root.get_control::<NumberBox>("ProgressValue");
    number_box.set_numeric_value(40.0);
    assert_eq!(bar.value(), 40.0);
    // An empty box goes back to 0. As in WinUI, the value the handler sets while the box raises
    // ValueChanged raises no second event, so the bar keeps its value.
    number_box.set_numeric_value(f64::NAN);
    assert_eq!(number_box.value(), 0.0);
    assert_eq!(bar.value(), 40.0);
    number_box.set_numeric_value(10.0);
    assert_eq!(bar.value(), 10.0);
}

#[test]
fn progress_state_options_show_the_paused_and_the_error_state() {
    let _application = start();
    let root = page("ProgressBar");
    let bar = root.get_control::<ProgressBar>("ProgressBar1");
    assert!(!bar.show_paused() && !bar.show_error());

    root.get_control::<RadioButton>("PausedRB").set_is_checked(Some(true));
    assert!(bar.show_paused() && !bar.show_error());
    assert!(bar.classes().contains(":paused"));

    root.get_control::<RadioButton>("ErrorRB").set_is_checked(Some(true));
    assert!(!bar.show_paused() && bar.show_error());
    assert!(bar.classes().contains(":error") && !bar.classes().contains(":paused"));

    root.get_control::<RadioButton>("RunningRB").set_is_checked(Some(true));
    assert!(!bar.show_paused() && !bar.show_error());
    assert!(!bar.classes().contains(":error"));
}

#[test]
fn expand_direction_option_sets_the_direction_and_the_alignment() {
    let _application = start();
    let root = page("Expander");
    let expander = root.get_control::<Expander>("Expander1");
    assert!(!expander.is_expanded());
    assert_eq!(
        (expander.expand_direction(), expander.vertical_alignment()),
        (ExpandDirection::Down, VerticalAlignment::Top)
    );
    let direction = root.get_control::<ComboBox>("ExpandDirectionComboBox");
    direction.set_selected_index(1);
    assert_eq!(
        (expander.expand_direction(), expander.vertical_alignment()),
        (ExpandDirection::Up, VerticalAlignment::Bottom)
    );
    direction.set_selected_index(0);
    assert_eq!(
        (expander.expand_direction(), expander.vertical_alignment()),
        (ExpandDirection::Down, VerticalAlignment::Top)
    );
    let aligned = root.get_control::<Expander>("Expander2");
    assert_eq!(aligned.width(), 500.0);
    assert_eq!(
        root.get_control::<TextBlock>("Expander2Header")
            .text()
            .as_deref(),
        Some("This text is centered")
    );
}

#[test]
fn flip_view_pager_follows_the_gallery_both_ways() {
    let _application = start();
    let root = page("PipsPager");
    let gallery = root.get_control::<Carousel>("Gallery");
    let pager = root.get_control::<PipsPager>("FlipViewPipsPager");
    assert_eq!(gallery.item_count(), 8);
    assert_eq!(pager.number_of_pages(), 8);
    // The source's TwoWay x:Bind of the gallery's SelectedIndex.
    pager.set_selected_page_index(3);
    assert_eq!(gallery.selected_index(), 3);
    gallery.set_selected_index(5);
    assert_eq!(pager.selected_page_index(), 5);
}

#[test]
fn options_pager_announces_the_page_and_follows_the_options() {
    let _application = start();
    let root = page("PipsPager");
    let pager = root.get_control::<PipsPager>("TestPipsPager2");
    assert_eq!(pager.number_of_pages(), 10);
    pager.set_selected_page_index(4);
    assert_eq!(
        root.get_control::<TextBlock>("PipsPagerAnnouncement")
            .text()
            .as_deref(),
        Some("Page 5 of 10 selected")
    );
    root.get_control::<ComboBox>("OrientationComboBox")
        .set_selected_index(1);
    assert_eq!(pager.orientation(), Orientation::Vertical);
    root.get_control::<ComboBox>("OrientationComboBox")
        .set_selected_index(0);
    assert_eq!(pager.orientation(), Orientation::Horizontal);
    root.get_control::<ComboBox>("PrevButtonComboBox")
        .set_selected_index(1);
    assert!(!pager.is_previous_button_visible());
    root.get_control::<ComboBox>("NextButtonComboBox")
        .set_selected_index(1);
    assert!(!pager.is_next_button_visible());
    root.get_control::<ComboBox>("NextButtonComboBox")
        .set_selected_index(0);
    assert!(pager.is_next_button_visible());
}

#[test]
fn tool_tip_examples_keep_the_source_tips_offset_and_placement() {
    use ferroui_base::metadata::from_markup_value;
    use ferroui_controls::automation::AutomationProperties;
    use ferroui_controls::{Button, Image, PlacementMode, ToolTip};
    let _application = start();
    let root = page("ToolTip");
    fn tip_text(control: &Control) -> Option<String> {
        ToolTip::get_tip(control).and_then(|value| value.downcast_ref::<String>().cloned())
    }
    // The simple ToolTip of the button.
    let button = root.get_control::<Button>("SimpleToolTipButton");
    assert_eq!(tip_text(&button).as_deref(), Some("Simple ToolTip"));
    // The offset ToolTip of the text: the source's VerticalOffset of -80.
    let text = root.get_control::<TextBlock>("OffsetToolTipText");
    assert_eq!(tip_text(&text).as_deref(), Some("Offset information."));
    assert_eq!(ToolTip::get_vertical_offset(&text), -80.0);
    // The image's ToolTip is placed to the right of the 400 by 266 image, the source's
    // PlacementRect, with the source's description.
    let image = root.get_control::<Image>("textBoxToPlace");
    // The source's cliff.jpg where the platform decodes it (the test platform may not).
    let decoded = crate::assets::load_bitmap(crate::status_samples::CLIFF).is_ok();
    assert_eq!(image.source().is_some(), decoded);
    assert_eq!((image.width(), image.height()), (400.0, 266.0));
    assert_eq!(ToolTip::get_placement(&image), PlacementMode::Right);
    let tool_tip = from_markup_value::<Ref<ToolTip>>(&ToolTip::get_tip(&image))
        .expect("the ToolTip of the image");
    assert_eq!(
        tool_tip
            .content()
            .and_then(|value| value.downcast_ref::<String>().cloned())
            .as_deref(),
        Some("Non-occluding information.")
    );
    assert_eq!(
        AutomationProperties::get_help_text(&tool_tip).as_deref(),
        Some("Non-occluding tooltip")
    );
}
