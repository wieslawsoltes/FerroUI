//! Factory-owned native display lifetime. Contexts have independent GL object
//! namespaces; the underlying healthy D3D/EGL device survives context churn.
use ferroui_base::reactive::IDisposable;
use std::{
    cell::Cell,
    collections::HashMap,
    rc::Rc,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread::ThreadId,
};

#[derive(Default)]
// Kept by the graphics factory, never by a Windows TLS destructor: releasing
// D3D devices under the loader lock during thread exit can deadlock the driver.
// Each adapter/render-thread pair has one current generation. Active leases
// retain replaced generations until their contexts have retired.
pub(super) struct DisplayCache(Mutex<HashMap<(ThreadId, u64), Arc<SharedDisplay>>>);
impl std::fmt::Debug for DisplayCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DisplayCache")
    }
}
impl DisplayCache {
    pub fn get(&self, adapter: u64) -> Option<Arc<SharedDisplay>> {
        self.0
            .lock()
            .unwrap()
            .get(&(std::thread::current().id(), adapter))
            .filter(|d| d.is_healthy())
            .cloned()
    }
    pub fn insert(&self, adapter: u64, display: Arc<SharedDisplay>) {
        // Drop a replaced device outside the cache lock.
        let old = self
            .0
            .lock()
            .unwrap()
            .insert((std::thread::current().id(), adapter), display);
        drop(old);
    }
}

pub(super) struct SharedDisplay {
    pub handle: isize,
    clients: AtomicUsize,
    healthy: Box<dyn Fn() -> bool + Send + Sync>,
    terminate: Box<dyn Fn() + Send + Sync>,
    release_device: Box<dyn Fn() + Send + Sync>,
}
impl SharedDisplay {
    pub fn new(
        handle: isize,
        healthy: impl Fn() -> bool + Send + Sync + 'static,
        terminate: impl Fn() + Send + Sync + 'static,
        release_device: impl Fn() + Send + Sync + 'static,
    ) -> Arc<Self> {
        Arc::new(Self {
            handle,
            clients: AtomicUsize::new(0),
            healthy: Box::new(healthy),
            terminate: Box::new(terminate),
            release_device: Box::new(release_device),
        })
    }
    pub fn is_healthy(&self) -> bool {
        (self.healthy)()
    }
    pub fn lease(self: &Arc<Self>) -> Rc<dyn IDisposable> {
        self.clients.fetch_add(1, Ordering::Relaxed);
        Rc::new(Lease {
            owner: self.clone(),
            released: Cell::new(false),
        })
    }
}
impl Drop for SharedDisplay {
    fn drop(&mut self) {
        debug_assert_eq!(*self.clients.get_mut(), 0);
        (self.release_device)();
    }
}
struct Lease {
    owner: Arc<SharedDisplay>,
    released: Cell<bool>,
}
impl IDisposable for Lease {
    fn dispose(&self) {
        if !self.released.replace(true) {
            let remaining = self.owner.clients.fetch_sub(1, Ordering::AcqRel) - 1;
            if remaining == 0 {
                (self.owner.terminate)();
            }
        }
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (
        Arc<SharedDisplay>,
        Arc<AtomicUsize>,
        Arc<AtomicUsize>,
        Arc<std::sync::atomic::AtomicBool>,
    ) {
        let terminated = Arc::new(AtomicUsize::new(0));
        let released = Arc::new(AtomicUsize::new(0));
        let healthy = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let (t, r, h) = (terminated.clone(), released.clone(), healthy.clone());
        (
            SharedDisplay::new(
                17,
                move || h.load(Ordering::Relaxed),
                move || {
                    t.fetch_add(1, Ordering::Relaxed);
                },
                move || {
                    r.fetch_add(1, Ordering::Relaxed);
                },
            ),
            terminated,
            released,
            healthy,
        )
    }
    #[test]
    fn one_context_cannot_terminate_another_contexts_display() {
        let (owner, terminated, released, _) = fixture();
        let a = owner.lease();
        let b = owner.lease();
        a.dispose();
        a.dispose();
        drop(a);
        assert_eq!(terminated.load(Ordering::Relaxed), 0);
        drop(owner);
        assert_eq!(released.load(Ordering::Relaxed), 0);
        b.dispose();
        assert_eq!(terminated.load(Ordering::Relaxed), 1);
        assert_eq!(released.load(Ordering::Relaxed), 0);
        drop(b);
        assert_eq!(released.load(Ordering::Relaxed), 1);
    }
    #[test]
    fn old_disposed_lease_cannot_terminate_a_reinitialized_display() {
        let (owner, terminated, released, _) = fixture();
        let a = owner.lease();
        a.dispose();
        assert_eq!(terminated.load(Ordering::Relaxed), 1);
        let b = owner.lease();
        drop(a);
        assert_eq!(terminated.load(Ordering::Relaxed), 1);
        b.dispose();
        assert_eq!(terminated.load(Ordering::Relaxed), 2);
        drop(b);
        drop(owner);
        assert_eq!(released.load(Ordering::Relaxed), 1);
    }
    #[test]
    fn failed_construction_releases_lease_and_does_not_pin_a_lost_device() {
        let (owner, terminated, released, healthy) = fixture();
        let lease = owner.lease();
        healthy.store(false, Ordering::Relaxed);
        assert!(!owner.is_healthy());
        drop(owner);
        assert_eq!(released.load(Ordering::Relaxed), 0);
        drop(lease);
        assert_eq!(terminated.load(Ordering::Relaxed), 1);
        assert_eq!(released.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn cache_replaces_a_lost_generation_without_invalidating_its_leases() {
        let cache = DisplayCache::default();
        let (old, terminated, released, healthy) = fixture();
        let lease = old.lease();
        cache.insert(9, old.clone());
        assert!(Arc::ptr_eq(&old, &cache.get(9).unwrap()));
        healthy.store(false, Ordering::Relaxed);
        assert!(cache.get(9).is_none());
        let (new, _, new_released, _) = fixture();
        cache.insert(9, new.clone());
        assert!(Arc::ptr_eq(&new, &cache.get(9).unwrap()));
        drop(old);
        assert_eq!(released.load(Ordering::Relaxed), 0);
        drop(lease);
        assert_eq!(terminated.load(Ordering::Relaxed), 1);
        assert_eq!(released.load(Ordering::Relaxed), 1);
        drop(new);
        drop(cache);
        assert_eq!(new_released.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn factory_cache_releases_devices_after_render_thread_exit() {
        let cache = Arc::new(DisplayCache::default());
        let worker_cache = cache.clone();
        let released = std::thread::spawn(move || {
            let (owner, _, released, _) = fixture();
            worker_cache.insert(9, owner);
            released
        })
        .join()
        .unwrap();
        // A different rendering thread cannot accidentally reuse its display.
        assert!(cache.get(9).is_none());
        assert_eq!(released.load(Ordering::Relaxed), 0);
        drop(cache);
        assert_eq!(released.load(Ordering::Relaxed), 1);
    }
}
