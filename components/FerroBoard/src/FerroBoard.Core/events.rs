//! Shared-model counterpart of FerroUI.System Event. The pinned framework event is Rc/RefCell based.
//! Dispatch is synchronous and snapshots subscriptions before calling user code, outside its lock.
use parking_lot::Mutex;
use std::sync::Arc;
pub type SharedEventHandler<T> = Arc<dyn Fn(&T) + Send + Sync>;
struct State<T: ?Sized> {
    next_id: u64,
    handlers: Arc<Vec<(u64, SharedEventHandler<T>)>>,
}
pub struct SharedEvent<T: ?Sized> {
    state: Mutex<State<T>>,
}
impl<T: ?Sized> Default for SharedEvent<T> {
    fn default() -> Self {
        Self {
            state: Mutex::new(State {
                next_id: 1,
                handlers: Arc::new(Vec::new()),
            }),
        }
    }
}
impl<T: ?Sized> SharedEvent<T> {
    pub fn add(&self, handler: SharedEventHandler<T>) -> u64 {
        let mut state = self.state.lock();
        let id = state.next_id;
        state.next_id = state
            .next_id
            .checked_add(1)
            .expect("event token space exhausted");
        Arc::make_mut(&mut state.handlers).push((id, handler));
        id
    }
    pub fn remove(&self, token: u64) -> bool {
        let removed = {
            let mut state = self.state.lock();
            let Some(i) = state.handlers.iter().position(|(id, _)| *id == token) else {
                return false;
            };
            Arc::make_mut(&mut state.handlers).remove(i)
        };
        drop(removed);
        true
    }
    pub fn handler_count(&self) -> usize {
        self.state.lock().handlers.len()
    }
    pub fn has_handlers(&self) -> bool {
        self.handler_count() != 0
    }
    pub fn raise(&self, sender: &T) {
        let handlers = self.state.lock().handlers.clone();
        for (_, handler) in handlers.iter() {
            handler(sender);
        }
    }
}
