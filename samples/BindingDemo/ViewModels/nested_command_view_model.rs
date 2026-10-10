//! Port of `ViewModels/NestedCommandViewModel.cs`.

use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::input::ICommand;
use mini_mvvm::{MiniCommand, ViewModelBase};
use std::rc::Rc;

pub struct NestedCommandViewModel {
    base: ViewModelBase,
    command: Rc<dyn ICommand>,
}

impl PartialEq for NestedCommandViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for NestedCommandViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl NestedCommandViewModel {
    pub fn new() -> Rc<NestedCommandViewModel> {
        Rc::new(Self { base: ViewModelBase::new(), command: MiniCommand::create(|| {}).as_command() })
    }

    pub fn command(&self) -> Rc<dyn ICommand> {
        self.command.clone()
    }
}

ferro_markup_type!(class NestedCommandViewModel {
    this: Rc<NestedCommandViewModel>,
    handles: [NestedCommandViewModel, Rc<NestedCommandViewModel>, Option<Rc<NestedCommandViewModel>>],
    constructors: [() => NestedCommandViewModel::new],
    properties: [
        Command: Rc<dyn ICommand> { get: |this: &Rc<NestedCommandViewModel>| this.command() },
    ],
    notify_property_changed: NestedCommandViewModel,
});
