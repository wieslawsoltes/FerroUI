//! The ProgressBar, Expander and PipsPager pages: the source's examples with their handlers
//! (`status_samples.rs`) and x:Bind. The InfoBar page is tested in `tests.rs` and
//! `x_bind_tests.rs`.

use crate::{examples, tests::start};
use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_base::Ref;
use ferroui_controls::{
    Carousel, ComboBox, Control, ExpandDirection, Expander, NumberBox, PipsPager, ProgressBar,
    TextBlock,
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
