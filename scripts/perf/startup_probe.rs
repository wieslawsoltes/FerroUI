//! Opt-in checkpoints for the real desktop entry points; no default-build work.
use ferroui_base::diagnostics::perf_counters::{self, PerfCounter};
use ferroui_base::reactive::IDisposable;
use ferroui_base::Visual;
use ferroui_controls::application_lifetimes::ClassicDesktopStyleApplicationLifetime;
use ferroui_controls::Window;
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();
static WINDOW_SEEN: AtomicBool = AtomicBool::new(false);
static RENDER_SEEN: AtomicBool = AtomicBool::new(false);
static COMPLETED: AtomicBool = AtomicBool::new(false);
thread_local! {
    static SUBSCRIPTIONS: RefCell<Vec<Rc<dyn IDisposable>>> = const { RefCell::new(Vec::new()) };
}

pub fn start() {
    START.set(Instant::now()).unwrap();
    checkpoint("main");
    Window::window_opened_event()
        .add_class_handler::<Window>(|window, _| {
            if WINDOW_SEEN.swap(true, Ordering::Relaxed) {
                return;
            }
            checkpoint("window_opened");
            complete_if_ready(window);
        })
        .detach();
}

pub fn watch_lifetime(lifetime: &Rc<ClassicDesktopStyleApplicationLifetime>) {
    let weak = Rc::downgrade(lifetime);
    lifetime
        .startup(move |_| {
            let lifetime = weak.upgrade().unwrap();
            let window = lifetime.main_window().expect("a benchmark main window");
            checkpoint("before_show");
            let weak = window.downgrade();
            // Subscribe before Show, so a synchronous initial commit is included.
            let subscription = window.renderer().scene_invalidated(Rc::new(move |_| {
                RENDER_SEEN.store(true, Ordering::Relaxed);
                if let Some(window) = weak.upgrade() {
                    complete_if_ready(&window);
                }
            }));
            SUBSCRIPTIONS.with(|items| items.borrow_mut().push(subscription));
        })
        .detach();
}

fn complete_if_ready(window: &Window) {
    if !WINDOW_SEEN.load(Ordering::Relaxed)
        || !RENDER_SEEN.load(Ordering::Relaxed)
        || COMPLETED.swap(true, Ordering::Relaxed)
    {
        return;
    }
    // CompositingRenderer raises the notification after commit.rendered()
    // completes. This is a completed compositor render, not a scanout fence.
    checkpoint("first_rendered");
    // Verify after the timed/memory checkpoint, without creating collections.
    let visual: &Visual = window;
    let nodes = visual_nodes(visual);
    println!(
        "STARTUP_VERIFY {{\"visual_nodes\":{nodes},\"width\":{},\"height\":{},\"scaling\":{}}}",
        window.bounds().width,
        window.bounds().height,
        window.render_scaling()
    );
    std::io::stdout().flush().unwrap();
}

fn visual_nodes(visual: &Visual) -> u64 {
    1 + visual.visual_children_snapshot().map_or(0, |children| {
        children.iter().map(|child| visual_nodes(child)).sum()
    })
}

pub fn checkpoint(phase: &str) {
    let elapsed_us = START.get().unwrap().elapsed().as_secs_f64() * 1e6;
    let heap = allocations::snapshot();
    let counters = perf_counters::snapshot();
    let memory = memory();
    println!("STARTUP_PROBE {{\"phase\":\"{phase}\",\"main_elapsed_us\":{elapsed_us},\"counted\":{},\"allocation_calls\":{},\"allocated_bytes\":{},\"live_requested_bytes\":{},\"peak_requested_bytes\":{},\"rss_bytes\":{},\"phys_footprint_bytes\":{},\"peak_phys_footprint_bytes\":{},\"styles_evaluated\":{},\"themes_evaluated\":{},\"logical_attachments\":{}}}",
        cfg!(feature="startup-probe-allocations"), heap.calls, heap.bytes, heap.live, heap.peak,
        memory.0, memory.1, memory.2, counters.get(PerfCounter::StylesEvaluated),
        counters.get(PerfCounter::ControlThemesEvaluated), counters.get(PerfCounter::LogicalTreeAttachments));
    std::io::stdout().flush().unwrap();
}

#[cfg(target_os = "macos")]
fn memory() -> (u64, u64, u64) {
    // Darwin's sys/resource.h RUSAGE_INFO_V4. All fields following ri_uuid
    // are uint64_t; the kernel fills exactly this version's structure.
    #[repr(C)]
    struct Usage {
        uuid: [u8; 16],
        values: [u64; 35],
    }
    #[link(name = "proc")]
    extern "C" {
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut std::ffi::c_void) -> i32;
    }
    let mut usage = Usage {
        uuid: [0; 16],
        values: [0; 35],
    };
    // SAFETY: Usage has the C layout/size of rusage_info_v4 and is writable.
    let result = unsafe {
        proc_pid_rusage(
            std::process::id() as i32,
            4,
            (&mut usage as *mut Usage).cast(),
        )
    };
    assert_eq!(result, 0, "proc_pid_rusage(RUSAGE_INFO_V4)");
    (usage.values[6], usage.values[7], usage.values[28])
}

#[cfg(not(target_os = "macos"))]
fn memory() -> (u64, u64, u64) {
    (0, 0, 0)
}

#[cfg(not(feature = "startup-probe-allocations"))]
mod allocations {
    #[derive(Default)]
    pub struct Snapshot {
        pub calls: u64,
        pub bytes: u64,
        pub live: u64,
        pub peak: u64,
    }
    pub fn snapshot() -> Snapshot {
        Snapshot::default()
    }
}

#[cfg(feature = "startup-probe-allocations")]
mod allocations {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
    static CALLS: AtomicU64 = AtomicU64::new(0);
    static BYTES: AtomicU64 = AtomicU64::new(0);
    static LIVE: AtomicU64 = AtomicU64::new(0);
    static PEAK: AtomicU64 = AtomicU64::new(0);
    pub struct Snapshot {
        pub calls: u64,
        pub bytes: u64,
        pub live: u64,
        pub peak: u64,
    }
    pub fn snapshot() -> Snapshot {
        Snapshot {
            calls: CALLS.load(Relaxed),
            bytes: BYTES.load(Relaxed),
            live: LIVE.load(Relaxed),
            peak: PEAK.load(Relaxed),
        }
    }
    fn add(size: usize) {
        CALLS.fetch_add(1, Relaxed);
        BYTES.fetch_add(size as u64, Relaxed);
        let current = LIVE.fetch_add(size as u64, Relaxed) + size as u64;
        PEAK.fetch_max(current, Relaxed);
    }
    struct Allocator;
    // SAFETY: allocation operations/arguments are forwarded unchanged to System;
    // counters use allocation-free atomics and count only successful operations.
    unsafe impl GlobalAlloc for Allocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let result = unsafe { System.alloc(layout) };
            if !result.is_null() {
                add(layout.size());
            }
            result
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            let result = unsafe { System.alloc_zeroed(layout) };
            if !result.is_null() {
                add(layout.size());
            }
            result
        }
        unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
            unsafe { System.dealloc(p, layout) };
            LIVE.fetch_sub(layout.size() as u64, Relaxed);
        }
        unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
            let result = unsafe { System.realloc(p, layout, size) };
            if !result.is_null() {
                // Reallocation is one allocation call and requests `size` bytes.
                // Replace the old live size atomically after a successful realloc.
                CALLS.fetch_add(1, Relaxed);
                BYTES.fetch_add(size as u64, Relaxed);
                let current = if size >= layout.size() {
                    let delta = (size - layout.size()) as u64;
                    LIVE.fetch_add(delta, Relaxed) + delta
                } else {
                    let delta = (layout.size() - size) as u64;
                    LIVE.fetch_sub(delta, Relaxed) - delta
                };
                PEAK.fetch_max(current, Relaxed);
            }
            result
        }
    }
    #[global_allocator]
    static ALLOCATOR: Allocator = Allocator;
}
