use ferroui_base::reactive::IDisposable;
use ferroui_controls::TopLevel;
use serde_json::{json, Value};
use std::{cell::Cell, rc::Rc, time::Instant};

pub(super) struct FrameStats {
    count: Rc<Cell<u64>>,
    last: Rc<Cell<Option<Instant>>>,
    interval: Rc<Cell<Option<f64>>>,
    subscription: Rc<dyn IDisposable>,
}
impl FrameStats {
    pub fn new(window: &TopLevel) -> Self {
        let count = Rc::new(Cell::new(0));
        let last = Rc::new(Cell::new(None::<Instant>));
        let interval = Rc::new(Cell::new(None));
        let c = count.clone();
        let l = last.clone();
        let i = interval.clone();
        // The compositing renderer raises this after the scene's composition batch is rendered.
        let subscription = window.renderer().scene_invalidated(Rc::new(move |_| {
            let now = Instant::now();
            if let Some(previous) = l.replace(Some(now)) {
                i.set(Some(now.duration_since(previous).as_secs_f64() * 1000.0));
            }
            c.set(c.get() + 1);
        }));
        Self {
            count,
            last,
            interval,
            subscription,
        }
    }
    pub fn snapshot(&self) -> Value {
        json!({"rendered_scene_updates":self.count.get(),"last_interval_ms":self.interval.get(),"since_last_update_ms":self.last.get().map(|t|t.elapsed().as_secs_f64()*1000.0)})
    }
}
impl Drop for FrameStats {
    fn drop(&mut self) {
        self.subscription.dispose();
    }
}
