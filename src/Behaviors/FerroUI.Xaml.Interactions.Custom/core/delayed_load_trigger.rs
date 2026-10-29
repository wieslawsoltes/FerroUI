//! Port of `Core/DelayedLoadTrigger.cs`.

use ferroui_base::animation::TimeSpan;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTriggerOfT};
use std::cell::RefCell;
use std::rc::Rc;

/// Invokes its actions after the associated control is attached to the visual tree and a delay
/// elapses.
#[repr(C)]
pub struct DelayedLoadTrigger {
    base: StyledElementTriggerOfT,
    // The timer and the subscription of its tick (the reference removes its delegate).
    timer: RefCell<Option<(Rc<DispatcherTimer>, Rc<dyn IDisposable>)>>,
}

ferro_class!(DelayedLoadTrigger: StyledElementTriggerOfT);
ferro_class_info!(DelayedLoadTrigger { new: DelayedLoadTrigger::new });
ferroui_base::ferro_impl_classes!(DelayedLoadTrigger: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl DelayedLoadTrigger {
        /// Identifies the `Delay` property.
        pub fn delay_property() -> StyledProperty<TimeSpan> {
            FerroProperty::register::<DelayedLoadTrigger, _>("Delay", TimeSpan::from_milliseconds(500.0))
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for DelayedLoadTrigger {
        fn on_attached_to_visual_tree(this: &Self) {
            let timer = DispatcherTimer::new();
            timer.set_interval(this.delay().to_duration().unwrap_or_default());
            let trigger = this.to_ref().downgrade();
            let tick = timer.tick(move |_timer: &DispatcherTimer| {
                if let Some(trigger) = trigger.upgrade() {
                    trigger.on_tick();
                }
            });
            timer.start();

            // The reference replaces the field without stopping a running timer.
            *this.timer.borrow_mut() = Some((timer, tick));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.dispose_timer();
        }
    }
}

impl DelayedLoadTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTriggerOfT::construct(<Control as StaticType>::TYPE), timer: RefCell::new(None) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the delay before actions are executed.
    pub fn delay(&self) -> TimeSpan {
        self.get_value(Self::delay_property())
    }

    pub fn set_delay(&self, value: TimeSpan) {
        self.set_value(Self::delay_property(), value)
    }

    fn on_tick(&self) {
        self.execute(None);
        self.dispose_timer();
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        if !self.is_enabled() {
            return;
        }

        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        Interaction::execute_actions(sender.as_ref(), self.actions().as_ref(), parameter);
    }

    fn dispose_timer(&self) {
        let timer = self.timer.borrow_mut().take();
        if let Some((timer, tick)) = timer {
            tick.dispose();
            timer.stop();
        }
    }
}
