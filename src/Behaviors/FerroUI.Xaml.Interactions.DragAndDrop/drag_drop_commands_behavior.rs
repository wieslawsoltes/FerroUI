//! Port of `DragDropCommandsBehavior.cs`.

use crate::values::boxed_event_args;
use crate::{DragAndDropEventsBehavior, DragAndDropEventsBehaviorImpl};
use ferroui_base::input::{DragEventArgs, ICommand};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, DirectProperty, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{CommandCanExecuteObserver, StyledElementBehaviorImpl, StyledElementBehaviorImplExt};
use std::cell::{Cell, OnceCell};
use std::rc::Rc;

/// Behavior that exposes commands for drag-and-drop events: each drag event executes its command
/// with the event args (or no parameter), and the `can_execute_*` properties tell whether each
/// command can execute.
#[repr(C)]
pub struct DragDropCommandsBehavior {
    base: DragAndDropEventsBehavior,
    drag_enter_command_can_execute_observer: OnceCell<CommandCanExecuteObserver>,
    drag_over_command_can_execute_observer: OnceCell<CommandCanExecuteObserver>,
    drag_leave_command_can_execute_observer: OnceCell<CommandCanExecuteObserver>,
    drop_command_can_execute_observer: OnceCell<CommandCanExecuteObserver>,
    can_execute_drag_enter_command: Cell<bool>,
    can_execute_drag_over_command: Cell<bool>,
    can_execute_drag_leave_command: Cell<bool>,
    can_execute_drop_command: Cell<bool>,
    pass_event_args_to_command: Cell<bool>,
}

ferro_class!(DragDropCommandsBehavior: DragAndDropEventsBehavior);
ferro_class_info!(DragDropCommandsBehavior {
    new: DragDropCommandsBehavior::new,
    markup: {
        properties: [
            PassEventArgsToCommand: bool {
                get: |this: &Ref<DragDropCommandsBehavior>| this.pass_event_args_to_command(),
                set: |this: &Ref<DragDropCommandsBehavior>, value: bool| this.set_pass_event_args_to_command(value)
            },
        ],
    },
});
ferroui_base::ferro_impl_classes!(DragDropCommandsBehavior: StyledElementImpl);

ferro_properties! {
    impl DragDropCommandsBehavior {
        /// Identifies the `DragEnterCommand` property.
        pub fn drag_enter_command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register::<DragDropCommandsBehavior, _>("DragEnterCommand", None)
        }

        /// Identifies the `DragOverCommand` property.
        pub fn drag_over_command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register::<DragDropCommandsBehavior, _>("DragOverCommand", None)
        }

        /// Identifies the `DragLeaveCommand` property.
        pub fn drag_leave_command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register::<DragDropCommandsBehavior, _>("DragLeaveCommand", None)
        }

        /// Identifies the `DropCommand` property.
        pub fn drop_command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register::<DragDropCommandsBehavior, _>("DropCommand", None)
        }

        /// Identifies the `CanExecuteCommandParameter` property.
        pub fn can_execute_command_parameter_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<DragDropCommandsBehavior, _>("CanExecuteCommandParameter", None)
        }

        /// Identifies the `CanExecuteDragEnterCommand` property.
        pub fn can_execute_drag_enter_command_property() -> DirectProperty<DragDropCommandsBehavior, bool> {
            FerroProperty::register_direct::<DragDropCommandsBehavior, _>(
                "CanExecuteDragEnterCommand",
                |behavior| behavior.can_execute_drag_enter_command(),
                None,
                false,
            )
        }

        /// Identifies the `CanExecuteDragOverCommand` property.
        pub fn can_execute_drag_over_command_property() -> DirectProperty<DragDropCommandsBehavior, bool> {
            FerroProperty::register_direct::<DragDropCommandsBehavior, _>(
                "CanExecuteDragOverCommand",
                |behavior| behavior.can_execute_drag_over_command(),
                None,
                false,
            )
        }

        /// Identifies the `CanExecuteDragLeaveCommand` property.
        pub fn can_execute_drag_leave_command_property() -> DirectProperty<DragDropCommandsBehavior, bool> {
            FerroProperty::register_direct::<DragDropCommandsBehavior, _>(
                "CanExecuteDragLeaveCommand",
                |behavior| behavior.can_execute_drag_leave_command(),
                None,
                false,
            )
        }

        /// Identifies the `CanExecuteDropCommand` property.
        pub fn can_execute_drop_command_property() -> DirectProperty<DragDropCommandsBehavior, bool> {
            FerroProperty::register_direct::<DragDropCommandsBehavior, _>(
                "CanExecuteDropCommand",
                |behavior| behavior.can_execute_drop_command(),
                None,
                false,
            )
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for DragDropCommandsBehavior {
        /// The constructor body: each observer reports to its `can_execute_*` property.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);

            let behavior = this.to_ref().downgrade();
            let _ = this.drag_enter_command_can_execute_observer.set(CommandCanExecuteObserver::new(move |value| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.set_can_execute_drag_enter_command(value);
                }
            }));
            let behavior = this.to_ref().downgrade();
            let _ = this.drag_over_command_can_execute_observer.set(CommandCanExecuteObserver::new(move |value| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.set_can_execute_drag_over_command(value);
                }
            }));
            let behavior = this.to_ref().downgrade();
            let _ = this.drag_leave_command_can_execute_observer.set(CommandCanExecuteObserver::new(move |value| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.set_can_execute_drag_leave_command(value);
                }
            }));
            let behavior = this.to_ref().downgrade();
            let _ = this.drop_command_can_execute_observer.set(CommandCanExecuteObserver::new(move |value| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.set_can_execute_drop_command(value);
                }
            }));
        }

        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            let property = change.property();
            if property == Self::drag_enter_command_property().as_property() {
                if let Some(observer) = this.drag_enter_command_can_execute_observer.get() {
                    observer.update_with(
                        this.drag_enter_command(),
                        this.resolve_can_execute_parameter(),
                        this.is_can_execute_parameter_known(),
                    );
                }
            } else if property == Self::drag_over_command_property().as_property() {
                if let Some(observer) = this.drag_over_command_can_execute_observer.get() {
                    observer.update_with(
                        this.drag_over_command(),
                        this.resolve_can_execute_parameter(),
                        this.is_can_execute_parameter_known(),
                    );
                }
            } else if property == Self::drag_leave_command_property().as_property() {
                if let Some(observer) = this.drag_leave_command_can_execute_observer.get() {
                    observer.update_with(
                        this.drag_leave_command(),
                        this.resolve_can_execute_parameter(),
                        this.is_can_execute_parameter_known(),
                    );
                }
            } else if property == Self::drop_command_property().as_property() {
                if let Some(observer) = this.drop_command_can_execute_observer.get() {
                    observer.update_with(
                        this.drop_command(),
                        this.resolve_can_execute_parameter(),
                        this.is_can_execute_parameter_known(),
                    );
                }
            } else if property == Self::can_execute_command_parameter_property().as_property() {
                this.update_can_execute_observers();
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for DragDropCommandsBehavior {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);

            this.start_can_execute_observers();
        }

        fn on_detaching(this: &Self) {
            for observer in this.observers().into_iter().flatten() {
                observer.stop();
            }

            Self::parent_on_detaching(this);
        }
    }
}

impl DragAndDropEventsBehaviorImpl for DragDropCommandsBehavior {
    fn on_drag_enter(this: &Self, _sender: Option<&BoxedValue>, e: &DragEventArgs) {
        this.execute_command(this.drag_enter_command(), e);
    }

    fn on_drag_over(this: &Self, _sender: Option<&BoxedValue>, e: &DragEventArgs) {
        this.execute_command(this.drag_over_command(), e);
    }

    fn on_drag_leave(this: &Self, _sender: Option<&BoxedValue>, e: &DragEventArgs) {
        this.execute_command(this.drag_leave_command(), e);
    }

    fn on_drop(this: &Self, _sender: Option<&BoxedValue>, e: &DragEventArgs) {
        this.execute_command(this.drop_command(), e);
    }
}

impl DragDropCommandsBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: DragAndDropEventsBehavior::construct(),
            drag_enter_command_can_execute_observer: OnceCell::new(),
            drag_over_command_can_execute_observer: OnceCell::new(),
            drag_leave_command_can_execute_observer: OnceCell::new(),
            drop_command_can_execute_observer: OnceCell::new(),
            can_execute_drag_enter_command: Cell::new(true),
            can_execute_drag_over_command: Cell::new(true),
            can_execute_drag_leave_command: Cell::new(true),
            can_execute_drop_command: Cell::new(true),
            pass_event_args_to_command: Cell::new(true),
        }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the command invoked on drag enter.
    pub fn drag_enter_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::drag_enter_command_property())
    }

    pub fn set_drag_enter_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::drag_enter_command_property(), value)
    }

    /// Gets or sets the command invoked on drag over.
    pub fn drag_over_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::drag_over_command_property())
    }

    pub fn set_drag_over_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::drag_over_command_property(), value)
    }

    /// Gets or sets the command invoked on drag leave.
    pub fn drag_leave_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::drag_leave_command_property())
    }

    pub fn set_drag_leave_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::drag_leave_command_property(), value)
    }

    /// Gets or sets the command invoked on drop.
    pub fn drop_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::drop_command_property())
    }

    pub fn set_drop_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::drop_command_property(), value)
    }

    /// Gets or sets the parameter used to evaluate the drag-and-drop commands before
    /// event-specific parameters are available. This property does not change the parameter
    /// passed to command execution.
    pub fn can_execute_command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::can_execute_command_parameter_property())
    }

    pub fn set_can_execute_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::can_execute_command_parameter_property(), value)
    }

    /// Gets a value indicating whether [`drag_enter_command`](Self::drag_enter_command) can execute
    /// with the current can-execute parameter.
    pub fn can_execute_drag_enter_command(&self) -> bool {
        self.can_execute_drag_enter_command.get()
    }

    fn set_can_execute_drag_enter_command(&self, value: bool) {
        self.set_and_raise_cell(Self::can_execute_drag_enter_command_property(), &self.can_execute_drag_enter_command, value);
    }

    /// Gets a value indicating whether [`drag_over_command`](Self::drag_over_command) can execute
    /// with the current can-execute parameter.
    pub fn can_execute_drag_over_command(&self) -> bool {
        self.can_execute_drag_over_command.get()
    }

    fn set_can_execute_drag_over_command(&self, value: bool) {
        self.set_and_raise_cell(Self::can_execute_drag_over_command_property(), &self.can_execute_drag_over_command, value);
    }

    /// Gets a value indicating whether [`drag_leave_command`](Self::drag_leave_command) can execute
    /// with the current can-execute parameter.
    pub fn can_execute_drag_leave_command(&self) -> bool {
        self.can_execute_drag_leave_command.get()
    }

    fn set_can_execute_drag_leave_command(&self, value: bool) {
        self.set_and_raise_cell(
            Self::can_execute_drag_leave_command_property(),
            &self.can_execute_drag_leave_command,
            value,
        );
    }

    /// Gets a value indicating whether [`drop_command`](Self::drop_command) can execute with the
    /// current can-execute parameter.
    pub fn can_execute_drop_command(&self) -> bool {
        self.can_execute_drop_command.get()
    }

    fn set_can_execute_drop_command(&self, value: bool) {
        self.set_and_raise_cell(Self::can_execute_drop_command_property(), &self.can_execute_drop_command, value);
    }

    /// Specifies whether the event args should be passed to the command.
    pub fn pass_event_args_to_command(&self) -> bool {
        self.pass_event_args_to_command.get()
    }

    pub fn set_pass_event_args_to_command(&self, value: bool) {
        if self.pass_event_args_to_command.get() == value {
            return;
        }

        self.pass_event_args_to_command.set(value);
        self.update_can_execute_observers();
    }

    fn execute_command(&self, command: Option<Rc<dyn ICommand>>, e: &DragEventArgs) {
        let Some(command) = command else {
            return;
        };

        let parameter = if self.pass_event_args_to_command() { Some(boxed_event_args(e)) } else { None };

        if command.can_execute(parameter.as_ref()) {
            command.execute(parameter.as_ref());
        }
    }

    /// The four observers, in the order of the reference.
    fn observers(&self) -> [Option<&CommandCanExecuteObserver>; 4] {
        [
            self.drag_enter_command_can_execute_observer.get(),
            self.drag_over_command_can_execute_observer.get(),
            self.drag_leave_command_can_execute_observer.get(),
            self.drop_command_can_execute_observer.get(),
        ]
    }

    fn commands(&self) -> [Option<Rc<dyn ICommand>>; 4] {
        [self.drag_enter_command(), self.drag_over_command(), self.drag_leave_command(), self.drop_command()]
    }

    fn start_can_execute_observers(&self) {
        let parameter = self.resolve_can_execute_parameter();
        let is_parameter_known = self.is_can_execute_parameter_known();

        for (observer, command) in self.observers().into_iter().zip(self.commands()) {
            if let Some(observer) = observer {
                observer.start_with(command, parameter.clone(), is_parameter_known);
            }
        }
    }

    fn update_can_execute_observers(&self) {
        let parameter = self.resolve_can_execute_parameter();
        let is_parameter_known = self.is_can_execute_parameter_known();

        for (observer, command) in self.observers().into_iter().zip(self.commands()) {
            if let Some(observer) = observer {
                observer.update_with(command, parameter.clone(), is_parameter_known);
            }
        }
    }

    fn resolve_can_execute_parameter(&self) -> Option<BoxedValue> {
        if self.is_set(Self::can_execute_command_parameter_property()) {
            self.can_execute_command_parameter()
        } else {
            None
        }
    }

    fn is_can_execute_parameter_known(&self) -> bool {
        self.is_set(Self::can_execute_command_parameter_property()) || !self.pass_event_args_to_command()
    }
}
