//! `ObservablePropertyViewModel` (`ReactiveUI.Binding.GeneratedCode.TestModels/TestModels/ObservablePropertyViewModel.cs`).

use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use ferroui_system::IObservable;
use std::cell::RefCell;
use std::rc::Rc;

/// An observable held by a property, which may be absent (`IObservable<string>?`).
type CommandObservable = Option<Rc<dyn IObservable<String>>>;

/// A view model whose properties hold observables, for `WhenAnyObservable`
/// (`ObservablePropertyViewModel`). It raises `PropertyChanged` only.
pub struct ObservablePropertyViewModel {
    my_command: RefCell<CommandObservable>,
    other_command: RefCell<CommandObservable>,
    property_changed: Event<str>,
}

impl ObservablePropertyViewModel {
    /// The view model with no observables.
    pub fn new() -> Rc<Self> {
        Rc::new(Self { my_command: RefCell::new(None), other_command: RefCell::new(None), property_changed: Event::new() })
    }

    /// `MyCommand`.
    pub fn my_command(&self) -> CommandObservable {
        self.my_command.borrow().clone()
    }

    /// Sets `MyCommand`; the same observable changes nothing.
    pub fn set_my_command(&self, value: CommandObservable) {
        self.set(&self.my_command, value, "MyCommand");
    }

    /// `OtherCommand`.
    pub fn other_command(&self) -> CommandObservable {
        self.other_command.borrow().clone()
    }

    /// Sets `OtherCommand`; the same observable changes nothing.
    pub fn set_other_command(&self, value: CommandObservable) {
        self.set(&self.other_command, value, "OtherCommand");
    }

    /// Stores `value` and raises `PropertyChanged`, unless it is the observable held (upstream
    /// compares the references).
    fn set(&self, field: &RefCell<CommandObservable>, value: CommandObservable, name: &str) {
        let same = match (&*field.borrow(), &value) {
            (Some(current), Some(new)) => Rc::ptr_eq(current, new),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }
        *field.borrow_mut() = value;
        self.property_changed.raise(name);
    }
}

impl INotifyPropertyChanged for ObservablePropertyViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}
