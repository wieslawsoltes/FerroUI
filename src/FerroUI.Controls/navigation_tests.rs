use crate::test_support::{test_scope, TestRoot};
use crate::{Border, Canvas, ColumnDefinitions, Control, Grid, StackPanel, WrapPanel};
use ferroui_base::input::{INavigableContainer, InputElement, NavigationDirection};
use ferroui_base::layout::Orientation;
use ferroui_base::{Ref, Size};

fn input(control: &Ref<Control>) -> Ref<InputElement> {
    control.clone().upcast()
}

#[test]
fn stack_panel_navigates_along_its_orientation_and_wraps() {
    let target = StackPanel::new();
    let first = Control::new();
    let second = Control::new();
    target.children().add(&first);
    target.children().add(&second);

    assert_eq!(target.get_control(NavigationDirection::Down, Some(&input(&first)), false), Some(input(&second)));
    assert_eq!(target.get_control(NavigationDirection::Right, Some(&input(&first)), false), None);
    assert_eq!(target.get_control(NavigationDirection::Down, Some(&input(&second)), false), None);
    assert_eq!(target.get_control(NavigationDirection::Down, Some(&input(&second)), true), Some(input(&first)));
    assert_eq!(target.get_control(NavigationDirection::Up, Some(&input(&first)), true), Some(input(&second)));
    assert_eq!(target.get_control(NavigationDirection::First, None, false), Some(input(&first)));
    assert_eq!(target.get_control(NavigationDirection::Last, None, false), Some(input(&second)));

    target.set_orientation(Orientation::Horizontal);
    assert_eq!(target.get_control(NavigationDirection::Right, Some(&input(&first)), false), Some(input(&second)));
    assert_eq!(target.get_control(NavigationDirection::Down, Some(&input(&first)), false), None);
}

#[test]
fn wrap_panel_navigates_along_its_orientation() {
    let target = WrapPanel::new();
    let first = Control::new();
    let second = Control::new();
    target.children().add(&first);
    target.children().add(&second);

    assert_eq!(target.get_control(NavigationDirection::Right, Some(&input(&first)), false), Some(input(&second)));
    assert_eq!(target.get_control(NavigationDirection::Down, Some(&input(&first)), false), None);
    assert_eq!(target.get_control(NavigationDirection::Next, Some(&input(&second)), true), None);
}

#[test]
fn canvas_has_no_directional_navigation() {
    let target = Canvas::new();
    let child = Control::new();
    target.children().add(&child);

    assert_eq!(target.get_control(NavigationDirection::Next, Some(&input(&child)), true), None);
}

#[test]
fn grid_adds_and_removes_the_grid_lines_visual() {
    let _scope = test_scope();
    let target = Grid::new();
    target.set_column_definitions(ColumnDefinitions::parse("*,*").unwrap());
    target.children().add(Border::new());
    let root = TestRoot::with_child(&target);
    root.execute_initial_layout_pass();
    assert_eq!(target.visual_children().count(), 1);

    target.set_show_grid_lines(true);
    target.measure(Size::new(100.0, 100.0));
    root.layout_manager().execute_layout_pass();
    assert_eq!(target.visual_children().count(), 2);
    assert_eq!(target.children().count(), 1);

    target.set_show_grid_lines(false);
    root.layout_manager().execute_layout_pass();
    assert_eq!(target.visual_children().count(), 1);
}
