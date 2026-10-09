//! Upstream's `PointerTestsBase` (base unit tests), for the pointer tests of the base library that need a
//! top-level and therefore live with the controls: its mocks of the pointer, the pointer device and the hit
//! tester, written out, and its input root.

use crate::presenters::ContentPresenter;
use crate::templates::FuncControlTemplate;
use crate::testing::{MockWindowImpl, MockWindowingPlatform};
use crate::{Control, Window};
use ferroui_base::input::raw::{IRawInputEventArgs, RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::{
    IInputDevice, IPointer, IPointerDevice, InputElement, PointerType, RawInputModifiers,
};
use ferroui_base::media::{Geometry, GeometryHitTestResult};
use ferroui_base::rendering::IHitTester;
use ferroui_base::{IntoRef, Point, Ref, Visual};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// Upstream `new Mock<IPointer>()` with `Type` set up: nothing is captured.
pub(crate) struct MockPointer {
    pointer_type: PointerType,
}

impl IPointer for MockPointer {
    fn id(&self) -> i32 {
        0
    }

    fn capture(&self, _control: Option<&Ref<InputElement>>) {}

    fn captured(&self) -> Option<Ref<InputElement>> {
        None
    }

    fn type_(&self) -> PointerType {
        self.pointer_type
    }

    fn is_primary(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Upstream `CreatePointerDeviceMock`: a device that answers its pointer. It does nothing with a raw event
/// unless a callback is set up (`SetMove`).
pub(crate) struct MockPointerDevice {
    pointer: Rc<dyn IPointer>,
    process_raw_event: RefCell<Option<Rc<dyn Fn()>>>,
}

impl MockPointerDevice {
    /// The pointer of the device.
    pub(crate) fn pointer(&self) -> Rc<dyn IPointer> {
        self.pointer.clone()
    }

    /// Upstream `deviceMock.Setup(d => d.ProcessRawEvent(..)).Callback(..)`.
    pub(crate) fn setup_process_raw_event(&self, callback: impl Fn() + 'static) {
        *self.process_raw_event.borrow_mut() = Some(Rc::new(callback));
    }
}

impl IInputDevice for MockPointerDevice {
    fn process_raw_event(&self, _ev: &dyn IRawInputEventArgs) {
        let callback = self.process_raw_event.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_pointer_device(&self) -> Option<&dyn IPointerDevice> {
        Some(self)
    }
}

impl IPointerDevice for MockPointerDevice {
    fn try_get_pointer(&self, _ev: &RawPointerEventArgs) -> Option<Rc<dyn IPointer>> {
        Some(self.pointer.clone())
    }
}

pub(crate) fn create_pointer_device_mock() -> Rc<MockPointerDevice> {
    Rc::new(MockPointerDevice {
        pointer: Rc::new(MockPointer { pointer_type: PointerType::Mouse }),
        process_raw_event: RefCell::new(None),
    })
}

/// Upstream `new Mock<IHitTester>()` with `SetHit`: the control set up is the answer for every point.
#[derive(Default)]
pub(crate) struct MockHitTester {
    hit: RefCell<Option<Ref<Visual>>>,
}

impl IHitTester for MockHitTester {
    fn hit_test(&self, _p: Point, _root: &Visual, _filter: Option<&dyn Fn(&Visual) -> bool>) -> Vec<Ref<Visual>> {
        self.hit.borrow().iter().cloned().collect()
    }

    fn hit_test_geometry(
        &self,
        _geometry: &Geometry,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Vec<GeometryHitTestResult> {
        Vec::new()
    }

    fn hit_test_first(&self, _p: Point, _root: &Visual, _filter: Option<&dyn Fn(&Visual) -> bool>) -> Option<Ref<Visual>> {
        self.hit.borrow().clone()
    }

    fn hit_test_first_geometry(
        &self,
        _geometry: &Geometry,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<GeometryHitTestResult> {
        None
    }
}

pub(crate) fn set_hit(renderer: &MockHitTester, hit: Option<Ref<Visual>>) {
    *renderer.hit.borrow_mut() = hit;
}

pub(crate) fn create_top_level_impl_mock() -> Rc<MockWindowImpl> {
    MockWindowingPlatform::create_window_mock()
}

pub(crate) fn create_input_root(
    window_impl: &Rc<MockWindowImpl>,
    child: impl IntoRef<Control>,
    hit_tester: &Rc<MockHitTester>,
) -> Ref<Window> {
    let root = Window::with_impl(window_impl.clone());
    root.set_width(100.0);
    root.set_height(100.0);
    root.set_content(Some(Control::boxed(child)));
    root.set_template(Some(FuncControlTemplate::for_type::<Window>(|w, _| {
        let presenter = ContentPresenter::new();
        presenter.set_content(w.content());
        presenter.upcast()
    })));
    root.set_hit_tester_override(Some(hit_tester.clone()));
    root.show();
    root
}

/// Upstream `impl.Object.Input!(CreateRawPointerMovedArgs(device, root, position))`.
pub(crate) fn raw_pointer_moved(
    window_impl: &MockWindowImpl,
    device: &Rc<MockPointerDevice>,
    root: &Window,
    position: Point,
) {
    let input = crate::platform::ITopLevelImpl::input(window_impl).expect("the window handles input");
    input(Rc::new(RawPointerEventArgs::new(
        device.clone(),
        0,
        root.input_root(),
        RawPointerEventType::Move,
        position,
        RawInputModifiers::NONE,
    )));
}
