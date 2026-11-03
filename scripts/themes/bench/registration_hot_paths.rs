//! Standalone Linux public-API probe for cold registration and warm conversion.
//!
//! Link this same file against reference and candidate ferroui-base rlibs with
//! the production flags. Each invocation prints:
//! `lane operations checksum thread_cpu_ns wall_ns`.
//! Run `check` first. Run timing lanes in fresh, alternating pinned processes.
//! The optional second argument overrides the iteration count. Property
//! registration intentionally retains definitions, as the public API does.

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::*;
use std::any::{Any, TypeId};
use std::hint::black_box;
use std::rc::Rc;
use std::time::Instant;

macro_rules! class {
    ($name:ident: $base:ident) => {
        #[repr(C)]
        pub struct $name { base: $base }
        ferro_class!($name: $base);
        ferro_impl_classes!($name: FerroObjectImpl);
        #[allow(dead_code)]
        impl $name {
            fn construct() -> Self { Self { base: $base::construct() } }
            fn new() -> Ref<Self> { instantiate(Self::construct()) }
        }
    };
}

class!(Target: FerroObject);
class!(Derived: Target);
class!(Other: FerroObject);
class!(Element0: Target);
class!(Element1: Target);
class!(Element2: Target);
class!(Element3: Target);
class!(Element4: Target);
class!(Element5: Target);
class!(RegistrationOwner: FerroObject);

fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue { Rc::new(value) }

fn cpu_ns() -> u64 {
    #[repr(C)]
    struct Timespec { seconds: i64, nanoseconds: i64 }
    unsafe extern "C" { fn clock_gettime(clock: i32, result: *mut Timespec) -> i32; }
    let mut time = Timespec { seconds: 0, nanoseconds: 0 };
    // Linux CLOCK_THREAD_CPUTIME_ID: time spent on this benchmark thread.
    assert_eq!(unsafe { clock_gettime(3, &mut time) }, 0);
    time.seconds as u64 * 1_000_000_000 + time.nanoseconds as u64
}

fn measure(lane: &str, iterations: usize, operations_per_iteration: usize, mut operation: impl FnMut() -> u64) {
    let wall = Instant::now();
    let cpu = cpu_ns();
    let mut checksum = 0_u64;
    for _ in 0..iterations {
        checksum = checksum.wrapping_add(black_box(operation()));
    }
    let elapsed_cpu = cpu_ns() - cpu;
    let elapsed_wall = wall.elapsed().as_nanos();
    println!("{lane} {} {} {elapsed_cpu} {elapsed_wall}", iterations * operations_per_iteration, black_box(checksum));
}

fn register_elements() {
    let registrations: [fn(); 8] = [
        ValueTypes::register_element_ref::<FerroObject>,
        ValueTypes::register_element_ref::<Target>,
        ValueTypes::register_element_ref::<Element0>,
        ValueTypes::register_element_ref::<Element1>,
        ValueTypes::register_element_ref::<Element2>,
        ValueTypes::register_element_ref::<Element3>,
        ValueTypes::register_element_ref::<Element4>,
        ValueTypes::register_element_ref::<Element5>,
    ];
    for register in black_box(registrations) { register(); }
}

fn register_properties(target: &Ref<Target>) -> u64 {
    let a = FerroProperty::register::<RegistrationOwner, i32>("Ordinal", 7);
    let b = FerroProperty::register::<RegistrationOwner, u64>("Counter", 91);
    let c = FerroProperty::register_with::<RegistrationOwner, f64>(
        "Scalar", StyledPropertyOptions::new(13.5_f64).coerce(|_, value| value.max(0.0)),
    );
    let d = FerroProperty::register::<RegistrationOwner, bool>("Flag", true);
    let e = FerroProperty::register_with::<RegistrationOwner, String>(
        "Text", StyledPropertyOptions::new(String::from("value")).validate(|value| !value.is_empty()),
    );
    let f = FerroProperty::register::<RegistrationOwner, Option<i32>>("Optional", Some(17));
    let g = FerroProperty::register::<RegistrationOwner, Option<Ref<Target>>>("Object", Some(target.clone()));
    let h = FerroProperty::register::<RegistrationOwner, ElementRef<Target>>("Element", ElementRef::of(target));
    [a.id(), b.id(), c.id(), d.id(), e.id(), f.id(), g.id(), h.id()].into_iter().map(u64::from).sum()
}

fn check() {
    register_elements();
    let target = Target::new();
    let source = boxed(target.clone());
    let destination = ValueType::of::<ElementRef<Target>>();
    let live = ValueTypes::try_convert_registered(&source, destination).unwrap();
    assert!(live.downcast_ref::<ElementRef<Target>>().unwrap().points_to(&target));
    let borrowed = ValueTypes::try_convert_for_property(&target, destination.id()).unwrap();
    assert!(borrowed.downcast_ref::<ElementRef<Target>>().unwrap().points_to(&target));
    let wrong = boxed(Other::new().upcast::<FerroObject>());
    assert!(ValueTypes::try_convert_registered(&wrong, destination).is_none());
    assert!(ValueTypes::try_convert_for_property(&true, destination.id()).is_none());
    let expired = { let transient = Target::new(); boxed(ElementRef::of(&transient)) };
    assert!(ValueTypes::try_convert_registered(&expired, ValueType::of::<Ref<Target>>()).is_none());
    let null = ValueTypes::try_convert_for_property(expired.as_any(), TypeId::of::<Option<Ref<Target>>>()).unwrap();
    assert!(null.downcast_ref::<Option<Ref<Target>>>().unwrap().is_none());
    let root = target.clone().upcast::<FerroObject>();
    assert!(ValueTypes::try_convert_for_property(&root, TypeId::of::<ElementRef<FerroObject>>()).is_some());
    let before = FerroPropertyRegistry::instance().get_registered(RegistrationOwner::TYPE).len();
    black_box(register_properties(&target));
    assert_eq!(FerroPropertyRegistry::instance().get_registered(RegistrationOwner::TYPE).len(), before + 8);
    println!("check ok");
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let lane = arguments.next().unwrap_or_else(|| "check".into());
    if lane == "check" { check(); return; }
    let default_count = match lane.as_str() {
        "register_element" => 10_000,
        "property_registration" => 1_000,
        _ => 1_000_000,
    };
    let count = arguments.next().map(|count| count.parse::<usize>().expect("positive iteration count"))
        .unwrap_or(default_count);
    assert!(count > 0);

    // Establish the registry and class state before any warm or repeated lane.
    register_elements();
    let target = Target::new();
    let source = boxed(target.clone());
    let destination = ValueType::of::<ElementRef<Target>>();
    match lane.as_str() {
        "register_element" => measure(&lane, count, 8, || { register_elements(); 8 }),
        "property_registration" => {
            let registry = FerroPropertyRegistry::instance();
            let before = registry.get_registered(RegistrationOwner::TYPE).len();
            measure(&lane, count, 8, || register_properties(black_box(&target)));
            assert_eq!(registry.get_registered(RegistrationOwner::TYPE).len(), before + count * 8);
        }
        "boxed_live" => measure(&lane, count, 1, || {
            ValueTypes::try_convert_registered(black_box(&source), black_box(destination)).is_some() as u64
        }),
        "borrowed_live" => measure(&lane, count, 1, || {
            ValueTypes::try_convert_for_property(black_box(&target as &dyn Any), black_box(destination.id())).is_some() as u64
        }),
        "boxed_wrong" => {
            let wrong = boxed(Other::new().upcast::<FerroObject>());
            measure(&lane, count, 1, || {
                ValueTypes::try_convert_registered(black_box(&wrong), black_box(destination)).is_none() as u64
            });
        }
        "borrowed_wrong" => measure(&lane, count, 1, || {
            ValueTypes::try_convert_for_property(black_box(&true as &dyn Any), black_box(destination.id())).is_none() as u64
        }),
        "boxed_expired" | "borrowed_expired" => {
            let expired = { let transient = Target::new(); boxed(ElementRef::of(&transient)) };
            if lane == "boxed_expired" {
                let destination = ValueType::of::<Ref<Target>>();
                measure(&lane, count, 1, || {
                    ValueTypes::try_convert_registered(black_box(&expired), black_box(destination)).is_none() as u64
                });
            } else {
                let destination = TypeId::of::<Option<Ref<Target>>>();
                measure(&lane, count, 1, || {
                    ValueTypes::try_convert_for_property(black_box(expired.as_any()), black_box(destination)).is_some() as u64
                });
            }
        }
        _ => panic!("unknown lane: {lane}; expected check, register_element, property_registration, boxed_live, borrowed_live, boxed_wrong, borrowed_wrong, boxed_expired, or borrowed_expired"),
    }
}
