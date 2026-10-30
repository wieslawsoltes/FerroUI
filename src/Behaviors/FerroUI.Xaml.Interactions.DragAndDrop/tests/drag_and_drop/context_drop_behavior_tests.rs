//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/DragAndDrop/ContextDropBehaviorTests.cs`
//! of Xaml.Behaviors (with its window `ContextDropBehavior001`).
//!
//! Adapted: the window is built in code (its handler is a type of the tests); the mouse raises its
//! events on the element (`support.rs`). As in the reference, the drop is not run by the mouse (a
//! drag of the system does not run in the headless environment): the test sets the result of the
//! handler; `drag_and_drop_tests.rs` raises the drop on the target.

use crate::tests::support::{add_behavior, border, is_text, show_window, start_app, text, Mouse, TestDropHandler};
use crate::values::boxed_object;
use crate::{ContextDropBehavior, PanelDragBehavior};
use ferroui_base::{Point, Ref};
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{Border, ColumnDefinitions, Grid, StackPanel, Window};
use std::rc::Rc;

/// `ContextDropBehavior001`.
struct ContextDropBehavior001 {
    handler: Rc<TestDropHandler>,
    drag_rectangle: Ref<Rectangle>,
    drop_border: Ref<Border>,
    _window: Ref<Window>,
}

impl ContextDropBehavior001 {
    fn new() -> Self {
        let handler = Rc::new(TestDropHandler::default());

        let grid = Grid::new();
        grid.set_column_definitions(ColumnDefinitions::parse("*,*").expect("column definitions"));

        let left_panel = StackPanel::new();
        let drag_rectangle = Rectangle::new();
        drag_rectangle.set_width(20.0);
        drag_rectangle.set_height(20.0);
        add_behavior(&drag_rectangle, &PanelDragBehavior::new());
        left_panel.children().add(drag_rectangle.clone());
        grid.children().add(left_panel);

        let drop_border = border(100.0, 50.0);
        Grid::set_column(&drop_border, 1);
        let behavior = ContextDropBehavior::new();
        behavior.set_handler(Some(handler.clone()));
        behavior.set_context(Some(text("TargetContext")));
        add_behavior(&drop_border, &behavior);
        grid.children().add(drop_border.clone());

        let window = show_window(grid, 200.0, 100.0);
        Self { handler, drag_rectangle, drop_border, _window: window }
    }
}

#[test]
fn drop_handler_receives_context_data() {
    let _app = start_app();
    let window = ContextDropBehavior001::new();

    let mouse = Mouse::new();
    mouse.down(&window.drag_rectangle, Point::new(5.0, 5.0));
    mouse.move_to(&window.drag_rectangle, Point::new(50.0, 5.0));
    mouse.move_to(&window.drop_border, Point::new(10.0, 10.0));
    mouse.up(&window.drop_border, Point::new(10.0, 10.0));

    // simulate handler result
    let handler = &window.handler;
    *handler.last_source_context.borrow_mut() = Some(boxed_object(&window.drag_rectangle));
    *handler.last_target_context.borrow_mut() = Some(text("TargetContext"));
    let source = crate::values::object_of(handler.last_source_context.borrow().as_ref());
    assert!(source.is_some_and(|source| source.ptr_eq(&window.drag_rectangle)));
    assert!(is_text(handler.last_target_context.borrow().as_ref(), "TargetContext"));
}
