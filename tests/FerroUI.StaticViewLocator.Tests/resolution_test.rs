//! Measure the warmed generated locator in its own process. Other fixture tests can add
//! process-wide type registrations, which allocate on the thread that next looks up a type.

use ferroui_base::{BoxedValue, FerroObject};
use ferroui_controls::templates::{IDataTemplate, ITemplateWithParam};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_static_view_locator_tests::constructed;
use ferroui_static_view_locator_tests::resolution::view_models::{
    ContractModel, DerivedModel, ExplicitModel, FilteredModel, UnresolvedModel,
};
use ferroui_static_view_locator_tests::resolution::ResolutionViewLocator;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

fn count() {
    let _ = ALLOCATIONS.try_with(|allocations| allocations.set(allocations.get() + 1));
}

struct CountingAllocator;

// SAFETY: each method forwards its arguments to System. The thread-local counter does not allocate.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: the caller upholds the contract of `alloc`.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: the caller upholds the contract of `alloc_zeroed`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: the caller upholds the contract of `dealloc`.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        // SAFETY: the caller upholds the contract of `realloc`.
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn measure_match(locator: &ResolutionViewLocator, value: &BoxedValue, count: usize) -> u64 {
    for _ in 0..count {
        std::hint::black_box(locator.match_(Some(value)));
    }
    let before = ALLOCATIONS.get();
    for _ in 0..count {
        std::hint::black_box(locator.match_(Some(value)));
    }
    ALLOCATIONS.get() - before
}

#[test]
fn generated_resolver_covers_all_paths_without_allocating_or_constructing_during_match() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let locator = ResolutionViewLocator;
    let exact: BoxedValue = Rc::new(ExplicitModel);
    let inherited: BoxedValue = Rc::new(DerivedModel::new().upcast::<FerroObject>());
    let through_interface: BoxedValue = Rc::new(ContractModel);
    let filtered: BoxedValue = Rc::new(FilteredModel);
    let unresolved: BoxedValue = Rc::new(UnresolvedModel);

    let before = constructed();
    assert!(locator.match_(Some(&exact)));
    assert!(locator.match_(Some(&inherited)));
    assert!(locator.match_(Some(&through_interface)));
    assert!(!locator.match_(Some(&filtered)));
    assert!(!locator.match_(Some(&unresolved)));
    assert!(!locator.match_(None));
    assert_eq!(constructed(), before, "match_ creates no view");

    assert_eq!(measure_match(&locator, &exact, 10_000), 0);
    assert_eq!(measure_match(&locator, &unresolved, 10_000), 0);
    assert_eq!(constructed(), before, "match_ creates no view");

    let view = |data: &BoxedValue| {
        locator
            .build(&Some(data.clone()))
            .expect("a view")
            .get_type()
            .name()
    };
    assert_eq!(view(&inherited), "RootView");
    assert_eq!(view(&through_interface), "ContractView");
    assert_eq!(view(&exact), "ExplicitOverrideView");
    assert_eq!(view(&unresolved), "TextBlock");
}
