//! Port of `Input/InactivityTrigger.cs`.

use ferroui_base::animation::TimeSpan;
use ferroui_base::input::{InputElement, KeyEventArgs, PointerEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StaticType, StyledProperty, Visual, WeakRef,
};
use ferroui_controls::{Control, TopLevel};
use ferroui_xaml_interactivity::{BehaviorImpl, BehaviorImplExt, Interaction, TriggerOfT};
use std::cell::RefCell;
use std::rc::Rc;

/// The top level a trigger listens to and its handlers.
struct TopLevelHandlers {
    top_level: WeakRef<TopLevel>,
    pointer_moved: RoutedEventHandlerToken,
    key_down: RoutedEventHandlerToken,
}

/// A trigger that fires when the user has been inactive (no mouse/keyboard input) for a specified
/// duration.
#[repr(C)]
pub struct InactivityTrigger {
    base: TriggerOfT,
    timer: RefCell<Option<(Rc<DispatcherTimer>, Rc<dyn IDisposable>)>>,
    top_level: RefCell<Option<TopLevelHandlers>>,
}

ferro_class!(InactivityTrigger: TriggerOfT);
ferro_class_info!(InactivityTrigger { new: InactivityTrigger::new });
ferroui_base::ferro_impl_classes!(InactivityTrigger: FerroObjectImpl);

ferro_properties! {
    impl InactivityTrigger {
        /// Identifies the `Timeout` property.
        pub fn timeout_property() -> StyledProperty<TimeSpan> {
            FerroProperty::register::<InactivityTrigger, _>("Timeout", TimeSpan::from_seconds(5.0))
        }
    }
}

ferroui_base::ferro_overrides! {
    impl BehaviorImpl for InactivityTrigger {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);

            let top_level = this.associated_object_as::<Control>().and_then(|control| {
                let visual: &Visual = &control;
                TopLevel::get_top_level(Some(visual))
            });
            let Some(top_level) = top_level else {
                return;
            };

            let timer = DispatcherTimer::new();
            timer.set_interval(this.timeout().to_duration().unwrap_or_default());
            let trigger = this.to_ref().downgrade();
            let tick = timer.tick(move |_timer: &DispatcherTimer| {
                if let Some(trigger) = trigger.upgrade() {
                    trigger.timer_tick();
                }
            });

            // Listen to the global events on the top level.
            let trigger = this.to_ref().downgrade();
            let pointer_moved = top_level.add_handler_with(
                InputElement::pointer_moved_event(),
                move |_sender: &Interactive, _e: &PointerEventArgs| {
                    if let Some(trigger) = trigger.upgrade() {
                        trigger.on_input();
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
            let trigger = this.to_ref().downgrade();
            let key_down = top_level.add_handler_with(
                InputElement::key_down_event(),
                move |_sender: &Interactive, _e: &KeyEventArgs| {
                    if let Some(trigger) = trigger.upgrade() {
                        trigger.on_input();
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );

            timer.start();

            // The reference replaces the fields without stopping a running timer.
            *this.timer.borrow_mut() = Some((timer, tick));
            *this.top_level.borrow_mut() =
                Some(TopLevelHandlers { top_level: top_level.downgrade(), pointer_moved, key_down });
        }

        fn on_detached_from_visual_tree(this: &Self) {
            Self::parent_on_detached_from_visual_tree(this);

            let timer = this.timer.borrow_mut().take();
            if let Some((timer, tick)) = timer {
                timer.stop();
                tick.dispose();
            }

            let handlers = this.top_level.borrow_mut().take();
            if let Some(handlers) = handlers {
                if let Some(top_level) = handlers.top_level.upgrade() {
                    top_level.remove_handler(InputElement::pointer_moved_event(), handlers.pointer_moved);
                    top_level.remove_handler(InputElement::key_down_event(), handlers.key_down);
                }
            }
        }
    }
}

impl InactivityTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TriggerOfT::construct(<Control as StaticType>::TYPE),
            timer: RefCell::new(None),
            top_level: RefCell::new(None),
        }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the inactivity timeout duration.
    pub fn timeout(&self) -> TimeSpan {
        self.get_value(Self::timeout_property())
    }

    pub fn set_timeout(&self, value: TimeSpan) {
        self.set_value(Self::timeout_property(), value)
    }

    fn on_input(&self) {
        self.reset_timer();
    }

    fn reset_timer(&self) {
        let timer = self.timer.borrow().as_ref().map(|(timer, _)| timer.clone());
        if let Some(timer) = timer {
            timer.stop();
            // Update the interval in case the property changed.
            timer.set_interval(self.timeout().to_duration().unwrap_or_default());
            timer.start();
        }
    }

    fn timer_tick(&self) {
        let timer = self.timer.borrow().as_ref().map(|(timer, _)| timer.clone());
        if let Some(timer) = timer {
            timer.stop();
        }

        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        Interaction::execute_actions(sender.as_ref(), Some(&self.actions()), None);
    }
}
