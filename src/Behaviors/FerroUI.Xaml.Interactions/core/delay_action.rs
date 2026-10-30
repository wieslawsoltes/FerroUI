//! Port of `Core/DelayAction.cs`.

use crate::action_task::{delay, ActionTask};
use ferroui_base::animation::TimeSpan;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that waits for a specified duration.
///
/// The action returns the task of the wait, which an [`AsyncActionGroup`](crate::AsyncActionGroup)
/// in sequence mode awaits before the next action.
#[repr(C)]
pub struct DelayAction {
    base: StyledElementAction,
}

ferro_class!(DelayAction: StyledElementAction);
ferro_class_info!(DelayAction { new: DelayAction::new });
ferroui_base::ferro_impl_classes!(DelayAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl DelayAction {
        /// Identifies the `Duration` property.
        pub fn duration_property() -> StyledProperty<TimeSpan> {
            FerroProperty::register::<DelayAction, _>("Duration", TimeSpan::ZERO)
        }
    }
}

impl StyledElementActionImpl for DelayAction {
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let duration = this.duration();
        // Minus one millisecond waits forever (`Timeout.InfiniteTimeSpan`).
        if duration == TimeSpan::from_milliseconds(-1.0) {
            return Some(Rc::new(ActionTask::spawn(std::future::pending::<()>())));
        }

        let Some(duration) = duration.to_duration() else {
            panic!(
                "The value needs to translate in milliseconds to -1 (signifying an infinite timeout), 0, or a positive \
                 integer less than or equal to the maximum allowed timer duration. (Parameter 'delay')"
            );
        };
        Some(Rc::new(ActionTask::spawn(delay(duration))))
    }
}

impl DelayAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the duration to wait. This is a property of the object model.
    pub fn duration(&self) -> TimeSpan {
        self.get_value(Self::duration_property())
    }

    pub fn set_duration(&self, value: TimeSpan) {
        self.set_value(Self::duration_property(), value)
    }
}
