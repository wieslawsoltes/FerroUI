use crate::presenters::ScrollContentPresenter;
use crate::templates::FuncTemplateNameScopeExtensions;
use crate::test_support::{test_scope, TestRoot};
use crate::{Border, Canvas, Control, ControlImpl, Decorator, ScrollViewer, StackPanel, WrapPanel};
use ferroui_base::controls::{NameScope, NameScopeRef};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutHelper, LayoutableImpl, Orientation, VerticalAlignment};
use ferroui_base::reactive::ObservableExt;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectExtensions, FerroObjectImpl, Rect, Ref, Size,
    StyledElementImpl, Thickness, Vector, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
struct TestControl {
    base: Control,
    available_size: Cell<Size>,
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
        this.available_size.set(available_size);
        Size::new(150.0, 150.0)
    }
}

impl TestControl {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            available_size: Cell::new(Size::default()),
        })
    }
}

fn border(width: f64, height: f64) -> Ref<Border> {
    let border = Border::new();
    border.set_width(width);
    border.set_height(height);
    border
}

fn border_with_height(height: f64) -> Ref<Border> {
    let border = Border::new();
    border.set_height(height);
    border
}

fn scrolling_presenter() -> Ref<ScrollContentPresenter> {
    let target = ScrollContentPresenter::new();
    target.set_can_horizontally_scroll(true);
    target.set_can_vertically_scroll(true);
    target
}

fn alignment_and_padding_are_applied_to_child_bounds(
    h: HorizontalAlignment,
    v: VerticalAlignment,
    expected_x: f64,
    expected_y: f64,
    expected_width: f64,
    expected_height: f64,
) {
    let content = Border::new();
    content.set_min_width(16.0);
    content.set_min_height(16.0);
    content.set_horizontal_alignment(h);
    content.set_vertical_alignment(v);
    let target = ScrollContentPresenter::new();
    target.set_padding(Thickness::uniform(10.0));
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(
        Rect::new(expected_x, expected_y, expected_width, expected_height),
        content.bounds()
    );
}

#[test]
fn alignment_and_padding_are_applied_to_child_bounds_cases() {
    use HorizontalAlignment as H;
    use VerticalAlignment as V;
    alignment_and_padding_are_applied_to_child_bounds(H::Stretch, V::Stretch, 10.0, 10.0, 80.0, 80.0);
    alignment_and_padding_are_applied_to_child_bounds(H::Left, V::Stretch, 10.0, 10.0, 16.0, 80.0);
    alignment_and_padding_are_applied_to_child_bounds(H::Right, V::Stretch, 74.0, 10.0, 16.0, 80.0);
    alignment_and_padding_are_applied_to_child_bounds(H::Center, V::Stretch, 42.0, 10.0, 16.0, 80.0);
    alignment_and_padding_are_applied_to_child_bounds(H::Stretch, V::Top, 10.0, 10.0, 80.0, 16.0);
    alignment_and_padding_are_applied_to_child_bounds(H::Stretch, V::Bottom, 10.0, 74.0, 80.0, 16.0);
    alignment_and_padding_are_applied_to_child_bounds(H::Stretch, V::Center, 10.0, 42.0, 80.0, 16.0);
}

#[test]
fn desired_size_is_content_size_plus_padding_when_smaller_than_available_size() {
    let content = Border::new();
    content.set_min_width(16.0);
    content.set_min_height(16.0);
    let target = ScrollContentPresenter::new();
    target.set_padding(Thickness::uniform(10.0));
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(Size::new(36.0, 36.0), target.desired_size());
}

#[test]
fn desired_size_is_available_size_when_content_larger_than_available_size() {
    let content = Border::new();
    content.set_min_width(160.0);
    content.set_min_height(160.0);
    let target = ScrollContentPresenter::new();
    target.set_padding(Thickness::uniform(10.0));
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(Size::new(100.0, 100.0), target.desired_size());
}

#[test]
fn content_can_be_larger_than_viewport() {
    let content = TestControl::new();
    let target = scrolling_presenter();
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(Rect::new(0.0, 0.0, 150.0, 150.0), content.bounds());
}

#[test]
fn content_can_be_offset() {
    let content = border(150.0, 150.0);
    let target = scrolling_presenter();
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));
    target.set_offset(Vector::new(25.0, 25.0));
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(Rect::new(-25.0, -25.0, 150.0, 150.0), content.bounds());
}

#[test]
fn measure_should_pass_bounded_x_if_cannot_scroll_horizontally() {
    let child = TestControl::new();
    let target = ScrollContentPresenter::new();
    target.set_can_vertically_scroll(true);
    target.set_content(Some(Control::boxed(&child)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));

    assert_eq!(Size::new(100.0, f64::INFINITY), child.available_size.get());
}

#[test]
fn measure_should_pass_unbounded_x_if_can_scroll_horizontally() {
    let child = TestControl::new();
    let target = scrolling_presenter();
    target.set_content(Some(Control::boxed(&child)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));

    assert_eq!(Size::INFINITY, child.available_size.get());
}

#[test]
fn arrange_should_set_viewport_and_extent_in_that_order() {
    let target = ScrollContentPresenter::new();
    target.set_content(Some(Control::boxed(border(40.0, 50.0))));

    let set: Rc<RefCell<Vec<&'static str>>> = Rc::new(RefCell::new(Vec::new()));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));

    let skip_first = |name: &'static str| {
        let set = set.clone();
        let skipped = Cell::new(false);
        move |_: Size| {
            if skipped.replace(true) {
                set.borrow_mut().push(name);
            }
        }
    };
    let _viewport = FerroObjectExtensions::get_observable(fo(&target), ScrollViewer::viewport_property())
        .subscribe_fn(skip_first("Viewport"));
    let _extent = FerroObjectExtensions::get_observable(fo(&target), ScrollViewer::extent_property())
        .subscribe_fn(skip_first("Extent"));

    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(vec!["Viewport", "Extent"], *set.borrow());
}

#[test]
fn should_correctly_arrange_child_larger_than_viewport() {
    let child = Canvas::new();
    child.set_min_width(150.0);
    child.set_min_height(150.0);
    let target = ScrollContentPresenter::new();
    target.set_content(Some(Control::boxed(&child)));

    target.update_child();
    target.measure(Size::INFINITY);
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(Size::new(150.0, 150.0), child.bounds().size());
}

fn wrap_panel_with_three_borders() -> Ref<WrapPanel> {
    let child = WrapPanel::new();
    for _ in 0..3 {
        child.children().add(border(40.0, 50.0));
    }
    child
}

#[test]
fn arrange_should_constrain_child_width_when_can_horizontally_scroll_false() {
    let child = wrap_panel_with_three_borders();
    let target = ScrollContentPresenter::new();
    target.set_content(Some(Control::boxed(&child)));
    target.set_can_horizontally_scroll(false);

    target.update_child();
    target.measure(Size::INFINITY);
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(100.0, child.bounds().width);
}

#[test]
fn measure_should_deflate_available_size_by_padding() {
    let child = TestControl::new();
    let target = ScrollContentPresenter::new();
    target.set_padding(Thickness::uniform(10.0));
    target.set_content(Some(Control::boxed(&child)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));

    assert_eq!(Size::new(80.0, 80.0), child.available_size.get());
}

#[test]
fn extent_should_include_padding() {
    let target = scrolling_presenter();
    target.set_padding(Thickness::uniform(10.0));
    target.set_content(Some(Control::boxed(border(100.0, 100.0))));

    target.update_child();
    target.measure(Size::new(50.0, 50.0));
    target.arrange(Rect::new(0.0, 0.0, 50.0, 50.0));

    assert_eq!(Size::new(50.0, 50.0), target.viewport());
    assert_eq!(Size::new(120.0, 120.0), target.extent());
}

fn stack_panel_with_two_borders() -> Ref<StackPanel> {
    let content = StackPanel::new();
    content.children().add(border_with_height(50.0));
    content.children().add(border_with_height(50.0));
    content
}

#[test]
fn content_larger_than_viewport_should_not_be_squashed_by_padding() {
    let content = stack_panel_with_two_borders();
    let target = ScrollContentPresenter::new();
    target.set_can_vertically_scroll(true);
    target.set_padding(Thickness::uniform(10.0));
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();
    target.measure(Size::new(50.0, 50.0));
    target.arrange(Rect::new(0.0, 0.0, 50.0, 50.0));

    assert_eq!(Rect::new(10.0, 10.0, 30.0, 100.0), content.bounds());
    assert_eq!(Size::new(50.0, 120.0), target.extent());
}

#[test]
fn bottom_padding_should_be_visible_when_scrolled_to_end() {
    let content = stack_panel_with_two_borders();
    let target = ScrollContentPresenter::new();
    target.set_can_vertically_scroll(true);
    target.set_padding(Thickness::uniform(10.0));
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();
    target.measure(Size::new(50.0, 50.0));
    target.arrange(Rect::new(0.0, 0.0, 50.0, 50.0));

    // Scroll to the end: extent (120) - viewport (50).
    target.set_offset(Vector::new(0.0, 70.0));
    target.arrange(Rect::new(0.0, 0.0, 50.0, 50.0));

    assert_eq!(Vector::new(0.0, 70.0), target.offset());

    // The whole content is scrolled into view and the bottom padding is
    // still visible.
    assert_eq!(-60.0, content.bounds().y);
    assert_eq!(40.0, content.bounds().bottom());
}

#[test]
fn extent_should_include_content_margin() {
    let content = border(100.0, 100.0);
    content.set_margin(Thickness::uniform(5.0));
    let target = ScrollContentPresenter::new();
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();
    target.measure(Size::new(50.0, 50.0));
    target.arrange(Rect::new(0.0, 0.0, 50.0, 50.0));

    assert_eq!(Size::new(110.0, 110.0), target.extent());
}

#[test]
fn extent_should_include_content_margin_scaled_with_layout_rounding() {
    let _scope = test_scope();
    let root = TestRoot::new();
    root.set_layout_scaling(1.25);
    root.set_use_layout_rounding(true);

    let content = border(200.0, 200.0);
    content.set_margin(Thickness::uniform(2.0));
    let target = ScrollContentPresenter::new();
    target.set_horizontal_alignment(HorizontalAlignment::Center);
    target.set_vertical_alignment(VerticalAlignment::Center);
    target.set_content(Some(Control::boxed(&content)));

    root.set_child(&target);
    target.update_child();
    target.measure(Size::new(1000.0, 1000.0));
    target.arrange(Rect::new(0.0, 0.0, 1000.0, 1000.0));

    assert_eq!(Size::new(203.2, 203.2), target.viewport());
    assert_eq!(Size::new(203.2, 203.2), target.extent());
}

#[test]
fn extent_should_be_rounded_to_viewport_when_close() {
    let _scope = test_scope();
    let root = TestRoot::new();
    root.set_layout_scaling(1.75);
    root.set_use_layout_rounding(true);

    let content = border(164.57142857142858, 164.57142857142858);
    content.set_margin(Thickness::uniform(6.0));
    let target = ScrollContentPresenter::new();
    target.set_horizontal_alignment(HorizontalAlignment::Center);
    target.set_vertical_alignment(VerticalAlignment::Center);
    target.set_content(Some(Control::boxed(&content)));

    root.set_child(&target);
    target.update_child();
    target.measure(Size::new(1000.0, 1000.0));
    target.arrange(Rect::new(0.0, 0.0, 1000.0, 1000.0));

    let child = target.child().unwrap();
    let non_rounded_viewport = child.bounds().size().inflate(LayoutHelper::round_layout_thickness(
        child.margin(),
        root.layout_scaling(),
    ));

    assert_eq!(Size::new(176.00000000000003, 176.00000000000003), non_rounded_viewport);
    assert_eq!(Size::new(176.0, 176.0), target.viewport());
    assert_eq!(Size::new(176.0, 176.0), target.extent());
}

#[test]
fn extent_width_should_be_arrange_width_when_can_scroll_horizontally_false() {
    let child = wrap_panel_with_three_borders();
    let target = ScrollContentPresenter::new();
    target.set_content(Some(Control::boxed(&child)));
    target.set_can_horizontally_scroll(false);

    target.update_child();
    target.measure(Size::INFINITY);
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(Size::new(100.0, 100.0), target.extent());
}

#[test]
fn setting_offset_should_invalidate_arrange() {
    let target = ScrollContentPresenter::new();
    target.set_content(Some(Control::boxed(border(140.0, 150.0))));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));
    target.set_offset(Vector::new(10.0, 100.0));

    assert!(target.is_measure_valid());
    assert!(!target.is_arrange_valid());
}

#[test]
fn bring_descendant_into_view_should_update_offset() {
    let target = scrolling_presenter();
    target.set_width(100.0);
    target.set_height(100.0);
    target.set_content(Some(Control::boxed(border(200.0, 200.0))));

    target.update_child();
    target.measure(Size::INFINITY);
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));
    target.bring_descendant_into_view(&target.child().unwrap(), Rect::new(200.0, 200.0, 0.0, 0.0));

    assert_eq!(Vector::new(100.0, 100.0), target.offset());
}

#[test]
fn bring_descendant_into_view_should_be_idempotent_before_next_layout_pass() {
    let panel = StackPanel::new();

    for _ in 0..100 {
        panel.children().add(border_with_height(20.0));
    }

    let target = ScrollContentPresenter::new();
    target.set_width(50.0);
    target.set_height(100.0);
    target.set_can_vertically_scroll(true);
    target.set_content(Some(Control::boxed(&panel)));

    target.update_child();
    target.measure(Size::INFINITY);
    target.arrange(Rect::new(0.0, 0.0, 50.0, 100.0));

    // The 50th child spans 1000..1020, so with a 100px viewport it is
    // brought into view by scrolling to 920.
    let child = panel.children().get(50);
    target.bring_descendant_into_view(&child, Rect::from_size(child.bounds().size()));

    assert_eq!(920.0, target.offset().y);
    assert!(!target.is_arrange_valid());

    target.bring_descendant_into_view(&child, Rect::from_size(child.bounds().size()));

    assert_eq!(920.0, target.offset().y);
}

#[test]
fn bring_descendant_into_view_should_handle_child_margin() {
    let border = border(200.0, 200.0);
    let decorator = Decorator::new();
    decorator.set_margin(Thickness::uniform(50.0));
    decorator.set_child(&border);
    let target = scrolling_presenter();
    target.set_width(100.0);
    target.set_height(100.0);
    target.set_content(Some(Control::boxed(&decorator)));

    target.update_child();
    target.measure(Size::INFINITY);
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));
    target.bring_descendant_into_view(&border, Rect::new(200.0, 200.0, 0.0, 0.0));

    assert_eq!(Vector::new(150.0, 150.0), target.offset());
}

/// A presenter of 200x100 holding a decorator with a vertical stack of 100
/// borders of 100x20, each registered as `Border{i}` in the name scope of
/// the presenter.
fn presenter_with_named_borders(margin: Thickness) -> (Ref<ScrollContentPresenter>, Ref<StackPanel>) {
    let namescope = NameScopeRef::new(NameScope::new());

    let content = StackPanel::new();
    content.set_orientation(Orientation::Vertical);
    content.set_width(100.0);
    content.set_margin(margin);

    for i in 0..100 {
        let child = border(100.0, 20.0);
        child.set_name(Some(format!("Border{i}")));
        content.children().add(child.register_in_name_scope(&*namescope));
    }

    let decorator = Decorator::new();
    decorator.set_child(&content);
    let target = scrolling_presenter();
    target.set_width(200.0);
    target.set_height(100.0);
    target.set_content(Some(Control::boxed(&decorator)));
    NameScope::set_name_scope(&target, Some(namescope));

    (target, content)
}

#[test]
fn bring_descendant_into_view_should_move_child_even_with_margin_in_parent() {
    let (target, content) = presenter_with_named_borders(Thickness::symmetric(0.0, 200.0));

    target.update_child();
    target.measure(Size::INFINITY);
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    // Border20 is at position 0,600 with bottom at Y=620
    let border20 = target.get_control::<Border>("Border20");
    target.bring_descendant_into_view(&border20, Rect::from_size(border20.bounds().size()));

    // With viewport Height of 100, border becomes fully visible when
    // aligned from the bottom at Offset Y=520, i.e. 620-100
    assert_eq!(Vector::new(0.0, 520.0), target.offset());

    // Reset stack panel's margin
    content.set_margin(Thickness::default());
    target.measure(Size::INFINITY);
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    // Border40 is at position 0,800 with bottom at Y=820
    let border40 = target.get_control::<Border>("Border40");
    target.bring_descendant_into_view(&border40, Rect::from_size(border40.bounds().size()));

    // With viewport Height of 100, border becomes fully visible when
    // aligned from the bottom at Offset Y=720, i.e. 820-100
    assert_eq!(Vector::new(0.0, 720.0), target.offset());
}

#[test]
fn bring_descendant_into_view_should_not_move_child_if_completely_in_view() {
    let (target, _content) = presenter_with_named_borders(Thickness::default());

    target.update_child();
    target.measure(Size::INFINITY);
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    let border3 = target.get_control::<Border>("Border3");
    target.bring_descendant_into_view(&border3, Rect::from_size(border3.bounds().size()));

    // Border3 is still in view, offset hasn't changed
    assert_eq!(Vector::new(0.0, 0.0), target.offset());
}

/// A presenter of 200x100 holding a decorator with a vertical stack of 100
/// borders of 100x20, with `border` at `index`.
fn presenter_with_border_at(index: usize, border: &Ref<Border>) -> Ref<ScrollContentPresenter> {
    let content = StackPanel::new();
    content.set_orientation(Orientation::Vertical);
    content.set_width(100.0);

    for i in 0..100 {
        let child = if i == index {
            border.clone()
        } else {
            self::border(100.0, 20.0)
        };
        content.children().add(child);
    }

    let decorator = Decorator::new();
    decorator.set_child(&content);
    let target = scrolling_presenter();
    target.set_width(200.0);
    target.set_height(100.0);
    target.set_content(Some(Control::boxed(&decorator)));
    target
}

#[test]
fn bring_descendant_into_view_should_move_child_at_least_partially_above_viewport() {
    // border position will be (0,60)
    let border = border(100.0, 20.0);
    let target = presenter_with_border_at(3, &border);

    target.update_child();
    target.measure(Size::INFINITY);
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    // move border to above the view port
    target.set_offset(Vector::new(0.0, 90.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    target.bring_descendant_into_view(&border, Rect::from_size(border.bounds().size()));

    assert_eq!(Vector::new(0.0, 60.0), target.offset());

    // move border to partially above the view port
    target.set_offset(Vector::new(0.0, 70.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    target.bring_descendant_into_view(&border, Rect::from_size(border.bounds().size()));

    assert_eq!(Vector::new(0.0, 60.0), target.offset());
}

#[test]
fn bring_descendant_into_view_should_not_move_child_if_completely_covers_viewport() {
    // border position will be (0,60)
    let border = border(100.0, 200.0);
    let target = presenter_with_border_at(3, &border);

    target.update_child();
    target.measure(Size::INFINITY);
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    // move border such that it's partially above viewport and partially
    // below viewport
    target.set_offset(Vector::new(0.0, 90.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    target.bring_descendant_into_view(&border, Rect::from_size(border.bounds().size()));

    assert_eq!(Vector::new(0.0, 90.0), target.offset());
}

#[test]
fn bring_descendant_into_view_should_move_child_at_least_partially_below_viewport() {
    // border position will be (0,180)
    let border = border(100.0, 20.0);
    let target = presenter_with_border_at(9, &border);

    target.update_child();
    target.measure(Size::INFINITY);
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    // border is at (0, 180) and below the viewport
    target.bring_descendant_into_view(&border, Rect::from_size(border.bounds().size()));

    assert_eq!(Vector::new(0.0, 100.0), target.offset());

    // move border to partially below the view port
    target.set_offset(Vector::new(0.0, 90.0));

    target.bring_descendant_into_view(&border, Rect::from_size(border.bounds().size()));
}

#[test]
fn nested_presenters_should_scroll_outer_when_content_exceeds_viewport() {
    let border = border(200.0, 25.0); // larger than viewport
    border.set_horizontal_alignment(HorizontalAlignment::Left);
    border.set_vertical_alignment(VerticalAlignment::Top);
    border.set_margin(Thickness::new(0.0, 120.0, 0.0, 0.0));

    let inner_presenter = scrolling_presenter();
    inner_presenter.set_width(100.0);
    inner_presenter.set_height(200.0);
    inner_presenter.set_content(Some(Control::boxed(&border)));

    let outer_presenter = scrolling_presenter();
    outer_presenter.set_width(100.0);
    outer_presenter.set_height(100.0);
    outer_presenter.set_content(Some(Control::boxed(&inner_presenter)));

    inner_presenter.update_child();
    outer_presenter.update_child();
    outer_presenter.measure(Size::new(100.0, 100.0));
    outer_presenter.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    border.bring_into_view();

    assert_eq!(Vector::new(0.0, 45.0), outer_presenter.offset());
    assert_eq!(Vector::new(0.0, 0.0), inner_presenter.offset());
}

#[test]
fn nested_presenters_should_scroll_outer_when_viewports_are_close() {
    let first = border_with_height(455.31818181818187);
    first.set_use_layout_rounding(false);
    let border = border(100.0, 37.94318181818182);
    border.set_use_layout_rounding(false);
    let panel = StackPanel::new();
    panel.children().add(&first);
    panel.children().add(&border);

    let inner_presenter = scrolling_presenter();
    inner_presenter.set_width(100.0);
    inner_presenter.set_height(493.2613636363636);
    inner_presenter.set_use_layout_rounding(false);
    inner_presenter.set_content(Some(Control::boxed(&panel)));

    let outer_presenter = scrolling_presenter();
    outer_presenter.set_width(100.0);
    outer_presenter.set_height(170.0568181818182);
    outer_presenter.set_use_layout_rounding(false);
    outer_presenter.set_content(Some(Control::boxed(&inner_presenter)));

    inner_presenter.update_child();
    outer_presenter.update_child();
    outer_presenter.measure(Size::new(100.0, 170.0568181818182));
    outer_presenter.arrange(Rect::new(0.0, 0.0, 100.0, 170.0568181818182));

    border.bring_into_view();

    assert_eq!(Vector::new(0.0, 323.20454545454544), outer_presenter.offset());
    assert_eq!(Vector::new(0.0, 0.0), inner_presenter.offset());
}

/// The object as its root class, for calls that would otherwise resolve to
/// a member of an intermediate class.
fn fo(object: &ferroui_base::FerroObject) -> &ferroui_base::FerroObject {
    object
}
