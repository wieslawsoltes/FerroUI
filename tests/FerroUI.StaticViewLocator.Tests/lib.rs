//! The tests of ferroui-static-view-locator that compile and run generated locators: a port of the
//! runtime tests of StaticViewLocator (`StaticViewLocatorGeneratorRuntimeTests.cs`,
//! `StaticViewLocatorGeneratorIntegrationRuntimeTests.cs`,
//! `StaticViewLocatorGeneratorCombinedAdapterTests.cs` and the run-time half of
//! `GeneratedResolverCoversAllPathsWithoutAllocatingOrConstructingDuringMatch`).
//!
//! The reference compiles the source of each test into an assembly of its own and runs the
//! generator over it. Here each test is a module of this crate, with its view models, views and
//! locator, and the build script runs the generator over the crate, as an application's does. Every
//! locator sees the view models of every module (the view models of a compilation), so a module
//! names its types so that the conventions of the other modules do not map them.

/// What the fixtures name.
pub mod prelude {
    pub use ferroui_base::input::InputElementImpl;
    pub use ferroui_base::interactivity::InteractiveImpl;
    pub use ferroui_base::layout::LayoutableImpl;
    pub use ferroui_base::{
        ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObject, FerroObjectImpl, Ref, StyledElementImpl,
        VisualImpl,
    };
    pub use ferroui_controls::primitives::TemplatedControlImpl;
    pub use ferroui_controls::{ContentControlImpl, Control, ControlImpl, UserControl};
    pub use ferroui_static_view_locator::{static_view_locator, static_view_mapping};
}

use std::cell::Cell;

ferroui_base::ferro_thread_local! {
    /// The views the fixtures created on the thread (`ConstructionCounter.Count`).
    static CONSTRUCTED: Cell<usize> = const { Cell::new(0) };
}

/// Counts a view created by a fixture.
pub fn count_construction() {
    CONSTRUCTED.with(|count| count.set(count.get() + 1));
}

/// The number of views the fixtures created on the thread.
pub fn constructed() -> usize {
    CONSTRUCTED.with(Cell::get)
}

/// A view of the tests: a class of the object model derived from `UserControl`, created by
/// `new()`, which counts it.
macro_rules! test_view {
    ($name:ident) => {
        #[repr(C)]
        pub struct $name {
            base: UserControl,
        }

        ferro_class!($name: UserControl);
        ferro_impl_classes!(
            $name: FerroObjectImpl,
            StyledElementImpl,
            VisualImpl,
            LayoutableImpl,
            InteractiveImpl,
            InputElementImpl,
            ControlImpl,
            TemplatedControlImpl,
            ContentControlImpl
        );
        ferro_class_info!($name { new: $name::new });

        impl $name {
            pub fn construct() -> Self {
                Self { base: UserControl::construct() }
            }

            pub fn new() -> Ref<Self> {
                $crate::count_construction();
                instantiate(Self::construct())
            }
        }
    };
}

// After the macro: the modules invoke it by its name.
pub mod combined;
pub mod covariant;
pub mod custom_build;
pub mod factory;
pub mod generic;
pub mod inheritance;
pub mod integration;
pub mod interfaces;
pub mod portal;
pub mod registered;
pub mod resolution;

#[cfg(test)]
pub(crate) mod support {
    use ferroui_base::BoxedValue;
    use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
    use ferroui_controls::Control;
    use ferroui_base::Ref;
    use std::rc::Rc;

    /// The unit test application of a test (the headless test attribute of the reference).
    pub fn start() -> UnitTestApplicationScope {
        UnitTestApplication::start(TestServices::styled_window())
    }

    /// A view model as the value of a property: the view model itself, shared.
    pub fn value<T: PartialEq + 'static>(view_model: T) -> BoxedValue {
        Rc::new(view_model)
    }

    /// The name of the class of a control (`GetType().Name`).
    pub fn class_name(control: &Ref<Control>) -> &'static str {
        control.get_type().name()
    }

    pub mod allocations {
        use std::alloc::{GlobalAlloc, Layout, System};
        use std::cell::Cell;

        ferroui_base::ferro_thread_local! {
            static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
        }

        fn count() {
            // A thread that is ending has no counter anymore.
            let _ = ALLOCATIONS.try_with(|allocations| allocations.set(allocations.get() + 1));
        }

        /// The allocations of the thread so far (`GC.GetAllocatedBytesForCurrentThread`, in
        /// allocations).
        pub fn current() -> u64 {
            ALLOCATIONS.try_with(Cell::get).unwrap_or(0)
        }

        struct CountingAllocator;

        // SAFETY: every member forwards to the system allocator with the arguments it was
        // given; the counter is a plain thread-local cell that does not allocate.
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
    }
}
