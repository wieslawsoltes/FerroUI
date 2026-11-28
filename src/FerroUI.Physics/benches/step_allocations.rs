//! Counts allocations in warmed simulation steps. Run with:
//! cargo bench -p ferroui-physics --bench physics-step-allocations
//!
//! This executable intentionally reports allocation requests rather than timing:
//! instrumentation changes allocator cost, and byte totals are not peak memory.

use ferroui_physics::prelude::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct CountingAllocator;

static TRACK: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static REQUESTED_BYTES: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn record_request(size: usize) {
    if TRACK.load(Ordering::Relaxed) {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        REQUESTED_BYTES.fetch_add(size, Ordering::Relaxed);
    }
}

// SAFETY: Every operation delegates to System with its unchanged pointer/layout.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_request(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record_request(new_size);
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

fn measure(name: &str, steps: usize, mut step: impl FnMut()) {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    REQUESTED_BYTES.store(0, Ordering::Relaxed);
    TRACK.store(true, Ordering::Relaxed);
    for _ in 0..steps {
        step();
    }
    TRACK.store(false, Ordering::Relaxed);
    println!(
        "{name}: steps={steps}, allocation_requests={}, requested_bytes={}",
        ALLOCATIONS.load(Ordering::Relaxed),
        REQUESTED_BYTES.load(Ordering::Relaxed)
    );
}

fn rigid_bodies() {
    let mut world = PhysicsWorld::new(Vec2::new(0.0, 9.81));
    world.add_body(BodyBuilder::static_body(Shape2D::rect(40.0, 1.0)).position((0.0, 10.0)));
    for y in 0..16 {
        for x in 0..16 {
            world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(0.6, 0.6))
                    .position((x as f32 * 0.61 - 5.0, 9.2 - y as f32 * 0.61))
                    .can_sleep(false)
                    .restitution(0.0),
            );
        }
    }
    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }
    measure("256 awake boxes", 240, || {
        std::hint::black_box(world.step(1.0 / 60.0));
    });
}

fn particles() {
    let mut world = ParticleSystem::new(ParticleConfig::default());
    world.spawn_grid(Aabb2::new(Vec2::new(0.5, 0.5), Vec2::new(15.5, 9.5)), 2048);
    for _ in 0..30 {
        world.step_cpu(1.0 / 60.0);
    }
    measure("2048 CPU particles", 120, || {
        std::hint::black_box(world.step_cpu(1.0 / 60.0));
    });
}

fn main() {
    println!("Backend: {}", SimdBackend::active().name());
    for round in 1..=5 {
        println!("Round {round}");
        rigid_bodies();
        particles();
    }
}
