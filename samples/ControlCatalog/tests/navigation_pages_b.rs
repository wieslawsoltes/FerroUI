//! Tests of the scroll-aware and transitions samples of `NavigationPage`. Not ports: the upstream
//! sample has no tests.

use super::support::*;
use crate::pages::{NavigationPageScrollAwarePage, NavigationPageTransitionsPage};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::media::TranslateTransform;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ObjectType, Ref, Vector, Visual};
use ferroui_controls::{
    Border, Button, ComboBox, ContentPage, Control, NavigationPage, ScrollViewer, Slider, TextBlock, Window,
};

fn show(control: &Ref<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(1100.0);
    window.set_height(800.0);
    window.set_content(Some(Control::boxed(control)));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    window
}

fn descendants<T: ObjectType>(root: &Visual) -> Vec<Ref<T>> {
    root.get_visual_descendants().filter_map(|visual| visual.cast::<T>()).collect()
}

fn translation_of(border: &Border) -> Option<Ref<TranslateTransform>> {
    border
        .render_transform()
        .and_then(|transform| transform.as_object().and_then(|object| object.to_ref().cast::<TranslateTransform>()))
}

fn button_with_content(root: &Visual, content: &str) -> Ref<Button> {
    descendants::<Button>(root)
        .into_iter()
        .find(|button| {
            button.content().is_some_and(|value| value.downcast_ref::<String>().is_some_and(|text| text == content))
        })
        .unwrap_or_else(|| panic!("no button {content}"))
}

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    Dispatcher::ui_thread().run_jobs(None);
}

#[test]
fn scroll_aware_page_translates_the_navigation_bar_with_the_scroll_offset() {
    let _app = start_catalog_application();
    let page = NavigationPageScrollAwarePage::new();
    let window = show(&page.clone().upcast());

    let demo_nav = page.get_control::<NavigationPage>("DemoNav");
    assert_eq!(1, demo_nav.stack_depth());
    let root_page = demo_nav.current_page().and_then(|page| page.cast::<ContentPage>()).expect("the root page");
    let scroll_viewer = root_page
        .content()
        .and_then(|content| Control::from_boxed(&content))
        .and_then(|content| content.cast::<ScrollViewer>())
        .expect("the scroll viewer of the root page");

    let nav_bar = descendants::<Border>(&demo_nav)
        .into_iter()
        .find(|border| border.name().as_deref() == Some("PART_NavigationBar"))
        .expect("the navigation bar");
    let transform = translation_of(&nav_bar).expect("the transform the watcher gives the bar");
    assert_eq!(0.0, transform.y());

    let bar_height = demo_nav.bar_height();
    assert!(bar_height > 20.0);

    // Within the height of the bar, the bar follows the offset.
    scroll_viewer.set_offset(Vector::new(0.0, 20.0));
    assert_eq!(20.0, scroll_viewer.offset().y);
    assert_eq!(-20.0, transform.y());

    // Further down, the bar is hidden.
    scroll_viewer.set_offset(Vector::new(0.0, bar_height + 200.0));
    assert_eq!(-bar_height, transform.y());

    // Scrolling up reveals it by the distance scrolled.
    scroll_viewer.set_offset(Vector::new(0.0, bar_height + 190.0));
    assert_eq!(-bar_height + 10.0, transform.y());

    // Unloading resets the bar and ends the subscription.
    window.set_content(None);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(0.0, transform.y());
    scroll_viewer.set_offset(Vector::new(0.0, 0.0));
    scroll_viewer.set_offset(Vector::new(0.0, 10.0));
    assert_eq!(0.0, transform.y());
    window.close();
}

#[test]
fn transitions_page_sets_the_transition_of_the_selection_and_pushes_pages() {
    let _app = start_catalog_application();
    let page = NavigationPageTransitionsPage::new();
    let window = show(&page.clone().upcast());

    let demo_nav = page.get_control::<NavigationPage>("DemoNav");
    let combo = page.get_control::<ComboBox>("TransitionCombo");
    let slider = page.get_control::<Slider>("DurationSlider");
    let label = page.get_control::<TextBlock>("DurationLabel");

    assert_eq!(1, demo_nav.stack_depth());
    assert_eq!(1, combo.selected_index());
    assert!(demo_nav.page_transition().is_some());

    slider.set_range_value(500.0);
    assert_eq!(Some(String::from("500 ms")), label.text());

    // Every entry of the list but "None" gives a transition.
    for index in [3, 4, 5, 6, 7, 8] {
        combo.set_selected_index(index);
        assert!(demo_nav.page_transition().is_some(), "entry {index}");
    }
    combo.set_selected_index(9);
    assert!(demo_nav.page_transition().is_none());

    // Without a transition a push and a pop complete at once.
    click(&button_with_content(&page, "Push Page"));
    assert_eq!(2, demo_nav.stack_depth());
    let pushed = demo_nav.current_page().expect("the pushed page");
    assert_eq!(Some(String::from("Page 1")), pushed.header().and_then(|header| header.downcast_ref::<String>().cloned()));
    click(&button_with_content(&page, "Pop"));
    assert_eq!(1, demo_nav.stack_depth());
    window.close();
}
