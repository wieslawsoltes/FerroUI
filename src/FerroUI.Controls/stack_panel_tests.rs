use crate::test_support::TestRoot;
use crate::{Border, Control, ControlImpl, Panel, StackPanel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Rect, Ref, Size, StyledElementImpl, Visual,
    VisualImpl,
};
use std::cell::Cell;

fn border(width: Option<f64>, height: Option<f64>) -> Ref<Border> {
    let border = Border::new();
    if let Some(width) = width {
        border.set_width(width);
    }
    if let Some(height) = height {
        border.set_height(height);
    }
    border
}

fn stack_panel(width: f64, height: f64, is_visible: bool) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.set_width(width);
    panel.set_height(height);
    if !is_visible {
        panel.set_is_visible(false);
    }
    panel
}

#[repr(C)]
struct TestControl {
    base: Control,
    measure_constraint: Cell<Size>,
    measure_size: Cell<Size>,
}

ferro_class!(TestControl: Control);
ferro_impl_classes!(
    TestControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl LayoutableImpl for TestControl {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.measure_constraint.set(available_size);
        this.measure_size.get()
    }
}

impl TestControl {
    fn new(measure_size: Size) -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            measure_constraint: Cell::new(Size::default()),
            measure_size: Cell::new(measure_size),
        })
    }

    fn horizontal(alignment: HorizontalAlignment, measure_size: Size) -> Ref<Self> {
        let control = Self::new(measure_size);
        control.set_horizontal_alignment(alignment);
        control
    }

    fn vertical(alignment: VerticalAlignment, measure_size: Size) -> Ref<Self> {
        let control = Self::new(measure_size);
        control.set_vertical_alignment(alignment);
        control
    }
}

#[test]
fn lays_out_children_vertically() {
    let target = StackPanel::new();
    target.children().add(border(Some(120.0), Some(20.0)));
    target.children().add(border(None, Some(30.0)));
    target.children().add(border(None, Some(50.0)));

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds().size(), Size::new(120.0, 100.0));
    assert_eq!(target.children().get(0).bounds(), Rect::new(0.0, 0.0, 120.0, 20.0));
    assert_eq!(target.children().get(1).bounds(), Rect::new(0.0, 20.0, 120.0, 30.0));
    assert_eq!(target.children().get(2).bounds(), Rect::new(0.0, 50.0, 120.0, 50.0));
}

#[test]
fn lays_out_children_horizontally() {
    let target = StackPanel::new();
    target.set_orientation(Orientation::Horizontal);
    target.children().add(border(Some(20.0), Some(120.0)));
    target.children().add(border(Some(30.0), None));
    target.children().add(border(Some(50.0), None));

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds().size(), Size::new(100.0, 120.0));
    assert_eq!(target.children().get(0).bounds(), Rect::new(0.0, 0.0, 20.0, 120.0));
    assert_eq!(target.children().get(1).bounds(), Rect::new(20.0, 0.0, 30.0, 120.0));
    assert_eq!(target.children().get(2).bounds(), Rect::new(50.0, 0.0, 50.0, 120.0));
}

#[test]
fn lays_out_children_vertically_with_spacing() {
    let target = StackPanel::new();
    target.set_spacing(10.0);
    target.children().add(border(Some(120.0), Some(20.0)));
    target.children().add(border(None, Some(30.0)));
    target.children().add(border(None, Some(50.0)));

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds().size(), Size::new(120.0, 120.0));
    assert_eq!(target.children().get(0).bounds(), Rect::new(0.0, 0.0, 120.0, 20.0));
    assert_eq!(target.children().get(1).bounds(), Rect::new(0.0, 30.0, 120.0, 30.0));
    assert_eq!(target.children().get(2).bounds(), Rect::new(0.0, 70.0, 120.0, 50.0));
}

#[test]
fn lays_out_children_horizontally_with_spacing() {
    let target = StackPanel::new();
    target.set_spacing(10.0);
    target.set_orientation(Orientation::Horizontal);
    target.children().add(border(Some(20.0), Some(120.0)));
    target.children().add(border(Some(30.0), None));
    target.children().add(border(Some(50.0), None));

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds().size(), Size::new(120.0, 120.0));
    assert_eq!(target.children().get(0).bounds(), Rect::new(0.0, 0.0, 20.0, 120.0));
    assert_eq!(target.children().get(1).bounds(), Rect::new(30.0, 0.0, 30.0, 120.0));
    assert_eq!(target.children().get(2).bounds(), Rect::new(70.0, 0.0, 50.0, 120.0));
}

#[test]
fn lays_out_children_vertically_even_if_larger_than_panel() {
    let target = StackPanel::new();
    target.set_height(60.0);
    target.children().add(border(Some(120.0), Some(20.0)));
    target.children().add(border(None, Some(30.0)));
    target.children().add(border(None, Some(50.0)));

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds().size(), Size::new(120.0, 60.0));
    assert_eq!(target.children().get(0).bounds(), Rect::new(0.0, 0.0, 120.0, 20.0));
    assert_eq!(target.children().get(1).bounds(), Rect::new(0.0, 20.0, 120.0, 30.0));
    assert_eq!(target.children().get(2).bounds(), Rect::new(0.0, 50.0, 120.0, 50.0));
}

#[test]
fn lays_out_children_horizontally_even_if_larger_than_panel() {
    let target = StackPanel::new();
    target.set_width(60.0);
    target.set_orientation(Orientation::Horizontal);
    target.children().add(border(Some(20.0), Some(120.0)));
    target.children().add(border(Some(30.0), None));
    target.children().add(border(Some(50.0), None));

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds().size(), Size::new(60.0, 120.0));
    assert_eq!(target.children().get(0).bounds(), Rect::new(0.0, 0.0, 20.0, 120.0));
    assert_eq!(target.children().get(1).bounds(), Rect::new(20.0, 0.0, 30.0, 120.0));
    assert_eq!(target.children().get(2).bounds(), Rect::new(50.0, 0.0, 50.0, 120.0));
}

#[test]
fn arranges_vertical_children_with_correct_bounds() {
    let target = StackPanel::new();
    target.set_orientation(Orientation::Vertical);
    for (alignment, width) in [
        (HorizontalAlignment::Left, 50.0),
        (HorizontalAlignment::Left, 150.0),
        (HorizontalAlignment::Center, 50.0),
        (HorizontalAlignment::Center, 150.0),
        (HorizontalAlignment::Right, 50.0),
        (HorizontalAlignment::Right, 150.0),
        (HorizontalAlignment::Stretch, 50.0),
        (HorizontalAlignment::Stretch, 150.0),
    ] {
        target.children().add(TestControl::horizontal(alignment, Size::new(width, 10.0)));
    }

    target.measure(Size::new(100.0, 150.0));
    assert_eq!(target.desired_size(), Size::new(100.0, 80.0));

    target.arrange(Rect::from_size(target.desired_size()));

    let bounds: Vec<Rect> = target.children().to_vec().iter().map(|x| x.bounds()).collect();

    assert_eq!(
        bounds,
        vec![
            Rect::new(0.0, 0.0, 50.0, 10.0),
            Rect::new(0.0, 10.0, 100.0, 10.0),
            Rect::new(25.0, 20.0, 50.0, 10.0),
            Rect::new(0.0, 30.0, 100.0, 10.0),
            Rect::new(50.0, 40.0, 50.0, 10.0),
            Rect::new(0.0, 50.0, 100.0, 10.0),
            Rect::new(0.0, 60.0, 100.0, 10.0),
            Rect::new(0.0, 70.0, 100.0, 10.0),
        ]
    );
}

#[test]
fn arranges_horizontal_children_with_correct_bounds() {
    let target = StackPanel::new();
    target.set_orientation(Orientation::Horizontal);
    for (alignment, height) in [
        (VerticalAlignment::Top, 50.0),
        (VerticalAlignment::Top, 150.0),
        (VerticalAlignment::Center, 50.0),
        (VerticalAlignment::Center, 150.0),
        (VerticalAlignment::Bottom, 50.0),
        (VerticalAlignment::Bottom, 150.0),
        (VerticalAlignment::Stretch, 50.0),
        (VerticalAlignment::Stretch, 150.0),
    ] {
        target.children().add(TestControl::vertical(alignment, Size::new(10.0, height)));
    }

    target.measure(Size::new(150.0, 100.0));
    assert_eq!(target.desired_size(), Size::new(80.0, 100.0));

    target.arrange(Rect::from_size(target.desired_size()));

    let bounds: Vec<Rect> = target.children().to_vec().iter().map(|x| x.bounds()).collect();

    assert_eq!(
        bounds,
        vec![
            Rect::new(0.0, 0.0, 10.0, 50.0),
            Rect::new(10.0, 0.0, 10.0, 100.0),
            Rect::new(20.0, 25.0, 10.0, 50.0),
            Rect::new(30.0, 0.0, 10.0, 100.0),
            Rect::new(40.0, 50.0, 10.0, 50.0),
            Rect::new(50.0, 0.0, 10.0, 100.0),
            Rect::new(60.0, 0.0, 10.0, 100.0),
            Rect::new(70.0, 0.0, 10.0, 100.0),
        ]
    );
}

#[test]
fn spacing_not_added_for_invisible_children() {
    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        let target_three_children_one_invisible = StackPanel::new();
        target_three_children_one_invisible.set_spacing(40.0);
        target_three_children_one_invisible.set_orientation(orientation);
        target_three_children_one_invisible.children().add(stack_panel(10.0, 10.0, false));
        target_three_children_one_invisible.children().add(stack_panel(10.0, 10.0, true));
        target_three_children_one_invisible.children().add(stack_panel(10.0, 10.0, true));

        let target_two_children_none_invisible = StackPanel::new();
        target_two_children_none_invisible.set_spacing(40.0);
        target_two_children_none_invisible.set_orientation(orientation);
        target_two_children_none_invisible.children().add(stack_panel(10.0, 10.0, true));
        target_two_children_none_invisible.children().add(stack_panel(10.0, 10.0, true));

        target_three_children_one_invisible.measure(Size::INFINITY);
        target_three_children_one_invisible
            .arrange(Rect::from_size(target_three_children_one_invisible.desired_size()));

        target_two_children_none_invisible.measure(Size::INFINITY);
        target_two_children_none_invisible.arrange(Rect::from_size(target_two_children_none_invisible.desired_size()));

        let size_with_two_children = target_two_children_none_invisible.bounds().size();
        let size_with_three_children = target_three_children_one_invisible.bounds().size();

        assert_eq!(size_with_two_children, size_with_three_children);
    }
}

#[test]
fn spacing_not_added_for_children_hidden_by_style_applied_during_measure() {
    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        let target = StackPanel::new();
        target.set_spacing(40.0);
        target.set_orientation(orientation);
        let hidden = stack_panel(10.0, 10.0, true);
        hidden.classes().add("hidden");
        target.children().add(hidden);
        target.children().add(stack_panel(10.0, 10.0, true));
        target.children().add(stack_panel(10.0, 10.0, true));

        let root = TestRoot::with_child(&target);

        root.styles().add(Style::with_setters(
            Selectors::of_type::<StackPanel>().class("hidden"),
            [Setter::new(Visual::is_visible_property(), false)],
        ));

        target.measure(Size::INFINITY);

        let expected =
            if orientation == Orientation::Horizontal { Size::new(60.0, 10.0) } else { Size::new(10.0, 60.0) };

        assert_eq!(target.desired_size(), expected);
    }
}

#[test]
fn only_arrange_visible_children() {
    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        let hidden_panel = Panel::new();
        hidden_panel.set_width(10.0);
        hidden_panel.set_height(10.0);
        hidden_panel.set_is_visible(false);
        let panel = Panel::new();
        panel.set_width(10.0);
        panel.set_height(10.0);

        let target = StackPanel::new();
        target.set_spacing(40.0);
        target.set_orientation(orientation);
        target.children().add(&hidden_panel);
        target.children().add(&panel);

        target.measure(Size::INFINITY);
        target.arrange(Rect::from_size(target.desired_size()));
        assert_eq!(panel.bounds(), Rect::new(0.0, 0.0, 10.0, 10.0));
    }
}

#[test]
fn arrange_raises_snap_points_changed_for_the_orientation() {
    use std::rc::Rc;

    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        let target = StackPanel::new();
        target.set_orientation(orientation);
        target.children().add(border(Some(10.0), Some(10.0)));
        let horizontal = Rc::new(Cell::new(0));
        let vertical = Rc::new(Cell::new(0));
        let raised = horizontal.clone();
        target.horizontal_snap_points_changed(move |_, _| raised.set(raised.get() + 1));
        let raised = vertical.clone();
        target.vertical_snap_points_changed(move |_, _| raised.set(raised.get() + 1));

        target.measure(Size::INFINITY);
        target.arrange(Rect::from_size(target.desired_size()));

        let expected = if orientation == Orientation::Horizontal { (1, 0) } else { (0, 1) };
        assert_eq!((horizontal.get(), vertical.get()), expected);
    }
}

#[test]
fn snap_points_are_computed_from_child_bounds() {
    use crate::primitives::SnapPointsAlignment;

    let target = StackPanel::new();
    target.set_spacing(5.0);
    target.children().add(border(Some(120.0), Some(20.0)));
    target.children().add(border(None, Some(30.0)));

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.get_irregular_snap_points(Orientation::Vertical, SnapPointsAlignment::Near), vec![0.0, 25.0]);
    assert_eq!(target.get_irregular_snap_points(Orientation::Vertical, SnapPointsAlignment::Center), vec![10.0, 40.0]);
    assert_eq!(target.get_irregular_snap_points(Orientation::Vertical, SnapPointsAlignment::Far), vec![20.0, 55.0]);
    assert!(target.get_irregular_snap_points(Orientation::Horizontal, SnapPointsAlignment::Near).is_empty());

    target.set_are_vertical_snap_points_regular(true);

    assert_eq!(target.get_regular_snap_points(Orientation::Vertical, SnapPointsAlignment::Far), (25.0, 20.0));
}
