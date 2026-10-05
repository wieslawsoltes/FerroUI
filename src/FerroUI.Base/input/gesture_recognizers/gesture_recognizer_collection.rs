use super::GestureRecognizer;
use crate::input::{
    IPointer, InputElement, Pointer, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
};
use crate::{FerroObject, Ref, StyledElement, WeakRef};
use std::cell::RefCell;
use std::rc::Rc;

/// The gesture recognizers attached to an input element.
///
/// The value is a shared handle: clones refer to the same collection, and
/// handles compare by identity.
#[derive(Clone)]
pub struct GestureRecognizerCollection(Rc<GestureRecognizerCollectionData>);

struct GestureRecognizerCollectionData {
    input_element: WeakRef<InputElement>,
    recognizers: RefCell<Vec<Ref<GestureRecognizer>>>,
}

impl PartialEq for GestureRecognizerCollection {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for GestureRecognizerCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GestureRecognizerCollection({})", self.count())
    }
}

impl GestureRecognizerCollection {
    /// Creates the collection of an input element.
    pub fn new(input_element: &Ref<InputElement>) -> Self {
        Self(Rc::new(GestureRecognizerCollectionData {
            input_element: input_element.downgrade(),
            recognizers: RefCell::new(Vec::new()),
        }))
    }

    fn snapshot(&self) -> Vec<Ref<GestureRecognizer>> {
        self.0.recognizers.borrow().clone()
    }

    /// Attaches a recognizer to the element.
    pub fn add(&self, recognizer: impl crate::IntoRef<GestureRecognizer>) {
        let recognizer = recognizer.into_ref();
        self.0.recognizers.borrow_mut().push(recognizer.clone());

        let Some(input_element) = self.0.input_element.upgrade() else { return };
        recognizer.set_target(Some(&input_element));

        // Make the recognizer a logical child of the element so that
        // bindings and inherited values work, and keep its templated parent
        // in sync with the element's.
        recognizer.set_parent(input_element.clone().upcast::<StyledElement>());
        recognizer.set_templated_parent(input_element.templated_parent());

        let weak = recognizer.downgrade();
        input_element.property_changed(move |e| {
            if e.property() == StyledElement::templated_parent_property().as_property() {
                if let Some(recognizer) = weak.upgrade() {
                    recognizer.set_templated_parent(e.get_new_value::<Option<Ref<FerroObject>>>());
                }
            }
        });
    }

    /// Detaches a recognizer from the element. Returns whether it was
    /// attached.
    pub fn remove(&self, recognizer: &GestureRecognizer) -> bool {
        let removed = {
            let mut recognizers = self.0.recognizers.borrow_mut();
            recognizers
                .iter()
                .position(|r| std::ptr::eq::<GestureRecognizer>(&**r, recognizer))
                .map(|index| recognizers.remove(index))
        };

        match removed {
            Some(removed) => {
                removed.set_target(None);
                removed.set_parent(None);
                true
            }
            None => false,
        }
    }

    /// The attached recognizers.
    pub fn to_vec(&self) -> Vec<Ref<GestureRecognizer>> {
        self.snapshot()
    }

    /// The number of attached recognizers.
    pub fn count(&self) -> usize {
        self.0.recognizers.borrow().len()
    }

    fn captured_recognizer(pointer: &Rc<dyn IPointer>) -> Option<Ref<GestureRecognizer>> {
        pointer.as_any().downcast_ref::<Pointer>().and_then(Pointer::captured_gesture_recognizer)
    }

    pub(crate) fn handle_pointer_pressed(&self, e: &PointerPressedEventArgs) -> bool {
        if self.0.recognizers.borrow().is_empty() {
            return false;
        }

        for r in self.snapshot() {
            r.pointer_pressed_internal(e);
        }

        e.handled()
    }

    pub(crate) fn handle_capture_lost(&self, pointer: &Rc<dyn IPointer>) {
        if self.0.recognizers.borrow().is_empty() || pointer.as_any().downcast_ref::<Pointer>().is_none() {
            return;
        }

        for r in self.snapshot() {
            if Self::captured_recognizer(pointer).as_ref() == Some(&r) {
                continue;
            }

            r.pointer_capture_lost_internal(pointer);
        }
    }

    pub(crate) fn handle_pointer_released(&self, e: &PointerReleasedEventArgs) -> bool {
        if self.0.recognizers.borrow().is_empty() {
            return false;
        }

        for r in self.snapshot() {
            if Self::captured_recognizer(e.pointer()).is_some() {
                break;
            }

            r.pointer_released_internal(e);
        }

        e.handled()
    }

    pub(crate) fn handle_pointer_moved(&self, e: &PointerEventArgs) -> bool {
        if self.0.recognizers.borrow().is_empty() {
            return false;
        }

        for r in self.snapshot() {
            if Self::captured_recognizer(e.pointer()).is_some() {
                break;
            }

            r.pointer_moved_internal(e);
        }

        e.handled()
    }
}
