use crate::interop::{input_helper, JsObject};
use ferroui_base::input::{InputElement, MouseDevice, Pointer, PointerType};
use ferroui_base::Ref;
use std::rc::Rc;

/// The pointer capture of the element of a top-level; replaced by a
/// recorder in the tests.
pub(crate) trait IPointerCapture {
    fn set_pointer_capture(&self, pointer_id: i64);
    fn release_pointer_capture(&self, pointer_id: i64);
}

/// The pointer capture of an element of the page.
pub(crate) struct ContainerPointerCapture {
    container: JsObject,
}

impl ContainerPointerCapture {
    pub(crate) fn new(container: JsObject) -> Self {
        Self { container }
    }
}

impl IPointerCapture for ContainerPointerCapture {
    fn set_pointer_capture(&self, pointer_id: i64) {
        input_helper::set_pointer_capture(&self.container, pointer_id);
    }

    fn release_pointer_capture(&self, pointer_id: i64) {
        input_helper::release_pointer_capture(&self.container, pointer_id);
    }
}

/// The mouse device of one pointer of the page: capturing its pointer
/// captures the pointer of the page to the element of the top-level.
///
/// The device the raw events carry is the wrapped [`MouseDevice`]
/// ([`device`](Self::device)), whose pointer is the pointer the original
/// declares as `BrowserMousePointer`.
pub(crate) struct BrowserMouseDevice {
    device: Rc<MouseDevice>,
    pointer_id: i64,
}

impl BrowserMouseDevice {
    pub(crate) fn new(pointer_id: i64, container: Rc<dyn IPointerCapture>) -> Self {
        Self { device: MouseDevice::with_pointer(Self::browser_mouse_pointer(pointer_id, container)), pointer_id }
    }

    /// The identifier the page gives the pointer.
    pub(crate) fn pointer_id(&self) -> i64 {
        self.pointer_id
    }

    /// The mouse device.
    pub(crate) fn device(&self) -> &Rc<MouseDevice> {
        &self.device
    }

    fn browser_mouse_pointer(pointer_id: i64, container: Rc<dyn IPointerCapture>) -> Rc<Pointer> {
        Pointer::with_platform_capture(
            Pointer::get_next_free_id(),
            PointerType::Mouse,
            true,
            move |element: Option<&Ref<InputElement>>| {
                if element.is_some() {
                    container.set_pointer_capture(pointer_id);
                } else {
                    container.release_pointer_capture(pointer_id);
                }
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::input::IPointer;
    use ferroui_base::threading::Dispatcher;
    use std::cell::RefCell;

    #[derive(Default)]
    struct RecordingCapture {
        calls: RefCell<Vec<(&'static str, i64)>>,
    }

    impl IPointerCapture for RecordingCapture {
        fn set_pointer_capture(&self, pointer_id: i64) {
            self.calls.borrow_mut().push(("set", pointer_id));
        }

        fn release_pointer_capture(&self, pointer_id: i64) {
            self.calls.borrow_mut().push(("release", pointer_id));
        }
    }

    #[test]
    fn the_device_keeps_the_identifier_of_the_pointer_of_the_page() {
        let capture = Rc::new(RecordingCapture::default());

        let device = BrowserMouseDevice::new(1_234_567_890_123, capture.clone());

        assert_eq!(1_234_567_890_123, device.pointer_id());
        assert_eq!(PointerType::Mouse, device.device().pointer().type_());
        assert!(device.device().pointer().is_primary());
        assert!(capture.calls.borrow().is_empty());
    }

    #[test]
    fn every_device_has_a_pointer_of_its_own() {
        let capture = Rc::new(RecordingCapture::default());

        let first = BrowserMouseDevice::new(1, capture.clone());
        let second = BrowserMouseDevice::new(2, capture);

        assert_ne!(first.device().pointer().id(), second.device().pointer().id());
    }

    #[test]
    fn capturing_the_pointer_captures_the_pointer_of_the_page_and_releasing_it_releases_it() {
        let _scope = Dispatcher::unit_test_scope();
        let capture = Rc::new(RecordingCapture::default());
        let device = BrowserMouseDevice::new(7, capture.clone());
        let element = InputElement::new();

        device.device().pointer().capture(Some(&element));
        assert_eq!(vec![("set", 7)], *capture.calls.borrow());

        device.device().pointer().capture(None);
        assert_eq!(vec![("set", 7), ("release", 7)], *capture.calls.borrow());
    }
}
