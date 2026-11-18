//! Emptied buffers released once a burst is over, and freed pages given back at rest (#836).
//!
//! An emptied buffer keeps its storage for its next use, at most
//! [`RETAINED_BUFFER_BYTES`](super::RETAINED_BUFFER_BYTES) of it
//! ([`ReleaseExcess`](super::ReleaseExcess)). A burst (the show of a large tree, the close of a
//! screen) runs several passes, and a buffer shrunk to that limit at the end of one is grown again
//! by the next, so the limit cannot be low; and what the buffers keep at the limit stays for as
//! long as the window lives (docs/porting/desktop-performance.md, section 9.98: about 95 MB of the
//! 135 MB a closed wall of 58,696 tiles left).
//!
//! So an emptied buffer is still taken down to that limit at once, as before
//! ([`ReleaseAtRest::release_burst`], which tells whether it gave storage back), and a holder
//! whose buffer gave storage back (a burst filled it) asks for the job of its thread
//! ([`rest_after_burst`]): the holder is registered (once in its life), and one job at the idle
//! priority of the dispatcher is posted (and not again while it waits). The job runs once the
//! burst is over, when nothing else is queued: it takes every registered buffer that is empty
//! down to [`RESTING_BUFFER_BYTES`], and then asks the allocator to give its free pages back to
//! the system ([`release_free_pages`]). No timer, and nothing in the steady path: a buffer within
//! the limit is checked as before (one comparison of its capacity), nothing is registered and no
//! job is posted until a buffer of the thread went beyond the limit. On a thread without a
//! dispatcher nothing is posted. The render thread has the same step when no tick is pending
//! (`ServerCompositor::render_core`), taken only after one of its buffers gave storage back.
//!
//! Not from upstream, whose buffers are collected with the heap.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::mem::size_of;

use crate::threading::{Dispatcher, DispatcherPriority};

/// What an emptied buffer keeps once the burst that filled it is over, in bytes.
pub const RESTING_BUFFER_BYTES: usize = 64 << 10;

/// How many items of `T` a resting buffer keeps.
const fn resting_items<T>(extra_bytes_per_item: usize) -> usize {
    let item = size_of::<T>() + extra_bytes_per_item;
    if item == 0 { usize::MAX } else { RESTING_BUFFER_BYTES / item }
}

/// A buffer that fills in bursts and gives its storage back once the burst is over.
pub trait ReleaseAtRest {
    /// Gives back the storage beyond [`RETAINED_BUFFER_BYTES`](super::RETAINED_BUFFER_BYTES)
    /// now, as [`release_excess`](super::ReleaseExcess::release_excess) does, and tells whether
    /// it gave any back: a burst filled the buffer, and its holder asks for the job of its thread
    /// then ([`rest_after_burst`]). A buffer within the limit is not touched.
    fn release_burst(&mut self) -> bool;

    /// Gives back the storage of an empty buffer beyond [`RESTING_BUFFER_BYTES`], and tells whether
    /// it gave any back. A buffer that holds content is not touched.
    fn release_at_rest(&mut self) -> bool;
}

macro_rules! release_at_rest {
    ($self:ident, $extra:expr, $item:ty) => {{
        let keep = resting_items::<$item>($extra);
        let release = $self.is_empty() && $self.capacity() > keep;
        if release {
            $self.shrink_to(keep);
        }
        release
    }};
}

macro_rules! release_burst {
    ($self:ident, $extra:expr, $item:ty) => {{
        let limit = super::RETAINED_BUFFER_BYTES / (size_of::<$item>() + $extra).max(1);
        let burst = $self.capacity() > limit;
        if burst {
            super::ReleaseExcess::release_excess($self);
        }
        burst
    }};
}

impl<T> ReleaseAtRest for Vec<T> {
    #[inline]
    fn release_burst(&mut self) -> bool {
        release_burst!(self, 0, T)
    }

    #[inline]
    fn release_at_rest(&mut self) -> bool {
        release_at_rest!(self, 0, T)
    }
}

impl<T> ReleaseAtRest for VecDeque<T> {
    #[inline]
    fn release_burst(&mut self) -> bool {
        release_burst!(self, 0, T)
    }

    #[inline]
    fn release_at_rest(&mut self) -> bool {
        release_at_rest!(self, 0, T)
    }
}

impl<T, S> ReleaseAtRest for HashSet<T, S>
where
    T: Eq + std::hash::Hash,
    S: std::hash::BuildHasher,
{
    #[inline]
    fn release_burst(&mut self) -> bool {
        release_burst!(self, 1, T)
    }

    #[inline]
    fn release_at_rest(&mut self) -> bool {
        release_at_rest!(self, 1, T)
    }
}

impl<K, V, S> ReleaseAtRest for HashMap<K, V, S>
where
    K: Eq + std::hash::Hash,
    S: std::hash::BuildHasher,
{
    #[inline]
    fn release_burst(&mut self) -> bool {
        release_burst!(self, 1, (K, V))
    }

    #[inline]
    fn release_at_rest(&mut self) -> bool {
        release_at_rest!(self, 1, (K, V))
    }
}

/// What releases the buffers of a holder: `false` once the holder is gone.
type Release = Box<dyn Fn() -> bool>;

#[derive(Default)]
struct Resting {
    /// What releases the buffers of each registered holder of the thread.
    holders: Vec<Release>,
    /// Whether the job is posted and has not run yet.
    posted: bool,
}

crate::ferro_thread_local! {
    static RESTING: RefCell<Resting> = RefCell::new(Resting::default());
}

/// Registers what releases the buffers of a holder once a burst is over: `release` takes each
/// empty buffer of the holder down to [`RESTING_BUFFER_BYTES`]
/// ([`ReleaseAtRest::release_at_rest`]) and returns `false` once the holder is gone (it holds
/// no buffer alive: a weak handle of the holder), which ends the registration. It is called on
/// this thread, by the job of the thread. [`rest_after_burst`] registers a holder when it first
/// needs it.
pub fn register_resting_buffers(release: impl Fn() -> bool + 'static) {
    let release: Release = Box::new(release);
    // While the thread is torn down there is no burst to come.
    let _ = RESTING.try_with(|resting| resting.borrow_mut().holders.push(release));
}

/// For a holder one of whose buffers gave storage back ([`ReleaseAtRest::release_burst`]
/// answered `true`): registers the holder the first time (`registered` is its flag, `release`
/// makes what releases its buffers, as [`register_resting_buffers`] takes it) and posts the job
/// of the thread, unless it is posted already. Nothing on a thread without a dispatcher: its
/// buffers stay at the limit they were taken down to.
pub fn rest_after_burst<F: Fn() -> bool + 'static>(registered: &Cell<bool>, release: impl FnOnce() -> F) {
    let Some(dispatcher) = Dispatcher::from_thread(std::thread::current().id()) else { return };
    if !registered.replace(true) {
        register_resting_buffers(release());
    }
    let post = RESTING.try_with(|resting| !std::mem::replace(&mut resting.borrow_mut().posted, true));
    if post == Ok(true) {
        dispatcher.post_local(release_resting_buffers, DispatcherPriority::SYSTEM_IDLE);
    }
}

/// The job: every registered buffer of the thread that is empty is taken down to
/// [`RESTING_BUFFER_BYTES`], and the allocator gives its free pages back.
pub fn release_resting_buffers() {
    // Taken out while they run: a release may register a holder.
    let Ok(holders) = RESTING.try_with(|resting| {
        let mut resting = resting.borrow_mut();
        resting.posted = false;
        std::mem::take(&mut resting.holders)
    }) else {
        return;
    };
    let mut alive: Vec<Release> = holders.into_iter().filter(|release| release()).collect();
    let _ = RESTING.try_with(|resting| {
        let mut resting = resting.borrow_mut();
        alive.append(&mut resting.holders);
        resting.holders = alive;
    });
    release_free_pages();
}

/// Whether [`release_free_pages`] asks the allocator: `FERROUI_RELEASE_FREE_PAGES=0` turns the
/// call off, so that resident memory can be measured with and without it.
fn releases_free_pages() -> bool {
    static RELEASES: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *RELEASES.get_or_init(|| std::env::var_os("FERROUI_RELEASE_FREE_PAGES").is_none_or(|value| value != "0"))
}

/// Asks the allocator of the system to give the pages it keeps free back to the system, where
/// the platform has such a call: `malloc_zone_pressure_relief` on Apple platforms, `malloc_trim`
/// with the GNU C library. Elsewhere it does nothing. A program with an allocator of its own is
/// not affected by it.
pub fn release_free_pages() {
    if !releases_free_pages() {
        return;
    }
    #[cfg(target_vendor = "apple")]
    {
        extern "C" {
            fn malloc_zone_pressure_relief(zone: *mut std::ffi::c_void, goal: usize) -> usize;
        }
        // SAFETY: the null zone is every zone of the process; a goal of 0 asks for all it can.
        unsafe {
            malloc_zone_pressure_relief(std::ptr::null_mut(), 0);
        }
    }
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        // SAFETY: `malloc_trim` takes a number of bytes to keep at the top of the heap.
        unsafe {
            libc::malloc_trim(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        register_resting_buffers, release_resting_buffers, rest_after_burst, ReleaseAtRest, RESTING_BUFFER_BYTES,
    };
    use crate::utilities::RETAINED_BUFFER_BYTES;
    use std::cell::{Cell, RefCell};
    use std::rc::{Rc, Weak};

    #[test]
    fn an_empty_buffer_at_rest_keeps_the_floor_and_one_with_content_is_left_alone() {
        let mut empty: Vec<u64> = Vec::with_capacity(RETAINED_BUFFER_BYTES / 8);
        assert!(empty.release_at_rest());
        assert!(!empty.release_at_rest());
        assert_eq!(empty.capacity(), RESTING_BUFFER_BYTES / 8);
        let mut full: Vec<u64> = vec![1; RETAINED_BUFFER_BYTES / 8];
        assert!(!full.release_at_rest());
        assert_eq!(full.capacity(), RETAINED_BUFFER_BYTES / 8);
    }

    #[test]
    fn without_a_dispatcher_a_buffer_is_released_to_the_limit_at_once() {
        // A thread of its own: it has no dispatcher, so no job can run there.
        std::thread::spawn(|| {
            let holder = Rc::new(RefCell::new(Vec::<u8>::with_capacity(3 * RETAINED_BUFFER_BYTES)));
            let weak: Weak<RefCell<Vec<u8>>> = Rc::downgrade(&holder);
            register_resting_buffers(move || {
                let Some(buffer) = weak.upgrade() else { return false };
                buffer.borrow_mut().release_at_rest();
                true
            });
            assert!(holder.borrow_mut().release_burst(), "the burst is given back to the limit");
            assert_eq!(holder.borrow().capacity(), RETAINED_BUFFER_BYTES);
            assert!(!holder.borrow_mut().release_burst(), "a buffer within the limit is not touched");

            // The job takes it to the floor, and forgets the holder once it is gone.
            release_resting_buffers();
            assert_eq!(holder.borrow().capacity(), RESTING_BUFFER_BYTES);
            drop(holder);
            release_resting_buffers();
            assert_eq!(super::RESTING.with(|resting| resting.borrow().holders.len()), 0);
        })
        .join()
        .unwrap();
    }

    #[test]
    fn a_holder_is_registered_after_a_burst_only_on_a_thread_with_a_dispatcher() {
        // A thread without a dispatcher: nothing is registered, nothing is posted.
        std::thread::spawn(|| {
            let registered = Cell::new(false);
            rest_after_burst(&registered, || || true);
            assert!(!registered.get());
            assert_eq!(super::RESTING.with(|resting| resting.borrow().holders.len()), 0);
            assert!(!super::RESTING.with(|resting| resting.borrow().posted));
        })
        .join()
        .unwrap();
    }
}
