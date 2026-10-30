//! Port of `ViewModels/DragDropCommandsViewModel.cs`.

use super::{Command, ViewModelBase};
use ferroui_base::ferro_markup_type;
use ferroui_base::input::ICommand;
use ferroui_reactiveui::reactive_object;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The view model of the page of the drag and drop commands: the status the commands of the drag
/// events write.
pub struct DragDropCommandsViewModel {
    base: ViewModelBase,
    status: RefCell<String>,
    enter_command: RefCell<Option<Rc<dyn ICommand>>>,
    over_command: RefCell<Option<Rc<dyn ICommand>>>,
    leave_command: RefCell<Option<Rc<dyn ICommand>>>,
    drop_command: RefCell<Option<Rc<dyn ICommand>>>,
}
reactive_object!(DragDropCommandsViewModel, base);
super::reactive_accessors!(DragDropCommandsViewModel {
    status, set_status, "Status": String;
});

impl PartialEq for DragDropCommandsViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// A command that sets the status of the view model (`ReactiveCommand.Create<DragEventArgs>(_ => Status = status)`).
fn status_command(
    view_model: &Weak<DragDropCommandsViewModel>,
    status: &'static str,
) -> Option<Rc<dyn ICommand>> {
    let view_model = view_model.clone();
    Some(Command::create_with(move |_| {
        if let Some(view_model) = view_model.upgrade() {
            view_model.set_status(status.to_string());
        }
    }))
}

impl DragDropCommandsViewModel {
    /// `DragDropCommandsViewModel()`.
    pub fn new() -> Rc<Self> {
        let view_model = Rc::new(Self {
            base: ViewModelBase::new(),
            status: RefCell::new("Waiting".to_string()),
            enter_command: RefCell::new(None),
            over_command: RefCell::new(None),
            leave_command: RefCell::new(None),
            drop_command: RefCell::new(None),
        });
        let weak = Rc::downgrade(&view_model);
        *view_model.enter_command.borrow_mut() = status_command(&weak, "DragEnter");
        *view_model.over_command.borrow_mut() = status_command(&weak, "DragOver");
        *view_model.leave_command.borrow_mut() = status_command(&weak, "DragLeave");
        *view_model.drop_command.borrow_mut() = status_command(&weak, "Dropped");
        view_model
    }
}

type Vm = Rc<DragDropCommandsViewModel>;

ferro_markup_type!(class DragDropCommandsViewModel {
    this: Rc<DragDropCommandsViewModel>,
    handles: [DragDropCommandsViewModel, Rc<DragDropCommandsViewModel>, Option<Rc<DragDropCommandsViewModel>>],
    constructors: [() => DragDropCommandsViewModel::new],
    properties: [
        Status: String { get: |this: &Vm| this.status(), set: |this: &Vm, value: String| this.set_status(value) },
        EnterCommand: Option<Rc<dyn ICommand>> { get: |this: &Vm| this.enter_command.borrow().clone() },
        OverCommand: Option<Rc<dyn ICommand>> { get: |this: &Vm| this.over_command.borrow().clone() },
        LeaveCommand: Option<Rc<dyn ICommand>> { get: |this: &Vm| this.leave_command.borrow().clone() },
        DropCommand: Option<Rc<dyn ICommand>> { get: |this: &Vm| this.drop_command.borrow().clone() },
    ],
    notify_property_changed: DragDropCommandsViewModel,
});
