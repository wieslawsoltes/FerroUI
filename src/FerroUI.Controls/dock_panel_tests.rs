use crate::{Border, Dock, DockPanel};
use ferroui_base::{Rect, Ref, Size};

fn border(width: f64, height: f64, dock: Dock) -> Ref<Border> {
    let border = Border::new();
    border.set_width(width);
    border.set_height(height);
    DockPanel::set_dock(&border, dock);
    border
}

#[test]
fn dock_panel_without_child() {
    let target = DockPanel::new();
    target.set_width(10.0);
    target.set_height(10.0);

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds(), Rect::new(0.0, 0.0, 10.0, 10.0));
}

#[test]
fn should_dock_controls_horizontal_first() {
    let target = DockPanel::new();
    target.children().add(border(500.0, 50.0, Dock::Top));
    target.children().add(border(500.0, 50.0, Dock::Bottom));
    target.children().add(border(50.0, 400.0, Dock::Left));
    target.children().add(border(50.0, 400.0, Dock::Right));
    target.children().add(Border::new());

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds(), Rect::new(0.0, 0.0, 500.0, 500.0));
    assert_eq!(target.children().get(0).bounds(), Rect::new(0.0, 0.0, 500.0, 50.0));
    assert_eq!(target.children().get(1).bounds(), Rect::new(0.0, 450.0, 500.0, 50.0));
    assert_eq!(target.children().get(2).bounds(), Rect::new(0.0, 50.0, 50.0, 400.0));
    assert_eq!(target.children().get(3).bounds(), Rect::new(450.0, 50.0, 50.0, 400.0));
    assert_eq!(target.children().get(4).bounds(), Rect::new(50.0, 50.0, 400.0, 400.0));
}

#[test]
fn should_dock_controls_vertical_first() {
    let target = DockPanel::new();
    target.children().add(border(50.0, 400.0, Dock::Left));
    target.children().add(border(50.0, 400.0, Dock::Right));
    target.children().add(border(500.0, 50.0, Dock::Top));
    target.children().add(border(500.0, 50.0, Dock::Bottom));
    target.children().add(Border::new());

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds(), Rect::new(0.0, 0.0, 600.0, 400.0));
    assert_eq!(target.children().get(0).bounds(), Rect::new(0.0, 0.0, 50.0, 400.0));
    assert_eq!(target.children().get(1).bounds(), Rect::new(550.0, 0.0, 50.0, 400.0));
    assert_eq!(target.children().get(2).bounds(), Rect::new(50.0, 0.0, 500.0, 50.0));
    assert_eq!(target.children().get(3).bounds(), Rect::new(50.0, 350.0, 500.0, 50.0));
    assert_eq!(target.children().get(4).bounds(), Rect::new(50.0, 50.0, 500.0, 300.0));
}

#[test]
fn should_dock_controls_with_spacing() {
    let target = DockPanel::new();
    target.set_horizontal_spacing(10.0);
    target.set_vertical_spacing(10.0);
    target.children().add(border(500.0, 50.0, Dock::Top));
    target.children().add(border(500.0, 50.0, Dock::Bottom));
    target.children().add(border(50.0, 400.0, Dock::Left));
    target.children().add(border(50.0, 400.0, Dock::Right));
    target.children().add(Border::new());

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds(), Rect::new(0.0, 0.0, 500.0, 520.0));
    assert_eq!(target.children().get(0).bounds(), Rect::new(0.0, 0.0, 500.0, 50.0));
    assert_eq!(target.children().get(1).bounds(), Rect::new(0.0, 470.0, 500.0, 50.0));
    assert_eq!(target.children().get(2).bounds(), Rect::new(0.0, 60.0, 50.0, 400.0));
    assert_eq!(target.children().get(3).bounds(), Rect::new(450.0, 60.0, 50.0, 400.0));
    assert_eq!(target.children().get(4).bounds(), Rect::new(60.0, 60.0, 380.0, 400.0));
}

#[test]
fn changing_child_dock_invalidates_measure() {
    let child = Border::new();
    DockPanel::set_dock(&child, Dock::Left);
    let target = DockPanel::new();
    target.children().add(&child);

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));
    assert!(target.is_measure_valid());

    DockPanel::set_dock(&child, Dock::Right);

    assert!(!target.is_measure_valid());
}
