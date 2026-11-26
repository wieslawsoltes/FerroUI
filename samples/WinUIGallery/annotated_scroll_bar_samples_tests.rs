//! The AnnotatedScrollBar page: the source's example with its code behind
//! (`annotated_scroll_bar_samples.rs`).

use crate::annotated_scroll_bar_samples::{item_color, items_per_row, offset_label, offset_of_item, populate_labels};
use crate::{examples, tests::start};
use ferroui_base::Ref;
use ferroui_controls::{AnnotatedScrollBar, Control, ScrollView, Slider};

fn page() -> Ref<Control> {
    examples::create(examples::find("AnnotatedScrollBar").unwrap())
        .unwrap_or_else(|error| panic!("AnnotatedScrollBar: {error}"))
}

#[test]
fn annotated_scroll_bar_code_behind_keeps_the_source_s_arithmetic() {
    // Six items of 120 px fit a 750 px repeater; the rows are 90 px.
    assert_eq!(items_per_row(750.0), 6);
    assert_eq!(items_per_row(0.0), 1);
    assert_eq!(items_per_row(50.0), 1);
    assert_eq!(offset_of_item(32, 6), 450.0);
    assert_eq!((item_color(31), item_color(32), item_color(89), item_color(249)), ("Azure", "Crimson", "Cyan", "Gold"));
    assert_eq!(offset_label(0.0, 6), "Azure");
    assert_eq!(offset_label(451.0, 6), "Crimson");
    assert_eq!(offset_label(100_000.0, 6), "Gold");
}

#[test]
fn annotated_scroll_bar_page_has_the_source_s_controls_and_labels() {
    let _application = start();
    let root = page();
    let bar = root.get_control::<AnnotatedScrollBar>("annotatedScrollBar");
    assert_eq!(bar.max_height(), 500.0);
    assert!(root.find_control::<ScrollView>("scrollView").is_some());
    populate_labels(&bar, 6);
    let labels = bar.labels().snapshot();
    let offsets: Vec<f64> = labels.iter().map(|label| label.scroll_offset()).collect();
    assert_eq!(offsets, vec![0.0, 450.0, 1170.0, 1350.0, 2340.0]);
    // The slider sets the maximum height of the bar.
    root.get_control::<Slider>("maxHeightSlider").set_range_value(200.0);
    assert_eq!(bar.max_height(), 200.0);
}

#[cfg(feature = "theme")]
#[test]
fn annotated_scroll_bar_drives_the_view_with_the_winui_theme() {
    use ferroui_controls::Window;
    let _application = crate::tests::start_winui();
    let root = page();
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    // The Loaded event of the view, which links the bar, is raised by the dispatcher.
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    window.update_layout();
    let view = root.get_control::<ScrollView>("scrollView");
    let bar = root.get_control::<AnnotatedScrollBar>("annotatedScrollBar");
    let presenter = view.scroll_presenter().expect("the presenter of the view");
    let controller = presenter.vertical_scroll_controller().expect("the bar drives the view once loaded");
    // The same controller (the data pointers; the vtables of two casts may differ).
    assert_eq!(std::rc::Rc::as_ptr(&controller) as *const (), std::rc::Rc::as_ptr(&bar.scroll_controller()) as *const ());
    assert!(view.scrollable_height() > 0.0);
}
