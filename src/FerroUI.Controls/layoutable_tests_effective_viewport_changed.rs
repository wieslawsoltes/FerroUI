//! Port of `LayoutableTests_EffectiveViewportChanged.cs` (base unit tests).
//! The tests need controls, so they live with the controls.
//!
//! The upstream tests are asynchronous only through `RunOnUIThread.Execute`
//! and completed tasks; they run synchronously here.

use crate::presenters::ScrollContentPresenter;
use crate::primitives::{ScrollBar, ScrollBarVisibility};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{test_scope, TestRoot};
use crate::{
    Border, Canvas, ColumnDefinition, CanvasImpl, ColumnDefinitions, Control, ControlImpl, Grid, GridLength, GridUnitType,
    PanelImpl, RowDefinition, RowDefinitions, ScrollViewer,
};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{EffectiveViewportChangedEventArgs, LayoutableImpl, LayoutableImplExt, Orientation};
use ferroui_base::media::{ITransform, RotateTransform, ScaleTransform, TranslateTransform};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Rect, Ref, RelativePoint, RelativeUnit, Size,
    StyledElementImpl, Thickness, Vector, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn effective_viewport_changed_not_raised_when_control_added_to_tree_and_layout_pass_has_not_run() {
    let _scope = test_scope();
    let root = create_root();
    let target = Canvas::new();
    let raised = Rc::new(Cell::new(0));

    let _token = target.effective_viewport_changed({
        let raised = raised.clone();
        move |_| raised.set(raised.get() + 1)
    });

    root.set_child(&target);

    assert_eq!(0, raised.get());
}

#[test]
fn effective_viewport_changed_raised_when_control_added_to_tree_and_layout_pass_has_run() {
    let _scope = test_scope();
    let root = create_root();
    let target = Canvas::new();
    let raised = Rc::new(Cell::new(0));

    let _token = target.effective_viewport_changed({
        let raised = raised.clone();
        move |_| raised.set(raised.get() + 1)
    });

    root.set_child(&target);

    assert_eq!(0, raised.get());

    execute_initial_layout_pass(&root);

    assert_eq!(1, raised.get());
}

#[test]
fn effective_viewport_changed_raised_when_root_layed_out_and_then_control_added_to_tree_and_layout_pass_runs() {
    let _scope = test_scope();
    let root = create_root();
    let target = Canvas::new();
    let raised = Rc::new(Cell::new(0));

    let _token = target.effective_viewport_changed({
        let raised = raised.clone();
        move |_| raised.set(raised.get() + 1)
    });

    execute_initial_layout_pass(&root);

    root.set_child(&target);

    assert_eq!(0, raised.get());

    execute_initial_layout_pass(&root);

    assert_eq!(1, raised.get());
}

#[test]
fn effective_viewport_changed_raised_before_layout_updated() {
    let _scope = test_scope();
    let root = create_root();
    let target = Canvas::new();
    let raised = Rc::new(Cell::new(0));

    let _token = target.effective_viewport_changed({
        let raised = raised.clone();
        move |_| raised.set(raised.get() + 1)
    });

    root.set_child(&target);

    execute_initial_layout_pass(&root);

    assert_eq!(1, raised.get());
}

#[test]
fn effective_viewport_changed_should_not_be_raised_twice_if_subcribed_in_attached_to_visual_tree() {
    let _scope = test_scope();
    let root = create_root();
    let target = Canvas::new();
    let raised = Rc::new(Cell::new(0));
    let tokens = Rc::new(RefCell::new(Vec::<Rc<dyn IDisposable>>::new()));

    let _attached = target.attached_to_visual_tree({
        let target = target.downgrade();
        let raised = raised.clone();
        let tokens = tokens.clone();
        move |_| {
            let raised = raised.clone();
            let token = target.upgrade().unwrap().effective_viewport_changed(move |_| raised.set(raised.get() + 1));
            tokens.borrow_mut().push(token);
        }
    });

    root.set_child(&target);

    execute_initial_layout_pass(&root);

    assert_eq!(1, raised.get());
}

#[test]
fn parent_affects_effective_viewport() {
    let _scope = test_scope();
    let root = create_root();
    let target = sized_canvas();
    let parent = sized_border(200.0, &target);
    let raised = Rc::new(Cell::new(0));

    root.set_child(&parent);

    let _token = target.effective_viewport_changed({
        let raised = raised.clone();
        move |e| {
            assert_eq!(Rect::new(-550.0, -400.0, 1200.0, 900.0), e.effective_viewport());
            raised.set(raised.get() + 1);
        }
    });

    execute_initial_layout_pass(&root);
}

#[test]
fn invalidating_in_handler_causes_layout_to_be_rerun_before_layout_updated_raised() {
    let _scope = test_scope();
    let root = create_root();
    let target = TestCanvas::new();
    let raised = Rc::new(Cell::new(0));
    let layout_updated_raised = Rc::new(Cell::new(0));

    let _layout_updated = root.layout_updated({
        let target = target.clone();
        let layout_updated_raised = layout_updated_raised.clone();
        move || {
            assert_eq!(2, target.measure_count.get());
            assert_eq!(2, target.arrange_count.get());
            layout_updated_raised.set(layout_updated_raised.get() + 1);
        }
    });

    let _token = target.effective_viewport_changed({
        let weak = target.downgrade();
        let raised = raised.clone();
        move |_| {
            weak.upgrade().unwrap().invalidate_measure();
            raised.set(raised.get() + 1);
        }
    });

    root.set_child(&target);

    execute_initial_layout_pass(&root);

    assert_eq!(1, raised.get());
    assert_eq!(1, layout_updated_raised.get());
}

#[test]
fn viewport_extends_beyond_centered_control() {
    let _scope = test_scope();
    let root = create_root();
    let target = Canvas::new();
    target.set_width(52.0);
    target.set_height(52.0);
    let raised = Rc::new(Cell::new(0));

    let _token = target.effective_viewport_changed({
        let raised = raised.clone();
        move |e| {
            assert_eq!(Rect::new(-574.0, -424.0, 1200.0, 900.0), e.effective_viewport());
            raised.set(raised.get() + 1);
        }
    });

    root.set_child(&target);

    execute_initial_layout_pass(&root);
    assert_eq!(1, raised.get());
}

#[test]
fn viewport_extends_beyond_nested_centered_control() {
    let _scope = test_scope();
    let root = create_root();
    let target = Canvas::new();
    target.set_width(52.0);
    target.set_height(52.0);
    let parent = sized_border(100.0, &target);
    let raised = Rc::new(Cell::new(0));

    let _token = target.effective_viewport_changed({
        let raised = raised.clone();
        move |e| {
            assert_eq!(Rect::new(-574.0, -424.0, 1200.0, 900.0), e.effective_viewport());
            raised.set(raised.get() + 1);
        }
    });

    root.set_child(&parent);

    execute_initial_layout_pass(&root);
    assert_eq!(1, raised.get());
}

/// `new ScrollViewer { Width = 100, Height = 100, Content = target, Template = ScrollViewerTemplate(),
/// HorizontalScrollBarVisibility = ScrollBarVisibility.Hidden }`.
fn create_scroller(target: &Ref<Canvas>) -> Ref<ScrollViewer> {
    let scroller = ScrollViewer::new();
    scroller.set_width(100.0);
    scroller.set_height(100.0);
    scroller.set_content(Some(Control::boxed(target)));
    scroller.set_template(Some(scroll_viewer_template()));
    scroller.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden);
    scroller
}

#[test]
fn scroll_viewer_determines_effective_viewport() {
    let _scope = test_scope();
    let root = create_root();
    let target = Canvas::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let scroller = create_scroller(&target);
    let raised = Rc::new(Cell::new(0));

    let _token = target.effective_viewport_changed({
        let raised = raised.clone();
        move |e| {
            assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), e.effective_viewport());
            raised.set(raised.get() + 1);
        }
    });

    root.set_child(&scroller);

    execute_initial_layout_pass(&root);
    assert_eq!(1, raised.get());
}

#[test]
fn scrolled_scroll_viewer_determines_effective_viewport() {
    let _scope = test_scope();
    let root = create_root();
    let target = Canvas::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let scroller = create_scroller(&target);
    let raised = Rc::new(Cell::new(0));

    root.set_child(&scroller);

    execute_initial_layout_pass(&root);
    scroller.set_offset(Vector::new(0.0, 10.0));

    let _token = execute_scroller_layout_pass(&root, &scroller, &target, {
        let raised = raised.clone();
        move |e| {
            assert_eq!(Rect::new(0.0, 10.0, 100.0, 100.0), e.effective_viewport());
            raised.set(raised.get() + 1);
        }
    });

    assert_eq!(1, raised.get());
}

#[test]
fn moving_parent_updates_effective_viewport() {
    let _scope = test_scope();
    let root = create_root();
    let target = sized_canvas();
    let parent = sized_border(200.0, &target);
    let raised = Rc::new(Cell::new(0));

    root.set_child(&parent);

    execute_initial_layout_pass(&root);

    let _token = target.effective_viewport_changed({
        let raised = raised.clone();
        move |e| {
            assert_eq!(Rect::new(-554.0, -400.0, 1200.0, 900.0), e.effective_viewport());
            raised.set(raised.get() + 1);
        }
    });

    parent.set_margin(Thickness::new(8.0, 0.0, 0.0, 0.0));
    execute_layout_pass(&root);

    assert_eq!(1, raised.get());
}

fn execute_initial_layout_pass(root: &TestRoot) {
    root.layout_manager().execute_initial_layout_pass();
}

fn execute_layout_pass(root: &TestRoot) {
    root.layout_manager().execute_layout_pass();
}

/// Subscribes `handler` to the effective viewport changes of `target` and
/// runs a layout pass. The returned token keeps the handler subscribed, as
/// the upstream helper leaves it subscribed.
fn execute_scroller_layout_pass(
    root: &TestRoot,
    _scroller: &ScrollViewer,
    target: &Control,
    handler: impl Fn(&EffectiveViewportChangedEventArgs) + 'static,
) -> Rc<dyn IDisposable> {
    let token = target.effective_viewport_changed(handler);
    root.layout_manager().execute_layout_pass();
    token
}

fn scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_control: &Ref<ScrollViewer>, scope: &NameScopeRef| {
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
        grid.children().add(presenter.register_in_name_scope(&**scope));

        let horizontal = ScrollBar::new();
        horizontal.set_name(Some("horizontalScrollBar".to_string()));
        horizontal.set_orientation(Orientation::Horizontal);
        Grid::set_row(&horizontal, 1);
        grid.children().add(horizontal.register_in_name_scope(&**scope));

        let vertical = ScrollBar::new();
        vertical.set_name(Some("verticalScrollBar".to_string()));
        vertical.set_orientation(Orientation::Vertical);
        Grid::set_column(&vertical, 1);
        grid.children().add(vertical.register_in_name_scope(&**scope));

        grid.upcast()
    })
}

/// A canvas that counts its measures and arranges.
#[repr(C)]
struct TestCanvas {
    base: Canvas,
    measure_count: Cell<i32>,
    arrange_count: Cell<i32>,
}

ferro_class!(TestCanvas: Canvas);
ferro_impl_classes!(
    TestCanvas: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    CanvasImpl
);

impl LayoutableImpl for TestCanvas {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.measure_count.set(this.measure_count.get() + 1);
        Self::parent_measure_override(this, available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        this.arrange_count.set(this.arrange_count.get() + 1);
        Self::parent_arrange_override(this, final_size)
    }
}

impl TestCanvas {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Canvas::construct(), measure_count: Cell::new(0), arrange_count: Cell::new(0) })
    }
}

fn create_root() -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.set_width(1200.0);
    root.set_height(900.0);
    root
}

fn sized_canvas() -> Ref<Canvas> {
    let target = Canvas::new();
    target.set_width(100.0);
    target.set_height(100.0);
    target
}

fn sized_border(size: f64, child: &Ref<Canvas>) -> Ref<Border> {
    let parent = Border::new();
    parent.set_width(size);
    parent.set_height(size);
    parent.set_child(child);
    parent
}

fn transform<T: ferroui_base::ObjectType>(value: Ref<T>) -> Option<Rc<dyn ITransform>>
where
    Ref<T>: Into<Rc<dyn ITransform>>,
{
    Some(value.into())
}

#[test]
fn translate_transform_doesnt_affect_effective_viewport() {
    let _scope = test_scope();
    let root = create_root();
    let target = sized_canvas();
    let parent = sized_border(200.0, &target);
    let raised = Rc::new(Cell::new(0));

    root.set_child(&parent);

    let _token = target.effective_viewport_changed({
        let raised = raised.clone();
        move |_| raised.set(raised.get() + 1)
    });
    root.execute_initial_layout_pass();

    raised.set(0); // The initial layout pass is expected to raise.

    target.set_render_transform(transform(TranslateTransform::with_offset(8.0, 0.0)));
    target.invalidate_measure();
    root.layout_manager().execute_layout_pass();

    assert_eq!(raised.get(), 0);
}

#[test]
fn translate_transform_on_parent_affects_effective_viewport() {
    let _scope = test_scope();
    let root = create_root();
    let target = sized_canvas();
    let parent = sized_border(200.0, &target);
    let raised = Rc::new(Cell::new(0));

    root.set_child(&parent);
    root.execute_initial_layout_pass();

    let _token = target.effective_viewport_changed({
        let raised = raised.clone();
        move |e| {
            assert_eq!(e.effective_viewport(), Rect::new(-558.0, -400.0, 1200.0, 900.0));
            raised.set(raised.get() + 1);
        }
    });

    // Change the parent render transform to move it. A layout is then needed
    // before the effective viewport change is raised.
    parent.set_render_transform(transform(TranslateTransform::with_offset(8.0, 0.0)));
    parent.invalidate_measure();
    root.layout_manager().execute_layout_pass();

    assert_eq!(raised.get(), 1);
}

#[test]
fn rotate_transform_on_parent_affects_effective_viewport() {
    let _scope = test_scope();
    let root = create_root();
    let target = sized_canvas();
    let parent = sized_border(200.0, &target);
    let raised = Rc::new(Cell::new(0));

    root.set_child(&parent);
    root.execute_initial_layout_pass();

    let _token = target.effective_viewport_changed({
        let raised = raised.clone();
        move |e| {
            // Pixel equality: the values are compared after truncation.
            let actual = e.effective_viewport();
            assert_eq!(
                (actual.x as i32, actual.y as i32, actual.width as i32, actual.height as i32),
                (-651, -792, 1484, 1484)
            );
            raised.set(raised.get() + 1);
        }
    });

    parent.set_render_transform_origin(RelativePoint::new(0.0, 0.0, RelativeUnit::Absolute));
    parent.set_render_transform(transform(RotateTransform::with_angle(45.0)));
    parent.invalidate_measure();
    root.layout_manager().execute_layout_pass();

    assert_eq!(raised.get(), 1);
}

#[test]
fn event_unsubscribed_while_inside_callback() {
    let _scope = test_scope();
    let root = create_root();
    let target = Canvas::new();
    let raised = Rc::new(Cell::new(0));
    let token = Rc::new(RefCell::new(None::<Rc<dyn IDisposable>>));

    *token.borrow_mut() = Some(target.effective_viewport_changed({
        let raised = raised.clone();
        let token = token.clone();
        move |_| {
            if let Some(token) = token.borrow_mut().take() {
                token.dispose();
            }
            raised.set(raised.get() + 1);
        }
    }));

    root.set_child(&target);

    execute_initial_layout_pass(&root);

    assert_eq!(1, raised.get());
}

#[test]
fn zero_scale_transform_sets_empty_effective_viewport() {
    let _scope = test_scope();
    let effective_viewport = Rc::new(Cell::new(Rect::from_size(Size::INFINITY)));

    let root = create_root();
    let target = sized_canvas();
    let parent = sized_border(100.0, &target);

    let _token = target.effective_viewport_changed({
        let effective_viewport = effective_viewport.clone();
        move |e| effective_viewport.set(e.effective_viewport())
    });

    root.set_child(&parent);
    root.execute_initial_layout_pass();

    parent.set_render_transform(transform(ScaleTransform::with_scale(0.0, 0.0)));
    root.layout_manager().execute_layout_pass();

    assert_eq!(effective_viewport.get(), Rect::new(0.0, 0.0, 0.0, 0.0));
}
