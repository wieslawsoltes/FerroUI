//! Port of `Actions/DelayedShowControlAction.cs`.

use super::control_of;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl, FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty, Visual};
use ferroui_controls::Control;
use ferroui_base::animation::TimeSpan;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::DispatcherTimer;
use std::cell::RefCell;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Shows the target control (or the sender) after a delay.
#[repr(C)]
pub struct DelayedShowControlAction {
    base: StyledElementAction,
}

ferro_class!(DelayedShowControlAction: StyledElementAction);
ferro_class_info!(DelayedShowControlAction {
    new: DelayedShowControlAction::new,
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(DelayedShowControlAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl DelayedShowControlAction {
        /// Identifies the `Delay` property.
        pub fn delay_property() -> StyledProperty<TimeSpan> {
            FerroProperty::register::<DelayedShowControlAction, _>("Delay", TimeSpan::from_milliseconds(500.0))
        }

        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own its target, so the value
        /// is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<DelayedShowControlAction, _>("TargetControl", None)
        }
    }
}

impl StyledElementActionImpl for DelayedShowControlAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let control = this.target_control().or_else(|| control_of(sender));
        let Some(control) = control else {
            return Some(Rc::new(false));
        };

        // The subscription of the tick, which the handler removes (`timer.Tick -= OnTick`).
        let tick: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
        let timer = DispatcherTimer::new();
        timer.set_interval(this.delay().to_duration().unwrap_or_default());
        let on_tick = {
            let tick = tick.clone();
            move |timer: &DispatcherTimer| {
                let subscription = tick.borrow_mut().take();
                if let Some(subscription) = subscription {
                    subscription.dispose();
                }
                timer.stop();
                control.set_current_value(Visual::is_visible_property(), true);
            }
        };
        *tick.borrow_mut() = Some(timer.tick(on_tick));
        timer.start();

        Some(Rc::new(true))
    }
}

impl DelayedShowControlAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the target control. If not set, the sender is used. Markup resolves a name
    /// given for it to the named element.
    pub fn target_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::target_control_property()))
    }

    pub fn set_target_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::target_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the delay before the control is shown. This is a property of the object model.
    pub fn delay(&self) -> TimeSpan {
        self.get_value(Self::delay_property())
    }

    pub fn set_delay(&self, value: TimeSpan) {
        self.set_value(Self::delay_property(), value)
    }
}
