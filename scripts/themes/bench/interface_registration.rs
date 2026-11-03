//! Standalone public-API probe for default interface registration.
//!
//! Link the same source against reference and candidate release libraries.
//! The optional argument is the number of repetitions (default 4096). Output
//! is registration wall ns, conversion wall ns, allocations, allocated bytes.
//! The allocation sample is one warmed pass over 32 class declarations.
//! Repeated timing runs should alternate pinned fresh processes.

use ferroui_base::{BoxedValue, FerroObject, ObjectType, Ref, Upcast};
use ferroui_base::data::core::{ValueType, ValueTypes};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

struct Counting;
static TRACK: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if TRACK.load(Ordering::Relaxed) && !p.is_null() {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout); }
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let p = unsafe { System.realloc(p, layout, size) };
        if TRACK.load(Ordering::Relaxed) && !p.is_null() {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(size, Ordering::Relaxed);
        }
        p
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;

#[derive(Clone, PartialEq)]
pub struct InterfaceHandle(Ref<FerroObject>);
impl<T: ObjectType + Upcast<FerroObject>> From<Ref<T>> for InterfaceHandle {
    fn from(object: Ref<T>) -> Self { Self(object.upcast()) }
}

macro_rules! classes {
    ($($name:ident),+ $(,)?) => {
        $(
            #[repr(C)]
            pub struct $name { base: FerroObject }
            ferroui_base::ferro_class!($name: FerroObject);
            ferroui_base::ferro_impl_classes!($name: ferroui_base::FerroObjectImpl);
            impl $name {
                pub fn construct() -> Self { Self { base: FerroObject::construct() } }
                pub fn new() -> Ref<Self> { ferroui_base::instantiate(Self::construct()) }
            }
            ferroui_base::ferro_class_info!($name { interfaces: [InterfaceHandle] });
        )+
        #[inline(never)]
        fn install() { $($name::__register_interfaces();)+ }
        fn objects() -> Vec<BoxedValue> { vec![$(Rc::new($name::new()) as BoxedValue),+] }
    };
}
classes!(C00,C01,C02,C03,C04,C05,C06,C07,C08,C09,C10,C11,C12,C13,C14,C15,
         C16,C17,C18,C19,C20,C21,C22,C23,C24,C25,C26,C27,C28,C29,C30,C31);

fn main() {
    let rounds = std::env::args().nth(1).and_then(|x| x.parse::<usize>().ok()).unwrap_or(4096);
    let samples = objects();
    install();
    let target = ValueType::of::<InterfaceHandle>();
    let nullable = ValueType::of::<Option<InterfaceHandle>>();
    for sample in &samples {
        let root = ValueTypes::as_object(&**sample).unwrap();
        let converted = ValueTypes::try_cast(sample, target).unwrap();
        assert!(converted.downcast_ref::<InterfaceHandle>().unwrap().0.ptr_eq(&root));
        let converted = ValueTypes::try_cast(sample, nullable).unwrap();
        assert!(converted.downcast_ref::<Option<InterfaceHandle>>().unwrap().as_ref().unwrap().0.ptr_eq(&root));
    }
    let null = ValueTypes::null_value(nullable).unwrap();
    assert!(null.downcast_ref::<Option<InterfaceHandle>>().unwrap().is_none());
    TRACK.store(true, Ordering::Relaxed);
    install();
    TRACK.store(false, Ordering::Relaxed);
    let allocations = ALLOCS.load(Ordering::Relaxed);
    let allocated_bytes = BYTES.load(Ordering::Relaxed);
    let t = Instant::now();
    for _ in 0..rounds { install(); }
    let registration_ns = t.elapsed().as_nanos();
    let t = Instant::now();
    let mut count = 0;
    for _ in 0..rounds {
        for sample in &samples {
            let converted = ValueTypes::try_cast(black_box(sample), black_box(target)).unwrap();
            black_box(converted.downcast_ref::<InterfaceHandle>().unwrap());
            count += 1;
        }
    }
    let conversion_ns = t.elapsed().as_nanos();
    black_box(count);
    println!("{registration_ns} {conversion_ns} {allocations} {allocated_bytes}");
}
