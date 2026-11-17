use crate::NormalizeResult;
use crate::composition_animated_custom_visual_handler::CompositionAnimatedCustomVisualHandler;
use crate::feedback::{Feedback, Report};
use crate::visual_payload::{PlaybackSettings, VisualCommand, VisualPayload};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{MediaExtensions, Stretch, StretchDirection};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::rendering::composition::{
    CompositionCustomVisual, ElementComposition, ICompositionCustomVisualHandler,
};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElementImpl, StyledProperty, Vector,
    Visual, VisualImpl, ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    instantiate,
};
use ferroui_controls::{Control, ControlImpl, ControlImplExt};
use ferroui_system::task::AbortHandle;
use skia_safe::Canvas;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

/// The content of a composition animation. All hooks run on the compositor's
/// render side. Implementations may own thread-affine rendering state.
pub trait CompositionAnimation: 'static {
    fn source_size(&self) -> Size;

    /// Declare a fixed loop length, or return `None` and report loop boundaries
    /// through `on_normalize_elapsed`.
    fn duration(&self) -> Option<Duration> {
        None
    }

    fn on_normalize_elapsed(&mut self, elapsed: Duration) -> NormalizeResult {
        NormalizeResult::new(elapsed, false)
    }

    fn on_render(&mut self, canvas: &Canvas, destination: Rect, position: Duration, running: bool);

    fn on_update(&mut self, _delta: Duration) {}
    fn on_started(&mut self) {}
    fn on_stopped(&mut self) {}
    fn on_disposed(&mut self) {}
}

/// Creates content on the render side each time the control attaches or the
/// animation source changes.
pub type CompositionAnimationFactory =
    Arc<dyn Fn() -> Result<Box<dyn CompositionAnimation>, String> + Send + Sync>;

/// A Skia composition control that can draw static or animated content.
#[repr(C)]
pub struct CompositionAnimatedControl {
    base: Control,
    visual: RefCell<Option<CompositionCustomVisual>>,
    factory: RefCell<Option<CompositionAnimationFactory>>,
    autoplay: Cell<bool>,
    feedback_task: RefCell<Option<AbortHandle>>,
    generation: Cell<u64>,
    revision: Cell<u64>,
    source_size: Cell<Size>,
    source_duration: Cell<Duration>,
    status: RefCell<Option<String>>,
    update: HandlerList<dyn Fn(Duration)>,
    started: HandlerList<dyn Fn()>,
    stopped: HandlerList<dyn Fn()>,
    disposed: HandlerList<dyn Fn()>,
}

ferro_class!(CompositionAnimatedControl: Control);
ferro_class_info!(CompositionAnimatedControl {
    new: CompositionAnimatedControl::new,
    markup: {
        namespace: "FerroUI.Controls.Lottie",
        properties: [
            Position: Duration { get: CompositionAnimatedControl::position },
        ],
    },
});
ferro_impl_classes!(CompositionAnimatedControl: StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl);

ferro_properties! {
    impl CompositionAnimatedControl {
        pub fn stretch_property() -> StyledProperty<Stretch> {
            FerroProperty::register::<Self, _>("Stretch", Stretch::Uniform)
        }
        pub fn stretch_direction_property() -> StyledProperty<StretchDirection> {
            FerroProperty::register::<Self, _>("StretchDirection", StretchDirection::Both)
        }
        pub fn repeat_count_property() -> StyledProperty<i32> {
            FerroProperty::register::<Self, _>("RepeatCount", Self::INFINITY)
        }
        pub fn playback_rate_property() -> StyledProperty<f64> {
            FerroProperty::register::<Self, _>("PlaybackRate", 1.0)
        }
        pub fn position_property() -> StyledProperty<Duration> {
            FerroProperty::register::<Self, _>("Position", Duration::ZERO)
        }
        pub fn source_duration_property() -> DirectProperty<CompositionAnimatedControl, Duration> {
            FerroProperty::register_direct::<Self, _>("SourceDuration", Self::source_duration, None, Duration::ZERO)
        }
        pub fn status_property() -> DirectProperty<CompositionAnimatedControl, Option<String>> {
            FerroProperty::register_direct::<Self, _>("Status", Self::status, None, None)
        }
    }
}

impl FerroObjectImpl for CompositionAnimatedControl {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::stretch_property().as_property()
            || property == Self::stretch_direction_property().as_property()
        {
            this.invalidate_measure();
            this.send(VisualCommand::Update);
        } else if property == Self::playback_rate_property().as_property() {
            this.send(VisualCommand::Update);
        } else if property == Self::repeat_count_property().as_property() {
            this.stop();
            this.start();
        } else if property == Visual::bounds_property().as_property() {
            this.update_visual_size();
        }
    }
}

impl LayoutableImpl for CompositionAnimatedControl {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        MediaExtensions::calculate_size(
            this.stretch(),
            available_size,
            this.source_size.get(),
            this.stretch_direction(),
        )
    }

    fn arrange_override(_this: &Self, final_size: Size) -> Size {
        // The child visual uses the actual layout slot as its viewport.
        final_size
    }
}

impl ControlImpl for CompositionAnimatedControl {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);
        let Some(element_visual) = ElementComposition::get_element_visual(this) else {
            return;
        };
        let feedback = Arc::new(Feedback::default());
        let receiver = feedback.clone();
        let weak = this.to_ref().downgrade();
        let task = Dispatcher::ui_thread().spawn_local(async move {
            loop {
                let (generation, report) = receiver.next().await;
                let Some(control) = weak.upgrade() else {
                    return;
                };
                if control.generation.get() == generation {
                    control.apply_report(generation, report);
                }
            }
        });
        *this.feedback_task.borrow_mut() = Some(task.abort_handle());

        let settings = this.settings();
        let visual = element_visual.compositor().create_custom_visual(move || {
            Rc::new(CompositionAnimatedCustomVisualHandler::new(
                settings, feedback,
            )) as Rc<dyn ICompositionCustomVisualHandler>
        });
        ElementComposition::set_element_child_visual(this, Some((*visual).clone()));
        *this.visual.borrow_mut() = Some(visual);
        this.update_visual_size();
        let factory = this.factory.borrow().clone();
        this.send(VisualCommand::Replace(factory, this.autoplay.get()));
    }

    fn on_unloaded(this: &Self, e: &RoutedEventArgs) {
        let visual = this.visual.borrow_mut().take();
        this.generation.set(this.generation.get().wrapping_add(1));
        if let Some(task) = this.feedback_task.borrow_mut().take() {
            task.abort();
        }
        if let Some(visual) = visual {
            ElementComposition::set_element_child_visual(this, None);
            visual.dispose();
        }
        this.reset_source_info();
        this.publish_status(None);
        Self::parent_on_unloaded(this, e);
        for (_, handler) in this.disposed.snapshot().iter() {
            handler();
        }
    }
}

impl Drop for CompositionAnimatedControl {
    fn drop(&mut self) {
        if let Some(task) = self.feedback_task.get_mut().take() {
            task.abort();
        }
    }
}

impl CompositionAnimatedControl {
    pub const INFINITY: i32 = -1;

    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            visual: RefCell::new(None),
            factory: RefCell::new(None),
            autoplay: Cell::new(false),
            feedback_task: RefCell::new(None),
            generation: Cell::new(0),
            revision: Cell::new(0),
            source_size: Cell::new(Size::default()),
            source_duration: Cell::new(Duration::ZERO),
            status: RefCell::new(None),
            update: HandlerList::new(),
            started: HandlerList::new(),
            stopped: HandlerList::new(),
            disposed: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn stretch(&self) -> Stretch {
        self.get_value(Self::stretch_property())
    }

    pub fn set_stretch(&self, stretch: Stretch) {
        self.set_value(Self::stretch_property(), stretch);
    }

    pub fn stretch_direction(&self) -> StretchDirection {
        self.get_value(Self::stretch_direction_property())
    }

    pub fn set_stretch_direction(&self, direction: StretchDirection) {
        self.set_value(Self::stretch_direction_property(), direction);
    }

    pub fn repeat_count(&self) -> i32 {
        self.get_value(Self::repeat_count_property())
    }

    pub fn set_repeat_count(&self, repeats: i32) {
        self.set_value(Self::repeat_count_property(), repeats.max(Self::INFINITY));
    }

    pub fn playback_rate(&self) -> f64 {
        self.get_value(Self::playback_rate_property())
    }

    pub fn set_playback_rate(&self, rate: f64) {
        if rate.is_finite() {
            self.set_value(Self::playback_rate_property(), rate.max(0.0));
        }
    }

    pub fn position(&self) -> Duration {
        self.get_value(Self::position_property())
    }

    pub fn source_duration(&self) -> Duration {
        self.source_duration.get()
    }

    /// A load or rendering message suitable for presenting beside the control.
    pub fn status(&self) -> Option<String> {
        self.status.borrow().clone()
    }

    /// Replaces the current animation. A factory can also be installed before
    /// attachment. `None` clears the visual.
    pub fn set_animation(&self, factory: Option<CompositionAnimationFactory>, autoplay: bool) {
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        *self.factory.borrow_mut() = factory.clone();
        self.autoplay.set(autoplay);
        self.send(VisualCommand::Replace(factory, autoplay));
        self.reset_source_info();
        if self.generation.get() == generation {
            self.publish_status(None);
        }
    }

    pub fn start(&self) {
        self.send_transport(VisualCommand::Start);
    }

    pub fn stop(&self) {
        self.send_transport(VisualCommand::Stop);
    }

    pub fn pause(&self) {
        self.send_transport(VisualCommand::Pause);
    }

    pub fn resume(&self) {
        self.send_transport(VisualCommand::Resume);
    }

    pub fn seek(&self, position: Duration) {
        if self.visual.borrow().is_none() {
            return;
        }
        let duration = self.source_duration();
        let position = if duration.is_zero() {
            position
        } else {
            position.min(duration)
        };
        if self.position() != position {
            self.send_transport(VisualCommand::Seek(position));
            self.set_value(Self::position_property(), position);
        }
    }

    pub fn redraw(&self) {
        self.send(VisualCommand::Redraw);
    }

    pub fn add_update(&self, handler: impl Fn(Duration) + 'static) -> Subscription {
        self.subscribe(
            |this| &this.update,
            Rc::new(handler) as Rc<dyn Fn(Duration)>,
        )
    }

    pub fn add_started(&self, handler: impl Fn() + 'static) -> Subscription {
        self.subscribe(|this| &this.started, Rc::new(handler) as Rc<dyn Fn()>)
    }

    pub fn add_stopped(&self, handler: impl Fn() + 'static) -> Subscription {
        self.subscribe(|this| &this.stopped, Rc::new(handler) as Rc<dyn Fn()>)
    }

    pub fn add_disposed(&self, handler: impl Fn() + 'static) -> Subscription {
        self.subscribe(|this| &this.disposed, Rc::new(handler) as Rc<dyn Fn()>)
    }

    fn subscribe<F: ?Sized + 'static>(
        &self,
        select: fn(&Self) -> &HandlerList<F>,
        handler: Rc<F>,
    ) -> Subscription {
        let token = select(self).add(handler);
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                select(&this).remove(token);
            }
        }))
    }

    pub(crate) fn publish_status(&self, status: Option<String>) {
        self.set_and_raise(Self::status_property(), &self.status, status);
    }

    fn reset_source_info(&self) {
        self.source_size.set(Size::default());
        self.invalidate_measure();
        self.set_value(Self::position_property(), Duration::ZERO);
        self.set_and_raise_cell(
            Self::source_duration_property(),
            &self.source_duration,
            Duration::ZERO,
        );
    }

    fn apply_report(&self, generation: u64, report: Report) {
        match report {
            Report::Ready { size, duration } => {
                self.source_size.set(size);
                self.invalidate_measure();
                self.set_and_raise_cell(
                    Self::source_duration_property(),
                    &self.source_duration,
                    duration,
                );
            }
            Report::LoadError(message) => self.publish_status(Some(message)),
            Report::Renderer(available) => {
                let status = if available {
                    "Ready"
                } else {
                    "Lottie playback requires the Skia renderer."
                };
                self.publish_status(Some(status.into()));
            }
            Report::Frame {
                revision,
                position,
                delta,
            } => {
                if self.revision.get() == revision {
                    self.set_value(Self::position_property(), position);
                }
                if !delta.is_zero() && self.generation.get() == generation {
                    for (_, handler) in self.update.snapshot().iter() {
                        handler(delta);
                    }
                }
            }
            Report::Started => {
                for (_, handler) in self.started.snapshot().iter() {
                    handler();
                }
            }
            Report::Stopped => {
                for (_, handler) in self.stopped.snapshot().iter() {
                    handler();
                }
            }
        }
    }

    fn settings(&self) -> PlaybackSettings {
        let playback_rate = self.playback_rate();
        PlaybackSettings {
            stretch: self.stretch(),
            stretch_direction: self.stretch_direction(),
            repeat_count: self.repeat_count().max(Self::INFINITY),
            playback_rate: if playback_rate.is_finite() {
                playback_rate.max(0.0)
            } else {
                1.0
            },
        }
    }

    fn update_visual_size(&self) {
        let visual = self.visual.borrow().clone();
        if let Some(visual) = visual {
            let size = self.bounds().size();
            visual
                .visual_props()
                .set_size(&**visual, Vector::new(size.width, size.height));
            self.send(VisualCommand::Redraw);
        }
    }

    fn send_transport(&self, command: VisualCommand) {
        self.revision.set(self.revision.get().wrapping_add(1));
        self.send(command);
    }

    fn send(&self, command: VisualCommand) {
        let visual = self.visual.borrow().clone();
        if let Some(visual) = visual {
            visual.send_handler_message(Arc::new(VisualPayload {
                command,
                generation: self.generation.get(),
                revision: self.revision.get(),
                settings: self.settings(),
            }));
        }
    }
}
