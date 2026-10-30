//! Port of `ViewModels/ReactiveUI/InteractionTriggerBehaviorViewModel.cs`.

use crate::view_models::{Command, ViewModelBase};
use ferroui_base::ferro_markup_type;
use ferroui_base::input::ICommand;
use ferroui_reactiveui::{reactive_object, Interaction};
use std::rc::Rc;

/// The view model of the page of the interaction trigger behavior: an interaction and the command
/// that asks it.
pub struct InteractionTriggerBehaviorViewModel {
    base: ViewModelBase,
    test_interaction: Interaction<(), ()>,
    trigger_command: Rc<dyn ICommand>,
}
reactive_object!(InteractionTriggerBehaviorViewModel, base);

impl PartialEq for InteractionTriggerBehaviorViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl InteractionTriggerBehaviorViewModel {
    /// `InteractionTriggerBehaviorViewModel()`.
    pub fn new() -> Rc<Self> {
        let test_interaction = Interaction::<(), ()>::new();
        let interaction = test_interaction.clone();
        Rc::new(Self {
            base: ViewModelBase::new(),
            test_interaction,
            trigger_command: Command::create(move || Self::trigger(&interaction)),
        })
    }

    /// `TestInteraction`.
    pub fn test_interaction(&self) -> Interaction<(), ()> {
        self.test_interaction.clone()
    }

    /// `Trigger()`: asks the interaction (`TestInteraction.Handle(Unit.Default).Subscribe()`).
    fn trigger(interaction: &Interaction<(), ()>) {
        let _ = interaction.handle(());
    }
}

type Vm = Rc<InteractionTriggerBehaviorViewModel>;

ferro_markup_type!(class InteractionTriggerBehaviorViewModel {
    this: Rc<InteractionTriggerBehaviorViewModel>,
    handles: [InteractionTriggerBehaviorViewModel, Rc<InteractionTriggerBehaviorViewModel>, Option<Rc<InteractionTriggerBehaviorViewModel>>],
    constructors: [() => InteractionTriggerBehaviorViewModel::new],
    properties: [
        TestInteraction: Interaction<(), ()> { get: |this: &Vm| this.test_interaction() },
        TriggerCommand: Rc<dyn ICommand> { get: |this: &Vm| this.trigger_command.clone() },
    ],
    notify_property_changed: InteractionTriggerBehaviorViewModel,
});
