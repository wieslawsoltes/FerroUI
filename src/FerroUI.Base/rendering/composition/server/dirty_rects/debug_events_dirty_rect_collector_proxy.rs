use super::IDirtyRectCollector;
use crate::platform::LtrbRect;
use crate::rendering::composition::ICompositionTargetDebugEvents;
use std::rc::Rc;

/// Forwards every rectangle to the wrapped collector and reports it to the
/// debug events sink.
pub struct DebugEventsDirtyRectCollectorProxy {
    inner: Rc<dyn IDirtyRectCollector>,
    events: std::sync::Arc<dyn ICompositionTargetDebugEvents>,
}

impl DebugEventsDirtyRectCollectorProxy {
    pub fn new(inner: Rc<dyn IDirtyRectCollector>, events: std::sync::Arc<dyn ICompositionTargetDebugEvents>) -> Self {
        Self { inner, events }
    }
}

impl IDirtyRectCollector for DebugEventsDirtyRectCollectorProxy {
    fn add_rect(&self, rect: LtrbRect) {
        self.inner.add_rect(rect);
        self.events.rect_invalidated(rect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[derive(Default)]
    struct Recorder {
        log: Rc<RefCell<Vec<String>>>,
        rendered: Cell<i32>,
        visited: Cell<i32>,
    }

    // SAFETY: the tests create and use this recorder on one thread; the
    // impls only satisfy the thread-safety bound of the contract.
    unsafe impl Send for Recorder {}
    unsafe impl Sync for Recorder {}

    impl IDirtyRectCollector for Recorder {
        fn add_rect(&self, rect: LtrbRect) {
            self.log.borrow_mut().push(format!("add {}", rect.left));
        }
    }

    impl ICompositionTargetDebugEvents for Recorder {
        fn rendered_visuals(&self) -> i32 {
            self.rendered.get()
        }
        fn set_rendered_visuals(&self, value: i32) {
            self.rendered.set(value);
        }
        fn visited_visuals(&self) -> i32 {
            self.visited.get()
        }
        fn set_visited_visuals(&self, value: i32) {
            self.visited.set(value);
        }
        fn rect_invalidated(&self, rc: LtrbRect) {
            self.log.borrow_mut().push(format!("invalidated {}", rc.left));
        }
    }

    #[test]
    fn forwards_to_inner_then_reports_event() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let inner = Rc::new(Recorder { log: log.clone(), ..Default::default() });
        let events = std::sync::Arc::new(Recorder { log: log.clone(), ..Default::default() });
        let proxy = DebugEventsDirtyRectCollectorProxy::new(inner, events);
        proxy.add_rect(LtrbRect::new(1.0, 0.0, 2.0, 2.0));
        proxy.add_rect(LtrbRect::new(7.0, 0.0, 9.0, 2.0));
        assert_eq!(vec!["add 1", "invalidated 1", "add 7", "invalidated 7"], *log.borrow());
    }
}
