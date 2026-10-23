//! Reproduce the allocation cost of tracking local styles during transforms.
//! Run: `cargo run -p ferrodesign-core --example style_allocations --locked`.
//!
//! Two otherwise identical documents contain the same style table and a node
//! with a synthetic 256 KiB embedded-image URI. One node links to that style;
//! the other retains identical effective paints without a binding. Each cohort
//! performs 16 public editor updates and preserves the expected binding. The
//! assertions require style tracking to add zero allocations and zero allocated
//! bytes beyond the existing editor/history/cache work for this fixture.
//!
//! Counts include successful allocation and reallocation requests made during
//! each update. Allocated bytes are cumulative requested sizes, not live memory
//! or RSS. Fixture construction, result formatting and teardown are excluded.
//! This single-threaded example neither decodes the synthetic URI nor renders
//! pixels. It measures core allocations only, with no browser/FPS or elapsed-
//! time claim. Existing history snapshots still copy paint/style data; equal
//! cohorts do not imply allocation-free editing or constant-time metadata.
//! Raw totals can vary with compiler/profile; compare cohorts in the same run.
use ferrodesign_core::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct CountingAllocator;
static TRACK: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let result = unsafe { System.alloc(layout) };
        if TRACK.load(Ordering::Relaxed) && !result.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        result
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(pointer, layout, size) };
        if TRACK.load(Ordering::Relaxed) && !result.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(size, Ordering::Relaxed);
        }
        result
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn fixture(bound: bool) -> (Editor, NodeId) {
    let mut document = Document::new("Style allocation comparison");
    let mut node = Node::new(NodeKind::Rectangle, "Image", 0.0, 0.0, 100.0, 100.0);
    node.fills = vec![Fill {
        paint: Paint::Image {
            href: format!("data:image/png;base64,{}", "A".repeat(256 * 1024)),
            scale: ImageScale::Fill,
            crop: None,
        },
        opacity: 1.0,
        visible: true,
    }];
    let style_id = 3;
    let style = Style {
        id: style_id,
        name: "Large image".into(),
        description: String::new(),
        value: StyleValue::from_node(LocalStyleKind::Fill, &node).unwrap(),
    };
    node.fill_style = bound.then_some(style_id);
    let id = document.insert_node(1, node).unwrap();
    document.styles.insert(style_id, style);
    document.next_id = 4;
    document.validate().unwrap();
    (Editor::new(document), id)
}

fn measure(bound: bool, layout: bool) -> (usize, usize) {
    let (mut editor, id) = fixture(bound);
    let mut allocation_count = 0;
    let mut byte_count = 0;
    for _ in 0..16 {
        ALLOCATIONS.store(0, Ordering::Relaxed);
        BYTES.store(0, Ordering::Relaxed);
        TRACK.store(true, Ordering::Relaxed);
        if layout {
            editor.update_nodes_with_layout(&[id], "Move image", |node| node.x += 1.0);
        } else {
            editor.update_node(id, "Move image", |node| node.x += 1.0);
        }
        TRACK.store(false, Ordering::Relaxed);
        allocation_count += ALLOCATIONS.load(Ordering::Relaxed);
        byte_count += BYTES.load(Ordering::Relaxed);
        assert_eq!(editor.document.nodes[&id].fill_style, bound.then_some(3));
    }
    (allocation_count, byte_count)
}

fn main() {
    for layout in [false, true] {
        let unbound = measure(false, layout);
        let bound = measure(true, layout);
        assert_eq!(bound.0, unbound.0, "Style tracking must add no allocation");
        assert_eq!(
            bound.1, unbound.1,
            "Style tracking must copy no additional payload"
        );
        println!("{{\"operation\":\"{}\",\"iterations\":16,\"imagePayloadBytes\":262144,\"unboundAllocations\":{},\"boundAllocations\":{},\"unboundAllocatedBytes\":{},\"boundAllocatedBytes\":{}}}",
            if layout { "update_nodes_with_layout" } else { "update_node" },
            unbound.0, bound.0, unbound.1, bound.1);
    }
}
