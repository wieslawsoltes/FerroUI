//! Port of `Core/DelayAction.cs`.

use crate::action_task::{delay, ActionTask};
use ferroui_base::animation::SignedDuration;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;
use std::time::Duration;

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
        ///
        /// A duration as long as the longest time span (`TimeSpan.MaxValue`) or longer, such as
        /// [`Duration::MAX`], waits forever: the `Timeout.InfiniteTimeSpan` of the reference
        /// (minus one millisecond) is a negative time span, which is not a `Duration`
        /// (DEVIATIONS.md, Behaviors).
        pub fn duration_property() -> StyledProperty<Duration> {
            FerroProperty::register::<DelayAction, _>("Duration", Duration::ZERO)
        }
    }
}

impl StyledElementActionImpl for DelayAction {
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let duration = this.duration();
        // The longest durations wait forever (`Timeout.InfiniteTimeSpan` of the reference): the
        // conversion to a time span saturates at the longest one.
        if SignedDuration::from(duration) == SignedDuration::MAX_VALUE {
            return Some(Rc::new(ActionTask::spawn(std::future::pending::<()>())));
        }

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

    /// Gets or sets the duration to wait; [`Duration::MAX`] waits forever (see
    /// [`duration_property`](Self::duration_property)). This is a property of the object model.
    pub fn duration(&self) -> Duration {
        self.get_value(Self::duration_property())
    }

    pub fn set_duration(&self, value: Duration) {
        self.set_value(Self::duration_property(), value)
    }
}
