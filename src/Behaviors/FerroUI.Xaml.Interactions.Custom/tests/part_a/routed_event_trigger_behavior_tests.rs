//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Custom/RoutedEventTriggerBehaviorTests.cs`
//! of Xaml.Behaviors (with its window `WindowClosedRoutedEventWindow.xaml` and the binding source
//! of its code file, loaded with the run-time loader).
//!
//! Adapted: the data context of the window is set by the test (a model with the command, which
//! the window of the reference creates in its document).

use super::support::{load, run_jobs, start_app};
use crate::RoutedEventTriggerBehavior;
use ferroui_base::data::core::Value;
use ferroui_base::data::model::Model as FerroModel;
use ferroui_base::input::ICommand;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{ferro_class, ferro_model, instantiate, BoxedValue, FerroObjectImpl, Ref};
use ferroui_controls::{Button, Control, Panel, Window};
use ferroui_xaml_interactivity::{Action, ActionImpl, Interaction};
use std::cell::Cell;
use std::rc::Rc;

/// `CountingCommand`: a command that can always execute and counts its executions.
pub(crate) struct CountingCommand {
    execution_count: Cell<i32>,
}

impl CountingCommand {
    fn execution_count(&self) -> i32 {
        self.execution_count.get()
    }
}

impl ICommand for CountingCommand {
    fn can_execute(&self, _parameter: Option<&BoxedValue>) -> bool {
        true
    }

    fn execute(&self, _parameter: Option<&BoxedValue>) {
        self.execution_count.set(self.execution_count.get() + 1);
    }

    fn can_execute_changed(&self, _handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

/// `WindowClosedBindingSource`: the data context of the window, with the command to execute.
pub(crate) struct WindowClosedBindingSource {
    close_command: Rc<CountingCommand>,
}

ferro_model!(WindowClosedBindingSource, |b| b
    .read_only::<Value<Rc<dyn ICommand>>>("CloseCommand", |source| source.close_command.clone() as Rc<dyn ICommand>));

impl WindowClosedBindingSource {
    fn new() -> Rc<Self> {
        <WindowClosedBindingSource as FerroModel>::new_model(Self {
            close_command: Rc::new(CountingCommand { execution_count: Cell::new(0) }),
        })
    }
}

/// `CountingAction`: an action of the object model that counts its executions.
#[repr(C)]
pub(crate) struct CountingAction {
    base: Action,
    execution_count: Cell<i32>,
}

ferro_class!(CountingAction: Action);
ferroui_base::ferro_impl_classes!(CountingAction: FerroObjectImpl);

impl ActionImpl for CountingAction {
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        this.execution_count.set(this.execution_count.get() + 1);
        None
    }
}

impl CountingAction {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Action::construct(), execution_count: Cell::new(0) })
    }

    fn execution_count(&self) -> i32 {
        self.execution_count.get()
    }
}

#[test]
fn window_closed_event_executes_bound_command() {
    let _app = start_app();
    let window =
        load::<Window>("WindowClosedRoutedEventWindow.xaml", include_str!("WindowClosedRoutedEventWindow.xaml"));
    let source = WindowClosedBindingSource::new();
    // A model is boxed as the shared object itself.
    window.set_data_context(Some(source.clone() as BoxedValue));
    let behaviors = Interaction::get_behaviors(&window);
    assert_eq!(1, behaviors.count());
    let behavior = behaviors.get(0).cast::<RoutedEventTriggerBehavior>().expect("a RoutedEventTriggerBehavior");
    let action = CountingAction::new();
    behavior.actions().expect("the actions").add(action.clone().upcast());

    window.show();
    window.close();
    run_jobs();

    assert_eq!(1, action.execution_count());
    assert_eq!(1, source.close_command.execution_count());
    assert!(behavior.associated_object().is_none());

    window.raise_event(&RoutedEventArgs::with_event(Window::window_closed_event()));

    assert_eq!(1, action.execution_count());
    assert_eq!(1, source.close_command.execution_count());
}

#[test]
fn control_routed_event_unsubscribes_when_control_leaves_visual_tree() {
    let _app = start_app();
    let button = Button::new();
    let panel = Panel::new();
    panel.children().add(&button);
    let window = Window::new();
    window.set_content(Some(Control::boxed(&panel)));
    let behavior = RoutedEventTriggerBehavior::new();
    behavior.set_routed_event(Some(Button::click_event().as_routed_event()));
    let action = CountingAction::new();
    behavior.actions().expect("the actions").add(action.clone().upcast());
    Interaction::get_behaviors(&button).add(behavior.clone().upcast());

    window.show();
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));

    assert_eq!(1, action.execution_count());

    panel.children().remove(&button);
    run_jobs();
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));

    assert_eq!(1, action.execution_count());
}
