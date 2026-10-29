//! Port of `Core/TimerTrigger.cs`.

use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::utilities::EventArgs;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{
    EventTriggerBase, EventTriggerBaseImpl, Interaction, StyledElementBehaviorImpl, StyledElementBehaviorImplExt,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// A trigger that invokes its actions after a specified interval.
///
/// The timer starts when the trigger is attached; the event of the trigger (see
/// [`EventTriggerBase`]) restarts it.
#[repr(C)]
pub struct TimerTrigger {
    base: EventTriggerBase,
    timer: RefCell<Option<(Rc<DispatcherTimer>, Rc<dyn IDisposable>)>>,
    current_tick: Cell<i32>,
}

ferro_class!(TimerTrigger: EventTriggerBase);
ferro_class_info!(TimerTrigger { new: TimerTrigger::new });
ferroui_base::ferro_impl_classes!(TimerTrigger: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl TimerTrigger {
        /// Identifies the `MillisecondsPerTick` property.
        pub fn milliseconds_per_tick_property() -> StyledProperty<i32> {
            FerroProperty::register::<TimerTrigger, _>("MillisecondsPerTick", 1000)
        }

        /// Identifies the `TotalTicks` property.
        pub fn total_ticks_property() -> StyledProperty<i32> {
            FerroProperty::register::<TimerTrigger, _>("TotalTicks", 1)
        }

        /// Identifies the `RepeatForever` property.
        pub fn repeat_forever_property() -> StyledProperty<bool> {
            FerroProperty::register::<TimerTrigger, _>("RepeatForever", false)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for TimerTrigger {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);
            this.start_timer();
        }

        fn on_detaching(this: &Self) {
            this.stop_timer();
            Self::parent_on_detaching(this);
        }
    }
}

ferroui_base::ferro_overrides! {
    impl EventTriggerBaseImpl for TimerTrigger {
        /// Restarts the timer.
        fn on_event(this: &Self, _event_args: Option<&BoxedValue>) {
            this.stop_timer();
            this.start_timer();
        }
    }
}

impl TimerTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: EventTriggerBase::construct(), timer: RefCell::new(None), current_tick: Cell::new(0) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the time, in milliseconds, between timer ticks.
    pub fn milliseconds_per_tick(&self) -> i32 {
        self.get_value(Self::milliseconds_per_tick_property())
    }

    pub fn set_milliseconds_per_tick(&self, value: i32) {
        self.set_value(Self::milliseconds_per_tick_property(), value)
    }

    /// Gets or sets the number of ticks after which the trigger stops firing.
    pub fn total_ticks(&self) -> i32 {
        self.get_value(Self::total_ticks_property())
    }

    pub fn set_total_ticks(&self, value: i32) {
        self.set_value(Self::total_ticks_property(), value)
    }

    /// Gets or sets a value indicating whether the timer repeats indefinitely.
    pub fn repeat_forever(&self) -> bool {
        self.get_value(Self::repeat_forever_property())
    }

    pub fn set_repeat_forever(&self, value: bool) {
        self.set_value(Self::repeat_forever_property(), value)
    }

    fn start_timer(&self) {
        if self.timer.borrow().is_some() {
            return;
        }

        self.current_tick.set(0);
        let milliseconds_per_tick = self.milliseconds_per_tick();
        if milliseconds_per_tick < 0 {
            panic!("TimeSpan period must be greater than or equal to zero. (Parameter 'value')");
        }

        let timer = DispatcherTimer::new();
        timer.set_interval(Duration::from_millis(milliseconds_per_tick as u64));
        let trigger = self.to_ref().downgrade();
        let tick = timer.tick(move |_timer: &DispatcherTimer| {
            if let Some(trigger) = trigger.upgrade() {
                trigger.on_tick();
            }
        });
        timer.start();
        *self.timer.borrow_mut() = Some((timer, tick));
    }

    fn stop_timer(&self) {
        let timer = self.timer.borrow_mut().take();
        let Some((timer, tick)) = timer else {
            return;
        };

        tick.dispose();
        timer.stop();
    }

    fn on_tick(&self) {
        self.current_tick.set(self.current_tick.get() + 1);
        self.execute(Some(&(Rc::new(EventArgs::EMPTY) as BoxedValue)));

        if !self.repeat_forever() && self.current_tick.get() >= self.total_ticks() {
            self.stop_timer();
        }
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        if !self.is_enabled() {
            return;
        }

        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        Interaction::execute_actions(sender.as_ref(), self.actions().as_ref(), parameter);
    }
}
