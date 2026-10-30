//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/DragAndDrop/ContextDragBehaviorTests.cs`
//! of Xaml.Behaviors (with its windows `ContextDragEscapeWindow` and
//! `VirtualizedContextDragWindow`).
//!
//! Adapted: the windows are built in code (they name the test behavior, a type of the tests); the
//! style of `VirtualizedContextDragWindow`, which gives each list box item a test behavior, is the
//! behaviors set on the item; the mouse and the keyboard raise their events on the element
//! (`support.rs`).

use super::test_context_drag_behavior::TestContextDragBehavior;
use crate::tests::support::{add_behavior, border, capture_rendered_frame, key_down, object, run_jobs, show_window, start_app, start_app_with_fluent_theme, Mouse};
use ferroui_base::input::{InputElement, Key, PointerPressedEventArgs};
use ferroui_base::interactivity::{Interactive, RoutingStrategies};
use ferroui_base::{Point, Ref};
use ferroui_controls::{Border, ItemsSource, ListBox, ListBoxItem, StackPanel, TextBlock};
use ferroui_xaml_interactivity::{BehaviorCollection, Interaction};
use std::cell::Cell;
use std::rc::Rc;

/// `ContextDragEscapeWindow`: a border with a test behavior that starts a drag at once, and a
/// selected list box item with a test behavior around some content.
struct ContextDragEscapeWindow {
    target_border: Ref<Border>,
    test_behavior: Ref<TestContextDragBehavior>,
    selected_item_content: Ref<Border>,
    _window: Ref<ferroui_controls::Window>,
}

impl ContextDragEscapeWindow {
    fn new() -> Self {
        let panel = StackPanel::new();

        let target_border = border(100.0, 100.0);
        let test_behavior = TestContextDragBehavior::new();
        test_behavior.set_horizontal_drag_threshold(0.0);
        test_behavior.set_vertical_drag_threshold(0.0);
        add_behavior(&target_border, &test_behavior);
        panel.children().add(target_border.clone());

        let item = ListBoxItem::new();
        item.set_is_selected(true);
        add_behavior(&item, &TestContextDragBehavior::new());
        let selected_item_content = border(100.0, 30.0);
        let text_block = TextBlock::new();
        text_block.set_text(Some("Selected item content"));
        selected_item_content.set_child(text_block);
        item.set_content(Some(ferroui_controls::Control::boxed(selected_item_content.clone())));
        panel.children().add(item);

        let window = show_window(panel, 400.0, 300.0);
        Self { target_border, test_behavior, selected_item_content, _window: window }
    }
}

#[test]
fn pointer_pressed_is_not_handled_for_selected_item_content() {
    let _app = start_app();
    let window = ContextDragEscapeWindow::new();
    let pointer_pressed = Rc::new(Cell::new(false));
    let pressed = pointer_pressed.clone();
    window
        .selected_item_content
        .add_handler(InputElement::pointer_pressed_event(), move |_: &Interactive, _: &PointerPressedEventArgs| {
            pressed.set(true)
        });

    Mouse::new().down(&window.selected_item_content, Point::new(5.0, 5.0));

    assert!(pointer_pressed.get());
}

#[test]
fn escape_cancels_drag() {
    let _app = start_app();
    let window = ContextDragEscapeWindow::new();

    let _ = window.target_border.focus();

    let mouse = Mouse::new();
    let start = Point::new(5.0, 5.0);
    mouse.down(&window.target_border, start);

    key_down(&window.target_border, Key::Escape);

    let moved = Point::new(20.0, 20.0);
    mouse.move_to(&window.target_border, moved);
    mouse.up(&window.target_border, moved);
    run_jobs();

    assert!(!window.test_behavior.before_called());
    assert!(!window.test_behavior.after_called());
}

#[test]
fn virtualized_list_box_item_replacement_transfers_context_drag_lifecycle() {
    // The application of the reference's tests has the Fluent theme: the list box needs its
    // template to realize containers.
    let _app = start_app_with_fluent_theme();
    let list_box = ListBox::new();
    list_box.set_items_source(Some(ItemsSource::from_values(0..100)));
    let window = show_window(list_box.clone(), 400.0, 210.0);
    run_jobs();

    list_box.scroll_into_view(50);
    run_jobs();
    capture_rendered_frame(&window);

    let container = list_box
        .container_from_index(50)
        .and_then(|container| container.cast::<ListBoxItem>())
        .expect("a list box item");
    // The behaviors the style of the reference gives the item.
    let old_behavior = TestContextDragBehavior::new();
    let old_behaviors = BehaviorCollection::new();
    old_behaviors.add(old_behavior.clone().upcast());
    Interaction::set_behaviors(&container, Some(old_behaviors.clone()));
    assert_eq!(1, old_behavior.attached_to_visual_tree_count());

    let behavior = TestContextDragBehavior::new();
    let behaviors = BehaviorCollection::new();
    behaviors.add(behavior.clone().upcast());

    Interaction::set_behaviors(&container, Some(behaviors.clone()));

    let handled = Rc::new(Cell::new(false));
    let observed = handled.clone();
    container.add_handler_with(
        InputElement::pointer_pressed_event(),
        move |_: &Interactive, e: &PointerPressedEventArgs| observed.set(e.handled()),
        RoutingStrategies::BUBBLE,
        true,
    );

    list_box.set_selected_index(50);
    let mouse = Mouse::new();
    mouse.down(&container, Point::new(5.0, 5.0));
    mouse.up(&container, Point::new(5.0, 5.0));

    assert_eq!(None, old_behaviors.associated_object());
    assert_eq!(None, old_behavior.associated_object());
    assert_eq!(1, old_behavior.attached_to_visual_tree_count());
    assert_eq!(1, old_behavior.detached_from_visual_tree_count());
    assert_eq!(Some(object(&container)), behavior.associated_object());
    assert_eq!(Some(object(&container)), behaviors.associated_object());
    assert_eq!(1, behavior.attached_to_visual_tree_count());
    assert_eq!(0, behavior.detached_from_visual_tree_count());
    assert!(handled.get());
}
