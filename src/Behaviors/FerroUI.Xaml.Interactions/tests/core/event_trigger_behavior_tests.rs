//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Core/EventTriggerBehaviorTests.cs` of
//! Xaml.Behaviors (with its windows `EventTriggerBehavior001.xaml` to `EventTriggerBehavior005.xaml`,
//! loaded with the run-time loader).
//!
//! Adapted: a click of a button is the Enter key on it; a tap is a press and a release of the
//! pointer on it (see `support.rs`); the jobs of the dispatcher run after showing the window; the
//! data context of `EventTriggerBehavior005` is set by the test (a model with the commands, which
//! the window of the reference creates in its document). The rendered frames the reference saves
//! are not taken.

use super::command::Command;
use crate::tests::support::{click, find, load, run_jobs, start_app, MouseTestHelper};
use ferroui_base::data::core::Value;
use ferroui_base::data::model::Model as FerroModel;
use ferroui_base::input::ICommand;
use ferroui_base::utilities::EventArgs;
use ferroui_base::{ferro_model, BoxedValue, Ref};
use ferroui_controls::{Button, Flyout, TextBox, Window};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `FlyoutEventBindingSource`: the data context of `EventTriggerBehavior005`, with a command for
/// each event of the flyout that records its executions.
pub(crate) struct FlyoutEventBindingSource {
    opened_command: Rc<dyn ICommand>,
    closed_command: Rc<dyn ICommand>,
    opened_count: Rc<Cell<i32>>,
    closed_count: Rc<Cell<i32>>,
    opened_parameter: Rc<RefCell<Option<BoxedValue>>>,
    closed_parameter: Rc<RefCell<Option<BoxedValue>>>,
}

ferro_model!(FlyoutEventBindingSource, |b| b
    .read_only::<Value<Rc<dyn ICommand>>>("OpenedCommand", |source| source.opened_command.clone())
    .read_only::<Value<Rc<dyn ICommand>>>("ClosedCommand", |source| source.closed_command.clone()));

impl FlyoutEventBindingSource {
    fn new() -> Rc<Self> {
        let opened_count = Rc::new(Cell::new(0));
        let closed_count = Rc::new(Cell::new(0));
        let opened_parameter = Rc::new(RefCell::new(None));
        let closed_parameter = Rc::new(RefCell::new(None));

        let (count, last) = (opened_count.clone(), opened_parameter.clone());
        let opened_command = Command::new(move |parameter| {
            count.set(count.get() + 1);
            *last.borrow_mut() = parameter.cloned();
        });
        let (count, last) = (closed_count.clone(), closed_parameter.clone());
        let closed_command = Command::new(move |parameter| {
            count.set(count.get() + 1);
            *last.borrow_mut() = parameter.cloned();
        });

        <FlyoutEventBindingSource as FerroModel>::new_model(Self {
            opened_command,
            closed_command,
            opened_count,
            closed_count,
            opened_parameter,
            closed_parameter,
        })
    }
}

fn target_text(window: &Window) -> Option<String> {
    find::<TextBox>(window, "TargetTextBox").text()
}

#[test]
fn event_trigger_behavior_001() {
    let _app = start_app();
    let window = load::<Window>("EventTriggerBehavior001.xaml", include_str!("EventTriggerBehavior001.xaml"));

    window.show();
    run_jobs();

    click(&find::<Button>(&window, "TargetButton"));

    assert_eq!(Some("Click Text".to_string()), target_text(&window));
}

#[test]
fn event_trigger_behavior_002() {
    let _app = start_app();
    let window = load::<Window>("EventTriggerBehavior002.xaml", include_str!("EventTriggerBehavior002.xaml"));

    window.show();
    run_jobs();

    MouseTestHelper::new().click(&find::<Button>(&window, "TargetButton"));

    assert_eq!(Some("Tapped Text".to_string()), target_text(&window));
}

#[test]
fn event_trigger_behavior_003() {
    let _app = start_app();
    let window = load::<Window>("EventTriggerBehavior003.xaml", include_str!("EventTriggerBehavior003.xaml"));

    window.show();
    run_jobs();

    MouseTestHelper::new().double_click(&find::<Button>(&window, "TargetButton"));

    assert_eq!(Some("DoubleTapped Text".to_string()), target_text(&window));
}

#[test]
fn event_trigger_behavior_004() {
    let _app = start_app();
    let window = load::<Window>("EventTriggerBehavior004.xaml", include_str!("EventTriggerBehavior004.xaml"));

    window.show();
    run_jobs();

    assert_eq!(Some("Loaded Text".to_string()), target_text(&window));
}

#[test]
fn event_trigger_behavior_005_flyout_events_execute_bound_commands() {
    let _app = start_app();
    let window = load::<Window>("EventTriggerBehavior005.xaml", include_str!("EventTriggerBehavior005.xaml"));
    let source = FlyoutEventBindingSource::new();
    // A model is boxed as the shared object itself.
    window.set_data_context(Some(source.clone() as BoxedValue));
    let target_button = find::<Button>(&window, "TargetButton");
    let flyout: Ref<Flyout> = target_button.flyout().and_then(|flyout| flyout.cast::<Flyout>()).expect("a flyout");

    window.show();
    run_jobs();
    flyout.show_at(&target_button);

    assert_eq!(1, source.opened_count.get());
    assert!(source.opened_parameter.borrow().as_ref().is_some_and(|parameter| parameter.is::<EventArgs>()));
    assert_eq!(0, source.closed_count.get());

    flyout.hide();

    assert_eq!(1, source.closed_count.get());
    assert!(source.closed_parameter.borrow().as_ref().is_some_and(|parameter| parameter.is::<EventArgs>()));
}
