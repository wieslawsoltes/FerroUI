//! Port of `RenderTarget/RenderTargetBitmapTrigger.cs`.

use crate::IRenderTargetBitmapRenderHost;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::utilities::EventArgs;
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

/// Trigger that calls [`IRenderTargetBitmapRenderHost::render`] periodically.
///
/// The args of the tick (empty event args) are the parameter of the actions.
#[repr(C)]
pub struct RenderTargetBitmapTrigger {
    base: StyledElementTrigger,
    timer: RefCell<Option<(Rc<DispatcherTimer>, Rc<dyn IDisposable>)>>,
}

ferro_class!(RenderTargetBitmapTrigger: StyledElementTrigger);
ferro_class_info!(RenderTargetBitmapTrigger {
    new: RenderTargetBitmapTrigger::new,
    markup: {
        property_attributes: [
            Target: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(RenderTargetBitmapTrigger: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl RenderTargetBitmapTrigger {
        /// Identifies the `Target` property.
        pub fn target_property() -> StyledProperty<Option<Rc<dyn IRenderTargetBitmapRenderHost>>> {
            FerroProperty::register::<RenderTargetBitmapTrigger, _>("Target", None)
        }

        /// Identifies the `MillisecondsPerTick` property.
        pub fn milliseconds_per_tick_property() -> StyledProperty<i32> {
            FerroProperty::register::<RenderTargetBitmapTrigger, _>("MillisecondsPerTick", 16)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for RenderTargetBitmapTrigger {
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

impl RenderTargetBitmapTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTrigger::construct(), timer: RefCell::new(None) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the render host that should be rendered. This is a property of the object
    /// model; markup resolves a name given for it to the named host.
    pub fn target(&self) -> Option<Rc<dyn IRenderTargetBitmapRenderHost>> {
        self.get_value(Self::target_property())
    }

    pub fn set_target(&self, value: Option<Rc<dyn IRenderTargetBitmapRenderHost>>) {
        self.set_value(Self::target_property(), value)
    }

    /// Gets or sets the interval, in milliseconds, between render ticks. This is a property of
    /// the object model.
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
        if let Some(target) = self.target() {
            target.render();
        }

        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        let parameter: BoxedValue = Rc::new(EventArgs::EMPTY);
        Interaction::execute_actions(sender.as_ref(), self.actions().as_ref(), Some(&parameter));
    }
}
