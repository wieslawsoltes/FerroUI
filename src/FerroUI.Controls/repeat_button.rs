use crate::primitives::TemplatedControlImpl;
use crate::{Button, ButtonImpl, ContentControlImpl, ControlImpl};
use ferroui_base::input::{
    InputElementImpl, InputElementImplExt, Key, KeyEventArgs, MouseButton, PointerPressedEventArgs,
    PointerReleasedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// Represents a control that raises its `Click` event repeatedly when it is
/// pressed and held.
#[repr(C)]
pub struct RepeatButton {
    base: Button,
    repeat_timer: RefCell<Option<Rc<DispatcherTimer>>>,
}

ferro_class!(RepeatButton: Button);
ferroui_base::ferro_class_info!(RepeatButton { new: RepeatButton::new });
ferro_impl_classes!(
    RepeatButton: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    ButtonImpl
);

impl FerroObjectImpl for RepeatButton {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Button::is_pressed_property().as_property() && !change.get_new_value::<bool>() {
            this.stop_timer();
        }
    }
}

impl InputElementImpl for RepeatButton {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);

        if e.key == Key::Space {
            this.start_timer();
        }
    }

    fn on_key_up(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_up(this, e);

        this.stop_timer();
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);

        if e.get_current_point(Some(this)).properties.is_left_button_pressed {
            this.start_timer();
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);

        if e.initial_press_mouse_button() == MouseButton::Left {
            this.stop_timer();
        }
    }
}

ferroui_base::ferro_properties! { impl RepeatButton {
    ferro_property!(
        /// Defines the `Interval` property.
        pub fn interval_property() -> StyledProperty<i32> {
            FerroProperty::register::<RepeatButton, _>("Interval", 100)
        }
    );

    ferro_property!(
        /// Defines the `Delay` property.
        pub fn delay_property() -> StyledProperty<i32> {
            FerroProperty::register::<RepeatButton, _>("Delay", 300)
        }
    );
} }

impl RepeatButton {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Button::construct(), repeat_timer: RefCell::new(None) }
    }

    /// Creates a repeat button.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The amount of time, in milliseconds, of repeating clicks.
    pub fn interval(&self) -> i32 {
        self.get_value(Self::interval_property())
    }

    pub fn set_interval(&self, value: i32) {
        self.set_value(Self::interval_property(), value)
    }

    /// The amount of time, in milliseconds, to wait before repeating begins.
    pub fn delay(&self) -> i32 {
        self.get_value(Self::delay_property())
    }

    pub fn set_delay(&self, value: i32) {
        self.set_value(Self::delay_property(), value)
    }

    /// A time span of the given number of milliseconds; a negative number
    /// is taken as zero.
    fn from_milliseconds(milliseconds: i32) -> Duration {
        Duration::from_millis(u64::try_from(milliseconds).unwrap_or(0))
    }

    fn start_timer(&self) {
        let existing = self.repeat_timer.borrow().clone();
        let repeat_timer = match existing {
            Some(repeat_timer) => repeat_timer,
            None => {
                let repeat_timer = DispatcherTimer::new();
                let weak = self.to_ref().downgrade();
                // The subscription lives as long as the timer.
                let _ = repeat_timer.tick(move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.repeat_timer_on_tick();
                    }
                });
                *self.repeat_timer.borrow_mut() = Some(repeat_timer.clone());
                repeat_timer
            }
        };

        if repeat_timer.is_enabled() {
            return;
        }

        repeat_timer.set_interval(Self::from_milliseconds(self.delay()));
        repeat_timer.start();
    }

    fn repeat_timer_on_tick(&self) {
        let interval = Self::from_milliseconds(self.interval());
        let repeat_timer = self.repeat_timer.borrow().clone();
        if let Some(repeat_timer) = repeat_timer {
            if repeat_timer.interval() != interval {
                repeat_timer.set_interval(interval);
            }
        }
        self.on_click();
    }

    fn stop_timer(&self) {
        let repeat_timer = self.repeat_timer.borrow().clone();
        if let Some(repeat_timer) = repeat_timer {
            repeat_timer.stop();
        }
    }
}

/// A running timer is kept alive by the dispatcher: stop it with the button.
impl Drop for RepeatButton {
    fn drop(&mut self) {
        if let Some(repeat_timer) = self.repeat_timer.get_mut().take() {
            repeat_timer.stop();
        }
    }
}
