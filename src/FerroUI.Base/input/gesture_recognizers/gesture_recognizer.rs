use crate::input::{
    IPointer, InputElement, Pointer, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
};
use crate::{ferro_class, ferro_impl_classes, FerroObjectImpl, Ref, StyledElement, StyledElementImpl, WeakRef};
use std::cell::RefCell;
use std::rc::Rc;

/// The base class of gesture recognizers.
///
/// A recognizer is attached to an input element through the element's
/// gesture recognizer collection and receives the pointer events of that
/// element. A recognizer that detects its gesture captures the pointer,
/// after which pointer input goes to the recognizer only.
#[repr(C)]
pub struct GestureRecognizer {
    base: StyledElement,
    /// The pointer of the pointer event currently being delivered.
    current_pointer: RefCell<Option<Rc<dyn IPointer>>>,
    target: RefCell<Option<WeakRef<InputElement>>>,
}

ferro_class! {
    GestureRecognizer: StyledElement, virtuals GestureRecognizerImpl: StyledElementImpl {
        /// Called when a pointer is pressed on the target.
        fn pointer_pressed(this, e: &PointerPressedEventArgs);
        /// Called when a pointer is released on the target.
        fn pointer_released(this, e: &PointerReleasedEventArgs);
        /// Called when a pointer moves over the target.
        fn pointer_moved(this, e: &PointerEventArgs);
        /// Called when the capture of a pointer is lost.
        fn pointer_capture_lost(this, pointer: &Rc<dyn IPointer>);
    }
}

ferro_impl_classes!(GestureRecognizer: FerroObjectImpl, StyledElementImpl);

/// The base class does nothing: recognizers override these (they are
/// abstract upstream).
impl GestureRecognizerImpl for GestureRecognizer {
    fn pointer_pressed(_this: &Self, _e: &PointerPressedEventArgs) {}

    fn pointer_released(_this: &Self, _e: &PointerReleasedEventArgs) {}

    fn pointer_moved(_this: &Self, _e: &PointerEventArgs) {}

    fn pointer_capture_lost(_this: &Self, _pointer: &Rc<dyn IPointer>) {}
}

impl GestureRecognizer {
    /// Creates the class data; see [`crate::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElement::construct(), current_pointer: RefCell::new(None), target: RefCell::new(None) }
    }

    /// The element the recognizer is attached to.
    pub fn target(&self) -> Option<Ref<InputElement>> {
        self.target.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub(crate) fn set_target(&self, value: Option<&Ref<InputElement>>) {
        *self.target.borrow_mut() = value.map(Ref::downgrade);
    }

    fn set_current_pointer(&self, value: Option<Rc<dyn IPointer>>) {
        let old = self.current_pointer.replace(value);
        drop(old);
    }

    pub(crate) fn pointer_pressed_internal(&self, e: &PointerPressedEventArgs) {
        self.set_current_pointer(Some(e.pointer().clone()));
        self.pointer_pressed(e);
        self.set_current_pointer(None);
    }

    pub(crate) fn pointer_released_internal(&self, e: &PointerReleasedEventArgs) {
        self.set_current_pointer(Some(e.pointer().clone()));
        self.pointer_released(e);
        self.set_current_pointer(None);
    }

    pub(crate) fn pointer_moved_internal(&self, e: &PointerEventArgs) {
        self.set_current_pointer(Some(e.pointer().clone()));
        self.pointer_moved(e);
        self.set_current_pointer(None);
    }

    /// Hands a pointer move to the recognizer as the input devices do for
    /// the recognizer that captured the pointer. For test helpers that
    /// raise pointer input themselves.
    #[cfg(any(test, feature = "testing"))]
    pub fn pointer_moved_for_testing(&self, e: &PointerEventArgs) {
        self.pointer_moved_internal(e);
    }

    /// Hands a pointer release to the recognizer as the input devices do
    /// for the recognizer that captured the pointer. For test helpers that
    /// raise pointer input themselves.
    #[cfg(any(test, feature = "testing"))]
    pub fn pointer_released_for_testing(&self, e: &PointerReleasedEventArgs) {
        self.pointer_released_internal(e);
    }

    pub(crate) fn pointer_capture_lost_internal(&self, pointer: &Rc<dyn IPointer>) {
        self.pointer_capture_lost(pointer);
    }

    /// Captures a pointer to the recognizer: further input of the pointer
    /// goes to the recognizer and other recognizers are skipped.
    pub fn capture(&self, pointer: &Rc<dyn IPointer>) {
        if let Some(pointer) = pointer.as_any().downcast_ref::<Pointer>() {
            pointer.capture_gesture_recognizer(Some(&self.to_ref()));
        }

        // Prevent gesture recognition for the event being delivered.
        let current = self.current_pointer.borrow().clone();
        if let Some(current) = current {
            if let Some(current) = current.as_any().downcast_ref::<Pointer>() {
                current.set_is_gesture_recognition_skipped(true);
            }
        }
    }
}
