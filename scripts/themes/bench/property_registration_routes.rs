//! Linux x86-64 public-API probe for property registration, direct owner
//! lookup, and boxed set/current-value dispatch. Each mode runs in a fresh
//! process. Times are thread CPU nanoseconds. The registration lane includes
//! the cold checks and allocations performed by the registry.
use ferroui_base::data::BindingPriority;
use std::{cell::Cell, rc::Rc};

use ferroui_base::*;
use std::{hint::black_box, ptr};

// Each owner contributes distinct typed registration calls for the same payload
// shapes, as occurs when several built-in controls share property types.
macro_rules! class {
    ($n:ident, $b:ident) => {
        #[repr(C)] pub struct $n { base: $b }
        ferro_class!($n: $b);
        ferro_impl_classes!($n: FerroObjectImpl);
        #[allow(dead_code)]
        impl $n {
            pub fn construct() -> Self { Self { base: $b::construct() } }
            pub fn object() -> Ref<FerroObject> { instantiate(Self::construct()).cast::<FerroObject>().unwrap() }
        }
    };
}
class!(Root, FerroObject);
class!(Mid, Root);
class!(Leaf, Mid);
class!(Side, Root);
class!(Other, FerroObject);
class!(Owner0, Root);
class!(Owner1, Root);
class!(Owner2, Root);
class!(Owner3, Root);
class!(Owner4, Root);
class!(Owner5, Root);
class!(Owner6, Root);
class!(Owner7, Root);
class!(Owner8, Root);
class!(Owner9, Root);
class!(Owner10, Root);
class!(Owner11, Root);

fn registrations<O: ObjectType>() -> u32 {
    let a = FerroProperty::register::<O, i32>("Ordinal", 17);
    let b = FerroProperty::register_with::<O, _>(
        "Scalar",
        StyledPropertyOptions::new(13.5_f64)
            .coerce(|_, v| v.max(0.0))
            .inherits(true),
    );
    let c = FerroProperty::register_with::<O, _>(
        "Text",
        StyledPropertyOptions::new(String::from("abc")).validate(|v| !v.is_empty()),
    );
    let d = FerroProperty::register_attached::<O, Root, _>("Flags", true);
    let e = FerroProperty::register_attached_with::<Root, O, _>(
        "Extra",
        StyledPropertyOptions::new(Some(7_i32)).assign_binding(true),
    );
    assert!(ptr::eq(a.owner_type(), O::TYPE) && ptr::eq(c.owner_type(), O::TYPE));
    assert!(
        ptr::eq(d.host_type().unwrap(), Root::TYPE) && ptr::eq(e.host_type().unwrap(), O::TYPE)
    );
    assert!(b.inherits() && e.assign_binding() && !a.is_attached() && d.is_attached());
    assert_eq!(b.property_type(), std::any::TypeId::of::<f64>());
    assert_eq!(c.property_type_name(), std::any::type_name::<String>());
    assert_eq!(a.get_default_value(O::TYPE), 17);
    a.id()
        .wrapping_add(b.id())
        .wrapping_add(c.id())
        .wrapping_add(d.id())
        .wrapping_add(e.id())
}
fn lookup<T: PropertyValue>(o: &FerroObject, p: &'static DirectPropertyBase<T>) -> u32 {
    black_box(FerroPropertyRegistry::instance())
        .get_registered_direct_for(black_box(o), black_box(p))
        .id()
}
#[inline(never)]
fn run_direct(
    root: &FerroObject,
    mid: &FerroObject,
    leaf: &FerroObject,
    side: &FerroObject,
    variant: usize,
    n: usize,
) -> u64 {
    let p = FerroProperty::register_direct::<Root, i32>("Draw", |_| 17, None, 0);
    let q = p.add_owner::<Mid>(|_| 24, None, None);
    let second = FerroProperty::register_direct::<Root, bool>("Flag", |_| true, None, false);
    let text = FerroProperty::register_direct::<Root, String>(
        "Text",
        |_| String::new(),
        None,
        String::new(),
    );
    let r = FerroPropertyRegistry::instance();
    assert!(ptr::eq(r.find_registered_direct(root, p).unwrap(), &**p));
    assert!(ptr::eq(r.find_registered_direct(mid, p).unwrap(), &**q));
    assert!(ptr::eq(r.find_registered_direct(leaf, p).unwrap(), &**q));
    assert!(ptr::eq(r.find_registered_direct(side, p).unwrap(), &**p));
    assert!(r.find_registered_direct(&Other::object(), p).is_none());
    let instances = match variant {
        0 => [root, root, root, root],
        1 => [mid, leaf, side, leaf],
        _ => [root, mid, leaf, side],
    };
    let instances = black_box(instances);
    let start = thread_cpu_ns();
    let mut sum = 0u64;
    for i in 0..n {
        let o = instances[i & 3];
        let id = match i % 3 {
            0 => lookup(o, p),
            1 => lookup(o, second),
            _ => lookup(o, text),
        };
        sum += u64::from(id);
    }
    black_box(sum);
    thread_cpu_ns() - start
}
fn run_registry() {
    let mode = std::env::args().nth(1).unwrap();
    if mode == "registration" {
        let start = thread_cpu_ns();
        let mut checksum = 0u32;
        let fs: [fn() -> u32; 12] = [
            registrations::<Owner0>,
            registrations::<Owner1>,
            registrations::<Owner2>,
            registrations::<Owner3>,
            registrations::<Owner4>,
            registrations::<Owner5>,
            registrations::<Owner6>,
            registrations::<Owner7>,
            registrations::<Owner8>,
            registrations::<Owner9>,
            registrations::<Owner10>,
            registrations::<Owner11>,
        ];
        for _ in 0..140 {
            for f in black_box(fs) {
                checksum = checksum.wrapping_add(f());
            }
        }
        println!("{} {}", thread_cpu_ns() - start, black_box(checksum));
    } else {
        let root = Root::object();
        let mid = Mid::object();
        let leaf = Leaf::object();
        let side = Side::object();
        for i in 0..3 {
            print!("{} ", run_direct(&root, &mid, &leaf, &side, i, 140000));
        }
        println!();
    }
}

fn thread_cpu_ns() -> u64 {
    #[repr(C)]
    struct Ts {
        sec: i64,
        nano: i64,
    }
    unsafe extern "C" {
        fn clock_gettime(id: i32, t: *mut Ts) -> i32;
    }
    let mut t = Ts { sec: 0, nano: 0 };
    assert_eq!(unsafe { clock_gettime(3, &mut t) }, 0);
    t.sec as u64 * 1000000000 + t.nano as u64
}
#[repr(C)]
pub struct MarkerOwner {
    base: FerroObject,
    number: Cell<i32>,
}
ferro_class!(MarkerOwner: FerroObject);
ferro_impl_classes!(MarkerOwner: FerroObjectImpl);

#[inline(never)]
fn apply(
    o: &FerroObject,
    p: &'static FerroProperty,
    v: &dyn std::any::Any,
    current: bool,
    priority: BindingPriority,
) {
    if current {
        o.set_current_value_untyped(p, v)
    } else {
        black_box(o.set_value_untyped(p, v, priority));
    }
}
fn run_markers() {
    let root = instantiate(MarkerOwner {
        base: FerroObject::construct(),
        number: Cell::new(11),
    });
    let s = FerroProperty::register::<MarkerOwner, i32>("Styled", 7);
    let direct = FerroProperty::register_direct::<MarkerOwner, _>(
        "Direct",
        |o| o.number.get(),
        Some(|o, v| o.number.set(v)),
        11,
    );
    let a = FerroProperty::register_attached::<MarkerOwner, MarkerOwner, i32>("Attach", 17);
    let n = FerroProperty::register::<MarkerOwner, f64>("Float", 4.5);
    let nested = FerroProperty::register::<MarkerOwner, Option<BoxedValue>>("Object", None);
    use BindingPriority::{Animation, LocalValue, Style};
    apply(&root, s, &23_i32, false, LocalValue);
    for priority in [Style, Animation] {
        apply(&root, s, &UnsetValueType, false, priority);
        assert_eq!(root.get_value(s), 23);
    }
    apply(&root, s, &DoNothingType, true, Style);
    assert_eq!(root.get_value(s), 23);
    apply(&root, s, &UnsetValueType, true, Style);
    assert_eq!(root.get_value(s), 7);
    for current in [false, true] {
        for priority in [Style, LocalValue, Animation] {
            apply(&root, direct, &27_i32, current, priority);
            assert_eq!(root.number.get(), 27);
            apply(&root, direct, &DoNothingType, current, priority);
            assert_eq!(root.number.get(), 27);
            apply(&root, direct, &UnsetValueType, current, priority);
            assert_eq!(root.number.get(), 11);
        }
        apply(&root, a, &28_i32, current, LocalValue);
        assert_eq!(root.get_value(a), 28);
        apply(&root, a, &UnsetValueType, current, LocalValue);
        assert_eq!(root.get_value(a), 17);
        apply(&root, n, &9_i32, current, LocalValue);
        assert_eq!(root.get_value(n), 9.);
        apply(
            &root,
            nested,
            &Some(Rc::new(42_i32) as BoxedValue),
            current,
            LocalValue,
        );
        apply(&root, nested, &DoNothingType, current, LocalValue);
        assert_eq!(
            root.get_value(nested).unwrap().downcast_ref::<i32>(),
            Some(&42)
        );
        apply(&root, nested, &UnsetValueType, current, LocalValue);
        assert!(root.get_value(nested).is_none());
    }
    let rounds = 90000;
    let v = [3_i32, 4];
    let markers: [&dyn std::any::Any; 2] = [&UnsetValueType, &DoNothingType];
    for (kind, p, current) in [
        (0, s.as_property(), false),
        (0, s.as_property(), true),
        (0, direct.as_property(), false),
        (0, direct.as_property(), true),
        (1, s.as_property(), false),
        (1, direct.as_property(), false),
        (2, n.as_property(), true),
    ] {
        let start = thread_cpu_ns();
        for i in 0..rounds {
            let value: &dyn std::any::Any = if kind == 1 {
                black_box(markers[i & 1])
            } else {
                black_box(&v[i & 1])
            };
            apply(
                black_box(&root),
                black_box(p),
                value,
                black_box(current),
                LocalValue,
            );
        }
        print!("{} ", thread_cpu_ns() - start);
    }
    println!();
    assert_eq!(root.number.get(), 11);
    assert_eq!(root.get_value(n), 4.0);
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("marker") => run_markers(),
        Some("registration" | "lookups") => run_registry(),
        _ => panic!("select registration, lookups or marker"),
    }
}
