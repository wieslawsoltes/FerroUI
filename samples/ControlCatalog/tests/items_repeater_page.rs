//! The Repeater page exercises the actual compiled document, native scroll
//! viewport, layout switching and notifying collection mutations.

use super::support::start_catalog_application;
use crate::pages::ItemsRepeaterPage;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Ref, Vector};
use ferroui_controls::{Button, Control, ScrollViewer, Window};
use ferroui_controls_repeater::{
    FlowLayout, ItemsRepeater, LinedFlowLayout, StackLayout, UniformGridLayout,
};

fn show(page: &Ref<ItemsRepeaterPage>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(1000.0);
    window.set_height(700.0);
    window.set_content(Some(Control::boxed(page.clone())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    window
}

fn click(page: &Ref<ItemsRepeaterPage>, name: &str) {
    page.get_control::<Button>(name)
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    Dispatcher::ui_thread().run_jobs(None);
}

#[test]
fn all_four_layouts_keep_a_large_collection_virtualized() {
    let _app = start_catalog_application();
    let page = ItemsRepeaterPage::new();
    let window = show(&page);
    let repeater = page.get_control::<ItemsRepeater>("Repeater");
    assert_eq!(repeater.items_source_view().unwrap().count(), 10_000);
    for (name, expected) in [
        ("StackLayoutButton", StackLayout::TYPE),
        ("UniformGridLayoutButton", UniformGridLayout::TYPE),
        ("FlowLayoutButton", FlowLayout::TYPE),
        ("LinedFlowLayoutButton", LinedFlowLayout::TYPE),
    ] {
        click(&page, name);
        assert_eq!(repeater.layout().unwrap().get_type(), expected);
        let count = repeater.realized_elements().len();
        assert!(
            count > 0 && count < 10_000,
            "{name}: realized {count} items"
        );
    }
    window.close();
}

#[test]
fn lined_flow_metadata_keeps_cards_inside_the_horizontal_viewport_after_resize_and_edits() {
    let _app = start_catalog_application();
    let page = ItemsRepeaterPage::new();
    let window = show(&page);
    let repeater = page.get_control::<ItemsRepeater>("Repeater");
    let scroller = page.get_control::<ScrollViewer>("RepeaterScroller");
    click(&page, "LinedFlowLayoutButton");
    let layout = repeater
        .layout()
        .unwrap()
        .cast::<LinedFlowLayout>()
        .unwrap();
    let assert_cards_fit = || {
        assert!(layout.uses_fast_path_layout());
        let width = scroller.viewport().width;
        assert!(width > 0.0);
        let elements = repeater.realized_elements();
        assert!(!elements.is_empty() && elements.len() < 10_000);
        for element in elements {
            let bounds = element.bounds();
            assert!(bounds.width >= 128.0);
            assert!(
                bounds.x >= 0.0 && bounds.right() <= width + 1.0,
                "item {}: {bounds:?} exceeds the {width}-DIP viewport",
                repeater.get_element_index(&element)
            );
        }
    };
    for width in [1000.0, 720.0, 1280.0] {
        window.set_width(width);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(scroller.viewport().width < width);
        assert_cards_fit();
    }
    for name in [
        "InsertItemButton",
        "MoveItemButton",
        "RemoveItemButton",
        "ResetItemsButton",
    ] {
        click(&page, name);
        assert_cards_fit();
    }
    window.close();
}

#[test]
fn collection_buttons_use_notifying_insert_remove_move_and_reset() {
    let _app = start_catalog_application();
    let page = ItemsRepeaterPage::new();
    let window = show(&page);
    let repeater = page.get_control::<ItemsRepeater>("Repeater");
    let view = repeater.items_source_view().unwrap();
    click(&page, "InsertItemButton");
    assert_eq!(view.count(), 10_001);
    assert_eq!(
        view.get_at(0)
            .unwrap()
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("Inserted item 10000")
    );
    click(&page, "MoveItemButton");
    assert_eq!(
        view.get_at(10_000)
            .unwrap()
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("Inserted item 10000")
    );
    click(&page, "RemoveItemButton");
    assert_eq!(view.count(), 10_000);
    assert_eq!(
        view.get_at(0)
            .unwrap()
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("Item 00001")
    );
    click(&page, "ResetItemsButton");
    assert_eq!(view.count(), 10_000);
    assert_eq!(
        view.get_at(0)
            .unwrap()
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("Item 00000")
    );
    window.close();
}

#[test]
fn native_scrolling_recycles_items_outside_the_viewport() {
    let _app = start_catalog_application();
    let page = ItemsRepeaterPage::new();
    let window = show(&page);
    let repeater = page.get_control::<ItemsRepeater>("Repeater");
    let scroller = page.get_control::<ScrollViewer>("RepeaterScroller");
    assert!(repeater.try_get_element(0).is_some());
    scroller.set_offset(Vector::new(0.0, 72_000.0));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(scroller.offset().y > 50_000.0);
    assert!(repeater.try_get_element(0).is_none());
    let realized = repeater.realized_elements();
    assert!(!realized.is_empty() && realized.len() < 10_000);
    assert!(realized
        .iter()
        .any(|element| repeater.get_element_index(element) >= 900));
    window.close();
}

#[test]
fn page_event_handlers_do_not_keep_the_page_alive() {
    let _app = start_catalog_application();
    let page = ItemsRepeaterPage::new();
    let weak = page.downgrade();
    let repeater = page.get_control::<ItemsRepeater>("Repeater");
    let button = page.get_control::<Button>("StackLayoutButton");
    drop(page);
    assert!(weak.upgrade().is_none());
    // Retained child controls can raise events after their page has gone.
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    repeater.invalidate_measure();
}
