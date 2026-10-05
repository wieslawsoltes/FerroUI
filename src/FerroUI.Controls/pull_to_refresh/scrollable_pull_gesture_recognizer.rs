use ferroui_base::input::gesture_recognizers::{GestureRecognizer, GestureRecognizerImpl};
use ferroui_base::input::{
    IPointer, IScrollable, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
    PullDirection, PullGestureEndedEventArgs, PullGestureEventArgs,
};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Point, Ref,
    StyledElementImpl, StyledProperty, Vector,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Recognizes a pull on a scrollable target that is scrolled to the edge
/// the pull starts from.
#[repr(C)]
pub struct ScrollablePullGestureRecognizer {
    base: GestureRecognizer,
    gesture_id: Cell<i32>,
    pull_in_progress: Cell<bool>,
    delta: f64,
    initial_position: Cell<Point>,
    tracking: RefCell<Option<Rc<dyn IPointer>>>,
    is_mouse_enabled: Cell<bool>,
}

ferro_class!(ScrollablePullGestureRecognizer: GestureRecognizer);
ferroui_base::ferro_class_info!(ScrollablePullGestureRecognizer { new: ScrollablePullGestureRecognizer::new });
ferro_impl_classes!(ScrollablePullGestureRecognizer: FerroObjectImpl, StyledElementImpl);

/// Resets the tracking state of the recognizer when dropped: the `finally`
/// block of the pointer released handler.
struct ReleaseGuard<'a> {
    recognizer: &'a ScrollablePullGestureRecognizer,
    pointer: &'a Rc<dyn IPointer>,
}

impl Drop for ReleaseGuard<'_> {
    fn drop(&mut self) {
        // `handle_pull` captures the pointer on every pointer move with a
        // positive delta. The (true, false) -> `end_pull` transition in the
        // pointer moved handler clears `pull_in_progress` without releasing
        // capture, so by the time we get here the gesture is no longer in
        // progress but the pointer can still be captured by this
        // recognizer. Always release capture so the next gesture starts
        // from a clean state.
        let is_tracking = self.recognizer.tracking.borrow().is_some();
        if is_tracking {
            self.pointer.capture(None);
        }

        self.recognizer.clear_tracking();
    }
}

impl GestureRecognizerImpl for ScrollablePullGestureRecognizer {
    fn pointer_capture_lost(this: &Self, pointer: &Rc<dyn IPointer>) {
        if this.is_tracking(pointer) {
            this.end_pull();
        }

        // The pointer released handler clears these fields; the capture lost
        // handler must do the same, otherwise the next gesture re-enters the
        // pointer moved handler with `pull_in_progress` set and reuses the
        // just-ended gesture id for a new pull gesture event.
        this.clear_tracking();
    }

    fn pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        let is_enabled_on_platform = (e.pointer().type_() == PointerType::Touch || e.pointer().type_() == PointerType::Pen) // either it is a touch device
            || this.is_mouse_enabled(); // or desktop is enabled

        if let Some(target) = this.target() {
            if is_enabled_on_platform {
                drop(this.tracking.replace(Some(e.pointer().clone())));
                this.initial_position.set(e.get_position(Some(&target)));
            }
        }
    }

    fn pointer_moved(this: &Self, e: &PointerEventArgs) {
        if !this.is_tracking(e.pointer()) {
            return;
        }

        let Some(target) = this.target() else { return };
        let can_pull = target.as_scrollable().is_some_and(|scrollable| this.can_pull(scrollable));

        if can_pull {
            let current_position = e.get_position(Some(&target));

            let delta = this.calculate_delta(current_position);

            let pulling = delta.y > 0.0 || delta.x > 0.0;
            let pull_in_progress = match (this.pull_in_progress.get(), pulling) {
                (false, false) => false,
                (false, true) => this.begin_pull(e, delta),
                (true, true) => this.handle_pull(e, delta),
                (true, false) => this.end_pull(),
            };
            this.pull_in_progress.set(pull_in_progress);
        }
    }

    fn pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        let _guard = ReleaseGuard { recognizer: this, pointer: e.pointer() };

        if this.pull_in_progress.get() {
            this.end_pull();
        }
    }
}

ferroui_base::ferro_properties! { impl ScrollablePullGestureRecognizer {
    ferro_property!(
        /// Defines the `PullDirection` property.
        pub fn pull_direction_property() -> StyledProperty<PullDirection> {
            FerroProperty::register::<ScrollablePullGestureRecognizer, _>("PullDirection", PullDirection::TopToBottom)
        }
    );
} }

impl ScrollablePullGestureRecognizer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: GestureRecognizer::construct(),
            gesture_id: Cell::new(0),
            pull_in_progress: Cell::new(false),
            delta: 1.0,
            initial_position: Cell::new(Point::default()),
            tracking: RefCell::new(None),
            is_mouse_enabled: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a recognizer for pulls in the given direction.
    pub fn with_direction(pull_direction: PullDirection, is_mouse_enabled: bool) -> Ref<Self> {
        let result = Self::new();
        result.set_pull_direction(pull_direction);
        result.set_is_mouse_enabled(is_mouse_enabled);
        result
    }

    pub fn pull_direction(&self) -> PullDirection {
        self.get_value(Self::pull_direction_property())
    }

    pub fn set_pull_direction(&self, value: PullDirection) {
        self.set_value(Self::pull_direction_property(), value)
    }

    pub fn is_mouse_enabled(&self) -> bool {
        self.is_mouse_enabled.get()
    }

    pub fn set_is_mouse_enabled(&self, value: bool) {
        self.is_mouse_enabled.set(value)
    }

    fn is_tracking(&self, pointer: &Rc<dyn IPointer>) -> bool {
        self.tracking.borrow().as_ref().is_some_and(|t| std::ptr::addr_eq(Rc::as_ptr(t), Rc::as_ptr(pointer)))
    }

    fn clear_tracking(&self) {
        drop(self.tracking.replace(None));
        self.initial_position.set(Point::default());
        self.pull_in_progress.set(false);
    }

    fn begin_pull(&self, e: &PointerEventArgs, delta: Vector) -> bool {
        self.gesture_id.set(PullGestureEventArgs::get_next_free_id());
        self.handle_pull(e, delta)
    }

    fn handle_pull(&self, e: &PointerEventArgs, delta: Vector) -> bool {
        self.capture(e.pointer());

        let pull_event_args = PullGestureEventArgs::new(self.gesture_id.get(), delta, self.pull_direction());
        if let Some(target) = self.target() {
            target.raise_event(&pull_event_args);
        }

        e.set_handled(pull_event_args.handled());
        true
    }

    fn end_pull(&self) -> bool {
        if let Some(target) = self.target() {
            target.raise_event(&PullGestureEndedEventArgs::new(self.gesture_id.get(), self.pull_direction()));
        }
        false
    }

    fn calculate_delta(&self, current_position: Point) -> Vector {
        let initial_position = self.initial_position.get();
        match self.pull_direction() {
            PullDirection::TopToBottom => Vector::new(0.0, current_position.y - initial_position.y),
            PullDirection::BottomToTop => Vector::new(0.0, initial_position.y - current_position.y),
            PullDirection::LeftToRight => Vector::new(current_position.x - initial_position.x, 0.0),
            PullDirection::RightToLeft => Vector::new(initial_position.x - current_position.x, 0.0),
        }
    }

    fn can_pull(&self, scrollable: &dyn IScrollable) -> bool {
        match self.pull_direction() {
            PullDirection::TopToBottom => scrollable.offset().y < self.delta,
            PullDirection::BottomToTop => {
                (scrollable.offset().y + scrollable.viewport().height - scrollable.extent().height).abs() <= self.delta
            }
            PullDirection::LeftToRight => scrollable.offset().x < self.delta,
            PullDirection::RightToLeft => {
                (scrollable.offset().x + scrollable.viewport().width - scrollable.extent().width).abs() <= self.delta
            }
        }
    }
}
