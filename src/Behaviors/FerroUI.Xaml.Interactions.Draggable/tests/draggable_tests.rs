//! Not from the reference: the logic of the drags that runs without a drag session.
//!
//! The hit tests and the computation of the reorder index of the item drag (the loop of its
//! `PointerMoved`), the drop target and the cell swap of the grid drag, the clamping of the
//! translation drags and the auto scroll offset, and the drag state the behaviors keep between a
//! press and a release (the `:dragging` pseudo class, the transforms they install).

use crate::auto_scroll_during_drag_behavior::auto_scroll_offset;
use crate::grid_drag_behavior::find_drop_target;
use crate::item_drag_behavior::{exceeds_drag_threshold, move_in_items_source, next_target_index, target_shift, TargetShift};
use crate::mouse_drag_element_behavior::constrain_to_parent;
use crate::tests::support::{show_window, start_app, Mouse};
use crate::{
    CanvasDragBehavior, GridDragBehavior, ItemDragBehavior, MouseDragElementBehavior, MultiMouseDragElementBehavior,
};
use ferroui_base::collections::FerroList;
use ferroui_base::layout::Orientation;
use ferroui_base::media::TranslateTransform;
use ferroui_base::{BoxedValue, Point, Rect, Ref, Size, Vector};
use ferroui_controls::{Border, Canvas, ColumnDefinitions, Control, Grid, ItemsSource, StackPanel};
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_xaml_interactivity::Interaction;
use std::rc::Rc;

/// A border of a fixed size.
fn border(width: f64, height: f64) -> Ref<Border> {
    let border = Border::new();
    border.set_width(width);
    border.set_height(height);
    border
}

/// Adds `behavior` to the behaviors of `control`.
fn add_behavior<T: ferroui_base::ObjectType + ferroui_base::Upcast<ferroui_base::FerroObject>>(control: &Control, behavior: &Ref<T>) {
    Interaction::get_behaviors(control).add(behavior.clone().upcast());
}

// --- the hit test of the item drag -------------------------------------------------------------

#[test]
fn a_later_item_moves_back_once_the_dragged_item_passes_its_middle() {
    let dragged = Rect::new(0.0, 0.0, 100.0, 20.0);
    let next = Rect::new(0.0, 20.0, 100.0, 20.0);

    // The end of the dragged item (20 + delta) reaches the middle of the next one (30) at 10.
    assert_eq!(TargetShift::None, target_shift(Orientation::Vertical, dragged, 9.0, next));
    assert_eq!(TargetShift::Backward, target_shift(Orientation::Vertical, dragged, 10.0, next));
    assert_eq!(TargetShift::Backward, target_shift(Orientation::Vertical, dragged, 35.0, next));
}

#[test]
fn an_earlier_item_moves_forward_once_the_dragged_item_passes_its_middle() {
    let dragged = Rect::new(0.0, 40.0, 100.0, 20.0);
    let previous = Rect::new(0.0, 20.0, 100.0, 20.0);

    // The start of the dragged item (40 + delta) reaches the middle of the previous one (30) at
    // -10.
    assert_eq!(TargetShift::None, target_shift(Orientation::Vertical, dragged, -9.0, previous));
    assert_eq!(TargetShift::Forward, target_shift(Orientation::Vertical, dragged, -10.0, previous));
}

#[test]
fn the_hit_test_follows_the_orientation() {
    let dragged = Rect::new(0.0, 0.0, 50.0, 20.0);
    let right = Rect::new(50.0, 0.0, 50.0, 20.0);

    assert_eq!(TargetShift::Backward, target_shift(Orientation::Horizontal, dragged, 25.0, right));
    // Vertically the two items start at the same place: neither is after the other.
    assert_eq!(TargetShift::None, target_shift(Orientation::Vertical, dragged, 25.0, right));
}

#[test]
fn the_target_index_is_the_last_item_passed_forward_and_the_first_passed_backward() {
    // Dragging down past items 1 and 2: the target is the last of them.
    let mut target = -1;
    target = next_target_index(target, 1, TargetShift::Backward);
    target = next_target_index(target, 2, TargetShift::Backward);
    target = next_target_index(target, 3, TargetShift::None);
    assert_eq!(2, target);

    // Dragging up past items 2 and 1: the target is the first of them.
    let mut target = -1;
    target = next_target_index(target, 1, TargetShift::Forward);
    target = next_target_index(target, 2, TargetShift::Forward);
    assert_eq!(1, target);

    assert_eq!(-1, next_target_index(-1, 4, TargetShift::None));
}

#[test]
fn the_drag_starts_past_the_threshold_of_its_orientation() {
    let start = Point::new(10.0, 10.0);

    assert!(!exceeds_drag_threshold(Orientation::Vertical, start, Point::new(10.0, 13.0), 3.0, 3.0));
    assert!(exceeds_drag_threshold(Orientation::Vertical, start, Point::new(10.0, 13.5), 3.0, 3.0));
    assert!(exceeds_drag_threshold(Orientation::Vertical, start, Point::new(10.0, 6.0), 3.0, 3.0));
    // Only the distance along the orientation counts.
    assert!(!exceeds_drag_threshold(Orientation::Vertical, start, Point::new(50.0, 10.0), 3.0, 3.0));
    assert!(exceeds_drag_threshold(Orientation::Horizontal, start, Point::new(50.0, 10.0), 3.0, 3.0));
}

#[test]
fn the_dragged_item_moves_to_the_target_index_of_an_items_source() {
    let list = FerroList::from_items(["Item1".to_string(), "Item2".to_string(), "Item3".to_string()]);
    let source = ItemsSource::new(Rc::new(list.clone()));

    assert!(move_in_items_source(&source, 0, 2));
    assert_eq!(vec!["Item2", "Item3", "Item1"], list.to_vec());

    let collection = ObservableCollection::from_items(vec![1, 2, 3]);
    let source = ItemsSource::new(Rc::new(collection.clone()));

    assert!(move_in_items_source(&source, 2, 0));
    assert_eq!(vec![3, 1, 2], collection.to_vec());
}

#[test]
fn an_items_source_of_an_unknown_list_is_not_moved() {
    let source = ItemsSource::from_items([None::<BoxedValue>, None]);

    assert!(!move_in_items_source(&source, 0, 1));
}

// --- the grid drag ---------------------------------------------------------------------------

#[test]
fn the_drop_target_of_the_grid_drag_is_the_child_under_the_pointer() {
    let _app = start_app();
    let grid = Grid::new();
    grid.set_column_definitions(ColumnDefinitions::parse("50,50").expect("column definitions"));
    let first = border(50.0, 50.0);
    let second = border(50.0, 50.0);
    Grid::set_column(&second, 1);
    grid.children().add(first.clone());
    grid.children().add(second.clone());
    let _window = show_window(grid.clone(), 100.0, 50.0);

    let dragged: Ref<Control> = first.clone().upcast();
    let children = grid.children().to_vec();

    let target = find_drop_target(children.clone(), &dragged, Point::new(75.0, 25.0));
    assert!(target.is_some_and(|target| target.ptr_eq(&second)));

    // The dragged control itself is not a target.
    assert!(find_drop_target(children, &dragged, Point::new(25.0, 25.0)).is_none());
}

#[test]
fn the_grid_drag_swaps_the_cells_it_copies() {
    let _app = start_app();
    let behavior = GridDragBehavior::new();
    let dragged = border(10.0, 10.0);
    let target = border(10.0, 10.0);
    Grid::set_column(&target, 1);
    Grid::set_row(&target, 2);
    Grid::set_column_span(&target, 3);

    // By default the column and the row are copied, not the spans.
    behavior.swap_cells(&dragged, &target);

    assert_eq!((1, 2, 1), (Grid::get_column(&dragged), Grid::get_row(&dragged), Grid::get_column_span(&dragged)));
    assert_eq!((0, 0, 3), (Grid::get_column(&target), Grid::get_row(&target), Grid::get_column_span(&target)));

    behavior.set_copy_column_span(true);
    behavior.swap_cells(&dragged, &target);

    assert_eq!(3, Grid::get_column_span(&dragged));
    assert_eq!(1, Grid::get_column_span(&target));
}

// --- the translation drags and the auto scroll -------------------------------------------------

#[test]
fn the_translation_stays_within_the_bounds_of_the_parent() {
    let parent = Rect::new(0.0, 0.0, 200.0, 100.0);
    let element = Rect::new(10.0, 20.0, 50.0, 30.0);

    assert_eq!((5.0, 5.0), constrain_to_parent(5.0, 5.0, parent, element));
    assert_eq!((-10.0, -20.0), constrain_to_parent(-50.0, -50.0, parent, element));
    assert_eq!((140.0, 50.0), constrain_to_parent(500.0, 500.0, parent, element));
}

#[test]
fn the_scroll_viewer_scrolls_towards_the_edge_the_pointer_is_near() {
    let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
    let extent = Size::new(300.0, 300.0);
    let offset = Vector::new(50.0, 50.0);

    // Away from the edges nothing scrolls.
    assert_eq!(offset, auto_scroll_offset(Point::new(50.0, 50.0), bounds, offset, extent, 10.0, 20.0));
    // Near the left and the bottom edges.
    assert_eq!(Vector::new(40.0, 60.0), auto_scroll_offset(Point::new(5.0, 95.0), bounds, offset, extent, 10.0, 20.0));
    // The offset stays within the scrollable range.
    assert_eq!(
        Vector::new(0.0, 200.0),
        auto_scroll_offset(Point::new(5.0, 95.0), bounds, Vector::new(5.0, 195.0), extent, 10.0, 20.0)
    );
}

// --- the drag state ----------------------------------------------------------------------------

#[test]
fn the_canvas_drag_marks_the_dragged_control_between_press_and_release() {
    let _app = start_app();
    let canvas = Canvas::new();
    let child = border(20.0, 20.0);
    canvas.children().add(child.clone());
    let behavior = CanvasDragBehavior::new();
    add_behavior(&child, &behavior);
    let _window = show_window(canvas, 200.0, 200.0);

    let mouse = Mouse::new();
    mouse.down(&child, Point::new(5.0, 5.0));
    assert!(child.classes().contains(":dragging"));

    mouse.move_to(&child, Point::new(15.0, 15.0));
    assert!(child.classes().contains(":dragging"));

    mouse.up(&child, Point::new(15.0, 15.0));
    assert!(!child.classes().contains(":dragging"));
}

#[test]
fn a_disabled_behavior_does_not_start_a_drag() {
    let _app = start_app();
    let canvas = Canvas::new();
    let child = border(20.0, 20.0);
    canvas.children().add(child.clone());
    let behavior = CanvasDragBehavior::new();
    behavior.set_is_enabled(false);
    add_behavior(&child, &behavior);
    let _window = show_window(canvas, 200.0, 200.0);

    Mouse::new().down(&child, Point::new(5.0, 5.0));

    assert!(!child.classes().contains(":dragging"));
}

#[test]
fn a_behavior_detached_from_the_visual_tree_removes_its_handlers() {
    let _app = start_app();
    let panel = StackPanel::new();
    let child = border(20.0, 20.0);
    panel.children().add(child.clone());
    let behavior = GridDragBehavior::new();
    add_behavior(&child, &behavior);
    let window = show_window(panel.clone(), 200.0, 200.0);

    window.set_content(None);
    crate::tests::support::run_jobs();
    // The panel keeps the child, which is no longer in the visual tree.
    Mouse::new().down(&child, Point::new(5.0, 5.0));

    assert!(!child.classes().contains(":dragging"));
}

#[test]
fn the_item_drag_marks_the_container_of_an_items_control() {
    let _app = start_app();
    let list_box = ferroui_controls::ListBox::new();
    list_box.set_items_source(Some(ItemsSource::from_strs(["Item1", "Item2", "Item3"])));
    let _window = show_window(list_box.clone(), 200.0, 200.0);

    let container = list_box.container_from_index(1).expect("the container of the second item");
    let behavior = ItemDragBehavior::new();
    behavior.set_orientation(Orientation::Vertical);
    add_behavior(&container, &behavior);

    let mouse = Mouse::new();
    mouse.down(&container, Point::new(5.0, 5.0));
    assert!(container.classes().contains(":dragging"));
    // The press installs a translation on the containers, the drag moves them.
    assert!(container.render_transform().is_some());

    mouse.up(&container, Point::new(5.0, 5.0));
    assert!(!container.classes().contains(":dragging"));
}

#[test]
fn the_mouse_drag_installs_a_translate_transform_on_press() {
    let _app = start_app();
    let panel = StackPanel::new();
    let child = border(20.0, 20.0);
    panel.children().add(child.clone());
    let behavior = MouseDragElementBehavior::new();
    add_behavior(&child, &behavior);
    let _window = show_window(panel, 200.0, 200.0);

    assert!(child.render_transform().is_none());

    Mouse::new().down(&child, Point::new(5.0, 5.0));

    let transform = child
        .render_transform()
        .and_then(|transform| transform.as_object().and_then(|object| object.to_ref().cast::<TranslateTransform>()));
    assert!(transform.is_some());
}

#[test]
fn the_multi_mouse_drag_moves_its_target_controls_with_the_associated_control() {
    let _app = start_app();
    let panel = StackPanel::new();
    let child = border(20.0, 20.0);
    let other = border(20.0, 20.0);
    panel.children().add(child.clone());
    panel.children().add(other.clone());
    let behavior = MultiMouseDragElementBehavior::new();
    // The collection is created on first use, and is the same afterwards.
    assert!(behavior.target_controls() == behavior.target_controls());
    behavior.target_controls().add(other.clone());
    add_behavior(&child, &behavior);
    let _window = show_window(panel, 200.0, 200.0);

    Mouse::new().down(&child, Point::new(5.0, 5.0));

    assert!(child.render_transform().is_some());
    assert!(other.render_transform().is_some());
}
