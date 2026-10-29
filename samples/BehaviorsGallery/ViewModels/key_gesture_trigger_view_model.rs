//! Port of `ViewModels/KeyGestureTriggerViewModel.cs`.

use super::{Command, ViewModelBase};
use ferroui_base::ferro_markup_type;
use ferroui_base::input::ICommand;
use ferroui_reactiveui::reactive_object;
use std::rc::Rc;

/// The view model of the page of the key gesture trigger: the command the gesture runs.
pub struct KeyGestureTriggerViewModel {
    base: ViewModelBase,
    trigger_command: Rc<dyn ICommand>,
}
reactive_object!(KeyGestureTriggerViewModel, base);

impl PartialEq for KeyGestureTriggerViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl KeyGestureTriggerViewModel {
    /// `KeyGestureTriggerViewModel()`.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            trigger_command: Command::create(Self::on_triggered),
        })
    }

    /// `OnTriggered()`.
    fn on_triggered() {
        println!("KeyGestureTrigger fired");
    }
}

type Vm = Rc<KeyGestureTriggerViewModel>;

ferro_markup_type!(class KeyGestureTriggerViewModel {
    this: Rc<KeyGestureTriggerViewModel>,
    handles: [KeyGestureTriggerViewModel, Rc<KeyGestureTriggerViewModel>, Option<Rc<KeyGestureTriggerViewModel>>],
    constructors: [() => KeyGestureTriggerViewModel::new],
    properties: [
        TriggerCommand: Rc<dyn ICommand> { get: |this: &Vm| this.trigger_command.clone() },
    ],
    notify_property_changed: KeyGestureTriggerViewModel,
});
