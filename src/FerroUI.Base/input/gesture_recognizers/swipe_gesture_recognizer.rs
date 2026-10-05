use super::{GestureRecognizer, GestureRecognizerImpl};
use crate::input::{
    IPointer, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
    SwipeGestureEndedEventArgs, SwipeGestureEventArgs,
};
use crate::platform::IPlatformSettings;
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroLocator, FerroObjectImpl, FerroProperty,
    LocatorExtensions, Point, Ref, StyledElementImpl, StyledProperty, Vector,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const DEFAULT_TAP_SIZE: f64 = 10.0;

/// A gesture recognizer that detects swipe gestures and raises the swipe
/// gesture event continuously as the pointer moves.
#[repr(C)]
pub struct SwipeGestureRecognizer {
    base: GestureRecognizer,
    swiping: Cell<bool>,
    tracked_root_point: Cell<Point>,
    tracking: RefCell<Option<Rc<dyn IPointer>>>,
    id: Cell<i32>,
    velocity: Cell<Vector>,
    /// The platform time stamp of the last move, in milliseconds.
    last_timestamp: Cell<u64>,
}

ferro_class!(SwipeGestureRecognizer: GestureRecognizer);
crate::ferro_class_info!(SwipeGestureRecognizer { new: SwipeGestureRecognizer::new });
ferro_impl_classes!(SwipeGestureRecognizer: FerroObjectImpl, StyledElementImpl);

impl GestureRecognizerImpl for SwipeGestureRecognizer {
    fn pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        if !this.is_enabled() {
            return;
        }

        let point = e.get_current_point(None);
        let type_ = e.pointer().type_();

        if (matches!(type_, PointerType::Touch | PointerType::Pen)
            || (this.is_mouse_enabled() && type_ == PointerType::Mouse))
            && point.properties.is_left_button_pressed
        {
            this.end_gesture();
            drop(this.tracking.replace(Some(e.pointer().clone())));
            this.id.set(SwipeGestureEventArgs::get_next_free_id());
            this.tracked_root_point.set(point.position);
            this.velocity.set(Vector::default());
            this.last_timestamp.set(0);
        }
    }

    fn pointer_moved(this: &Self, e: &PointerEventArgs) {
        if !this.is_tracking(e.pointer()) {
            return;
        }

        let root_point = e.get_position(None);
        let threshold = this.get_effective_threshold();
        let tracked = this.tracked_root_point.get();

        if !this.swiping.get() {
            let horizontal_triggered = this.can_horizontally_swipe() && (tracked.x - root_point.x).abs() > threshold;
            let vertical_triggered = this.can_vertically_swipe() && (tracked.y - root_point.y).abs() > threshold;

            if horizontal_triggered || vertical_triggered {
                this.swiping.set(true);

                this.tracked_root_point.set(Point::new(
                    if horizontal_triggered {
                        tracked.x - if tracked.x >= root_point.x { threshold } else { -threshold }
                    } else {
                        root_point.x
                    },
                    if vertical_triggered {
                        tracked.y - if tracked.y >= root_point.y { threshold } else { -threshold }
                    } else {
                        root_point.y
                    },
                ));

                this.capture(e.pointer());
            }
        }

        if this.swiping.get() {
            let delta = Vector::from(this.tracked_root_point.get() - root_point);

            let now = e.timestamp();
            if this.last_timestamp.get() > 0 {
                let elapsed_seconds = now.wrapping_sub(this.last_timestamp.get()) as i64 as f64 / 1000.0;
                if elapsed_seconds > 0.0 {
                    let instant_velocity = delta / elapsed_seconds;
                    this.velocity.set(this.velocity.get() * 0.5 + instant_velocity * 0.5);
                }
            }
            this.last_timestamp.set(now);

            if let Some(target) = this.target() {
                target.raise_event(&SwipeGestureEventArgs::new(this.id.get(), delta, this.velocity.get()));
            }
            this.tracked_root_point.set(root_point);
            e.set_handled(true);
        }
    }

    fn pointer_capture_lost(this: &Self, pointer: &Rc<dyn IPointer>) {
        if this.is_tracking(pointer) {
            this.end_gesture();
        }
    }

    fn pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        if this.is_tracking(e.pointer()) && this.swiping.get() {
            e.set_handled(true);
            this.end_gesture();
        }
    }
}

crate::ferro_properties! { impl SwipeGestureRecognizer {
    ferro_property!(
        /// Defines the `CanHorizontallySwipe` property.
        pub fn can_horizontally_swipe_property() -> StyledProperty<bool> {
            FerroProperty::register::<SwipeGestureRecognizer, _>("CanHorizontallySwipe", false)
        }
    );

    ferro_property!(
        /// Defines the `CanVerticallySwipe` property.
        pub fn can_vertically_swipe_property() -> StyledProperty<bool> {
            FerroProperty::register::<SwipeGestureRecognizer, _>("CanVerticallySwipe", false)
        }
    );

    ferro_property!(
        /// Defines the `Threshold` property.
        pub fn threshold_property() -> StyledProperty<f64> {
            FerroProperty::register::<SwipeGestureRecognizer, _>("Threshold", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `IsMouseEnabled` property.
        pub fn is_mouse_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<SwipeGestureRecognizer, _>("IsMouseEnabled", false)
        }
    );

    ferro_property!(
        /// Defines the `IsEnabled` property.
        pub fn is_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<SwipeGestureRecognizer, _>("IsEnabled", true)
        }
    );
} }

impl SwipeGestureRecognizer {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: GestureRecognizer::construct(),
            swiping: Cell::new(false),
            tracked_root_point: Cell::new(Point::default()),
            tracking: RefCell::new(None),
            id: Cell::new(0),
            velocity: Cell::new(Vector::default()),
            last_timestamp: Cell::new(0),
        })
    }

    /// Whether horizontal swipes are tracked.
    pub fn can_horizontally_swipe(&self) -> bool {
        self.get_value(Self::can_horizontally_swipe_property())
    }

    pub fn set_can_horizontally_swipe(&self, value: bool) {
        self.set_value(Self::can_horizontally_swipe_property(), value)
    }

    /// Whether vertical swipes are tracked.
    pub fn can_vertically_swipe(&self) -> bool {
        self.get_value(Self::can_vertically_swipe_property())
    }

    pub fn set_can_vertically_swipe(&self, value: bool) {
        self.set_value(Self::can_vertically_swipe_property(), value)
    }

    /// The minimum pointer movement in pixels before a swipe is recognized.
    /// A value of 0 (the default) uses half of the platform's touch tap
    /// size.
    pub fn threshold(&self) -> f64 {
        self.get_value(Self::threshold_property())
    }

    pub fn set_threshold(&self, value: f64) {
        self.set_value(Self::threshold_property(), value)
    }

    /// Whether mouse pointer events trigger swipe gestures. Defaults to
    /// false; touch and pen are always enabled.
    pub fn is_mouse_enabled(&self) -> bool {
        self.get_value(Self::is_mouse_enabled_property())
    }

    pub fn set_is_mouse_enabled(&self, value: bool) {
        self.set_value(Self::is_mouse_enabled_property(), value)
    }

    /// Whether this recognizer responds to pointer events. Defaults to
    /// true.
    pub fn is_enabled(&self) -> bool {
        self.get_value(Self::is_enabled_property())
    }

    pub fn set_is_enabled(&self, value: bool) {
        self.set_value(Self::is_enabled_property(), value)
    }

    fn is_tracking(&self, pointer: &Rc<dyn IPointer>) -> bool {
        self.tracking.borrow().as_ref().is_some_and(|t| std::ptr::addr_eq(Rc::as_ptr(t), Rc::as_ptr(pointer)))
    }

    fn end_gesture(&self) {
        drop(self.tracking.replace(None));
        if self.swiping.get() {
            self.swiping.set(false);
            let ended_args = SwipeGestureEndedEventArgs::new(self.id.get(), self.velocity.get());
            self.velocity.set(Vector::default());
            self.last_timestamp.set(0);
            self.id.set(0);
            if let Some(target) = self.target() {
                target.raise_event(&ended_args);
            }
        }
    }

    fn get_effective_threshold(&self) -> f64 {
        let configured = self.threshold();
        if configured > 0.0 {
            return configured;
        }

        let tap_size = FerroLocator::current()
            .get_service::<dyn IPlatformSettings>()
            .map_or(DEFAULT_TAP_SIZE, |settings| settings.get_tap_size(PointerType::Touch).height);

        tap_size / 2.0
    }
}
