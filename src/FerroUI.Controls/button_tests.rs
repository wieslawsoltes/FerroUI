use crate::i_clickable_control::as_clickable_control;
use crate::mouse_test_helper::MouseTestHelper;
use crate::test_command::TestCommand;
use crate::test_support::{test_scope, TestRoot};
use crate::testing::{TestServices, UnitTestApplication};
use crate::{Button, Control, Panel, StackPanel, Window};
use ferroui_base::media::{Geometry, GeometryHitTestResult};
use ferroui_base::rendering::IHitTester;
use ferroui_base::Visual;
use ferroui_base::data::core::Value;
use ferroui_base::data::model::Model;
use ferroui_base::data::ReflectionBinding;
use ferroui_base::input::{AccessKeyEventArgs, ICommand, InputElement, Key, KeyEventArgs, MouseButton};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::TranslateTransform;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{ferro_model, BoxedValue, Point, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn counter() -> (Rc<Cell<i32>>, Rc<Cell<i32>>) {
    let raised = Rc::new(Cell::new(0));
    (raised.clone(), raised)
}

fn count_clicks(target: &Button) -> Rc<Cell<i32>> {
    let (raised, result) = counter();
    target.click(move |_, _| raised.set(raised.get() + 1));
    result
}

/// The hit tester of the reference tests: the root of the search when it
/// contains the point.
struct BoundsHitTester;

impl IHitTester for BoundsHitTester {
    fn hit_test(&self, p: Point, root: &Visual, _filter: Option<&dyn Fn(&Visual) -> bool>) -> Vec<Ref<Visual>> {
        if root.bounds().contains(p) {
            vec![root.to_ref()]
        } else {
            Vec::new()
        }
    }

    fn hit_test_geometry(
        &self,
        _geometry: &Geometry,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Vec<GeometryHitTestResult> {
        Vec::new()
    }

    fn hit_test_first(&self, p: Point, root: &Visual, filter: Option<&dyn Fn(&Visual) -> bool>) -> Option<Ref<Visual>> {
        self.hit_test(p, root, filter).into_iter().next()
    }

    fn hit_test_first_geometry(
        &self,
        _geometry: &Geometry,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<GeometryHitTestResult> {
        None
    }
}

fn window_with_content(content: impl ferroui_base::IntoRef<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(content)));
    window
}

fn create_key_down_event(key: Key, source: Option<&Ref<Button>>) -> KeyEventArgs {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.set_routed_event(Some(InputElement::key_down_event()));
    if let Some(source) = source {
        e.set_source(source);
    }
    e
}

fn create_key_up_event(key: Key) -> KeyEventArgs {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.set_routed_event(Some(InputElement::key_up_event()));
    e
}

fn raise_pointer_pressed(helper: &MouseTestHelper, button: &Button, click_count: i32, mouse_button: MouseButton, position: Point) {
    helper.down_at(button, mouse_button, position, click_count);
}

fn raise_pointer_released(helper: &MouseTestHelper, button: &Button, mouse_button: MouseButton, pt: Point) {
    helper.up_at(button, mouse_button, pt);
}

fn is_captured_by(helper: &MouseTestHelper, target: &Ref<Button>) -> bool {
    let target: Ref<InputElement> = target.clone().upcast();
    helper.captured() == Some(target)
}

#[test]
fn button_is_disabled_when_command_is_disabled() {
    let _scope = test_scope();
    let command = TestCommand::new(false);
    let target = Button::new();
    target.set_command(command.as_command());
    let _root = TestRoot::with_child(&target);

    assert!(!target.is_effectively_enabled());
    command.set_is_enabled(true);
    assert!(target.is_effectively_enabled());
    command.set_is_enabled(false);
    assert!(!target.is_effectively_enabled());
}

#[test]
fn button_is_disabled_when_command_is_enabled_but_is_enabled_is_false() {
    let _scope = test_scope();
    let command = TestCommand::new(true);
    let target = Button::new();
    target.set_is_enabled(false);
    target.set_command(command.as_command());

    let _root = TestRoot::with_child(&target);

    assert!(!target.is_effectively_enabled());
}

struct ViewModel {
    command: Rc<TestCommand>,
}

ferro_model!(ViewModel, |b| b
    .read_only::<Value<Option<Rc<dyn ICommand>>>>("Command", |vm| vm.command.as_command()));

/// A data context without a `Command` property.
struct EmptyViewModel;

ferro_model!(EmptyViewModel, |b| b);

#[test]
fn button_is_disabled_when_bound_command_doesnt_exist() {
    let target = Button::new();
    target.bind_binding(Button::command_property(), &ReflectionBinding::new("Command"));

    assert!(target.is_enabled());
    assert!(!target.is_effectively_enabled());
}

#[test]
fn button_is_disabled_when_bound_command_is_removed() {
    let view_model = Model::new_model(ViewModel { command: TestCommand::new(true) });

    let target = Button::new();
    target.set_data_context(Some(view_model));
    target.bind_binding(Button::command_property(), &ReflectionBinding::new("Command"));

    assert!(target.is_enabled());
    assert!(target.is_effectively_enabled());

    target.set_data_context(None);

    assert!(target.is_enabled());
    assert!(!target.is_effectively_enabled());
}

#[test]
fn button_is_enabled_when_bound_command_is_added() {
    let _scope = test_scope();
    let view_model = Model::new_model(ViewModel { command: TestCommand::new(true) });

    let target = Button::new();
    target.set_data_context(Some(Model::new_model(EmptyViewModel)));
    target.bind_binding(Button::command_property(), &ReflectionBinding::new("Command"));
    let _root = TestRoot::with_child(&target);

    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));

    assert!(target.is_enabled());
    assert!(!target.is_effectively_enabled());

    target.set_data_context(Some(view_model));

    assert!(target.is_enabled());
    assert!(target.is_effectively_enabled());
}

#[test]
fn button_is_disabled_when_disabled_bound_command_is_added() {
    let view_model = Model::new_model(ViewModel { command: TestCommand::new(false) });

    let target = Button::new();
    target.set_data_context(Some(Model::new_model(EmptyViewModel)));
    target.bind_binding(Button::command_property(), &ReflectionBinding::new("Command"));

    assert!(target.is_enabled());
    assert!(!target.is_effectively_enabled());

    target.set_data_context(Some(view_model));

    assert!(target.is_enabled());
    assert!(!target.is_effectively_enabled());
}

#[test]
fn button_raises_click() {
    let helper = MouseTestHelper::new();
    let pt = Point::new(50.0, 50.0);

    let _app = UnitTestApplication::start(TestServices::styled_window());

    let root = Window::new();
    root.set_hit_tester_override(Some(Rc::new(BoundsHitTester)));
    let target = Button::new();
    target.set_width(100.0);
    target.set_height(100.0);
    target.set_vertical_alignment(VerticalAlignment::Top);
    target.set_horizontal_alignment(HorizontalAlignment::Left);
    root.set_content(Some(Control::boxed(&target)));
    root.show();

    let clicked = count_clicks(&target);

    helper.enter(&target);
    helper.move_(&target, pt);
    raise_pointer_pressed(&helper, &target, 1, MouseButton::Left, pt);

    assert!(is_captured_by(&helper, &target));

    raise_pointer_released(&helper, &target, MouseButton::Left, pt);

    assert!(helper.captured().is_none());

    assert_eq!(clicked.get(), 1);
}

#[test]
fn button_does_not_raise_click_when_pointer_released_outside() {
    let _scope = test_scope();
    let helper = MouseTestHelper::new();
    let root = TestRoot::new();
    let target = Button::new();
    target.set_width(100.0);
    target.set_height(100.0);
    root.set_child(&target);

    let clicked = count_clicks(&target);

    helper.enter(&target);
    helper.move_(&target, Point::new(50.0, 50.0));
    raise_pointer_pressed(&helper, &target, 1, MouseButton::Left, Point::new(50.0, 50.0));
    helper.leave(&target);

    assert!(is_captured_by(&helper, &target));

    raise_pointer_released(&helper, &target, MouseButton::Left, Point::new(200.0, 50.0));

    assert!(helper.captured().is_none());

    assert_eq!(clicked.get(), 0);
}

#[test]
fn button_with_render_transform_raises_click() {
    let helper = MouseTestHelper::new();
    let pt = Point::new(150.0, 50.0);

    let _app = UnitTestApplication::start(TestServices::styled_window());

    let root = Window::new();
    root.set_hit_tester_override(Some(Rc::new(BoundsHitTester)));
    let target = Button::new();
    target.set_width(100.0);
    target.set_height(100.0);
    target.set_vertical_alignment(VerticalAlignment::Top);
    target.set_horizontal_alignment(HorizontalAlignment::Left);
    target.set_render_transform(Some(TranslateTransform::with_offset(100.0, 0.0).into()));
    root.set_content(Some(Control::boxed(&target)));
    root.show();

    // The actual bounds of the button are 100,0,100,100: translated 100
    // pixels along x, so a pointer at x=150 should trigger a click. The
    // button must not rely on its bounds to decide whether the pointer is
    // over it but on hit testing, which takes the rendered bounds into
    // account.

    Dispatcher::ui_thread().run_jobs(None);

    let clicked = count_clicks(&target);

    helper.enter(&target);
    helper.move_(&target, pt);
    raise_pointer_pressed(&helper, &target, 1, MouseButton::Left, pt);

    assert!(is_captured_by(&helper, &target));

    raise_pointer_released(&helper, &target, MouseButton::Left, pt);

    assert!(helper.captured().is_none());

    assert_eq!(clicked.get(), 1);
}

#[test]
fn button_does_not_subscribe_to_command_can_execute_changed_until_added_to_logical_tree() {
    let command = TestCommand::new(true);
    let target = Button::new();
    target.set_command(command.as_command());

    assert_eq!(0, command.subscription_count());
}

#[test]
fn button_subscribes_to_command_can_execute_changed_when_added_to_logical_tree() {
    let _scope = test_scope();
    let command = TestCommand::new(true);
    let target = Button::new();
    target.set_command(command.as_command());
    let _root = TestRoot::with_child(&target);

    assert_eq!(1, command.subscription_count());
}

#[test]
fn button_unsubscribes_from_command_can_execute_changed_when_removed_from_logical_tree() {
    let _scope = test_scope();
    let command = TestCommand::new(true);
    let target = Button::new();
    target.set_command(command.as_command());
    let root = TestRoot::with_child(&target);

    root.set_child(None);
    assert_eq!(0, command.subscription_count());
}

#[test]
fn button_invokes_can_execute_when_command_parameter_changed() {
    let target = Button::new();
    let raised = count_clicks(&target);

    target.raise_event(&AccessKeyEventArgs::new("b", false));

    assert_eq!(1, raised.get());
}

#[test]
fn button_invokes_doesnt_execute_when_button_disabled() {
    let target = Button::new();

    target.set_is_enabled(false);
    let raised = count_clicks(&target);

    target.raise_event(&AccessKeyEventArgs::new("b", false));

    assert_eq!(0, raised.get());
}

#[test]
fn button_is_default_works() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = Button::new();
    let window = window_with_content(&target);
    window.show();

    let raised = count_clicks(&target);

    target.set_is_default(false);
    window.raise_event(&create_key_down_event(Key::Enter, None));
    assert_eq!(0, raised.get());

    target.set_is_default(true);
    window.raise_event(&create_key_down_event(Key::Enter, None));
    assert_eq!(1, raised.get());

    target.set_is_default(false);
    window.raise_event(&create_key_down_event(Key::Enter, None));
    assert_eq!(1, raised.get());

    target.set_is_default(true);
    window.raise_event(&create_key_down_event(Key::Enter, None));
    assert_eq!(2, raised.get());

    window.set_content(None);
    // To check if the handler was raised on the button when it's detached,
    // it is passed as the source manually.
    window.raise_event(&create_key_down_event(Key::Enter, Some(&target)));
    assert_eq!(2, raised.get());
}

#[test]
fn button_is_default_should_not_work_when_button_is_not_effectively_visible() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let panel = Panel::new();
    let target = Button::new();
    panel.children().add(&target);
    let window = window_with_content(&panel);
    window.show();

    let raised = count_clicks(&target);

    target.set_is_default(true);
    panel.set_is_visible(false);
    window.raise_event(&create_key_down_event(Key::Enter, None));
    assert_eq!(0, raised.get());
}

#[test]
fn button_is_cancel_works() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = Button::new();
    let window = window_with_content(&target);
    window.show();

    let raised = count_clicks(&target);

    target.set_is_cancel(false);
    window.raise_event(&create_key_down_event(Key::Escape, None));
    assert_eq!(0, raised.get());

    target.set_is_cancel(true);
    window.raise_event(&create_key_down_event(Key::Escape, None));
    assert_eq!(1, raised.get());

    target.set_is_cancel(false);
    window.raise_event(&create_key_down_event(Key::Escape, None));
    assert_eq!(1, raised.get());

    target.set_is_cancel(true);
    window.raise_event(&create_key_down_event(Key::Escape, None));
    assert_eq!(2, raised.get());

    window.set_content(None);
    window.raise_event(&create_key_down_event(Key::Escape, Some(&target)));
    assert_eq!(2, raised.get());
}

#[test]
fn button_is_cancel_should_not_work_when_button_is_not_effectively_visible() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let panel = Panel::new();
    let target = Button::new();
    panel.children().add(&target);
    let window = window_with_content(&panel);
    window.show();

    let raised = count_clicks(&target);

    target.set_is_cancel(true);
    panel.set_is_visible(false);
    window.raise_event(&create_key_down_event(Key::Escape, None));
    assert_eq!(0, raised.get());
}

#[test]
fn button_command_parameter_does_not_change_while_execution() {
    let _scope = test_scope();
    let target = Button::new();
    let initial: BoxedValue = Rc::new("A".to_string());
    let last_parameter: Rc<RefCell<Option<BoxedValue>>> = Rc::new(RefCell::new(Some(initial.clone())));
    let only_once = Cell::new(false);
    let executed = Rc::new(Cell::new(false));

    let weak_target = target.downgrade();
    let last_in_can_execute = last_parameter.clone();
    let last_in_execute = last_parameter.clone();
    let executed_flag = executed.clone();
    let command = TestCommand::with_can_execute_and_execute(
        move |parameter| {
            if !only_once.replace(true) {
                let next: BoxedValue = Rc::new(1234_i32);
                weak_target.upgrade().unwrap().set_command_parameter(Some(next));
            }
            *last_in_can_execute.borrow_mut() = parameter.cloned();
            true
        },
        move |parameter| {
            assert!(*last_in_execute.borrow() == parameter.cloned());
            executed_flag.set(true);
        },
    );
    target.set_command_parameter(Some(initial));
    target.set_command(command.as_command());
    let _root = TestRoot::with_child(&target);

    as_clickable_control(&target).unwrap().raise_click();

    assert!(executed.get());
}

/// A focusable control stands in for the text box of the reference test.
#[test]
fn should_not_fire_click_event_on_space_key_when_it_is_not_focus() {
    let _app = UnitTestApplication::start(TestServices::focusable_window());
    let target = Control::new();
    target.set_focusable(true);
    let button = Button::new();
    button.set_content(Some(Control::boxed(&target)));

    let window = window_with_content(&button);
    window.show();

    let raised = count_clicks(&button);
    assert!(target.focus());
    assert!(target.is_focused());
    target.raise_event(&create_key_down_event(Key::Space, None));
    target.raise_event(&create_key_up_event(Key::Space));
    assert_eq!(0, raised.get());
}

#[test]
fn button_unpressed_when_disabled() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let helper = MouseTestHelper::new();
    let target = Button::new();
    // Disabling a control implies focus loss, and focus loss has its own
    // code path to un-press the button. So that path has to be avoided to
    // get an accurate result.
    target.set_focusable(false);

    let window = window_with_content(&target);
    window.show();

    raise_pointer_pressed(&helper, &target, 1, MouseButton::Left, Point::new(50.0, 50.0));

    assert!(target.is_pressed());
    assert!(!target.is_focused());
    target.set_is_enabled(false);
    assert!(!target.is_pressed());
}

#[test]
fn button_unpressed_when_focus_lost() {
    let _app = UnitTestApplication::start(TestServices::focusable_window());
    let helper = MouseTestHelper::new();
    let target = Button::new();
    let other = Button::new();

    let panel = StackPanel::new();
    panel.children().add(&target);
    panel.children().add(&other);
    let window = window_with_content(&panel);
    window.show();

    raise_pointer_pressed(&helper, &target, 1, MouseButton::Left, Point::new(50.0, 50.0));

    assert!(target.is_pressed());
    assert!(target.is_focused());
    assert!(other.focus());
    assert!(!target.is_pressed());
}

// Additional tests of the platform feedback, which the reference suite
// does not cover.
mod platform_feedback_tests {
    use super::*;
    use crate::platform::{
        FeedbackAction, FeedbackType, IPlatformFeedback, PlatformFeedback, PlatformFeedbackExtensions,
    };
    use crate::testing::MockWindowingPlatform;

    #[derive(Default)]
    struct RecordingFeedback {
        performed: RefCell<Vec<(FeedbackAction, FeedbackType)>>,
        result: Cell<bool>,
    }

    impl IPlatformFeedback for RecordingFeedback {
        fn perform(&self, feedback: FeedbackAction, type_: FeedbackType) -> bool {
            self.performed.borrow_mut().push((feedback, type_));
            self.result.get()
        }
    }

    fn window_with_feedback(content: &Ref<Button>) -> (Ref<Window>, Rc<RecordingFeedback>) {
        let feedback = Rc::new(RecordingFeedback::default());
        let window_impl = MockWindowingPlatform::create_window_mock();
        window_impl.setup_feature::<dyn IPlatformFeedback>(feedback.clone());
        let window = Window::with_impl(window_impl);
        window.set_content(Some(Control::boxed(content)));
        window.show();
        (window, feedback)
    }

    #[test]
    fn button_feedback_type_defaults_to_auto() {
        let _scope = test_scope();
        let target = Button::new();
        let other = Control::new();

        assert_eq!(FeedbackType::Auto, PlatformFeedback::get_feedback_type(&target));
        assert_eq!(FeedbackType::None, PlatformFeedback::get_feedback_type(&other));
    }

    #[test]
    fn click_performs_the_click_feedback_of_the_platform() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let target = Button::new();
        let (_window, feedback) = window_with_feedback(&target);

        target.perform_click();

        assert_eq!(vec![(FeedbackAction::click(), FeedbackType::Auto)], *feedback.performed.borrow());

        PlatformFeedback::set_feedback_type(&target, FeedbackType::Haptic);
        target.perform_click();

        assert_eq!(Some(&(FeedbackAction::click(), FeedbackType::Haptic)), feedback.performed.borrow().last());
    }

    #[test]
    fn disabled_button_performs_no_feedback() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let target = Button::new();
        let (_window, feedback) = window_with_feedback(&target);
        target.set_is_enabled(false);

        target.perform_click();

        assert!(feedback.performed.borrow().is_empty());
    }

    #[test]
    fn perform_feedback_answers_what_the_platform_answers() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let target = Button::new();
        let (_window, feedback) = window_with_feedback(&target);

        assert!(!target.perform_feedback(FeedbackAction::hold()));
        feedback.result.set(true);
        assert!(target.perform_feedback(FeedbackAction::hold()));
        assert_eq!(2, feedback.performed.borrow().len());

        // No feedback is requested for the element.
        PlatformFeedback::set_feedback_type(&target, FeedbackType::None);
        assert!(!target.perform_feedback(FeedbackAction::hold()));
        assert_eq!(2, feedback.performed.borrow().len());
    }

    #[test]
    fn perform_feedback_needs_a_top_level_with_the_feature() {
        let _app = UnitTestApplication::start(TestServices::styled_window());

        // Not in a top-level.
        let detached = Button::new();
        let _root = TestRoot::with_child(&detached);
        assert!(!detached.perform_feedback(FeedbackAction::click()));

        // In a top-level whose platform has no feedback.
        let target = Button::new();
        let window = window_with_content(&target);
        window.show();
        assert!(!target.perform_feedback(FeedbackAction::click()));
    }
}
