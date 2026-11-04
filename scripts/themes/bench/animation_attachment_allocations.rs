//! Allocation requests and requested bytes for warmed animation attachment.
//!
//! Eight uncounted warmups precede sixteen independent counted attachments.
//! The interval includes Observable::return_(true) and Animation::apply.
//! Target/animation setup, stepping, assertions, disposal, lease destruction,
//! sample storage, and printing are outside the interval. This companion is
//! separate from the unchanged keyframe timing workload.

use ferroui_base::animation::{
    Animation, Cue, FillMode, IClock, IterationCount, KeyFrame, PlayState, TimeSpan,
};
use ferroui_base::layout::Layoutable;
use ferroui_base::reactive::{Disposable, IDisposable, IObservable, IObserver, Observable};
use ferroui_base::styling::Setter;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

struct CountingAllocator;

static COUNTING: AtomicBool = AtomicBool::new(false);
static REQUESTS: AtomicU64 = AtomicU64::new(0);
static REQUESTED_BYTES: AtomicU64 = AtomicU64::new(0);

fn record(size: usize) {
    if COUNTING.load(Ordering::Relaxed) {
        REQUESTS.fetch_add(1, Ordering::Relaxed);
        REQUESTED_BYTES.fetch_add(size as u64, Ordering::Relaxed);
    }
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(size);
        unsafe { System.realloc(pointer, layout, size) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

struct ManualClock {
    observer: RefCell<Option<Rc<dyn IObserver<TimeSpan>>>>,
    state: Cell<PlayState>,
}

impl ManualClock {
    fn step(&self, ticks: i64) {
        let observer = self.observer.borrow().clone().unwrap();
        observer.on_next(TimeSpan::from_ticks(ticks));
    }
}

impl IObservable<TimeSpan> for ManualClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        *self.observer.borrow_mut() = Some(observer);
        Disposable::empty()
    }
}

impl IClock for ManualClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }

    fn set_play_state(&self, value: PlayState) {
        self.state.set(value);
    }
}

fn main() {
    let target = Layoutable::new();
    target.set_width(100.0);
    let animation = Animation::new();
    animation.set_duration(TimeSpan::from_ticks(1_000_000));
    animation.set_fill_mode(FillMode::Both);
    animation.set_iteration_count(IterationCount::INFINITE);
    for (cue, width) in [(0.0, 100.0), (0.5, 250.0), (1.0, 200.0)] {
        animation.children().add(KeyFrame::with_cue(
            Cue::new(cue),
            [Setter::new(Layoutable::width_property(), width) as _],
        ));
    }
    let clock = Rc::new(ManualClock {
        observer: RefCell::new(None),
        state: Cell::new(PlayState::Run),
    });
    let mut samples = [(0u64, 0u64); 16];
    for index in 0..24 {
        REQUESTS.store(0, Ordering::Relaxed);
        REQUESTED_BYTES.store(0, Ordering::Relaxed);
        COUNTING.store(index >= 8, Ordering::Relaxed);
        let subscription = animation.apply(
            &target,
            Some(clock.clone()),
            Observable::return_(true),
            None,
            false,
        );
        COUNTING.store(false, Ordering::Relaxed);
        let sample = (
            REQUESTS.load(Ordering::Relaxed),
            REQUESTED_BYTES.load(Ordering::Relaxed),
        );
        clock.step(0);
        clock.step(500_000);
        assert_eq!(target.width(), 250.0);
        subscription.dispose();
        drop(subscription);
        if index >= 8 {
            samples[index - 8] = sample;
        }
    }
    assert!(samples.iter().all(|sample| *sample == samples[0]));
    for (index, (requests, bytes)) in samples.iter().enumerate() {
        println!("attachment {index} {requests} {bytes}");
    }
}
