use super::velocity_tracker::VelocityTracker;
use super::{GestureRecognizer, GestureRecognizerImpl};
use crate::input::{
    IPointer, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
    ScrollGestureEndedEventArgs, ScrollGestureEventArgs, ScrollGestureInertiaStartingEventArgs,
};
use crate::media::MediaContext;
use crate::platform::IPlatformSettings;
use crate::threading::{Dispatcher, DispatcherPriority};
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, DirectProperty, DirectPropertyMetadata,
    FerroLocator, FerroObjectImpl, FerroProperty, LocatorExtensions, Point, Ref, Size, StyledElementImpl, Vector,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

fn default_scroll_start_distance() -> i32 {
    let height = FerroLocator::current()
        .get_service::<dyn IPlatformSettings>()
        .map_or(10.0, |settings| settings.get_tap_size(PointerType::Touch).height);
    (height / 2.0) as i32
}

/// Recognizes the scroll gesture: a touch or pen drag, optionally followed
/// by inertia.
#[repr(C)]
pub struct ScrollGestureRecognizer {
    base: GestureRecognizer,
    can_horizontally_scroll: Cell<bool>,
    can_vertically_scroll: Cell<bool>,
    is_scroll_inertia_enabled: Cell<bool>,
    offset: Cell<Option<Vector>>,
    viewport: Cell<Option<Size>>,
    extent: Cell<Option<Size>>,
    scroll_start_distance: Cell<i32>,

    scrolling: Cell<bool>,
    tracked_root_point: Cell<Point>,
    tracking: RefCell<Option<Rc<dyn IPointer>>>,
    /// Whether an inertia animation is running, and the frame time it
    /// started at (set by the first frame).
    inertia_running: Cell<bool>,
    gesture_id: Cell<i32>,
    pointer_pressed_point: Cell<Point>,
    velocity_tracker: RefCell<Option<VelocityTracker>>,

    /// Movement per second.
    inertia: Cell<Option<Vector>>,
    last_move_timestamp: Cell<Option<u64>>,
    last_time: Cell<Option<Duration>>,
    inertia_start_time: Cell<Option<Duration>>,
    current_inertia_gesture_id: Cell<i32>,
    delta: Cell<Point>,
}

ferro_class!(ScrollGestureRecognizer: GestureRecognizer);
crate::ferro_class_info!(ScrollGestureRecognizer { new: ScrollGestureRecognizer::new });
ferro_impl_classes!(ScrollGestureRecognizer: FerroObjectImpl, StyledElementImpl);

impl GestureRecognizerImpl for ScrollGestureRecognizer {
    fn pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        let point = e.get_current_point(None);

        if matches!(e.pointer().type_(), PointerType::Touch | PointerType::Pen)
            && point.properties.is_left_button_pressed
        {
            this.end_gesture();
            drop(this.tracking.replace(Some(e.pointer().clone())));
            this.inertia.set(None);
            this.gesture_id.set(ScrollGestureEventArgs::get_next_free_id());
            this.tracked_root_point.set(point.position);
            this.pointer_pressed_point.set(point.position);

            let mut tracker = VelocityTracker::new();
            tracker.add_position(Duration::from_millis(e.timestamp()), Vector::default());
            *this.velocity_tracker.borrow_mut() = Some(tracker);
        }
    }

    fn pointer_moved(this: &Self, e: &PointerEventArgs) {
        if !this.is_tracking(e.pointer()) {
            return;
        }

        let root_point = e.get_position(None);
        let start_distance = this.scroll_start_distance() as f64;

        if !this.scrolling.get() {
            let tracked = this.tracked_root_point.get();
            let (offset, viewport, extent) = (this.offset(), this.viewport(), this.extent());

            // At the end of the scrollable range in the direction of the
            // drag: `offset + viewport - extent == 0`, when all are known.
            let at_end = |offset: Option<f64>, viewport: Option<f64>, extent: Option<f64>| match (offset, viewport, extent)
            {
                (Some(offset), Some(viewport), Some(extent)) => offset + viewport - extent == 0.0,
                _ => false,
            };

            if this.can_vertically_scroll() {
                let delta = tracked.y - root_point.y;

                if offset.map(|o| o.y) == Some(0.0) && delta < 0.0 {
                    return;
                }

                if at_end(offset.map(|o| o.y), viewport.map(|v| v.height), extent.map(|x| x.height)) && delta > 0.0 {
                    return;
                }

                if delta.abs() > start_distance {
                    this.scrolling.set(true);
                }
            }

            if this.can_horizontally_scroll() {
                let delta = tracked.x - root_point.x;

                if offset.map(|o| o.x) == Some(0.0) && delta < 0.0 {
                    return;
                }

                if at_end(offset.map(|o| o.x), viewport.map(|v| v.width), extent.map(|x| x.width)) && delta > 0.0 {
                    return;
                }

                if delta.abs() > start_distance {
                    this.scrolling.set(true);
                }
            }

            if this.scrolling.get() {
                // Correct the tracked point with the start distance, so
                // scrolling does not start with a skip of the start
                // distance.
                this.tracked_root_point.set(Point::new(
                    tracked.x - if tracked.x >= root_point.x { start_distance } else { -start_distance },
                    tracked.y - if tracked.y >= root_point.y { start_distance } else { -start_distance },
                ));
            }
        }

        if this.scrolling.get() {
            let vector = Vector::from(this.tracked_root_point.get() - root_point);

            let old_delta = this.delta.get();
            let pressed = this.pointer_pressed_point.get();
            let delta = Point::new(pressed.x - root_point.x, pressed.y - root_point.y);
            this.delta.set(delta);

            if old_delta == delta {
                return;
            }

            if let Some(tracker) = this.velocity_tracker.borrow_mut().as_mut() {
                tracker.add_position(Duration::from_millis(e.timestamp()), Vector::new(delta.x, delta.y));
            }

            this.last_move_timestamp.set(Some(e.timestamp()));

            let scroll_event_args = ScrollGestureEventArgs::new(this.gesture_id.get(), vector);
            if let Some(target) = this.target() {
                target.raise_event(&scroll_event_args);
            }

            this.tracked_root_point.set(root_point);

            e.set_handled(scroll_event_args.handled());

            if e.handled() {
                this.capture(e.pointer());
            }
        }
    }

    fn pointer_capture_lost(this: &Self, pointer: &Rc<dyn IPointer>) {
        if this.is_tracking(pointer) {
            this.end_gesture();
        }
    }

    fn pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        if !(this.is_tracking(e.pointer()) && this.scrolling.get()) {
            return;
        }

        let inertia = this
            .velocity_tracker
            .borrow()
            .as_ref()
            .map_or(Vector::default(), |tracker| {
                tracker.get_fling_velocity(Duration::from_millis(e.timestamp())).pixels_per_second
            });
        this.inertia.set(Some(inertia));

        e.set_handled(true);

        let last_move = this.last_move_timestamp.get();
        let too_late = last_move.is_some_and(|last| e.timestamp().wrapping_sub(last) > 200);

        if inertia == Vector::default()
            || e.timestamp() == 0
            || last_move == Some(0)
            || too_late
            || !this.is_scroll_inertia_enabled()
        {
            this.end_gesture();
        } else {
            drop(this.tracking.replace(None));
            this.inertia_running.set(true);
            this.last_time.set(None);
            this.inertia_start_time.set(None);
            this.current_inertia_gesture_id.set(this.gesture_id.get());

            if let Some(target) = this.target() {
                target.raise_event(&ScrollGestureInertiaStartingEventArgs::new(this.gesture_id.get(), inertia));
            }

            this.request_animation_frame();
        }
    }
}

impl ScrollGestureRecognizer {
    /// Pixels per second speed that is considered to be the stop of
    /// inertial scroll.
    pub const INERTIAL_SCROLL_SPEED_END: f64 = 5.0;

    /// The resistance applied to inertial scrolling.
    pub const INERTIAL_RESISTANCE: f64 = 0.15;

}

crate::ferro_properties! { impl ScrollGestureRecognizer {
    ferro_property!(
        /// Defines the `CanHorizontallyScroll` property.
        pub fn can_horizontally_scroll_property() -> DirectProperty<ScrollGestureRecognizer, bool> {
            FerroProperty::register_direct::<ScrollGestureRecognizer, _>(
                "CanHorizontallyScroll",
                |o| o.can_horizontally_scroll(),
                Some(|o, v| o.set_can_horizontally_scroll(v)),
                false,
            )
        }
    );

    ferro_property!(
        /// Defines the `CanVerticallyScroll` property.
        pub fn can_vertically_scroll_property() -> DirectProperty<ScrollGestureRecognizer, bool> {
            FerroProperty::register_direct::<ScrollGestureRecognizer, _>(
                "CanVerticallyScroll",
                |o| o.can_vertically_scroll(),
                Some(|o, v| o.set_can_vertically_scroll(v)),
                false,
            )
        }
    );

    ferro_property!(
        /// Defines the `IsScrollInertiaEnabled` property.
        pub fn is_scroll_inertia_enabled_property() -> DirectProperty<ScrollGestureRecognizer, bool> {
            FerroProperty::register_direct::<ScrollGestureRecognizer, _>(
                "IsScrollInertiaEnabled",
                |o| o.is_scroll_inertia_enabled(),
                Some(|o, v| o.set_is_scroll_inertia_enabled(v)),
                false,
            )
        }
    );

    ferro_property!(
        /// Defines the `ScrollStartDistance` property.
        pub fn scroll_start_distance_property() -> DirectProperty<ScrollGestureRecognizer, i32> {
            FerroProperty::register_direct_with::<ScrollGestureRecognizer, _>(
                "ScrollStartDistance",
                |o| o.scroll_start_distance(),
                Some(|o, v| o.set_scroll_start_distance(v)),
                DirectPropertyMetadata::new(Some(default_scroll_start_distance())),
            )
        }
    );

    ferro_property!(
        /// Defines the `Offset` property.
        pub fn offset_property() -> DirectProperty<ScrollGestureRecognizer, Option<Vector>> {
            FerroProperty::register_direct::<ScrollGestureRecognizer, _>(
                "Offset",
                |o| o.offset(),
                Some(|o, v| o.set_offset(v)),
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `Extent` property.
        pub fn extent_property() -> DirectProperty<ScrollGestureRecognizer, Option<Size>> {
            FerroProperty::register_direct::<ScrollGestureRecognizer, _>(
                "Extent",
                |o| o.extent(),
                Some(|o, v| o.set_extent(v)),
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `Viewport` property.
        pub fn viewport_property() -> DirectProperty<ScrollGestureRecognizer, Option<Size>> {
            FerroProperty::register_direct::<ScrollGestureRecognizer, _>(
                "Viewport",
                |o| o.viewport(),
                Some(|o, v| o.set_viewport(v)),
                None,
            )
        }
    );
} }

impl ScrollGestureRecognizer {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: GestureRecognizer::construct(),
            can_horizontally_scroll: Cell::new(false),
            can_vertically_scroll: Cell::new(false),
            is_scroll_inertia_enabled: Cell::new(false),
            offset: Cell::new(None),
            viewport: Cell::new(None),
            extent: Cell::new(None),
            scroll_start_distance: Cell::new(default_scroll_start_distance()),
            scrolling: Cell::new(false),
            tracked_root_point: Cell::new(Point::default()),
            tracking: RefCell::new(None),
            inertia_running: Cell::new(false),
            gesture_id: Cell::new(0),
            pointer_pressed_point: Cell::new(Point::default()),
            velocity_tracker: RefCell::new(None),
            inertia: Cell::new(None),
            last_move_timestamp: Cell::new(None),
            last_time: Cell::new(None),
            inertia_start_time: Cell::new(None),
            current_inertia_gesture_id: Cell::new(0),
            delta: Cell::new(Point::default()),
        })
    }

    /// Whether the content can be scrolled horizontally.
    pub fn can_horizontally_scroll(&self) -> bool {
        self.can_horizontally_scroll.get()
    }

    pub fn set_can_horizontally_scroll(&self, value: bool) {
        self.set_and_raise_cell(Self::can_horizontally_scroll_property(), &self.can_horizontally_scroll, value);
    }

    /// Whether the content can be scrolled vertically.
    pub fn can_vertically_scroll(&self) -> bool {
        self.can_vertically_scroll.get()
    }

    pub fn set_can_vertically_scroll(&self, value: bool) {
        self.set_and_raise_cell(Self::can_vertically_scroll_property(), &self.can_vertically_scroll, value);
    }

    /// Whether scroll inertia is enabled.
    pub fn is_scroll_inertia_enabled(&self) -> bool {
        self.is_scroll_inertia_enabled.get()
    }

    pub fn set_is_scroll_inertia_enabled(&self, value: bool) {
        self.set_and_raise_cell(Self::is_scroll_inertia_enabled_property(), &self.is_scroll_inertia_enabled, value);
    }

    /// The minimum distance the pointer must move before scrolling starts.
    pub fn scroll_start_distance(&self) -> i32 {
        self.scroll_start_distance.get()
    }

    pub fn set_scroll_start_distance(&self, value: i32) {
        self.set_and_raise_cell(Self::scroll_start_distance_property(), &self.scroll_start_distance, value);
    }

    /// The extent of the scrollable content, if known.
    pub fn extent(&self) -> Option<Size> {
        self.extent.get()
    }

    pub fn set_extent(&self, value: Option<Size>) {
        self.set_and_raise_cell(Self::extent_property(), &self.extent, value);
    }

    /// The current scroll offset, if known.
    pub fn offset(&self) -> Option<Vector> {
        self.offset.get()
    }

    pub fn set_offset(&self, value: Option<Vector>) {
        self.set_and_raise_cell(Self::offset_property(), &self.offset, value);
    }

    /// The size of the viewport, if known.
    pub fn viewport(&self) -> Option<Size> {
        self.viewport.get()
    }

    pub fn set_viewport(&self, value: Option<Size>) {
        self.set_and_raise_cell(Self::viewport_property(), &self.viewport, value);
    }

    fn is_tracking(&self, pointer: &Rc<dyn IPointer>) -> bool {
        self.tracking.borrow().as_ref().is_some_and(|t| std::ptr::addr_eq(Rc::as_ptr(t), Rc::as_ptr(pointer)))
    }

    fn end_gesture(&self) {
        drop(self.tracking.replace(None));
        if self.scrolling.get() {
            self.inertia_running.set(false);
            self.inertia.set(Some(Vector::default()));
            self.delta.set(Point::default());
            self.scrolling.set(false);
            drop(self.velocity_tracker.replace(None));
            if let Some(target) = self.target() {
                target.raise_event(&ScrollGestureEndedEventArgs::new(self.gesture_id.get()));
            }
            self.gesture_id.set(0);
            self.last_move_timestamp.set(None);
        }
    }

    fn request_animation_frame(&self) {
        // The animation frames of the top level are not available in the
        // base library, so the global media context is used.
        let weak = self.to_ref().downgrade();
        MediaContext::instance().request_animation_frame(move |frame_time| {
            if let Some(this) = weak.upgrade() {
                this.on_animation_requested(frame_time.to_duration().unwrap_or_default());
            }
        });
    }

    fn on_animation_requested(&self, frame_time: Duration) {
        // Calculate the current speed and dispatch the next inertia event.
        // This is done asynchronously so the events run with input
        // priority.
        let weak = self.to_ref().downgrade();
        drop(Dispatcher::current_dispatcher().invoke_async_local_with_priority(
            move || {
                if let Some(this) = weak.upgrade() {
                    this.inertia_tick(frame_time);
                }
            },
            DispatcherPriority::INPUT,
        ));
    }

    fn inertia_tick(&self, frame_time: Duration) {
        // Another gesture has started, finish the current one
        let Some(inertia) = self.inertia.get() else { return };
        if self.gesture_id.get() != self.current_inertia_gesture_id.get() || !self.inertia_running.get() {
            return;
        }

        // The inertia clock starts with the first frame.
        let start = *self.inertia_start_time.get().get_or_insert(frame_time);
        self.inertia_start_time.set(Some(start));
        let last_time = self.last_time.get().unwrap_or(start);

        let elapsed_since_last_tick = frame_time.saturating_sub(last_time);
        self.last_time.set(Some(frame_time));

        let speed =
            inertia * Self::INERTIAL_RESISTANCE.powf(frame_time.saturating_sub(start).as_secs_f64());
        let distance = speed * elapsed_since_last_tick.as_secs_f64();
        let scroll_gesture_event_args = ScrollGestureEventArgs::new(self.gesture_id.get(), distance);
        if let Some(target) = self.target() {
            target.raise_event(&scroll_gesture_event_args);
        }

        if !scroll_gesture_event_args.handled() || scroll_gesture_event_args.should_end_scroll_gesture() {
            self.end_gesture();
            return;
        }

        // End the gesture using the inertial scroll speed end only in the
        // direction of scrolling
        let (vertical, horizontal) = (self.can_vertically_scroll(), self.can_horizontally_scroll());
        if vertical
            && horizontal
            && speed.x.abs() < Self::INERTIAL_SCROLL_SPEED_END
            && speed.y.abs() <= Self::INERTIAL_SCROLL_SPEED_END
        {
            // NO-OP
        } else if vertical && speed.y.abs() <= Self::INERTIAL_SCROLL_SPEED_END {
            self.end_gesture();
            return;
        } else if horizontal && speed.x.abs() < Self::INERTIAL_SCROLL_SPEED_END {
            self.end_gesture();
            return;
        }

        // Reschedule on the next animation frame.
        self.request_animation_frame();
    }
}
