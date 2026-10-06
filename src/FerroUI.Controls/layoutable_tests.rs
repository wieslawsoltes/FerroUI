//! Port of `LayoutableTests.cs` (base unit tests). Most tests use
//! decorators, panels, borders and the test root, so they live with the
//! controls.
//!
//! The mock layout manager of the upstream tests (`Mock<ILayoutManager>`,
//! whose members do nothing) maps to [`MockLayoutManager`], which records the
//! calls the tests verify.

use crate::layout_test_control::LayoutTestRoot;
use crate::test_support::{test_scope, TestRoot};
use crate::testing::{TestServices, UnitTestApplication};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use crate::{Border, Decorator, StackPanel, TextBlock};
use ferroui_base::layout::{
    HorizontalAlignment, ILayoutManager, Layoutable, LayoutableImpl, LayoutableImplExt, VerticalAlignment,
};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Rect, Ref, Size, StyledElementImpl,
    StyledProperty, Thickness, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// Declares one test per data row of a parameterized test.
macro_rules! theory {
    ($func:ident: $($name:ident($($arg:expr),* $(,)?));+ $(;)?) => {
        $(
            #[test]
            fn $name() {
                $func($($arg),*)
            }
        )+
    };
}

/// A layout manager whose members do nothing and record the calls.
#[derive(Default)]
struct MockLayoutManager {
    invalidate_measure_calls: RefCell<Vec<Ref<Layoutable>>>,
    invalidate_arrange_calls: RefCell<Vec<Ref<Layoutable>>>,
    layout_updated_adds: Cell<i32>,
    layout_updated_removes: Cell<i32>,
}

impl MockLayoutManager {
    /// C# `target.Invocations.Clear()`.
    fn clear_invocations(&self) {
        self.invalidate_measure_calls.borrow_mut().clear();
        self.invalidate_arrange_calls.borrow_mut().clear();
        self.layout_updated_adds.set(0);
        self.layout_updated_removes.set(0);
    }

    fn invalidate_measure_count(&self, control: &Layoutable) -> usize {
        self.invalidate_measure_calls.borrow().iter().filter(|x| std::ptr::eq(&***x, control)).count()
    }

    fn invalidate_arrange_count(&self, control: &Layoutable) -> usize {
        self.invalidate_arrange_calls.borrow().iter().filter(|x| std::ptr::eq(&***x, control)).count()
    }
}

impl ILayoutManager for MockLayoutManager {
    fn add_layout_updated(&self, _handler: Rc<dyn Fn()>) -> u64 {
        self.layout_updated_adds.set(self.layout_updated_adds.get() + 1);
        0
    }

    fn remove_layout_updated(&self, _token: u64) {
        self.layout_updated_removes.set(self.layout_updated_removes.get() + 1);
    }

    fn invalidate_measure(&self, control: &Layoutable) {
        self.invalidate_measure_calls.borrow_mut().push(control.to_ref());
    }

    fn invalidate_arrange(&self, control: &Layoutable) {
        self.invalidate_arrange_calls.borrow_mut().push(control.to_ref());
    }

    fn execute_layout_pass(&self) {}

    fn execute_initial_layout_pass(&self) {}

    fn register_effective_viewport_listener(&self, _control: &Layoutable) {}

    fn unregister_effective_viewport_listener(&self, _control: &Layoutable) {}

    fn dispose(&self) {}
}

fn margin_is_applied_to_measure_override_size(
    l: f64,
    t: f64,
    r: f64,
    b: f64,
    expected_width: f64,
    expected_height: f64,
) {
    let target = TestLayoutable::new();
    target.set_margin(Thickness::new(l, t, r, b));

    target.measure(Size::new(100.0, 100.0));

    assert_eq!(Size::new(expected_width, expected_height), target.measure_size.get());
}

theory!(margin_is_applied_to_measure_override_size:
    margin_is_applied_to_measure_override_size_1(0.0, 0.0, 0.0, 0.0, 100.0, 100.0);
    margin_is_applied_to_measure_override_size_2(10.0, 0.0, 0.0, 0.0, 90.0, 100.0);
    margin_is_applied_to_measure_override_size_3(10.0, 0.0, 5.0, 0.0, 85.0, 100.0);
    margin_is_applied_to_measure_override_size_4(0.0, 10.0, 0.0, 0.0, 100.0, 90.0);
    margin_is_applied_to_measure_override_size_5(0.0, 10.0, 0.0, 5.0, 100.0, 85.0);
    margin_is_applied_to_measure_override_size_6(4.0, 4.0, 6.0, 7.0, 90.0, 89.0));

fn horizontal_alignment_is_applied_to_arrange_override_size(h: HorizontalAlignment, expected_width: f64) {
    let target = TestLayoutable::new();
    target.set_horizontal_alignment(h);

    target.measure(Size::INFINITY);
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(Size::new(expected_width, 100.0), target.arrange_size.get());
}

theory!(horizontal_alignment_is_applied_to_arrange_override_size:
    horizontal_alignment_is_applied_to_arrange_override_size_1(HorizontalAlignment::Stretch, 100.0);
    horizontal_alignment_is_applied_to_arrange_override_size_2(HorizontalAlignment::Left, 10.0);
    horizontal_alignment_is_applied_to_arrange_override_size_3(HorizontalAlignment::Center, 10.0);
    horizontal_alignment_is_applied_to_arrange_override_size_4(HorizontalAlignment::Right, 10.0));

fn vertical_alignment_is_applied_to_arrange_override_size(v: VerticalAlignment, expected_height: f64) {
    let target = TestLayoutable::new();
    target.set_vertical_alignment(v);

    target.measure(Size::INFINITY);
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(Size::new(100.0, expected_height), target.arrange_size.get());
}

theory!(vertical_alignment_is_applied_to_arrange_override_size:
    vertical_alignment_is_applied_to_arrange_override_size_1(VerticalAlignment::Stretch, 100.0);
    vertical_alignment_is_applied_to_arrange_override_size_2(VerticalAlignment::Top, 10.0);
    vertical_alignment_is_applied_to_arrange_override_size_3(VerticalAlignment::Center, 10.0);
    vertical_alignment_is_applied_to_arrange_override_size_4(VerticalAlignment::Bottom, 10.0));

fn margin_is_applied_to_arrange_override_size(
    l: f64,
    t: f64,
    r: f64,
    b: f64,
    expected_width: f64,
    expected_height: f64,
) {
    let target = TestLayoutable::new();
    target.set_margin(Thickness::new(l, t, r, b));

    target.measure(Size::INFINITY);
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(Size::new(expected_width, expected_height), target.arrange_size.get());
}

theory!(margin_is_applied_to_arrange_override_size:
    margin_is_applied_to_arrange_override_size_1(0.0, 0.0, 0.0, 0.0, 100.0, 100.0);
    margin_is_applied_to_arrange_override_size_2(10.0, 0.0, 0.0, 0.0, 90.0, 100.0);
    margin_is_applied_to_arrange_override_size_3(10.0, 0.0, 5.0, 0.0, 85.0, 100.0);
    margin_is_applied_to_arrange_override_size_4(0.0, 10.0, 0.0, 0.0, 100.0, 90.0);
    margin_is_applied_to_arrange_override_size_5(0.0, 10.0, 0.0, 5.0, 100.0, 85.0);
    margin_is_applied_to_arrange_override_size_6(4.0, 4.0, 6.0, 7.0, 90.0, 89.0));

/// `new LayoutTestRoot { Child = control, LayoutManager = target }`.
fn layout_test_root_with_mock(control: &Ref<Decorator>, target: &Rc<MockLayoutManager>) -> Ref<LayoutTestRoot> {
    let root = LayoutTestRoot::new();
    root.set_child(control);
    root.set_layout_manager(target.clone());
    root
}

#[test]
fn only_calls_layout_manager_invalidate_measure_once() {
    let _scope = test_scope();
    let target = Rc::new(MockLayoutManager::default());
    let control = Decorator::new();
    let root = layout_test_root_with_mock(&control, &target);

    root.measure(Size::INFINITY);
    root.arrange(Rect::from_size(root.desired_size()));
    target.clear_invocations();

    control.invalidate_measure();
    control.invalidate_measure();

    assert_eq!(1, target.invalidate_measure_count(&control));
}

#[test]
fn only_calls_layout_manager_invalidate_arrange_once() {
    let _scope = test_scope();
    let target = Rc::new(MockLayoutManager::default());
    let control = Decorator::new();
    let root = layout_test_root_with_mock(&control, &target);

    root.measure(Size::INFINITY);
    root.arrange(Rect::from_size(root.desired_size()));
    target.clear_invocations();

    control.invalidate_arrange();
    control.invalidate_arrange();

    assert_eq!(1, target.invalidate_arrange_count(&control));
}

#[test]
fn attaching_control_to_tree_invalidates_parent_measure() {
    let _scope = test_scope();
    let target = Rc::new(MockLayoutManager::default());
    let control = Decorator::new();
    let root = layout_test_root_with_mock(&control, &target);

    root.measure(Size::INFINITY);
    root.arrange(Rect::from_size(root.desired_size()));
    assert!(control.is_measure_valid());

    root.set_child(None);
    root.measure(Size::INFINITY);
    root.arrange(Rect::from_size(root.desired_size()));

    assert!(!control.is_measure_valid());
    assert!(root.is_measure_valid());

    target.clear_invocations();

    root.set_child(&control);

    assert!(!root.is_measure_valid());
    assert!(!control.is_measure_valid());
    assert_eq!(1, target.invalidate_measure_count(&root));
}

#[test]
fn layout_updated_is_called_at_end_of_layout_pass() {
    let _scope = test_scope();
    let border2 = Border::new();
    let border1 = Border::new();
    border1.set_child(&border2);
    let root = TestRoot::new();
    root.set_child(&border1);
    let raised = Rc::new(Cell::new(0));

    let validate_bounds = {
        let border1 = border1.clone();
        let border2 = border2.clone();
        let raised = raised.clone();
        move || {
            assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), border1.bounds());
            assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), border2.bounds());
            raised.set(raised.get() + 1);
        }
    };

    let _s1 = root.layout_updated(validate_bounds.clone());
    let _s2 = border1.layout_updated(validate_bounds.clone());
    let _s3 = border2.layout_updated(validate_bounds);

    root.measure(Size::new(100.0, 100.0));
    root.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    root.layout_manager().execute_layout_pass();

    assert_eq!(3, raised.get());
    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), border1.bounds());
    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), border2.bounds());
}

/// `new TestRoot { Child = child, LayoutManager = layoutManager }`.
fn test_root_with_mock(child: &Ref<Border>, layout_manager: &Rc<MockLayoutManager>) -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.set_child(child);
    root.set_layout_manager(layout_manager.clone());
    root
}

#[test]
fn layout_updated_subscribes_to_layout_manager() {
    let _scope = test_scope();
    let target = Border::new();
    let child = Border::new();
    child.set_child(&target);
    let layout_manager = Rc::new(MockLayoutManager::default());

    let _root = test_root_with_mock(&child, &layout_manager);

    layout_manager.clear_invocations();
    let handler = target.layout_updated(|| {});

    assert_eq!(1, layout_manager.layout_updated_adds.get());

    layout_manager.clear_invocations();
    handler.dispose();

    assert_eq!(1, layout_manager.layout_updated_removes.get());
}

#[test]
fn layout_manager_layout_updated_is_subscribed_when_attached_to_tree() {
    let _scope = test_scope();
    let border1 = Border::new();
    let layout_manager = Rc::new(MockLayoutManager::default());

    let _root = test_root_with_mock(&border1, &layout_manager);

    let border2 = Border::new();
    let _subscription = border2.layout_updated(|| {});

    layout_manager.clear_invocations();
    border1.set_child(&border2);

    assert_eq!(1, layout_manager.layout_updated_adds.get());
}

#[test]
fn layout_manager_layout_updated_is_unsubscribed_when_detached_from_tree() {
    let _scope = test_scope();
    let border1 = Border::new();
    let layout_manager = Rc::new(MockLayoutManager::default());

    let _root = test_root_with_mock(&border1, &layout_manager);

    let border2 = Border::new();
    let _subscription = border2.layout_updated(|| {});
    border1.set_child(&border2);

    layout_manager.clear_invocations();
    border1.set_child(None);

    assert_eq!(1, layout_manager.layout_updated_removes.get());
}

#[test]
fn layout_manager_layout_updated_should_not_be_subscribed_twice_in_attached_to_visual_tree() {
    let _scope = test_scope();
    let border1 = Border::new();
    let layout_manager = Rc::new(MockLayoutManager::default());

    let _root = test_root_with_mock(&border1, &layout_manager);

    let border2 = Border::new();
    let subscriptions = Rc::new(RefCell::new(Vec::new()));
    let _attached = border2.attached_to_visual_tree({
        let border2 = border2.downgrade();
        let subscriptions = subscriptions.clone();
        move |_| {
            let border2 = border2.upgrade().unwrap();
            subscriptions.borrow_mut().push(border2.layout_updated(|| {}));
        }
    });

    layout_manager.clear_invocations();
    border1.set_child(&border2);

    assert_eq!(1, layout_manager.layout_updated_adds.get());
}

#[test]
fn making_control_invisible_should_invalidate_parent_measure() {
    let _scope = test_scope();
    let child = Border::new();
    child.set_width(100.0);
    let target = StackPanel::new();
    target.children().add(child.clone());

    target.measure(Size::new(f64::INFINITY, f64::INFINITY));
    target.arrange(Rect::from_size(target.desired_size()));

    assert!(target.is_measure_valid());
    assert!(target.is_arrange_valid());
    assert!(child.is_measure_valid());
    assert!(child.is_arrange_valid());

    child.set_is_visible(false);

    assert!(!target.is_measure_valid());
    assert!(!target.is_arrange_valid());
    assert!(child.is_measure_valid());
    assert!(child.is_arrange_valid());
}

#[test]
fn making_control_visible_should_invalidate_own_and_parent_measure() {
    let _scope = test_scope();
    let child = Border::new();
    child.set_width(100.0);
    child.set_is_visible(false);
    let target = StackPanel::new();
    target.children().add(child.clone());

    target.measure(Size::new(f64::INFINITY, f64::INFINITY));
    target.arrange(Rect::from_size(target.desired_size()));

    assert!(target.is_measure_valid());
    assert!(target.is_arrange_valid());
    assert!(child.is_measure_valid());
    assert!(!child.is_arrange_valid());

    child.set_is_visible(true);

    assert!(!target.is_measure_valid());
    assert!(!target.is_arrange_valid());
    assert!(!child.is_measure_valid());
    assert!(!child.is_arrange_valid());
}

#[test]
fn measuring_invisible_control_should_not_invalidate_parent_measure() {
    let _scope = test_scope();
    let child = Border::new();
    child.set_width(100.0);
    let target = StackPanel::new();
    target.children().add(child.clone());

    target.measure(Size::new(f64::INFINITY, f64::INFINITY));
    target.arrange(Rect::from_size(target.desired_size()));

    assert!(target.is_measure_valid());
    assert!(target.is_arrange_valid());
    assert_eq!(Size::new(100.0, 0.0), child.desired_size());

    child.set_is_visible(false);
    assert_eq!(Size::default(), child.desired_size());

    target.measure(Size::new(f64::INFINITY, f64::INFINITY));
    target.arrange(Rect::from_size(target.desired_size()));
    child.measure(Size::new(f64::INFINITY, f64::INFINITY));

    assert!(target.is_measure_valid());
    assert!(target.is_arrange_valid());
    assert_eq!(Size::default(), child.desired_size());
}

#[test]
fn size_properties_reject_invalid_values() {
    let _scope = test_scope();
    let target = Layoutable::new();

    let set_should_throw = |properies: &[&'static StyledProperty<f64>], value: f64| {
        for prop in properies {
            let result = catch_unwind(AssertUnwindSafe(|| target.set_value(*prop, value)));
            assert!(result.is_err(), "{} accepted {value}", prop.name());
        }
    };

    set_should_throw(&[Layoutable::width_property(), Layoutable::height_property()], f64::INFINITY);
    set_should_throw(&[Layoutable::width_property(), Layoutable::height_property()], -10.0);

    set_should_throw(&[Layoutable::min_width_property(), Layoutable::min_height_property()], f64::INFINITY);
    set_should_throw(&[Layoutable::min_width_property(), Layoutable::min_height_property()], -10.0);

    set_should_throw(&[Layoutable::max_width_property(), Layoutable::max_height_property()], -10.0);
}

#[test]
fn constraint_and_negative_margin() {
    // The preset of this crate has no font manager or text shaper; the text
    // test scope of the base crate supplies the ones of the upstream preset.
    let _app = UnitTestApplication::start(TestServices::mock_platform_render_interface());
    let _text = TextTestScope::new();

    let text_block = TextBlock::new();
    text_block.set_margin(Thickness::uniform(-10.0));
    text_block.set_text(Some("Lorem ipsum dolor sit amet"));

    let border = Border::new();
    border.set_max_width(100.0);
    border.set_child(&text_block);

    border.measure(Size::new(f64::INFINITY, f64::INFINITY));
    border.arrange(Rect::from_position_size(Default::default(), border.desired_size()));

    assert_eq!(Size::new(100.0, 0.0), border.desired_size());
    assert_eq!(Rect::new(0.0, 0.0, 100.0, 0.0), border.bounds());
    assert_eq!(Size::new(100.0, 0.0), text_block.desired_size());
    assert_eq!(Rect::new(-10.0, -10.0, 120.0, 20.0), text_block.bounds());
}

#[repr(C)]
struct TestLayoutable {
    base: Layoutable,
    arrange_size: Cell<Size>,
    measure_result: Cell<Size>,
    measure_size: Cell<Size>,
}

ferro_class!(TestLayoutable: Layoutable);
ferro_impl_classes!(TestLayoutable: FerroObjectImpl, StyledElementImpl, VisualImpl);

impl LayoutableImpl for TestLayoutable {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.measure_size.set(available_size);
        this.measure_result.get()
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        this.arrange_size.set(final_size);
        Self::parent_arrange_override(this, final_size)
    }
}

impl TestLayoutable {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Layoutable::construct(),
            arrange_size: Cell::new(Size::default()),
            measure_result: Cell::new(Size::new(10.0, 10.0)),
            measure_size: Cell::new(Size::default()),
        })
    }
}
