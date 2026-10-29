//! Port of `Core/TaskCompletedTrigger.cs`.

use crate::action_task::{ActionTask, CatchUnwind};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTrigger};
use std::rc::Rc;

/// A trigger that invokes its actions when the supplied task completes.
///
/// The task is an [`ActionTask`] (the `Task` of the reference); the actions run in a job of the
/// dispatcher after it completed, whether it ran to the end, failed or was canceled.
#[repr(C)]
pub struct TaskCompletedTrigger {
    base: StyledElementTrigger,
}

ferro_class!(TaskCompletedTrigger: StyledElementTrigger);
ferro_class_info!(TaskCompletedTrigger { new: TaskCompletedTrigger::new });
ferroui_base::ferro_impl_classes!(TaskCompletedTrigger: StyledElementImpl, StyledElementBehaviorImpl);

ferro_properties! {
    impl TaskCompletedTrigger {
        /// Identifies the `Task` property.
        pub fn task_property() -> StyledProperty<Option<ActionTask>> {
            FerroProperty::register::<TaskCompletedTrigger, _>("Task", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for TaskCompletedTrigger {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::task_property().as_property() {
                this.on_task_changed(change.get_new_value::<Option<ActionTask>>());
            }
        }
    }
}

impl TaskCompletedTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTrigger::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the task monitored by the trigger.
    pub fn task(&self) -> Option<ActionTask> {
        self.get_value(Self::task_property())
    }

    pub fn set_task(&self, value: Option<ActionTask>) {
        self.set_value(Self::task_property(), value)
    }

    fn on_task_changed(&self, task: Option<ActionTask>) {
        let Some(task) = task else {
            return;
        };

        // `task.ContinueWith(..)`: the continuation runs whatever the outcome of the task.
        let trigger: Ref<TaskCompletedTrigger> = self.to_ref();
        drop(ActionTask::spawn(async move {
            let _ = CatchUnwind::new(task).await;
            Dispatcher::ui_thread().post_local(
                move || {
                    let sender = trigger
                        .associated_object()
                        .map(|associated_object| Rc::new(associated_object) as BoxedValue);
                    Interaction::execute_actions(sender.as_ref(), trigger.actions().as_ref(), None);
                },
                DispatcherPriority::DEFAULT,
            );
        }));
    }
}
