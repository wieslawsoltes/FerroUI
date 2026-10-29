//! Port of `Core/AsyncActionGroup.cs`.

use super::{attach_actions_to_logical_tree, detach_actions_from_logical_tree};
use crate::action_task::ActionTask;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledElementImplExt,
    StyledProperty,
};
use ferroui_xaml_interactivity::{ActionCollection, IAction, StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Defines the mode of execution for [`AsyncActionGroup`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AsyncActionMode {
    /// Actions are executed sequentially. If an action returns a task, execution waits for it to
    /// complete before moving to the next action.
    #[default]
    Sequence,

    /// Actions are executed in parallel.
    Parallel,
}

impl std::fmt::Display for AsyncActionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Sequence => "Sequence",
            Self::Parallel => "Parallel",
        })
    }
}

/// An action that executes its child actions asynchronously, either in sequence or in parallel.
///
/// While the group is in the logical tree, its styled element actions are its logical children.
#[repr(C)]
pub struct AsyncActionGroup {
    base: StyledElementAction,
}

ferro_class!(AsyncActionGroup: StyledElementAction);
ferro_class_info!(AsyncActionGroup {
    new: AsyncActionGroup::new,
    markup: {
        content: Actions,
    },
});

ferro_properties! {
    impl AsyncActionGroup {
        /// Identifies the `Mode` property.
        pub fn mode_property() -> StyledProperty<AsyncActionMode> {
            FerroProperty::register::<AsyncActionGroup, _>("Mode", AsyncActionMode::Sequence)
        }

        /// Identifies the `Actions` property.
        pub fn actions_property() -> StyledProperty<Option<ActionCollection>> {
            FerroProperty::register::<AsyncActionGroup, _>("Actions", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for AsyncActionGroup {
        /// The constructor body: the group starts with an empty collection of actions.
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
    impl StyledElementImpl for AsyncActionGroup {
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

impl StyledElementActionImpl for AsyncActionGroup {
    /// Executes the child actions and returns the task of their execution ([`ActionTask`]).
    fn execute(this: &Self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        this.actions()?;

        // The execution must return synchronously; the caller who wants to wait for it awaits the
        // task it returns.
        Some(Rc::new(this.execute_async(sender, parameter)))
    }
}

/// The task a result of an action is, if it is one (`result is Task task`).
fn task_of(result: Option<&BoxedValue>) -> Option<ActionTask> {
    result.and_then(|result| result.downcast_ref::<ActionTask>()).cloned()
}

/// Executes the actions of `items` from `start` in sequence, awaiting the task an action returns
/// before the next action.
async fn execute_sequence(
    items: Rc<Vec<Ref<FerroObject>>>,
    start: usize,
    sender: Option<BoxedValue>,
    parameter: Option<BoxedValue>,
) {
    for item in items.iter().skip(start) {
        let result = match <dyn IAction>::of(item) {
            Some(action) => action.execute(sender.as_ref(), parameter.as_ref()),
            None => continue,
        };

        if let Some(task) = task_of(result.as_ref()) {
            let _ = task.await;
        }
    }
}

impl AsyncActionGroup {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the execution mode. This is a property of the object model.
    pub fn mode(&self) -> AsyncActionMode {
        self.get_value(Self::mode_property())
    }

    pub fn set_mode(&self, value: AsyncActionMode) {
        self.set_value(Self::mode_property(), value)
    }

    /// Gets or sets the collection of actions to execute. This is a property of the object model.
    pub fn actions(&self) -> Option<ActionCollection> {
        self.get_value(Self::actions_property())
    }

    pub fn set_actions(&self, value: Option<ActionCollection>) {
        self.set_value(Self::actions_property(), value)
    }

    /// The asynchronous execution of the actions (`ExecuteAsync`): it runs synchronously up to the
    /// first task to wait for, as an asynchronous method does, and the rest runs in jobs of the
    /// dispatcher.
    fn execute_async(&self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> ActionTask {
        let Some(actions) = self.actions() else {
            return ActionTask::completed();
        };

        let items = actions.snapshot();

        if self.mode() == AsyncActionMode::Sequence {
            for (index, item) in items.iter().enumerate() {
                let Some(action) = <dyn IAction>::of(item) else {
                    continue;
                };

                let result = action.execute(sender, parameter);
                if let Some(task) = task_of(result.as_ref()) {
                    let rest = items.clone();
                    let sender = sender.cloned();
                    let parameter = parameter.cloned();
                    return ActionTask::spawn(async move {
                        let _ = task.await;
                        execute_sequence(rest, index + 1, sender, parameter).await;
                    });
                }
            }

            return ActionTask::completed();
        }

        // Parallel.
        let mut tasks = Vec::new();
        for item in items.iter() {
            let Some(action) = <dyn IAction>::of(item) else {
                continue;
            };

            let result = action.execute(sender, parameter);
            if let Some(task) = task_of(result.as_ref()) {
                tasks.push(task);
            }
        }

        if tasks.is_empty() {
            return ActionTask::completed();
        }

        ActionTask::spawn(async move {
            for task in tasks {
                let _ = task.await;
            }
        })
    }
}
