// Appended to an unchanged public-API receiver test module by
// prepare_callable_validation.py. The linked Base exports the macros and
// invocation implementation; this file supplies no receiver implementation.

#[repr(C)]
struct ReceiverTimespec {
    seconds: i64,
    nanoseconds: i64,
}

unsafe extern "C" {
    fn clock_gettime(clock: i32, time: *mut ReceiverTimespec) -> i32;
}

fn receiver_cpu_nanoseconds() -> u64 {
    let mut time = ReceiverTimespec { seconds: 0, nanoseconds: 0 };
    // Linux CLOCK_THREAD_CPUTIME_ID. The producer requires native GNU/Linux.
    assert_eq!(unsafe { clock_gettime(3, &mut time) }, 0);
    time.seconds as u64 * 1_000_000_000 + time.nanoseconds as u64
}

pub fn run_receiver_probe(mode: &str) {
    if mode == "check" {
        exact_shared_receiver_is_borrowed_for_the_call();
        all_instance_member_forms_keep_the_typed_receiver_and_allow_reentry();
        converted_shared_receiver_owns_its_handle_and_releases_registry_borrows();
        shared_receiver_cleanup_survives_exact_and_converted_callback_panics();
        custom_clone_keeps_its_value_side_effects_order_and_unwind_cleanup();
        object_receivers_accept_exact_and_base_handles_and_reject_wrong_or_null_values();
        public_argument_conversion_still_clones_shared_handles();
        boxed_receiver_preserves_outer_box_shortcut_and_exact_nested_identity();
        println!("receiver_checks 8");
        return;
    }

    use std::hint::black_box;
    use std::time::Instant;

    // All receiver/input construction and registration happen outside the
    // timed interval. Calls include their ordinary result/error destruction.
    let drops = Rc::new(Cell::new(0));
    let receiver = SharedReceiver::new(17, &drops);
    let object = ReceiverObject::new();
    register_receiver_conversion();
    let value = shared_property("Value");
    let object_value = ReceiverObject::TYPE.markup().unwrap().find_property("Value").unwrap();
    let (invoke, arguments, operations): (MarkupInvoke, Vec<MarkupValue>, usize) = match mode {
        "rc-get" => (value.get.unwrap(), vec![boxed(receiver.clone())], 400_000),
        "rc-set" => (value.set.unwrap(), vec![boxed(receiver.clone()), boxed(17)], 600_000),
        "rc-method" => (
            SharedReceiver::MARKUP.find_methods("Add").next().unwrap().invoke,
            vec![boxed(receiver.clone()), boxed(0)],
            400_000,
        ),
        "ref-get" => (object_value.get.unwrap(), vec![boxed(object.clone())], 400_000),
        "ref-set" => (object_value.set.unwrap(), vec![boxed(object.clone()), boxed(17)], 600_000),
        "converted-get" => (
            value.get.unwrap(),
            vec![boxed(ConvertedReceiver(receiver.clone()))],
            150_000,
        ),
        "wrong-type" => (value.get.unwrap(), vec![boxed("unconvertible".to_owned())], 150_000),
        "wrong-arity" => (value.get.unwrap(), vec![], 1_000_000),
        _ => panic!("unknown receiver workload: {mode}"),
    };
    // Warm the exact lane before sampling, without timing or replacing it.
    for _ in 0..1_024 {
        drop(black_box(invoke(black_box(&arguments))));
    }
    let wall = Instant::now();
    let cpu = receiver_cpu_nanoseconds();
    for _ in 0..operations {
        drop(black_box(invoke(black_box(&arguments))));
    }
    let elapsed = receiver_cpu_nanoseconds() - cpu;
    println!("{mode} {operations} {elapsed} {}", wall.elapsed().as_nanos());
}
