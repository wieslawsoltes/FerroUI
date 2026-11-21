//! Process-wide Current storage and its specific notification contract.
//! Copyright (c) 2026 Wiesław Šoltés. MIT.
use super::RibbonStrings;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
pub type RibbonStringsChangedHandler = dyn Fn(&()) + Send + Sync;
#[derive(Default)]
struct Handlers {
    next: u64,
    values: Vec<(u64, Arc<RibbonStringsChangedHandler>)>,
}
/// CurrentChanged runs on the setter's thread, in subscription order. Changes
/// to subscriptions during a notification affect later notifications only.
#[derive(Default)]
pub struct RibbonStringsChanged {
    handlers: Mutex<Handlers>,
}
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}
impl RibbonStringsChanged {
    pub fn add(&self, handler: Arc<RibbonStringsChangedHandler>) -> u64 {
        let mut state = lock(&self.handlers);
        state.next = state
            .next
            .checked_add(1)
            .expect("localization handler token overflow");
        let token = state.next;
        state.values.push((token, handler));
        token
    }
    pub fn remove(&self, token: u64) -> bool {
        let removed = {
            let mut state = lock(&self.handlers);
            state
                .values
                .iter()
                .position(|(id, _)| *id == token)
                .map(|i| state.values.remove(i))
        };
        let found = removed.is_some();
        drop(removed);
        found
    }
    pub fn handler_count(&self) -> usize {
        lock(&self.handlers).values.len()
    }
    pub fn has_handlers(&self) -> bool {
        self.handler_count() != 0
    }
    pub(super) fn raise(&self) {
        let snapshot = lock(&self.handlers).values.clone();
        // Keep the complete snapshot alive until the notification returns,
        // including callbacks already invoked and concurrently removed.
        for (_, handler) in &snapshot {
            handler(&());
        }
    }
}
pub(super) struct CurrentState {
    pub value: Mutex<Option<Arc<RibbonStrings>>>,
    pub changed: Arc<RibbonStringsChanged>,
}
pub(super) fn current_state() -> &'static CurrentState {
    static STATE: OnceLock<CurrentState> = OnceLock::new();
    STATE.get_or_init(|| CurrentState {
        value: Mutex::new(None),
        changed: Arc::default(),
    })
}
pub(super) fn read() -> Option<Arc<RibbonStrings>> {
    lock(&current_state().value).clone()
}
pub(super) fn replace(value: Arc<RibbonStrings>) -> Option<Arc<RibbonStrings>> {
    lock(&current_state().value).replace(value)
}
