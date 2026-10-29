//! Port of `ViewModels/SampleViewModel.cs`.

use super::{Command, ViewModelBase};
use ferroui_base::ferro_markup_type;
use ferroui_base::input::ICommand;
use ferroui_reactiveui::reactive_object;
use std::rc::Rc;

/// The view model of the sample view (`SampleView`): commands for its loading and unloading.
pub struct SampleViewModel {
    base: ViewModelBase,
    loaded_command: Rc<dyn ICommand>,
    unloaded_command: Rc<dyn ICommand>,
}
reactive_object!(SampleViewModel, base);

impl PartialEq for SampleViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl SampleViewModel {
    /// `SampleViewModel()`.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            loaded_command: Command::create(Self::loaded),
            unloaded_command: Command::create(Self::unloaded),
        })
    }

    /// `LoadedCommand`.
    pub fn loaded_command(&self) -> Rc<dyn ICommand> {
        self.loaded_command.clone()
    }

    /// `UnloadedCommand`.
    pub fn unloaded_command(&self) -> Rc<dyn ICommand> {
        self.unloaded_command.clone()
    }

    fn loaded() {
        println!("Loaded");
    }

    fn unloaded() {
        println!("Unloaded");
    }
}

ferro_markup_type!(class SampleViewModel {
    this: Rc<SampleViewModel>,
    handles: [SampleViewModel, Rc<SampleViewModel>, Option<Rc<SampleViewModel>>],
    constructors: [() => SampleViewModel::new],
    properties: [
        LoadedCommand: Rc<dyn ICommand> { get: |this: &Rc<SampleViewModel>| this.loaded_command() },
        UnloadedCommand: Rc<dyn ICommand> { get: |this: &Rc<SampleViewModel>| this.unloaded_command() },
    ],
    notify_property_changed: SampleViewModel,
});
