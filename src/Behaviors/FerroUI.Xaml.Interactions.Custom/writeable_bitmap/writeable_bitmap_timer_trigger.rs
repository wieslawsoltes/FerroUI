//! Port of `WriteableBitmap/WriteableBitmapTimerTrigger.cs`.

use crate::WriteableBitmapRef;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{
    Interaction, StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementTrigger,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// A trigger that fires its actions on a timer and passes a
/// [`WriteableBitmap`](ferroui_base::media::imaging::WriteableBitmap) as parameter.
#[repr(C)]
pub struct WriteableBitmapTimerTrigger {
    base: StyledElementTrigger,
    timer: RefCell<Option<(Rc<DispatcherTimer>, Rc<dyn IDisposable>)>>,
}

ferro_class!(WriteableBitmapTimerTrigger: StyledElementTrigger);
ferro_class_info!(WriteableBitmapTimerTrigger {
    new: WriteableBitmapTimerTrigger::new,
    markup: {
        property_attributes: [
            Bitmap: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(WriteableBitmapTimerTrigger: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl WriteableBitmapTimerTrigger {
        /// Identifies the `Bitmap` property.
        pub fn bitmap_property() -> StyledProperty<Option<WriteableBitmapRef>> {
            FerroProperty::register::<WriteableBitmapTimerTrigger, _>("Bitmap", None)
        }

        /// Identifies the `MillisecondsPerTick` property.
        pub fn milliseconds_per_tick_property() -> StyledProperty<i32> {
            FerroProperty::register::<WriteableBitmapTimerTrigger, _>("MillisecondsPerTick", 16)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for WriteableBitmapTimerTrigger {
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

impl WriteableBitmapTimerTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTrigger::construct(), timer: RefCell::new(None) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the bitmap passed to actions. This is a property of the object model.
    pub fn bitmap(&self) -> Option<WriteableBitmapRef> {
        self.get_value(Self::bitmap_property())
    }

    pub fn set_bitmap(&self, value: Option<WriteableBitmapRef>) {
        self.set_value(Self::bitmap_property(), value)
    }

    /// Gets or sets the timer interval in milliseconds. This is a property of the object model.
    pub fn milliseconds_per_tick(&self) -> i32 {
        self.get_value(Self::milliseconds_per_tick_property())
    }

    pub fn set_milliseconds_per_tick(&self, value: i32) {
        self.set_value(Self::milliseconds_per_tick_property(), value)
    }

    fn start_timer(&self) {
        if self.timer.borrow().is_some() {
            return;
        }

        let timer = DispatcherTimer::new();
        timer.set_interval(Duration::from_millis(self.milliseconds_per_tick().max(0) as u64));
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
        if let Some((timer, tick)) = timer {
            tick.dispose();
            timer.stop();
        }
    }

    fn on_tick(&self) {
        if !self.is_enabled() {
            return;
        }

        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        let parameter = self.bitmap().map(|bitmap| Rc::new(bitmap) as BoxedValue);
        Interaction::execute_actions(sender.as_ref(), self.actions().as_ref(), parameter.as_ref());
    }
}
