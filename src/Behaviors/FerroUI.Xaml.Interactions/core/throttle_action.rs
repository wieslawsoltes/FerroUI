//! Port of `Core/ThrottleAction.cs`.

use super::{attach_actions_to_logical_tree, detach_actions_from_logical_tree};
use ferroui_base::animation::TimeSpan;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::utilities::DateTime;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledElementImplExt, StyledProperty,
};
use ferroui_xaml_interactivity::{ActionCollection, Interaction, StyledElementAction, StyledElementActionImpl};
use std::cell::Cell;

/// An action that will execute its child actions immediately, but ignores subsequent invocations
/// until a specified interval has passed.
///
/// While the action is in the logical tree, its styled element actions are its logical children.
#[repr(C)]
pub struct ThrottleAction {
    base: StyledElementAction,
    last_execution_time: Cell<DateTime>,
}

ferro_class!(ThrottleAction: StyledElementAction);
ferro_class_info!(ThrottleAction {
    new: ThrottleAction::new,
    markup: {
        content: Actions,
    },
});

ferro_properties! {
    impl ThrottleAction {
        /// Identifies the `Interval` property.
        pub fn interval_property() -> StyledProperty<TimeSpan> {
            FerroProperty::register::<ThrottleAction, _>("Interval", TimeSpan::ZERO)
        }

        /// Identifies the `Actions` property.
        pub fn actions_property() -> StyledProperty<Option<ActionCollection>> {
            FerroProperty::register::<ThrottleAction, _>("Actions", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for ThrottleAction {
        /// The constructor body: the action starts with an empty collection of actions.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);
            this.set_current_value(Self::actions_property(), Some(ActionCollection::new()));
        }

        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::actions_property().as_property() {
                let old_actions = change.get_old_value::<Option<ActionCollection>>().flatten();
                let new_actions = change.get_new_value::<Option<ActionCollection>>();

                if let Some(old_actions) = &old_actions {
                    if this.is_attached_to_logical_tree() {
                        detach_actions_from_logical_tree(Some(old_actions), this);
                    }
                }

                if let Some(new_actions) = &new_actions {
                    if this.is_attached_to_logical_tree() {
                        attach_actions_to_logical_tree(Some(new_actions), this);
                    }
                }
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementImpl for ThrottleAction {
        fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
            Self::parent_on_attached_to_logical_tree(this, e);
            attach_actions_to_logical_tree(this.actions().as_ref(), this);
        }

        fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
            detach_actions_from_logical_tree(this.actions().as_ref(), this);
            Self::parent_on_detached_from_logical_tree(this, e);
        }
    }
}

impl StyledElementActionImpl for ThrottleAction {
    /// Executes the child actions unless they ran less than [`interval`](ThrottleAction::interval)
    /// ago. Returns nothing.
    fn execute(this: &Self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let now = DateTime::now();
        if now - this.last_execution_time.get() >= this.interval() {
            this.last_execution_time.set(now);
            Interaction::execute_actions(sender, this.actions().as_ref(), parameter);
        }

        None
    }
}

impl ThrottleAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct(), last_execution_time: Cell::new(DateTime::MIN_VALUE) }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the minimum interval between executions. This is a property of the object
    /// model.
    pub fn interval(&self) -> TimeSpan {
        self.get_value(Self::interval_property())
    }

    pub fn set_interval(&self, value: TimeSpan) {
        self.set_value(Self::interval_property(), value)
    }

    /// Gets or sets the collection of actions to execute. This is a property of the object model.
    pub fn actions(&self) -> Option<ActionCollection> {
        self.get_value(Self::actions_property())
    }

    pub fn set_actions(&self, value: Option<ActionCollection>) {
        self.set_value(Self::actions_property(), value)
    }
}
