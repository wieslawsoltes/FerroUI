//! Port of `ExecuteCommand/Core/ExecuteCommandBehaviorBase.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::ICommand;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, BoxedValue, DirectProperty, ElementRef, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Nullable, Ref, StaticType, StyledElementImpl,
    StyledProperty,
};
use ferroui_controls::{Control, TopLevel};
use ferroui_xaml_interactivity::{
    CommandCanExecuteObserver, DisposingBehaviorImpl, StyledElementBehaviorImpl, StyledElementBehaviorImplExt,
};
use std::cell::{Cell, OnceCell};
use std::rc::Rc;

/// The base class of the behaviors that execute a command (the `ExecuteCommandOn*Behavior`
/// classes): the command, its parameter, whether it can execute, and the element to focus when it
/// runs.
///
/// The class is abstract: a deriving class implements
/// [`AttachedToVisualTreeBehaviorImpl::on_attached_to_visual_tree_override`] to listen for its
/// event while the control is in the visual tree, and calls
/// [`execute_command`](ExecuteCommandBehaviorBaseImpl::execute_command) when it fires.
#[repr(C)]
pub struct ExecuteCommandBehaviorBase {
    base: AttachedToVisualTreeBehavior,
    command_can_execute_observer: OnceCell<CommandCanExecuteObserver>,
    can_execute_command: Cell<bool>,
}

ferro_class! {
    ExecuteCommandBehaviorBase: AttachedToVisualTreeBehavior,
    virtuals ExecuteCommandBehaviorBaseImpl: AttachedToVisualTreeBehaviorImpl {
        /// Executes the associated command.
        ///
        /// Returns true if the command was executed; otherwise, false.
        fn execute_command(this) -> bool;
    }
}

ferro_class_info!(ExecuteCommandBehaviorBase {
    markup: {
        property_attributes: [
            FocusControl: [ResolveByName],
            SourceControl: [ResolveByName],
        ],
    },
});

ferroui_base::ferro_impl_classes!(
    ExecuteCommandBehaviorBase: StyledElementImpl,
    DisposingBehaviorImpl,
    AttachedToVisualTreeBehaviorImpl,
);

impl ExecuteCommandBehaviorBaseImpl for ExecuteCommandBehaviorBase {
    fn execute_command(this: &Self) -> bool {
        if !this.is_enabled() {
            return false;
        }

        let Some(associated_object) = this.associated_object_as::<Control>() else {
            return false;
        };
        if !associated_object.is_visible() || !associated_object.is_enabled() {
            return false;
        }

        let command_parameter = this.command_parameter();
        let Some(command) = this.command().filter(|command| command.can_execute(command_parameter.as_ref())) else {
            return false;
        };

        if this.focus_top_level() {
            let behavior: Ref<ExecuteCommandBehaviorBase> = this.to_ref();
            Dispatcher::ui_thread().post_local(
                move || {
                    let top_level = behavior.top_level().or_else(|| {
                        behavior
                            .associated_object_as::<Control>()
                            .and_then(|associated_object| associated_object.get_self_and_logical_ancestors().last())
                            .and_then(|root| root.cast::<TopLevel>())
                    });
                    if let Some(top_level) = top_level {
                        top_level.focus();
                    }
                },
                DispatcherPriority::DEFAULT,
            );
        }

        if let Some(focus_control) = this.focus_control() {
            Dispatcher::ui_thread().post_local(
                move || {
                    focus_control.focus();
                },
                DispatcherPriority::DEFAULT,
            );
        }

        command.execute(command_parameter.as_ref());
        true
    }
}

ferro_properties! {
    impl ExecuteCommandBehaviorBase {
        /// Identifies the `TopLevel` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own the top level, so the
        /// value is an element reference.
        pub fn top_level_property() -> StyledProperty<Option<ElementRef<TopLevel>>> {
            ValueTypes::register_element_ref::<TopLevel>();
            FerroProperty::register::<ExecuteCommandBehaviorBase, _>("TopLevel", None)
        }

        /// Identifies the `Command` property.
        pub fn command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register::<ExecuteCommandBehaviorBase, _>("Command", None)
        }

        /// Identifies the `CanExecuteCommand` property.
        pub fn can_execute_command_property() -> DirectProperty<ExecuteCommandBehaviorBase, bool> {
            FerroProperty::register_direct::<ExecuteCommandBehaviorBase, _>(
                "CanExecuteCommand",
                |behavior| behavior.can_execute_command(),
                None,
                false,
            )
        }

        /// Identifies the `CommandParameter` property.
        pub fn command_parameter_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ExecuteCommandBehaviorBase, _>("CommandParameter", None)
        }

        /// Identifies the `FocusTopLevel` property.
        pub fn focus_top_level_property() -> StyledProperty<bool> {
            FerroProperty::register::<ExecuteCommandBehaviorBase, _>("FocusTopLevel", false)
        }

        /// Identifies the `FocusControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the reference registers this property under the
        /// name `CommandParameter` (`nameof(CommandParameter)`), a second property of that name;
        /// here it is registered as `FocusControl` (defect corrected). The behavior does not own
        /// the control, so the value is an element reference.
        pub fn focus_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<ExecuteCommandBehaviorBase, _>("FocusControl", None)
        }

        /// Identifies the `SourceControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own the control, so the
        /// value is an element reference.
        pub fn source_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<ExecuteCommandBehaviorBase, _>("SourceControl", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for ExecuteCommandBehaviorBase {
        /// The constructor body: the observer reports to the `CanExecuteCommand` property.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);

            let behavior = this.to_ref().downgrade();
            let _ = this.command_can_execute_observer.set(CommandCanExecuteObserver::new(move |value| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.set_can_execute_command(value);
                }
            }));
        }

        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            let property = change.property();
            if property == Self::command_property().as_property()
                || property == Self::command_parameter_property().as_property()
            {
                if let Some(observer) = this.command_can_execute_observer.get() {
                    observer.update(this.command(), this.command_parameter());
                }
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ExecuteCommandBehaviorBase {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);

            if let Some(observer) = this.command_can_execute_observer.get() {
                observer.start(this.command(), this.command_parameter());
            }
        }

        fn on_detaching(this: &Self) {
            if let Some(observer) = this.command_can_execute_observer.get() {
                observer.stop();
            }

            Self::parent_on_detaching(this);
        }
    }
}

impl ExecuteCommandBehaviorBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: AttachedToVisualTreeBehavior::construct(<Control as StaticType>::TYPE),
            command_can_execute_observer: OnceCell::new(),
            can_execute_command: Cell::new(true),
        }
    }

    /// Gets or sets the top level to focus when [`focus_top_level`](Self::focus_top_level) is
    /// set; when it is null, the root of the logical tree of the associated control.
    pub fn top_level(&self) -> Option<Ref<TopLevel>> {
        ElementRef::resolve(&self.get_value(Self::top_level_property()))
    }

    pub fn set_top_level(&self, value: impl Into<Nullable<TopLevel>>) {
        self.set_value(Self::top_level_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the command to execute.
    pub fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::command_property())
    }

    pub fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::command_property(), value)
    }

    /// Gets a value indicating whether [`command`](Self::command) can execute with the current
    /// [`command_parameter`](Self::command_parameter).
    pub fn can_execute_command(&self) -> bool {
        self.can_execute_command.get()
    }

    fn set_can_execute_command(&self, value: bool) {
        self.set_and_raise_cell(Self::can_execute_command_property(), &self.can_execute_command, value);
    }

    /// Gets or sets the parameter of the command.
    pub fn command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::command_parameter_property())
    }

    pub fn set_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::command_parameter_property(), value)
    }

    /// Gets or sets a value indicating whether the top level is focused when the command runs.
    pub fn focus_top_level(&self) -> bool {
        self.get_value(Self::focus_top_level_property())
    }

    pub fn set_focus_top_level(&self, value: bool) {
        self.set_value(Self::focus_top_level_property(), value)
    }

    /// Gets or sets the control focused when the command runs. Markup resolves a name given for
    /// it to the named element.
    pub fn focus_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::focus_control_property()))
    }

    pub fn set_focus_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::focus_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the control whose event the behavior listens for; when it is null, the
    /// associated control. Markup resolves a name given for it to the named element.
    pub fn source_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::source_control_property()))
    }

    pub fn set_source_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::source_control_property(), ElementRef::from_nullable(value.into().0))
    }
}
