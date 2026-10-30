//! Port of `Control/DelayedLoadBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::animation::TimeSpan;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref, StaticType,
    StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::cell::RefCell;
use std::rc::Rc;

/// Hides the associated control when it is attached to the visual tree and shows it again after
/// a delay.
#[repr(C)]
pub struct DelayedLoadBehavior {
    base: AttachedToVisualTreeBehavior,
    // The timer and the subscription of its tick (the reference removes its delegate).
    timer: RefCell<Option<(Rc<DispatcherTimer>, Rc<dyn IDisposable>)>>,
}

ferro_class!(DelayedLoadBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(DelayedLoadBehavior { new: DelayedLoadBehavior::new });
ferroui_base::ferro_impl_classes!(
    DelayedLoadBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl DelayedLoadBehavior {
        /// Identifies the `Delay` property.
        pub fn delay_property() -> StyledProperty<TimeSpan> {
            FerroProperty::register::<DelayedLoadBehavior, _>("Delay", TimeSpan::from_milliseconds(500.0))
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for DelayedLoadBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return DisposableAction::empty();
            };

            associated_object.set_is_visible(false);

            let timer = DispatcherTimer::new();
            timer.set_interval(this.delay().to_duration().unwrap_or_default());
            let behavior = this.to_ref().downgrade();
            let tick = timer.tick(move |_timer: &DispatcherTimer| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.on_tick();
                }
            });
            timer.start();

            // The reference replaces the field without stopping a running timer.
            *this.timer.borrow_mut() = Some((timer, tick));

            let behavior = this.to_ref().downgrade();
            DisposableAction::create(move || {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.dispose_timer();
                }
            })
        }
    }
}

impl DelayedLoadBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<Control as StaticType>::TYPE), timer: RefCell::new(None) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the delay before the associated control is shown. This is a property of the
    /// object model.
    pub fn delay(&self) -> TimeSpan {
        self.get_value(Self::delay_property())
    }

    pub fn set_delay(&self, value: TimeSpan) {
        self.set_value(Self::delay_property(), value)
    }

    fn on_tick(&self) {
        if let Some(associated_object) = self.associated_object_as::<Control>() {
            associated_object.set_current_value(Visual::is_visible_property(), true);
        }
        self.dispose_timer();
    }

    fn dispose_timer(&self) {
        let timer = self.timer.borrow_mut().take();
        if let Some((timer, tick)) = timer {
            tick.dispose();
            timer.stop();
        }
    }
}
