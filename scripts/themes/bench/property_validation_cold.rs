//! Public cold boxed-binding routes. This fixture was reconstructed after the
//! temporary stage-6 artifacts were lost; it is not an exact historical replay.
//! Setup, conversion registration and the two input boxes are outside timing.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::{BindingNotification, BindingPriority};
use ferroui_base::reactive::{IObserver, LightweightSubject};
use ferroui_base::{BoxedValue, FerroObject, FerroProperty, StyledProperty};
use std::hint::black_box;
use std::rc::Rc;
use std::time::Instant;

#[derive(PartialEq)]
struct Convertible(i32);

#[derive(Clone, Copy)]
enum Payload {
    Notification,
    Conversion,
}

fn sample(
    host: &FerroObject,
    property: &'static StyledProperty<i32>,
    priority: BindingPriority,
    payload: Payload,
    name: &str,
    count: usize,
) -> (i64, i64, u128) {
    let pool: [BoxedValue; 2] = match payload {
        Payload::Notification => [3, 17].map(|value| {
            Rc::new(BindingNotification::new(Some(Rc::new(value)))) as BoxedValue
        }),
        Payload::Conversion => [3, 17].map(|value| Rc::new(Convertible(value)) as BoxedValue),
    };
    let feed = Rc::new(LightweightSubject::<BoxedValue>::new());
    let binding = host.bind_property_untyped(property, feed.clone(), priority);
    let mut checksum = 0i64;
    let wall = Instant::now();
    let start = crate::boxed_timing::thread_cpu_ns();
    #[cfg(ferro_count_allocations)]
    crate::allocation::begin();
    for i in 0..count {
        feed.on_next(black_box(&pool[i & 1]).clone());
        checksum += black_box(host.get_value(property)) as i64;
    }
    #[cfg(ferro_count_allocations)]
    let allocation = crate::allocation::end();
    let cpu = crate::boxed_timing::thread_cpu_ns() - start;
    let wall = wall.elapsed().as_nanos();
    binding.dispose();
    assert_eq!(checksum, 10 * count as i64);
    assert_eq!(host.get_value(property), 1);
    #[cfg(ferro_count_allocations)]
    println!("ALLOCATION {name} {count} {checksum} {} {}", allocation.requests, allocation.requested_bytes);
    #[cfg(not(ferro_count_allocations))]
    let _ = name;
    (checksum, cpu, wall)
}

#[test]
#[ignore]
fn profile_cold_binding() {
    ValueTypes::register_cast::<Convertible, i32>(|value| value.0);
    let host = FerroObject::new();
    let property = FerroProperty::register::<FerroObject, _>("ColdBindingProbe", 1);
    let lanes = [
        ("local_notification", BindingPriority::LocalValue, Payload::Notification),
        ("style_notification", BindingPriority::Style, Payload::Notification),
        ("local_conversion", BindingPriority::LocalValue, Payload::Conversion),
        ("style_conversion", BindingPriority::Style, Payload::Conversion),
    ];
    let round = std::env::var("BINDING_ROUND").unwrap_or_default().parse::<usize>().unwrap_or_default();
    for offset in 0..lanes.len() {
        let (name, priority, payload) = lanes[(offset + round) % lanes.len()];
        let count = 40_000;
        sample(&host, property, priority, payload, name, count / 20);
        let (sum, cpu, wall) = sample(&host, property, priority, payload, name, count);
        println!("COLD {name} {count} {sum} {cpu} {wall}");
    }
}
