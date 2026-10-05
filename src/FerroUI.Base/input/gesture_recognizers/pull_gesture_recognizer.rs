use super::{GestureRecognizer, GestureRecognizerImpl};
use crate::input::{
    IPointer, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType, PullDirection,
    PullGestureEndedEventArgs, PullGestureEventArgs,
};
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Point, Ref,
    StyledElementImpl, StyledProperty, Vector,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The size of the edge strip a pull must start in.
pub(crate) const MIN_PULL_DETECTION_SIZE: f64 = 50.0;

/// Recognizes the pull gesture: a drag from an edge of the target, as used
/// by pull-to-refresh.
#[repr(C)]
pub struct PullGestureRecognizer {
    base: GestureRecognizer,
    initial_position: Cell<Point>,
    gesture_id: Cell<i32>,
    tracking: RefCell<Option<Rc<dyn IPointer>>>,
    pull_in_progress: Cell<bool>,
}

ferro_class!(PullGestureRecognizer: GestureRecognizer);
crate::ferro_class_info!(PullGestureRecognizer { new: PullGestureRecognizer::new });
ferro_impl_classes!(PullGestureRecognizer: FerroObjectImpl, StyledElementImpl);

impl GestureRecognizerImpl for PullGestureRecognizer {
    fn pointer_capture_lost(this: &Self, pointer: &Rc<dyn IPointer>) {
        if this.is_tracking(pointer) {
            this.end_pull();
        }
    }

    fn pointer_moved(this: &Self, e: &PointerEventArgs) {
        if !this.is_tracking(e.pointer()) {
            return;
        }
        let Some(target) = this.target() else { return };

        let current_position = e.get_position(Some(&target));
        this.capture(e.pointer());

        let initial_position = this.initial_position.get();
        let pull_direction = this.pull_direction();
        let mut delta = Vector::default();

        match pull_direction {
            PullDirection::TopToBottom => {
                if current_position.y > initial_position.y {
                    delta = Vector::new(0.0, current_position.y - initial_position.y);
                }
            }
            PullDirection::BottomToTop => {
                if current_position.y < initial_position.y {
                    delta = Vector::new(0.0, initial_position.y - current_position.y);
                }
            }
            PullDirection::LeftToRight => {
                if current_position.x > initial_position.x {
                    delta = Vector::new(current_position.x - initial_position.x, 0.0);
                }
            }
            PullDirection::RightToLeft => {
                if current_position.x < initial_position.x {
                    delta = Vector::new(initial_position.x - current_position.x, 0.0);
                }
            }
        }

        this.pull_in_progress.set(true);
        let pull_event_args = PullGestureEventArgs::new(this.gesture_id.get(), delta, pull_direction);
        target.raise_event(&pull_event_args);

        e.set_handled(pull_event_args.handled());
    }

    fn pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        let Some(target) = this.target() else { return };

        if e.pointer().type_() != PointerType::Touch && e.pointer().type_() != PointerType::Pen {
            return;
        }

        let position = e.get_position(Some(&target));
        let bounds = target.bounds();

        let can_pull = match this.pull_direction() {
            PullDirection::TopToBottom => position.y < MIN_PULL_DETECTION_SIZE.max(bounds.height * 0.1),
            PullDirection::BottomToTop => {
                position.y > (bounds.height - MIN_PULL_DETECTION_SIZE).min(bounds.height - (bounds.height * 0.1))
            }
            PullDirection::LeftToRight => position.x < MIN_PULL_DETECTION_SIZE.max(bounds.width * 0.1),
            PullDirection::RightToLeft => {
                position.x > (bounds.width - MIN_PULL_DETECTION_SIZE).min(bounds.width - (bounds.width * 0.1))
            }
        };

        if can_pull {
            this.gesture_id.set(PullGestureEventArgs::get_next_free_id());
            drop(this.tracking.replace(Some(e.pointer().clone())));
            this.initial_position.set(position);
        }
    }

    fn pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        if this.is_tracking(e.pointer()) && this.pull_in_progress.get() {
            this.end_pull();
        }
    }
}

crate::ferro_properties! { impl PullGestureRecognizer {
    ferro_property!(
        /// Defines the `PullDirection` property.
        pub fn pull_direction_property() -> StyledProperty<PullDirection> {
            FerroProperty::register::<PullGestureRecognizer, _>("PullDirection", PullDirection::TopToBottom)
        }
    );
} }

impl PullGestureRecognizer {
    /// Creates a recognizer for pulls from the top.
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: GestureRecognizer::construct(),
            initial_position: Cell::new(Point::default()),
            gesture_id: Cell::new(0),
            tracking: RefCell::new(None),
            pull_in_progress: Cell::new(false),
        })
    }

    /// Creates a recognizer for pulls in the given direction.
    pub fn with_direction(pull_direction: PullDirection) -> Ref<Self> {
        let result = Self::new();
        result.set_pull_direction(pull_direction);
        result
    }

    /// The direction the target is pulled in.
    pub fn pull_direction(&self) -> PullDirection {
        self.get_value(Self::pull_direction_property())
    }

    pub fn set_pull_direction(&self, value: PullDirection) {
        self.set_value(Self::pull_direction_property(), value)
    }

    fn is_tracking(&self, pointer: &Rc<dyn IPointer>) -> bool {
        self.tracking.borrow().as_ref().is_some_and(|t| std::ptr::addr_eq(Rc::as_ptr(t), Rc::as_ptr(pointer)))
    }

    fn end_pull(&self) {
        drop(self.tracking.replace(None));
        self.initial_position.set(Point::default());
        self.pull_in_progress.set(false);

        if let Some(target) = self.target() {
            target.raise_event(&PullGestureEndedEventArgs::new(self.gesture_id.get(), self.pull_direction()));
        }
    }
}
