//! Port of `ViewModels/PointerTriggersViewModel.cs`.

use super::{Command, ViewModelBase};
use ferroui_base::input::ICommand;
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_reactiveui::reactive_object;
use std::rc::Rc;

/// The view model of the page of the pointer triggers: commands that write the positions of the
/// pointer.
pub struct PointerTriggersViewModel {
    base: ViewModelBase,
    pointer_pressed_command: Rc<dyn ICommand>,
    pointer_released_command: Rc<dyn ICommand>,
    pointer_moved_command: Rc<dyn ICommand>,
}
reactive_object!(PointerTriggersViewModel, base);

impl PartialEq for PointerTriggersViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// The point `(X, Y)` the pointer converter passes, as the value tuple of the reference prints.
fn point_text(point: Option<&BoxedValue>) -> String {
    match point.and_then(|point| point.downcast_ref::<(f64, f64)>()) {
        Some((x, y)) => format!("({x}, {y})"),
        None => "()".to_string(),
    }
}

impl PointerTriggersViewModel {
    /// `PointerTriggersViewModel()`.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            pointer_pressed_command: Command::create_with(Self::pointer_pressed),
            pointer_released_command: Command::create_with(Self::pointer_released),
            pointer_moved_command: Command::create_with(Self::pointer_moved),
        })
    }

    /// `PointerPressed((double X, double Y))`.
    fn pointer_pressed(point: Option<&BoxedValue>) {
        println!("Pressed: {}", point_text(point));
    }

    /// `PointerReleased((double X, double Y))`.
    fn pointer_released(point: Option<&BoxedValue>) {
        println!("Released: {}", point_text(point));
    }

    /// `PointerMoved((double X, double Y))`.
    fn pointer_moved(point: Option<&BoxedValue>) {
        println!("Moved: {}", point_text(point));
    }
}

type Vm = Rc<PointerTriggersViewModel>;

ferro_markup_type!(class PointerTriggersViewModel {
    this: Rc<PointerTriggersViewModel>,
    handles: [PointerTriggersViewModel, Rc<PointerTriggersViewModel>, Option<Rc<PointerTriggersViewModel>>],
    constructors: [() => PointerTriggersViewModel::new],
    properties: [
        PointerPressedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.pointer_pressed_command.clone() },
        PointerReleasedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.pointer_released_command.clone() },
        PointerMovedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.pointer_moved_command.clone() },
    ],
    notify_property_changed: PointerTriggersViewModel,
});
