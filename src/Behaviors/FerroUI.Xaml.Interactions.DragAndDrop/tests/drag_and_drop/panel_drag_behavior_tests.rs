//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/DragAndDrop/PanelDragBehaviorTests.cs`
//! of Xaml.Behaviors (with its window `PanelDragDrop001`).
//!
//! Adapted: the window is built in code; the mouse raises its events on the element
//! (`support.rs`). As in the reference, the drop is not run by the mouse (a drag of the system
//! does not run in the headless environment): the test moves the rectangle itself;
//! `drag_and_drop_tests.rs` raises the drop on the target panel.

use crate::tests::support::{add_behavior, show_window, start_app, Mouse};
use crate::{PanelDragBehavior, PanelDropBehavior};
use ferroui_base::{Point, Ref};
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{ColumnDefinitions, Control, Grid, StackPanel, Window};

/// `PanelDragDrop001`.
struct PanelDragDrop001 {
    left_panel: Ref<StackPanel>,
    right_panel: Ref<StackPanel>,
    drag_rectangle: Ref<Rectangle>,
    _window: Ref<Window>,
}

impl PanelDragDrop001 {
    fn new() -> Self {
        let grid = Grid::new();
        grid.set_column_definitions(ColumnDefinitions::parse("*,*").expect("column definitions"));

        let left_panel = StackPanel::new();
        add_behavior(&left_panel, &PanelDropBehavior::new());
        let drag_rectangle = Rectangle::new();
        drag_rectangle.set_width(20.0);
        drag_rectangle.set_height(20.0);
        add_behavior(&drag_rectangle, &PanelDragBehavior::new());
        left_panel.children().add(drag_rectangle.clone());
        grid.children().add(left_panel.clone());

        let right_panel = StackPanel::new();
        Grid::set_column(&right_panel, 1);
        add_behavior(&right_panel, &PanelDropBehavior::new());
        grid.children().add(right_panel.clone());

        let window = show_window(grid, 200.0, 100.0);
        Self { left_panel, right_panel, drag_rectangle, _window: window }
    }
}

#[test]
fn drag_rectangle_moves_between_panels() {
    let _app = start_app();
    let window = PanelDragDrop001::new();

    // start drag from rectangle and drop on right panel
    let mouse = Mouse::new();
    mouse.down(&window.drag_rectangle, Point::new(5.0, 5.0));
    mouse.move_to(&window.drag_rectangle, Point::new(50.0, 5.0));
    mouse.move_to(&window.right_panel, Point::new(10.0, 10.0));
    mouse.up(&window.right_panel, Point::new(10.0, 10.0));

    // simulate drop result
    window.left_panel.children().remove(window.drag_rectangle.clone());
    window.right_panel.children().add(window.drag_rectangle.clone());

    let rectangle: Ref<Control> = window.drag_rectangle.clone().upcast();
    assert!(!window.left_panel.children().contains(&rectangle));
    assert!(window.right_panel.children().contains(&rectangle));
}
