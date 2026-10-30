//! Port of `Core/DebounceAction.cs`.

use super::{attach_actions_to_logical_tree, detach_actions_from_logical_tree};
use ferroui_base::animation::TimeSpan;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledElementImplExt, StyledProperty,
};
use ferroui_xaml_interactivity::{ActionCollection, Interaction, StyledElementAction, StyledElementActionImpl};
use std::cell::RefCell;
use std::rc::Rc;

/// An action that will execute its child actions after a specified delay. If invoked again before
/// the delay elapses, the timer is reset.
///
/// While the action is in the logical tree, its styled element actions are its logical children.
#[repr(C)]
pub struct DebounceAction {
    base: StyledElementAction,
    timer: RefCell<Option<(Rc<DispatcherTimer>, Rc<dyn IDisposable>)>>,
    last_sender: RefCell<Option<BoxedValue>>,
    last_parameter: RefCell<Option<BoxedValue>>,
}

ferro_class!(DebounceAction: StyledElementAction);
ferro_class_info!(DebounceAction {
    new: DebounceAction::new,
    markup: {
        content: Actions,
    },
});

ferro_properties! {
    impl DebounceAction {
        /// Identifies the `Delay` property.
        pub fn delay_property() -> StyledProperty<TimeSpan> {
            FerroProperty::register::<DebounceAction, _>("Delay", TimeSpan::ZERO)
        }

        /// Identifies the `Actions` property.
        pub fn actions_property() -> StyledProperty<Option<ActionCollection>> {
            FerroProperty::register::<DebounceAction, _>("Actions", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for DebounceAction {
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
    impl StyledElementImpl for DebounceAction {
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

impl StyledElementActionImpl for DebounceAction {
    /// Starts (or restarts) the delay; the child actions run with the sender and the parameter
    /// of the last invocation when it elapses. Returns nothing.
    ///
    /// # Panics
    /// Panics when the delay is negative (the argument exception of the timer of the reference).
    fn execute(this: &Self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        *this.last_sender.borrow_mut() = sender.cloned();
        *this.last_parameter.borrow_mut() = parameter.cloned();

        let timer = {
            let mut timer = this.timer.borrow_mut();
            let (timer, _) = timer.get_or_insert_with(|| {
                let new_timer = DispatcherTimer::new();
                let action = this.to_ref().downgrade();
                let tick = new_timer.tick(move |_timer: &DispatcherTimer| {
                    if let Some(action) = action.upgrade() {
                        action.timer_tick();
                    }
                });
                (new_timer, tick)
            });
            timer.clone()
        };

        let Some(interval) = this.delay().to_duration() else {
            panic!("TimeSpan period must be greater than or equal to zero. (Parameter 'value')");
        };
        timer.set_interval(interval);
        timer.stop();
        timer.start();

        None
    }
}

impl DebounceAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementAction::construct(),
            timer: RefCell::new(None),
            last_sender: RefCell::new(None),
            last_parameter: RefCell::new(None),
        }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the delay to wait before executing the actions. This is a property of the
    /// object model.
    pub fn delay(&self) -> TimeSpan {
        self.get_value(Self::delay_property())
    }

    pub fn set_delay(&self, value: TimeSpan) {
        self.set_value(Self::delay_property(), value)
    }

    /// Gets or sets the collection of actions to execute. This is a property of the object model.
    pub fn actions(&self) -> Option<ActionCollection> {
        self.get_value(Self::actions_property())
    }

    pub fn set_actions(&self, value: Option<ActionCollection>) {
        self.set_value(Self::actions_property(), value)
    }

    fn timer_tick(&self) {
        let timer = self.timer.borrow().as_ref().map(|(timer, _)| timer.clone());
        if let Some(timer) = timer {
            timer.stop();
        }

        let sender = self.last_sender.borrow_mut().take();
        let parameter = self.last_parameter.borrow_mut().take();
        Interaction::execute_actions(sender.as_ref(), self.actions().as_ref(), parameter.as_ref());
    }
}
