//! Port of `ViewModels/ApplicationViewModel.cs`.

use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::input::ICommand;
use ferroui_controls::Application;
use mini_mvvm::{MiniCommand, ViewModelBase};
use std::rc::Rc;

/// The view model of the application: the commands of its menus.
pub struct ApplicationViewModel {
    base: ViewModelBase,
    exit_command: Rc<MiniCommand>,
    restore_default: Rc<MiniCommand>,
}

impl PartialEq for ApplicationViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for ApplicationViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl ApplicationViewModel {
    pub fn new() -> Rc<ApplicationViewModel> {
        let exit_command = MiniCommand::create(|| {
            let lifetime = Application::current().and_then(|application| application.application_lifetime());
            if let Some(lifetime) = lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()) {
                lifetime.shutdown(0);
            }
        });

        let restore_default = MiniCommand::create(|| {});

        Rc::new(Self { base: ViewModelBase::new(), exit_command, restore_default })
    }

    pub fn exit_command(&self) -> Rc<MiniCommand> {
        self.exit_command.clone()
    }

    pub fn restore_default(&self) -> Rc<MiniCommand> {
        self.restore_default.clone()
    }
}

ferro_markup_type!(class ApplicationViewModel {
    this: Rc<ApplicationViewModel>,
    handles: [ApplicationViewModel, Rc<ApplicationViewModel>, Option<Rc<ApplicationViewModel>>],
    constructors: [() => ApplicationViewModel::new],
    properties: [
        ExitCommand: Rc<dyn ICommand> { get: |this: &Rc<ApplicationViewModel>| this.exit_command().as_command() },
        RestoreDefault: Rc<dyn ICommand> {
            get: |this: &Rc<ApplicationViewModel>| this.restore_default().as_command()
        },
    ],
    notify_property_changed: ApplicationViewModel,
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_commands_execute_without_an_application() {
        let view_model = ApplicationViewModel::new();
        assert!(view_model.exit_command().can_execute(None));
        view_model.exit_command().execute(None);
        view_model.restore_default().execute(None);
        assert!(view_model.restore_default().can_execute(None));
    }
}
