//! A test-only host with observable calls, weak model ownership and callback hooks.

use ferrodock_model::core::*;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum HostCall {
    SetWindow,
    IsTracked,
    Tracked(bool),
    Position(f64, f64),
    GetPosition,
    Size(f64, f64),
    GetSize,
    State(DockWindowState),
    GetState,
    Title(Option<String>),
    Layout,
    Present(bool),
    Exit,
    IsClosed,
    Active,
}

type HostCallback = Rc<dyn Fn(&HostCall)>;

pub(crate) struct RecordingHost {
    pub(crate) tracked: Cell<bool>,
    window: RefCell<Option<Weak<dyn IDockWindow>>>,
    pub(crate) position: Cell<(f64, f64)>,
    pub(crate) size: Cell<(f64, f64)>,
    pub(crate) state: Cell<DockWindowState>,
    pub(crate) title: RefCell<Option<String>>,
    pub(crate) layout: RefCell<Option<Rc<dyn IDock>>>,
    pub(crate) closed: Cell<Option<bool>>,
    pub(crate) calls: RefCell<Vec<HostCall>>,
    pub(crate) on_call: RefCell<Option<HostCallback>>,
}
impl RecordingHost {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            tracked: Cell::new(false),
            window: RefCell::new(None),
            position: Cell::new((0.0, 0.0)),
            size: Cell::new((0.0, 0.0)),
            state: Cell::new(DockWindowState::Normal),
            title: RefCell::new(None),
            layout: RefCell::new(None),
            closed: Cell::new(None),
            calls: RefCell::new(Vec::new()),
            on_call: RefCell::new(None),
        })
    }
    fn record(&self, call: HostCall) {
        self.calls.borrow_mut().push(call.clone());
        let callback = self.on_call.borrow().clone();
        if let Some(callback) = callback {
            callback(&call);
        }
    }
}
impl IHostWindowCloseState for RecordingHost {
    fn is_closed(&self) -> bool {
        self.record(HostCall::IsClosed);
        self.closed.get().expect("optional close-state membership")
    }
}
impl IHostWindow for RecordingHost {
    fn as_host_window_close_state(&self) -> Option<&dyn IHostWindowCloseState> {
        self.closed
            .get()
            .map(|_| self as &dyn IHostWindowCloseState)
    }
    fn host_window_state(&self) -> Option<Rc<dyn IHostWindowState>> {
        None
    }
    fn is_tracked(&self) -> bool {
        self.record(HostCall::IsTracked);
        self.tracked.get()
    }
    fn set_is_tracked(&self, value: bool) {
        self.tracked.set(value);
        self.record(HostCall::Tracked(value));
    }
    fn window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.window.borrow().as_ref().and_then(Weak::upgrade)
    }
    fn set_window(&self, value: Option<Rc<dyn IDockWindow>>) {
        *self.window.borrow_mut() = value.as_ref().map(Rc::downgrade);
        self.record(HostCall::SetWindow);
    }
    fn present(&self, is_dialog: bool) {
        self.record(HostCall::Present(is_dialog));
    }
    fn exit(&self) {
        self.record(HostCall::Exit);
    }
    fn set_position(&self, x: f64, y: f64) {
        self.position.set((x, y));
        self.record(HostCall::Position(x, y));
    }
    fn get_position(&self) -> (f64, f64) {
        self.record(HostCall::GetPosition);
        self.position.get()
    }
    fn set_size(&self, width: f64, height: f64) {
        self.size.set((width, height));
        self.record(HostCall::Size(width, height));
    }
    fn get_size(&self) -> (f64, f64) {
        self.record(HostCall::GetSize);
        self.size.get()
    }
    fn set_window_state(&self, state: DockWindowState) {
        self.state.set(state);
        self.record(HostCall::State(state));
    }
    fn get_window_state(&self) -> DockWindowState {
        self.record(HostCall::GetState);
        self.state.get()
    }
    fn set_title(&self, title: Option<String>) {
        *self.title.borrow_mut() = title.clone();
        self.record(HostCall::Title(title));
    }
    fn set_layout(&self, layout: Rc<dyn IDock>) {
        *self.layout.borrow_mut() = Some(layout);
        self.record(HostCall::Layout);
    }
    fn set_active(&self) {
        self.record(HostCall::Active);
    }
}
