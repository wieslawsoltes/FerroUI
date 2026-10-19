//! `TestWhenAnyObsViewModel` (`ReactiveUI.Tests/WhenAny/TestWhenAnyObsViewModel.cs`): the view
//! model of the tests of `WhenAnyObservable`.

use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::Observable;
use ferroui_reactiveui::{reactive_object, ReactiveCommand, ReactiveObject};
use ferroui_system::IObservable;
use std::cell::RefCell;
use std::rc::Rc;

/// A command and the same command as the observable of its results, which a property path to an
/// observable reads: one `Rc` per command, so that the observation sees the same observable each
/// time it reads the property.
struct CommandSlot<P, R> {
    command: ReactiveCommand<P, R>,
    results: Rc<dyn IObservable<R>>,
}

impl<P: Clone + Default + 'static, R: Clone + 'static> CommandSlot<P, R> {
    fn new(command: ReactiveCommand<P, R>) -> Self {
        let results: Rc<dyn IObservable<R>> = Rc::new(command.clone());
        Self { command, results }
    }
}

/// A view model with three commands that produce their parameter (`TestWhenAnyObsViewModel`).
///
/// Upstream's commands are plain properties (`{ get; set; }`) that do not notify; so are these.
/// A command is a [`ReactiveCommand`]; `WhenAnyObservable` takes a property that holds an
/// observable, so each command is also offered as the observable of its results
/// (`command1_results`, ...), which the tests name `Command1`, ... in their paths
/// (`prop!(TestWhenAnyObsViewModel => command1_results as "Command1")`).
pub struct TestWhenAnyObsViewModel {
    reactive: ReactiveObject,
    command1: RefCell<Option<CommandSlot<i32, i32>>>,
    command2: RefCell<CommandSlot<i32, i32>>,
    command3: RefCell<CommandSlot<String, String>>,
}
reactive_object!(TestWhenAnyObsViewModel);

impl Default for TestWhenAnyObsViewModel {
    /// The commands made from `Signal.Emit` (`Observable::return_`), delivering on the immediate
    /// scheduler (the constructor `TestWhenAnyObsViewModel()`).
    fn default() -> Self {
        Self {
            reactive: ReactiveObject::new(),
            command1: RefCell::new(Some(CommandSlot::new(Self::emit_command()))),
            command2: RefCell::new(CommandSlot::new(Self::emit_command())),
            command3: RefCell::new(CommandSlot::new(Self::emit_command())),
        }
    }
}

impl TestWhenAnyObsViewModel {
    /// A view model with its three commands, shared as view models are.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// A command that produces its parameter, on the immediate scheduler
    /// (`ReactiveCommand.CreateFromObservable<T, T>(Signal.Emit, outputScheduler: Sequencer.Immediate)`).
    fn emit_command<T: Clone + Default + 'static>() -> ReactiveCommand<T, T> {
        ReactiveCommand::create_from_observable_param_with(|value: T| Observable::return_(value), None, Some(ImmediateScheduler::instance()))
    }

    /// `Command1`.
    pub fn command1(&self) -> Option<ReactiveCommand<i32, i32>> {
        self.command1.borrow().as_ref().map(|slot| slot.command.clone())
    }

    /// Sets `Command1`, without notification.
    pub fn set_command1(&self, value: Option<ReactiveCommand<i32, i32>>) {
        *self.command1.borrow_mut() = value.map(CommandSlot::new);
    }

    /// `Command1` as the observable of its results.
    pub fn command1_results(&self) -> Option<Rc<dyn IObservable<i32>>> {
        self.command1.borrow().as_ref().map(|slot| slot.results.clone())
    }

    /// `Command2`.
    pub fn command2(&self) -> ReactiveCommand<i32, i32> {
        self.command2.borrow().command.clone()
    }

    /// Sets `Command2`, without notification.
    pub fn set_command2(&self, value: ReactiveCommand<i32, i32>) {
        *self.command2.borrow_mut() = CommandSlot::new(value);
    }

    /// `Command2` as the observable of its results.
    pub fn command2_results(&self) -> Rc<dyn IObservable<i32>> {
        self.command2.borrow().results.clone()
    }

    /// `Command3`.
    pub fn command3(&self) -> ReactiveCommand<String, String> {
        self.command3.borrow().command.clone()
    }

    /// Sets `Command3`, without notification.
    pub fn set_command3(&self, value: ReactiveCommand<String, String>) {
        *self.command3.borrow_mut() = CommandSlot::new(value);
    }

    /// `Command3` as the observable of its results.
    pub fn command3_results(&self) -> Rc<dyn IObservable<String>> {
        self.command3.borrow().results.clone()
    }
}
