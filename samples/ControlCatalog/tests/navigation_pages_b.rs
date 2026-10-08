//! Tests of the scroll-aware sample of `NavigationPage`. Not ports: the upstream sample has no tests.

use super::support::*;
use crate::pages::NavigationPageScrollAwarePage;
use ferroui_base::media::TranslateTransform;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ObjectType, Ref, Vector, Visual};
use ferroui_controls::{Border, ContentPage, Control, NavigationPage, ScrollViewer, Window};

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
