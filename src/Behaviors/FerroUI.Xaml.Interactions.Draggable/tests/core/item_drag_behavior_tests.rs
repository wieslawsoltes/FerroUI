//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Core/ItemDragBehaviorTests.cs` of
//! Xaml.Behaviors (with its windows `ItemDragBehaviorVertical`, `ItemDragBehaviorHorizontal` and
//! `ListReorderDragBehaviorWindow`, `.xaml` documents here, loaded with the run-time loader).
//!
//! Adapted: the mouse raises the pointer events on the container (`support.rs`); the
//! `ObservableCollection` of the reference is the observable collection of the framework, given to
//! the items control by the test (the windows of the reference bind it from their data context).
//! The reference skips the two reorder tests ("Drag not supported in headless environment"): its
//! headless window hit-tests the moves of the mouse over the parent. Here the press, the moves and
//! the release are raised on the dragged container (the element under the pointer at the press,
//! which then holds the pointer) at the positions of the reference relative to the parent, so the
//! drag runs and the tests are not skipped.

use crate::tests::support::{capture_rendered_frame, find, load_window, object, run_jobs, start_app, Mouse};
use crate::ListReorderDragBehavior;
use ferroui_base::input::KeyModifiers;
use ferroui_base::{Point, Ref};
use ferroui_controls::{Control, ItemsControl, ListBox, ListBoxItem, Window};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::ItemsSource;
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_xaml_interactivity::Interaction;
use std::rc::Rc;

fn list_reorder_drag_behavior_window() -> Ref<Window> {
    load_window("ListReorderDragBehaviorWindow.xaml", include_str!("ListReorderDragBehaviorWindow.xaml"))
}

fn assert_drag_can_start(window: &Window, list_box: &ListBox, index: i32) {
    let container = list_box
        .container_from_index(index)
        .and_then(|container| container.cast::<ListBoxItem>())
        .expect("a list box item");
    let behaviors = container.get_value(Interaction::behaviors_property()).expect("a behavior collection");
    assert_eq!(1, behaviors.count());
    let behavior = behaviors.get(0).cast::<ListReorderDragBehavior>().expect("a list reorder drag behavior");

    assert_eq!(Some(object(&container)), behaviors.associated_object());
    assert_eq!(Some(object(&container)), behavior.associated_object());

    list_box.set_selected_index(index);

    let mouse = Mouse::new();
    mouse.down(&container, Point::new(5.0, 5.0));

    assert!(container.classes().contains(":dragging"));

    mouse.up(&container, Point::new(5.0, 5.0));

    assert!(!container.classes().contains(":dragging"));
    let _ = window;
}

/// `Drag(window, parent, container, horizontal)`: presses the left button on the middle of
/// `container`, moves the pointer by a third of the size of the container at a time, three times
/// that size along the orientation, and releases it (the positions relative to `parent`).
fn drag(parent: &Control, container: &Control, horizontal: bool) {
    let bounds = container.bounds();
    let start_local = Point::new(bounds.width / 2.0, bounds.height / 2.0);
    let start = container.translate_point(start_local, parent).unwrap_or_default();
    let mouse = Mouse::new();
    mouse.down_over(container, parent, start, KeyModifiers::NONE);

    let step = if horizontal { bounds.width / 3.0 } else { bounds.height / 3.0 };
    let total = if horizontal { bounds.width * 3.0 } else { bounds.height * 3.0 };

    let mut moved = step;
    while moved <= total {
        let point =
            if horizontal { Point::new(start.x + moved, start.y) } else { Point::new(start.x, start.y + moved) };
        mouse.move_over(container, parent, point);
        moved += step;
    }

    let end = if horizontal { Point::new(start.x + total, start.y) } else { Point::new(start.x, start.y + total) };
    mouse.up_over(container, parent, end);
}

/// `new ObservableCollection<string>(["Item1", "Item2", "Item3"])` (the `Items` of the windows).
fn items() -> ObservableCollection<String> {
    ObservableCollection::from_items(vec!["Item1".to_string(), "Item2".to_string(), "Item3".to_string()])
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn item_drag_behavior_reorders_vertical() {
    let _app = start_app();
    let window = load_window("ItemDragBehaviorVertical.xaml", include_str!("ItemDragBehaviorVertical.xaml"));
    let items = items();
    let list_box = find::<ListBox>(&window, "TargetListBox");
    list_box.set_items_source(Some(ItemsSource::new(Rc::new(items.clone()))));

    window.show();
    capture_rendered_frame(&window);

    let containers = list_box.get_realized_containers();
    assert!(containers.iter().all(|container| container.clone().cast::<ListBoxItem>().is_some()));
    assert_eq!(strings(&["Item1", "Item2", "Item3"]), items.to_vec());

    drag(&list_box, &containers[0], false);

    assert_eq!(strings(&["Item2", "Item3", "Item1"]), items.to_vec());
}

#[test]
fn item_drag_behavior_reorders_horizontal() {
    let _app = start_app();
    let window = load_window("ItemDragBehaviorHorizontal.xaml", include_str!("ItemDragBehaviorHorizontal.xaml"));
    let items = items();
    let items_control = find::<ItemsControl>(&window, "TargetItemsControl");
    items_control.set_items_source(Some(ItemsSource::new(Rc::new(items.clone()))));

    window.show();
    capture_rendered_frame(&window);

    let containers = items_control.get_realized_containers();
    assert!(containers.iter().all(|container| container.clone().cast::<ContentPresenter>().is_some()));
    assert_eq!(strings(&["Item1", "Item2", "Item3"]), items.to_vec());

    drag(&items_control, &containers[0], true);

    assert_eq!(strings(&["Item2", "Item3", "Item1"]), items.to_vec());
}

fn show(window: &Window) {
    window.show();
    run_jobs();
    capture_rendered_frame(window);
}

#[test]
fn list_reorder_drag_behavior_can_start_again_after_virtualized_item_moves() {
    let _app = start_app();
    let items = ObservableCollection::from_items((0..100).collect::<Vec<i32>>());
    let window = list_reorder_drag_behavior_window();
    let list_box = find::<ListBox>(&window, "TargetListBox");
    list_box.set_items_source(Some(ItemsSource::new(Rc::new(items.clone()))));

    show(&window);

    assert_drag_can_start(&window, &list_box, 0);

    let moved_item = items.get(0);
    items.move_item(0, 5);
    run_jobs();
    capture_rendered_frame(&window);

    assert_eq!(moved_item, items.get(5));
    assert_drag_can_start(&window, &list_box, 5);
}

#[test]
fn list_reorder_drag_behavior_can_start_after_list_reattaches() {
    let _app = start_app();
    let items = ObservableCollection::from_items((0..100).collect::<Vec<i32>>());
    let window = list_reorder_drag_behavior_window();
    let list_box = find::<ListBox>(&window, "TargetListBox");
    list_box.set_items_source(Some(ItemsSource::new(Rc::new(items))));

    show(&window);

    assert_drag_can_start(&window, &list_box, 0);

    window.set_content(None);
    run_jobs();
    window.set_content(Some(ferroui_controls::Control::boxed(list_box.clone())));
    run_jobs();
    capture_rendered_frame(&window);

    assert_drag_can_start(&window, &list_box, 0);
}
