//! Development-only heap observations around the real Rust QR adapter. This
//! does not replace a CPU memory-traffic profiler or measure native SDK copies.
use ferroui_media::frames::*;
use ferroui_media_qr::*;
use rxing::Writer;
use std::{
    hint::black_box,
    sync::Arc,
    time::{Duration, Instant},
};

// Isolated executable: the production library never replaces an allocator.
mod heap {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
    pub struct Observed;
    static ACTIVE: AtomicBool = AtomicBool::new(false);
    static PLANE: AtomicUsize = AtomicUsize::new(0);
    static ALLOCS: AtomicUsize = AtomicUsize::new(0);
    static REALLOCS: AtomicUsize = AtomicUsize::new(0);
    static BYTES: AtomicUsize = AtomicUsize::new(0);
    static MAX: AtomicUsize = AtomicUsize::new(0);
    static LARGE: AtomicUsize = AtomicUsize::new(0);
    static LARGE_REALLOCS: AtomicUsize = AtomicUsize::new(0);
    fn record(size: usize, reallocation: bool) {
        if !ACTIVE.load(Relaxed) {
            return;
        }
        ALLOCS.fetch_add(1, Relaxed);
        REALLOCS.fetch_add(usize::from(reallocation), Relaxed);
        BYTES.fetch_add(size, Relaxed);
        MAX.fetch_max(size, Relaxed);
        if size >= PLANE.load(Relaxed) {
            LARGE.fetch_add(1, Relaxed);
            LARGE_REALLOCS.fetch_add(usize::from(reallocation), Relaxed);
        }
    }
    // SAFETY: every allocation operation delegates unchanged to System. The
    // observer uses only atomics, performs no allocations and never dereferences
    // caller memory. Realloc preserves System's pointer/layout contract.
    unsafe impl GlobalAlloc for Observed {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            record(layout.size(), false);
            unsafe { System.alloc(layout) }
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            record(layout.size(), false);
            unsafe { System.alloc_zeroed(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            unsafe { System.dealloc(ptr, layout) }
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
            record(size, true);
            unsafe { System.realloc(ptr, layout, size) }
        }
    }
    #[derive(Debug)]
    pub struct Stats {
        pub allocations: usize,
        pub reallocations: usize,
        pub requested_bytes: usize,
        pub largest_request: usize,
        pub full_plane_requests: usize,
        pub full_plane_reallocations: usize,
    }
    struct Scope;
    impl Drop for Scope {
        fn drop(&mut self) {
            ACTIVE.store(false, Relaxed);
        }
    }
    pub fn observe<T>(plane_bytes: usize, work: impl FnOnce() -> T) -> (T, Stats) {
        assert!(!ACTIVE.load(Relaxed));
        for value in [&ALLOCS, &REALLOCS, &BYTES, &MAX, &LARGE, &LARGE_REALLOCS] {
            value.store(0, Relaxed);
        }
        PLANE.store(plane_bytes, Relaxed);
        ACTIVE.store(true, Relaxed);
        let scope = Scope;
        let result = work();
        drop(scope);
        (
            result,
            Stats {
                allocations: ALLOCS.load(Relaxed),
                reallocations: REALLOCS.load(Relaxed),
                requested_bytes: BYTES.load(Relaxed),
                largest_request: MAX.load(Relaxed),
                full_plane_requests: LARGE.load(Relaxed),
                full_plane_reallocations: LARGE_REALLOCS.load(Relaxed),
            },
        )
    }
}
#[global_allocator]
static ALLOCATOR: heap::Observed = heap::Observed;
const W: usize = 1280;
const H: usize = 720;
const LEFT: &str = "ferroui-copy-audit-left";
const RIGHT: &str = "ferroui-copy-audit-right";

fn fixture(stride: usize) -> Arc<[u8]> {
    let mut pixels = vec![255u8; stride * H * 3 / 2];
    pixels[stride * H..].fill(128);
    for (text, left) in [(LEFT, 100), (RIGHT, 800)] {
        let qr = rxing::qrcode::QRCodeWriter::default()
            .encode(text, &rxing::BarcodeFormat::QR_CODE, 320, 320)
            .unwrap();
        for y in 0..320 {
            for x in 0..320 {
                pixels[(y + 200) * stride + x + left] =
                    if qr.get(x as u32, y as u32) { 0 } else { 255 };
            }
        }
    }
    pixels.into()
}
fn analyze(
    pixels: Arc<[u8]>,
    stride: usize,
    sequence: u64,
) -> (
    Vec<QrCandidate>,
    heap::Stats,
    FrameTransferSnapshot,
    Duration,
) {
    let budget = FrameBudget::new(Default::default());
    let bytes = pixels.len() as u64;
    budget
        .counters()
        .cpu_transfer(
            PixelTransferPurpose::Probe,
            bytes,
            PresentationRoute::SharedTexture,
        )
        .unwrap();
    let frame = budget
        .lease(
            FrameMetadata {
                identity: FrameIdentity {
                    source: 1,
                    generation: 1,
                    sequence,
                },
                width: W as u32,
                height: H as u32,
                visible: FrameRegion {
                    x: 0,
                    y: 0,
                    width: W as u32,
                    height: H as u32,
                },
                format: FrameFormat::Nv12,
                color: FrameColor::Rec709Limited,
                alpha: FrameAlpha::Opaque,
                rotation: FrameRotation::None,
                mirrored: false,
                top_left_origin: true,
                timestamp: Duration::ZERO,
                device: None,
                device_generation: 1,
                planes: vec![
                    FramePlane {
                        offset: 0,
                        stride: stride as u64,
                        length: (stride * H) as u64,
                    },
                    FramePlane {
                        offset: (stride * H) as u64,
                        stride: stride as u64,
                        length: (stride * H / 2) as u64,
                    },
                ],
                retained_bytes: bytes,
            },
            pixels.clone(),
        )
        .unwrap();
    let gate = QrAnalysisGate::new(QrOptions {
        max_age: Duration::from_secs(30),
        ..Default::default()
    });
    gate.activate(1, 1);
    let active = gate.begin(frame, Duration::ZERO).unwrap();
    let begun = Instant::now();
    let (result, stats) = heap::observe(W * H, || {
        decode(
            &active,
            AnalysisPlane {
                data: &pixels,
                width: W as u32,
                height: H as u32,
                row_stride: stride,
                pixel_stride: 1,
            },
        )
    });
    let elapsed = begun.elapsed();
    let mut result = result.unwrap();
    result.sort_by(|a, b| a.text.cmp(&b.text));
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].text.as_deref(), Some(LEFT));
    assert_eq!(result[1].text.as_deref(), Some(RIGHT));
    let counters = active.counters().snapshot();
    assert_eq!(counters.preview.bytes, 0);
    (result, stats, counters, elapsed)
}
fn main() {
    let packed = fixture(W);
    let padded = fixture(W + 16);
    let (copy, negative) = heap::observe(W * H, || black_box(packed[..W * H].to_vec()));
    assert_eq!(copy.len(), W * H);
    assert!(
        negative.full_plane_requests > 0,
        "negative control must detect a full-plane clone"
    );
    println!("NEGATIVE_CONTROL {negative:?}");
    drop(copy);
    for (name, input, stride) in [("packed", packed, W), ("padded", padded, W + 16)] {
        for iteration in 0..3 {
            let (_, stats, counters, elapsed) = analyze(input.clone(), stride, iteration + 1);
            println!("RXING_0_9_3 {name} iteration={iteration} width={W} height={H} elapsed_us={} heap={stats:?} adapter_analysis={:?}", elapsed.as_micros(), counters.analysis);
            assert_eq!(
                counters.analysis.bytes,
                if stride == W { 0 } else { (W * H) as u64 }
            );
            assert_eq!(
                stats.full_plane_reallocations, 0,
                "full-plane repacking must reserve once"
            );
            assert_eq!(
                stats.full_plane_requests,
                usize::from(stride != W),
                "new full-plane-sized heap storage requires copy audit"
            );
            black_box((
                stats.allocations,
                stats.reallocations,
                stats.requested_bytes,
                stats.largest_request,
                stats.full_plane_reallocations,
            ));
        }
    }
    println!("PASS observed Rust heap requests and adapter repacking; excludes stack/inline copies, native SDK internals and unrelated workload profiles");
}
