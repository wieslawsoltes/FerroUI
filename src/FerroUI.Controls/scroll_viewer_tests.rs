use crate::mouse_test_helper::MouseTestHelper;
use crate::presenters::{ContentPresenter, ScrollContentPresenter};
use crate::primitives::{RangeBase, ScrollBar, ScrollBarVisibility, TemplatedControl, Thumb, Track};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::{boxed_str, test_scope, use_test_text_block, TestRoot, TestTextBlock};
use crate::test_support_buttons::focus_scope;
use crate::test_support_scrolling::TestScrollable;
use crate::{
    Border, Button, ColumnDefinition, ColumnDefinitions, Control, ControlImpl, Grid, GridLength, GridUnitType, Panel,
    PanelImpl, RowDefinition, RowDefinitions, ScrollViewer, StackPanel,
};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::data::{BindingMode, IndexerBinding, TemplateBinding};
use ferroui_base::input::{InputElement, InputElementImpl, Key, KeyEventArgs, MouseButton};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, LayoutableImplExt, Orientation, VerticalAlignment};
use ferroui_base::media::{Brushes, FlowDirection};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroProperty, Point, Ref, Size, StyledElementImpl,
    Thickness, Vector, Visual, VisualImpl,
};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
struct TestPanel {
    base: Panel,
    desired_width: Cell<f64>,
    measure_override_calls: Cell<i32>,
    arrange_override_calls: Cell<i32>,
}

ferro_class!(TestPanel: Panel);
ferro_impl_classes!(
    TestPanel: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

impl LayoutableImpl for TestPanel {
    fn measure_override(this: &Self, _available_size: Size) -> Size {
        this.measure_override_calls.set(this.measure_override_calls.get() + 1);
        Size::new(this.desired_width.get(), 1.0)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        this.arrange_override_calls.set(this.arrange_override_calls.get() + 1);
        Self::parent_arrange_override(this, final_size)
    }
}

impl TestPanel {
    fn new(desired_width: f64) -> Ref<Self> {
        instantiate(Self {
            base: Panel::construct(),
            desired_width: Cell::new(desired_width),
            measure_override_calls: Cell::new(0),
            arrange_override_calls: Cell::new(0),
        })
    }

    fn reset(&self) {
        self.measure_override_calls.set(0);
        self.arrange_override_calls.set(0);
    }
}

#[repr(C)]
struct TestContent {
    base: Control,
    measure_size: Cell<Size>,
}

ferro_class!(TestContent: Control);
ferro_impl_classes!(
    TestContent: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl LayoutableImpl for TestContent {
    fn measure_override(this: &Self, _available_size: Size) -> Size {
        this.measure_size.get()
    }
}

impl TestContent {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            measure_size: Cell::new(Size::new(1000.0, 2000.0)),
        })
    }
}

fn indexer(source: &Ref<ScrollBar>, property: &'static FerroProperty, mode: BindingMode) -> IndexerBinding {
    IndexerBinding::new(source.clone().upcast(), property, mode)
}

fn create_thumb_template(_control: &Ref<Thumb>, _scope: &NameScopeRef) -> Ref<Control> {
    let border = Border::new();
    border.set_background(Some(Brushes::gray()));
    border.upcast()
}

fn create_scroll_bar_template(scroll_bar: &Ref<ScrollBar>, scope: &NameScopeRef) -> Ref<Control> {
    let thumb = Thumb::new();
    thumb.set_template(Some(FuncControlTemplate::for_type::<Thumb>(create_thumb_template)));

    let track = Track::new();
    track.set_name(Some("track".to_string()));
    track.set_is_direction_reversed(true);
    track.bind_binding(
        Track::minimum_property().as_property(),
        &indexer(
            scroll_bar,
            RangeBase::minimum_property().as_property(),
            BindingMode::OneWay,
        ),
    );
    track.bind_binding(
        Track::maximum_property().as_property(),
        &indexer(
            scroll_bar,
            RangeBase::maximum_property().as_property(),
            BindingMode::OneWay,
        ),
    );
    track.bind_binding(
        Track::value_property().as_property(),
        &indexer(
            scroll_bar,
            RangeBase::value_property().as_property(),
            BindingMode::TwoWay,
        ),
    );
    track.bind_binding(
        Track::viewport_size_property().as_property(),
        &indexer(
            scroll_bar,
            ScrollBar::viewport_size_property().as_property(),
            BindingMode::OneWay,
        ),
    );
    track.bind_binding(
        Track::orientation_property().as_property(),
        &indexer(
            scroll_bar,
            ScrollBar::orientation_property().as_property(),
            BindingMode::OneWay,
        ),
    );
    let templated_parent = scroll_bar
        .templated_parent()
        .expect("the scroll bar has a templated parent");
    track.bind_binding(
        Track::defer_thumb_drag_property().as_property(),
        &IndexerBinding::new(
            templated_parent,
            ScrollViewer::is_deferred_scrolling_enabled_property().as_property(),
            BindingMode::OneWay,
        ),
    );
    track.set_thumb(thumb);

    let border = Border::new();
    border.set_child(track.register_in_name_scope(&**scope));
    border.upcast()
}

pub(crate) fn create_template(_control: &Ref<ScrollViewer>, scope: &NameScopeRef) -> Ref<Control> {
    let grid = Grid::new();
    grid.set_column_definitions(ColumnDefinitions::from_items([
        ColumnDefinition::with_value(1.0, GridUnitType::Star),
        ColumnDefinition::with_width(GridLength::AUTO),
    ]));
    grid.set_row_definitions(RowDefinitions::from_items([
        RowDefinition::with_value(1.0, GridUnitType::Star),
        RowDefinition::with_height(GridLength::AUTO),
    ]));

    let presenter = ScrollContentPresenter::new();
    presenter.set_name(Some("PART_ContentPresenter".to_string()));
    presenter.bind_binding(
        ContentPresenter::padding_property().as_property(),
        &TemplateBinding::new(TemplatedControl::padding_property().as_property()),
    );
    grid.children().add(presenter.register_in_name_scope(&**scope));

    let horizontal = ScrollBar::new();
    horizontal.set_name(Some("PART_HorizontalScrollBar".to_string()));
    horizontal.set_orientation(Orientation::Horizontal);
    horizontal.set_template(Some(FuncControlTemplate::for_type::<ScrollBar>(
        create_scroll_bar_template,
    )));
    horizontal.bind_binding(
        ScrollBar::visibility_property().as_property(),
        &TemplateBinding::new(ScrollViewer::horizontal_scroll_bar_visibility_property().as_property()),
    );
    Grid::set_row(&horizontal, 1);
    grid.children().add(horizontal.register_in_name_scope(&**scope));

    let vertical = ScrollBar::new();
    vertical.set_name(Some("PART_VerticalScrollBar".to_string()));
    vertical.set_orientation(Orientation::Vertical);
    vertical.set_template(Some(FuncControlTemplate::for_type::<ScrollBar>(
        create_scroll_bar_template,
    )));
    vertical.bind_binding(
        ScrollBar::visibility_property().as_property(),
        &TemplateBinding::new(ScrollViewer::vertical_scroll_bar_visibility_property().as_property()),
    );
    Grid::set_column(&vertical, 1);
    grid.children().add(vertical.register_in_name_scope(&**scope));

    grid.upcast()
}

fn templated_target() -> Ref<ScrollViewer> {
    let target = ScrollViewer::new();
    target.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(create_template)));
    target
}

fn initialize_scroll_viewer(target: &Ref<ScrollViewer>) {
    target.apply_template();

    let presenter = target.presenter().unwrap().cast::<ScrollContentPresenter>().unwrap();
    presenter.attach_to_scroll_viewer();
    presenter.update_child();
}

fn get_vertical_thumb(target: &Ref<ScrollViewer>) -> Ref<Thumb> {
    let scrollbar = target
        .get_template_descendants()
        .into_iter()
        .find(|x| x.name().as_deref() == Some("PART_VerticalScrollBar"))
        .and_then(|x| x.cast::<ScrollBar>())
        .expect("the vertical scroll bar");
    let track = scrollbar
        .get_template_descendants()
        .into_iter()
        .find(|x| x.name().as_deref() == Some("track"))
        .and_then(|x| x.cast::<Track>())
        .expect("the track");
    track.thumb().expect("the thumb")
}

fn get_root_point(control: &Visual, p: Point) -> Point {
    if let Some(root) = control.visual_root() {
        if let Some(m) = control.transform_to_visual(&root) {
            return p * m;
        }
    }

    panic!("Could not get the point in root coordinates.");
}

fn key_down(target: &Ref<ScrollViewer>, key: Key) {
    let mut e = KeyEventArgs::new();
    e.set_routed_event(Some(InputElement::key_down_event()));
    e.key = key;
    target.raise_event(&e);
}

fn scroll_viewer_with(extent: Size, viewport: Size, offset: Vector) -> Ref<ScrollViewer> {
    let target = ScrollViewer::new();
    target.set_extent(extent);
    target.set_viewport(viewport);
    target.set_offset(offset);
    target
}

#[test]
fn content_is_created() {
    let _scope = test_scope();
    use_test_text_block();

    let target = templated_target();
    target.set_content(boxed_str("Foo"));

    initialize_scroll_viewer(&target);

    assert!(target.presenter().unwrap().child().unwrap().is::<TestTextBlock>());
}

#[test]
fn offset_should_be_coerced_to_viewport() {
    let _scope = test_scope();
    let target = scroll_viewer_with(Size::new(20.0, 20.0), Size::new(10.0, 10.0), Vector::new(12.0, 12.0));

    assert_eq!(Vector::new(10.0, 10.0), target.offset());
}

#[test]
fn setting_offset_to_nan_does_not_cause_infinite_coerce_recursion() {
    let _scope = test_scope();
    use_test_text_block();

    let target = templated_target();
    target.set_content(boxed_str("Foo"));
    target.set_extent(Size::new(100.0, 100.0));
    target.set_viewport(Size::new(10.0, 10.0));

    initialize_scroll_viewer(&target);

    target.set_offset(Vector::new(0.0, f64::NAN));

    assert!(!target.offset().x.is_nan());
    assert!(!target.offset().y.is_nan());
}

#[test]
fn test_scroll_to_home() {
    let _scope = test_scope();
    let target = scroll_viewer_with(Size::new(50.0, 50.0), Size::new(10.0, 10.0), Vector::new(25.0, 25.0));
    target.scroll_to_home();

    assert_eq!(Vector::new(0.0, 0.0), target.offset());
}

#[test]
fn test_scroll_to_end() {
    let _scope = test_scope();
    let target = scroll_viewer_with(Size::new(50.0, 50.0), Size::new(10.0, 10.0), Vector::new(25.0, 25.0));
    target.scroll_to_end();

    assert_eq!(Vector::new(0.0, 40.0), target.offset());
}

#[test]
fn small_change_should_be_16() {
    let _scope = test_scope();
    let target = ScrollViewer::new();

    assert_eq!(Size::new(16.0, 16.0), target.small_change());
}

#[test]
fn large_change_should_be_viewport() {
    let _scope = test_scope();
    let target = ScrollViewer::new();
    target.set_viewport(Size::new(104.0, 143.0));

    assert_eq!(Size::new(104.0, 143.0), target.large_change());
}

#[test]
fn small_change_should_come_from_i_logical_scrollable_if_present() {
    let _scope = test_scope();
    let child = TestScrollable::new();
    child.set_scroll_size(Size::new(12.0, 43.0));
    child.set_page_scroll_size(Size::default());

    let target = templated_target();
    target.set_content(Some(Control::boxed(&child)));

    initialize_scroll_viewer(&target);

    assert_eq!(Size::new(12.0, 43.0), target.small_change());
}

#[test]
fn large_change_should_come_from_i_logical_scrollable_if_present() {
    let _scope = test_scope();
    let child = TestScrollable::new();
    child.set_scroll_size(Size::default());
    child.set_page_scroll_size(Size::new(45.0, 67.0));

    let target = templated_target();
    target.set_content(Some(Control::boxed(&child)));

    initialize_scroll_viewer(&target);

    assert_eq!(Size::new(45.0, 67.0), target.large_change());
}

#[test]
fn changing_extent_should_raise_scroll_changed() {
    let _scope = test_scope();
    let target = ScrollViewer::new();
    let root = TestRoot::with_child(&target);
    let raised = Rc::new(Cell::new(0));

    target.set_extent(Size::new(100.0, 100.0));
    target.set_viewport(Size::new(50.0, 50.0));
    target.set_offset(Vector::new(10.0, 10.0));

    root.layout_manager().execute_initial_layout_pass();

    let handler_raised = raised.clone();
    target.scroll_changed(move |_, e| {
        assert_eq!(Vector::new(11.0, 12.0), e.extent_delta());
        assert_eq!(Vector::default(), e.offset_delta());
        assert_eq!(Vector::default(), e.viewport_delta());
        handler_raised.set(handler_raised.get() + 1);
    });

    target.set_extent(Size::new(111.0, 112.0));

    assert_eq!(0, raised.get());

    root.layout_manager().execute_layout_pass();

    assert_eq!(1, raised.get());
}

#[test]
fn changing_offset_should_raise_scroll_changed() {
    let _scope = test_scope();
    let target = ScrollViewer::new();
    let root = TestRoot::with_child(&target);
    let raised = Rc::new(Cell::new(0));

    target.set_extent(Size::new(100.0, 100.0));
    target.set_viewport(Size::new(50.0, 50.0));
    target.set_offset(Vector::new(10.0, 10.0));

    root.layout_manager().execute_initial_layout_pass();

    let handler_raised = raised.clone();
    target.scroll_changed(move |_, e| {
        assert_eq!(Vector::default(), e.extent_delta());
        assert_eq!(Vector::new(12.0, 14.0), e.offset_delta());
        assert_eq!(Vector::default(), e.viewport_delta());
        handler_raised.set(handler_raised.get() + 1);
    });

    target.set_offset(Vector::new(22.0, 24.0));

    assert_eq!(0, raised.get());

    root.layout_manager().execute_layout_pass();

    assert_eq!(1, raised.get());
}

#[test]
fn changing_viewport_should_raise_scroll_changed() {
    let _scope = test_scope();
    let target = ScrollViewer::new();
    let root = TestRoot::with_child(&target);
    let raised = Rc::new(Cell::new(0));

    target.set_extent(Size::new(100.0, 100.0));
    target.set_viewport(Size::new(50.0, 50.0));
    target.set_offset(Vector::new(10.0, 10.0));

    root.layout_manager().execute_initial_layout_pass();

    let handler_raised = raised.clone();
    target.scroll_changed(move |_, e| {
        assert_eq!(Vector::default(), e.extent_delta());
        assert_eq!(Vector::default(), e.offset_delta());
        assert_eq!(Vector::new(6.0, 8.0), e.viewport_delta());
        handler_raised.set(handler_raised.get() + 1);
    });

    target.set_viewport(Size::new(56.0, 58.0));

    assert_eq!(0, raised.get());

    root.layout_manager().execute_layout_pass();

    assert_eq!(1, raised.get());
}

#[test]
fn reducing_extent_should_constrain_offset() {
    let _scope = test_scope();
    let target = templated_target();
    let root = TestRoot::with_child(&target);
    let raised = Rc::new(Cell::new(0));

    target.set_extent(Size::new(100.0, 100.0));
    target.set_viewport(Size::new(50.0, 50.0));
    target.set_offset(Vector::new(50.0, 50.0));

    root.layout_manager().execute_initial_layout_pass();

    let handler_raised = raised.clone();
    target.scroll_changed(move |_, e| {
        assert_eq!(Vector::new(-30.0, -30.0), e.extent_delta());
        assert_eq!(Vector::new(-30.0, -30.0), e.offset_delta());
        assert_eq!(Vector::default(), e.viewport_delta());
        handler_raised.set(handler_raised.get() + 1);
    });

    target.set_extent(Size::new(70.0, 70.0));

    assert_eq!(0, raised.get());

    root.layout_manager().execute_layout_pass();

    assert_eq!(1, raised.get());
    assert_eq!(Vector::new(20.0, 20.0), target.offset());
}

#[test]
fn padding_should_be_included_in_extent() {
    let _scope = test_scope();
    const ITEM_COUNT: usize = 19;
    const ITEM_HEIGHT: f64 = 32.0;
    const PADDING: f64 = 50.0;
    const VIEWPORT_HEIGHT: f64 = 200.0;

    let content = StackPanel::new();

    for _ in 0..ITEM_COUNT {
        let item = Border::new();
        item.set_height(ITEM_HEIGHT);
        content.children().add(item);
    }

    let target = templated_target();
    target.set_padding(Thickness::uniform(PADDING));
    target.set_height(VIEWPORT_HEIGHT);
    target.set_content(Some(Control::boxed(&content)));
    let root = TestRoot::with_child(&target);

    root.layout_manager().execute_initial_layout_pass();

    assert_eq!(VIEWPORT_HEIGHT, target.viewport().height);
    assert_eq!(ITEM_COUNT as f64 * ITEM_HEIGHT + 2.0 * PADDING, target.extent().height);

    // The content is arranged below the top padding and isn't squashed by it.
    assert_eq!(PADDING, content.bounds().y);
    assert_eq!(ITEM_COUNT as f64 * ITEM_HEIGHT, content.bounds().height);

    target.scroll_to_end();
    root.layout_manager().execute_layout_pass();

    assert_eq!(
        ITEM_COUNT as f64 * ITEM_HEIGHT + 2.0 * PADDING - VIEWPORT_HEIGHT,
        target.offset().y
    );

    // Once scrolled to the end, the last item is fully visible and the
    // bottom padding is still displayed below it.
    assert_eq!(VIEWPORT_HEIGHT - PADDING, content.bounds().bottom());
}

#[test]
fn padding_should_not_make_content_scrollable_when_scroll_viewer_is_sized_to_content() {
    let _scope = test_scope();
    let content = Border::new();
    content.set_width(100.0);
    content.set_height(100.0);

    let target = templated_target();
    target.set_padding(Thickness::uniform(50.0));
    target.set_horizontal_alignment(HorizontalAlignment::Left);
    target.set_vertical_alignment(VerticalAlignment::Top);
    target.set_content(Some(Control::boxed(&content)));
    let root = TestRoot::with_child(&target);

    root.layout_manager().execute_initial_layout_pass();

    // The padding is included in the desired size, so the content still
    // fits exactly.
    assert_eq!(Size::new(200.0, 200.0), target.bounds().size());
    assert_eq!(Size::new(200.0, 200.0), target.viewport());
    assert_eq!(Size::new(200.0, 200.0), target.extent());
}

#[test]
fn scroll_does_not_jump_when_viewport_becomes_smaller_while_dragging_scroll_bar_thumb() {
    let _scope = test_scope();
    let mouse = MouseTestHelper::new();
    let content = TestContent::new();
    content.measure_size.set(Size::new(1000.0, 10000.0));

    let target = templated_target();
    target.set_content(Some(Control::boxed(&content)));
    let root = TestRoot::with_child(&target);

    root.layout_manager().execute_initial_layout_pass();

    assert_eq!(Size::new(1000.0, 10000.0), target.extent());
    assert_eq!(Size::new(1000.0, 1000.0), target.viewport());

    // We're working in absolute coordinates (i.e. relative to the root) and
    // clicking on the center of the vertical thumb.
    let thumb = get_vertical_thumb(&target);
    let mut p = get_root_point(&thumb, thumb.bounds().center());

    // Press the mouse button in the center of the thumb.
    mouse.down_at(&thumb, MouseButton::Left, p, 1);
    root.layout_manager().execute_layout_pass();

    // Drag the thumb down 300 pixels.
    p += Vector::new(0.0, 300.0);
    mouse.move_(&thumb, p);
    root.layout_manager().execute_layout_pass();

    assert_eq!(Vector::new(0.0, 3000.0), target.offset());
    assert_eq!(300.0, thumb.bounds().y);

    // Now the extent changes from 10,000 to 5000.
    content.measure_size.set(content.measure_size.get() / 2.0);
    content.invalidate_measure();
    root.layout_manager().execute_layout_pass();

    // Due to the extent change, the thumb moves down but the value remains
    // the same.
    assert_eq!(600.0, thumb.bounds().y);
    assert_eq!(Vector::new(0.0, 3000.0), target.offset());

    // Drag the thumb down another 100 pixels.
    p += Vector::new(0.0, 100.0);
    mouse.move_(&thumb, p);
    root.layout_manager().execute_layout_pass();

    // The drag should not cause the offset/thumb to jump *up* to the current
    // absolute mouse position, i.e. it should move down in the direction of
    // the drag even if the absolute mouse position is now above the thumb.
    assert_eq!(700.0, thumb.bounds().y);
    assert_eq!(Vector::new(0.0, 3500.0), target.offset());
}

#[test]
fn thumb_does_not_become_detached_from_mouse_position_when_scrolling_past_the_start() {
    let _scope = test_scope();
    let mouse = MouseTestHelper::new();
    let content = TestContent::new();
    let target = templated_target();
    target.set_content(Some(Control::boxed(&content)));
    let root = TestRoot::with_child(&target);

    root.layout_manager().execute_initial_layout_pass();

    assert_eq!(Size::new(1000.0, 2000.0), target.extent());
    assert_eq!(Size::new(1000.0, 1000.0), target.viewport());

    // We're working in absolute coordinates (i.e. relative to the root) and
    // clicking on the center of the vertical thumb.
    let thumb = get_vertical_thumb(&target);
    let mut p = get_root_point(&thumb, thumb.bounds().center());

    // Press the mouse button in the center of the thumb.
    mouse.down_at(&thumb, MouseButton::Left, p, 1);
    root.layout_manager().execute_layout_pass();

    // Drag the thumb down 100 pixels.
    p += Vector::new(0.0, 100.0);
    mouse.move_(&thumb, p);
    root.layout_manager().execute_layout_pass();

    assert_eq!(Vector::new(0.0, 200.0), target.offset());
    assert_eq!(100.0, thumb.bounds().y);

    // Drag the thumb up 200 pixels - 100 pixels past the top of the
    // scrollbar.
    p -= Vector::new(0.0, 200.0);
    mouse.move_(&thumb, p);
    root.layout_manager().execute_layout_pass();

    assert_eq!(Vector::new(0.0, 0.0), target.offset());
    assert_eq!(0.0, thumb.bounds().y);

    // Drag the thumb back down 200 pixels.
    p += Vector::new(0.0, 200.0);
    mouse.move_(&thumb, p);
    root.layout_manager().execute_layout_pass();

    // We should now be back in the state after we first scrolled down 100
    // pixels.
    assert_eq!(Vector::new(0.0, 200.0), target.offset());
    assert_eq!(100.0, thumb.bounds().y);
}

#[test]
fn deferred_scrolling_defers_scrolling_until_pointer_up() {
    let _scope = test_scope();
    let mouse = MouseTestHelper::new();
    let content = TestContent::new();
    let target = templated_target();
    target.set_is_deferred_scrolling_enabled(true);
    target.set_content(Some(Control::boxed(&content)));
    let root = TestRoot::with_child(&target);

    root.layout_manager().execute_initial_layout_pass();

    // We're working in absolute coordinates (i.e. relative to the root) and
    // clicking on the center of the vertical thumb.
    let thumb = get_vertical_thumb(&target);
    let mut p = get_root_point(&thumb, thumb.bounds().center());

    assert_eq!(Vector::ZERO, target.offset());
    assert_eq!(0.0, thumb.bounds().y);

    // Press the mouse button in the center of the thumb.
    mouse.down_at(&thumb, MouseButton::Left, p, 1);
    root.layout_manager().execute_layout_pass();

    // Drag the thumb down 100 pixels.
    p += Vector::new(0.0, 100.0);
    mouse.move_(&thumb, p);
    root.layout_manager().execute_layout_pass();

    assert_eq!(Vector::ZERO, target.offset()); // no change to scroll...
    assert_eq!(100.0, thumb.bounds().y); // ...but the Thumb has moved

    // Release the mouse
    mouse.up_at(&thumb, MouseButton::Left, p);

    assert_eq!(Vector::new(0.0, 200.0), target.offset());
    assert_eq!(100.0, thumb.bounds().y);
}

fn stack_panel_with_two_buttons() -> Ref<StackPanel> {
    let content = StackPanel::new();
    for _ in 0..2 {
        let button = Button::new();
        button.set_width(100.0);
        button.set_height(900.0);
        content.children().add(button);
    }
    content
}

#[test]
fn bring_into_view_on_focus_change_scrolls_child_control_into_view_when_focused() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let content = stack_panel_with_two_buttons();

    let target = templated_target();
    target.set_content(Some(Control::boxed(&content)));
    let root = TestRoot::with_child(&target);

    root.layout_manager().execute_initial_layout_pass();

    let button = content.children().get(1).cast::<Button>().unwrap();

    button.focus();

    assert_eq!(Vector::new(0.0, 800.0), target.offset());
}

/// The reference test runs without focus services and leaves the property
/// at its default, so that nothing is focused at all; here focus works and
/// the property is set to false, which is what the test name describes.
#[test]
fn bring_into_view_on_focus_change_false_does_not_scroll_child_control_into_view_when_focused() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let content = stack_panel_with_two_buttons();

    let target = templated_target();
    target.set_bring_into_view_on_focus_change(false);
    target.set_content(Some(Control::boxed(&content)));
    let root = TestRoot::with_child(&target);

    root.layout_manager().execute_initial_layout_pass();

    let button = content.children().get(1).cast::<Button>().unwrap();

    assert!(button.focus());

    assert_eq!(Vector::new(0.0, 0.0), target.offset());
}

#[test]
fn menu_scroll_bar_should_be_visible_when_specified_visible() {
    use ferroui_base::data::converters::IMultiValueConverter;
    use ferroui_base::data::core::ValueType;
    use ferroui_base::BoxedValue;

    let converter = crate::converters::MenuScrollingVisibilityConverter::instance();
    let args: Vec<Option<BoxedValue>> = vec![
        Some(Rc::new(ScrollBarVisibility::Visible)),
        Some(Rc::new(400.0f64)),
        Some(Rc::new(1800.0f64)),
        Some(Rc::new(500.0f64)),
    ];
    let parameter: BoxedValue = Rc::new(String::from("0"));
    let result = converter
        .convert(&args, ValueType::of::<ScrollBarVisibility>(), Some(&parameter), &ferroui_base::utilities::CultureInfo::current_culture())
        .unwrap();
    assert_eq!(result.and_then(|result| result.downcast_ref::<bool>().copied()), Some(true));
}

#[test]
fn scroll_bar_visibility_should_invalidate_measure_and_arrange() {
    let _scope = test_scope();
    let panel = TestPanel::new(100_000.0);

    let target = templated_target();
    target.set_content(Some(Control::boxed(&panel)));
    target.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto);
    let root = TestRoot::with_child(&target);
    root.layout_manager().execute_initial_layout_pass();

    panel.reset();
    target.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    root.layout_manager().execute_layout_pass();

    assert_eq!(1, panel.measure_override_calls.get());
    assert_eq!(1, panel.arrange_override_calls.get());
}

#[test]
fn focus_key_input_should_scroll() {
    let _scope = test_scope();
    let panel = Panel::new();
    panel.set_width(100_000.0);
    panel.set_height(100_000.0);

    let target = templated_target();
    target.set_content(Some(Control::boxed(&panel)));
    let root = TestRoot::with_child(&target);
    root.layout_manager().execute_initial_layout_pass();

    // Page down and page up
    key_down(&target, Key::PageDown);
    assert_eq!(Vector::new(0.0, target.viewport().height), target.offset());
    key_down(&target, Key::PageUp);
    assert_eq!(Vector::new(0.0, 0.0), target.offset());

    // Per-line scrolling in all directions with arrow keys
    key_down(&target, Key::Down);
    assert_eq!(Vector::new(0.0, target.small_change().height), target.offset());
    key_down(&target, Key::Right);
    assert_eq!(
        Vector::new(target.small_change().width, target.small_change().height),
        target.offset()
    );
    key_down(&target, Key::Up);
    assert_eq!(Vector::new(ScrollViewer::DEFAULT_SMALL_CHANGE, 0.0), target.offset());
    key_down(&target, Key::Left);
    assert_eq!(Vector::new(0.0, 0.0), target.offset());

    // Scrolling horizontally with a right-to-left flow direction
    target.set_flow_direction(FlowDirection::RightToLeft);
    key_down(&target, Key::Left);
    assert_eq!(Vector::new(target.small_change().width, 0.0), target.offset());
    key_down(&target, Key::Right);
    assert_eq!(Vector::new(0.0, 0.0), target.offset());
}

#[test]
fn presenter_preserves_owner_bindings_when_scroll_viewer_is_reattached() {
    use crate::testing::{TestServices, UnitTestApplication};
    use crate::Window;
    use ferroui_base::layout::ILayoutManager;

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let content = Border::new();
    content.set_width(200.0);
    content.set_height(200.0);
    // The reference runs with the simple theme; the test theme of this crate
    // has no theme for a scroll viewer, so it gets the template of these tests.
    let target = templated_target();
    target.set_content(Some(Control::boxed(content.clone())));
    let window = Window::new();
    window.set_content(Some(Control::boxed(target.clone())));
    window.set_width(100.0);
    window.set_height(100.0);
    window.show();
    window.layout_manager().execute_initial_layout_pass();
    let presenter = target.presenter().and_then(|presenter| presenter.cast::<ScrollContentPresenter>());
    let presenter = presenter.expect("a scroll content presenter");
    assert!(presenter.child().is_some_and(|child| child.ptr_eq(&content)));

    window.set_content(None);
    assert!(presenter.child().is_some_and(|child| child.ptr_eq(&content)));
    let replacement = Border::new();
    replacement.set_width(300.0);
    replacement.set_height(300.0);
    target.set_content(Some(Control::boxed(replacement.clone())));
    assert!(presenter.child().is_none());

    window.set_content(Some(Control::boxed(target.clone())));
    window.layout_manager().execute_layout_pass();
    assert!(target.presenter().is_some_and(|current| current.ptr_eq(&presenter)));
    assert!(presenter.child().is_some_and(|child| child.ptr_eq(&replacement)));
    target.set_offset(Vector::new(0.0, 20.0));
    assert_eq!(target.offset(), presenter.offset());
    assert_eq!(20.0, presenter.offset().y);
    window.close();
}

