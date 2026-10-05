use super::{GestureRecognizer, GestureRecognizerImpl};
use crate::input::{
    IPointer, PinchEndedEventArgs, PinchEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerType,
};
use crate::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Ref, StyledElementImpl, Vector};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn same_pointer(a: &Option<Rc<dyn IPointer>>, b: &Rc<dyn IPointer>) -> bool {
    a.as_ref().is_some_and(|a| std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)))
}

/// Recognizes the pinch gesture: two contacts moving closer together or
/// further apart, and rotating around each other.
#[repr(C)]
pub struct PinchGestureRecognizer {
    base: GestureRecognizer,
    initial_distance: Cell<f32>,
    first_contact: RefCell<Option<Rc<dyn IPointer>>>,
    first_point: Cell<Point>,
    second_contact: RefCell<Option<Rc<dyn IPointer>>>,
    second_point: Cell<Point>,
    origin: Cell<Point>,
    previous_angle: Cell<f64>,
}

ferro_class!(PinchGestureRecognizer: GestureRecognizer);
crate::ferro_class_info!(PinchGestureRecognizer { new: PinchGestureRecognizer::new });
ferro_impl_classes!(PinchGestureRecognizer: FerroObjectImpl, StyledElementImpl);

impl GestureRecognizerImpl for PinchGestureRecognizer {
    fn pointer_capture_lost(this: &Self, pointer: &Rc<dyn IPointer>) {
        this.remove_contact(pointer);
    }

    fn pointer_moved(this: &Self, e: &PointerEventArgs) {
        let Some(target) = this.target() else { return };

        if same_pointer(&this.first_contact.borrow(), e.pointer()) {
            this.first_point.set(e.get_position(Some(&target)));
        } else if same_pointer(&this.second_contact.borrow(), e.pointer()) {
            this.second_point.set(e.get_position(Some(&target)));
        } else {
            return;
        }

        if this.first_contact.borrow().is_some() && this.second_contact.borrow().is_some() {
            let distance = Self::get_distance(this.first_point.get(), this.second_point.get());
            let scale = (distance / this.initial_distance.get()) as f64;
            let degree = Self::get_angle_degree_from_points(this.first_point.get(), this.second_point.get());

            let pinch_event_args =
                PinchEventArgs::with_angle(scale, this.origin.get(), degree, this.previous_angle.get() - degree);
            this.previous_angle.set(degree);
            target.raise_event(&pinch_event_args);

            e.set_handled(pinch_event_args.handled());
            e.prevent_gesture_recognition();
        }
    }

    fn pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        let Some(target) = this.target() else { return };

        if e.pointer().type_() != PointerType::Touch && e.pointer().type_() != PointerType::Pen {
            return;
        }

        if this.first_contact.borrow().is_none() {
            *this.first_contact.borrow_mut() = Some(e.pointer().clone());
            this.first_point.set(e.get_position(Some(&target)));
            return;
        } else if this.second_contact.borrow().is_none() && !same_pointer(&this.first_contact.borrow(), e.pointer()) {
            *this.second_contact.borrow_mut() = Some(e.pointer().clone());
            this.second_point.set(e.get_position(Some(&target)));
        } else {
            return;
        }

        let first = this.first_contact.borrow().clone();
        let second = this.second_contact.borrow().clone();

        if let (Some(first), Some(second)) = (first, second) {
            let (first_point, second_point) = (this.first_point.get(), this.second_point.get());
            this.initial_distance.set(Self::get_distance(first_point, second_point));

            this.origin
                .set(Point::new((first_point.x + second_point.x) / 2.0, (first_point.y + second_point.y) / 2.0));

            this.previous_angle.set(Self::get_angle_degree_from_points(first_point, second_point));

            this.capture(&first);
            this.capture(&second);
            e.prevent_gesture_recognition();
        }
    }

    fn pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        if this.remove_contact(e.pointer()) {
            e.prevent_gesture_recognition();
        }
    }
}

impl PinchGestureRecognizer {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: GestureRecognizer::construct(),
            initial_distance: Cell::new(0.0),
            first_contact: RefCell::new(None),
            first_point: Cell::new(Point::default()),
            second_contact: RefCell::new(None),
            second_point: Cell::new(Point::default()),
            origin: Cell::new(Point::default()),
            previous_angle: Cell::new(0.0),
        })
    }

    fn remove_contact(&self, pointer: &Rc<dyn IPointer>) -> bool {
        let is_first = same_pointer(&self.first_contact.borrow(), pointer);
        let is_second = same_pointer(&self.second_contact.borrow(), pointer);

        if is_first || is_second {
            if is_second {
                drop(self.second_contact.replace(None));
            }

            if is_first {
                let second = self.second_contact.replace(None);
                drop(self.first_contact.replace(second));
            }

            if let Some(target) = self.target() {
                target.raise_event(&PinchEndedEventArgs::new());
            }

            return true;
        }

        false
    }

    fn get_distance(a: Point, b: Point) -> f32 {
        let length = b - a;
        Vector::new(length.x, length.y).length() as f32
    }

    fn get_angle_degree_from_points(a: Point, b: Point) -> f64 {
        let delta_x = a.x - b.x;
        // The sign is reversed, because on the screen the Y axes are
        // reversed with respect to the Cartesian plane.
        let delta_y = -(a.y - b.y);
        // radians from -pi to +pi
        let rad = delta_x.atan2(delta_y);
        // atan2 returns a value in degrees from -180 to +180; to get the
        // angle between 0 and 360 degrees add 180 degrees.
        (rad * (180.0 / std::f64::consts::PI)) + 180.0
    }
}
