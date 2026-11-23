use crate::reactive::{
    batch, effect, memo, node_size, on_cleanup, owned_entries, signal, subscriber_count, subscriber_entries,
    tracking_lists, untrack, Owner, Signal,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn in_owner<R>(f: impl FnOnce() -> R) -> (R, Owner) {
    let owner = Owner::new_root();
    (owner.with(f), owner)
}

#[test]
fn an_effect_runs_now_and_again_when_a_signal_it_read_changes() {
    let runs = Rc::new(Cell::new(0));
    let seen = Rc::new(Cell::new(0));
    let (count, owner) = in_owner(|| {
        let count = signal(1);
        let (runs, seen) = (runs.clone(), seen.clone());
        effect(move || {
            runs.set(runs.get() + 1);
            seen.set(count.get());
        });
        count
    });
    assert_eq!((runs.get(), seen.get()), (1, 1));
    count.set(5);
    assert_eq!((runs.get(), seen.get()), (2, 5));
    count.update(|value| *value += 1);
    assert_eq!(seen.get(), 6);
    owner.dispose();
}

#[test]
fn an_effect_follows_only_what_it_read_last() {
    let runs = Rc::new(Cell::new(0));
    let (signals, owner) = in_owner(|| {
        let use_a = signal(true);
        let (a, b) = (signal(0), signal(0));
        let runs = runs.clone();
        effect(move || {
            runs.set(runs.get() + 1);
            if use_a.get() {
                a.get();
            } else {
                b.get();
            }
        });
        (use_a, a, b)
    });
    let (use_a, a, b) = signals;
    b.set(1);
    assert_eq!(runs.get(), 1, "b was not read");
    use_a.set(false);
    assert_eq!(runs.get(), 2);
    a.set(1);
    assert_eq!(runs.get(), 2, "a is no longer read");
    b.set(2);
    assert_eq!(runs.get(), 3);
    owner.dispose();
}

#[test]
fn a_memo_is_computed_once_per_change_and_read_like_a_signal() {
    let computed = Rc::new(Cell::new(0));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let (celsius, owner) = in_owner(|| {
        let celsius = signal(100.0_f64);
        let computed = computed.clone();
        let fahrenheit = memo(move |_| {
            computed.set(computed.get() + 1);
            celsius.get() * 9.0 / 5.0 + 32.0
        });
        let seen = seen.clone();
        effect(move || seen.borrow_mut().push(fahrenheit.get()));
        celsius
    });
    assert_eq!(*seen.borrow(), vec![212.0]);
    celsius.set(0.0);
    assert_eq!(*seen.borrow(), vec![212.0, 32.0]);
    assert_eq!(computed.get(), 2);
    owner.dispose();
}

#[test]
fn a_batch_runs_the_effects_once() {
    let runs = Rc::new(Cell::new(0));
    let (signals, owner) = in_owner(|| {
        let (a, b) = (signal(0), signal(0));
        let runs = runs.clone();
        effect(move || {
            runs.set(runs.get() + 1);
            a.get();
            b.get();
        });
        (a, b)
    });
    batch(|| {
        signals.0.set(1);
        signals.1.set(1);
    });
    assert_eq!(runs.get(), 2);
    owner.dispose();
}

#[test]
fn disposing_an_owner_stops_its_effects_and_runs_its_cleanups() {
    let runs = Rc::new(Cell::new(0));
    let cleaned = Rc::new(Cell::new(false));
    let source = signal(0);
    let owner = Owner::new_root();
    owner.with(|| {
        let (runs, cleaned) = (runs.clone(), cleaned.clone());
        effect(move || {
            runs.set(runs.get() + 1);
            source.get();
        });
        on_cleanup(move || cleaned.set(true));
    });
    owner.dispose();
    assert!(cleaned.get());
    source.set(1);
    assert_eq!(runs.get(), 1);
    assert!(!owner.is_alive());
}

#[test]
fn an_effect_disposes_what_it_made_before_it_runs_again() {
    let made = Rc::new(RefCell::new(Vec::new()));
    let (trigger, owner) = in_owner(|| {
        let trigger = signal(0);
        let made = made.clone();
        effect(move || {
            trigger.get();
            made.borrow_mut().push(signal(0));
        });
        trigger
    });
    trigger.set(1);
    let made = made.borrow();
    assert!(!made[0].is_alive());
    assert!(made[1].is_alive());
    drop(made);
    owner.dispose();
}

#[test]
fn untrack_reads_without_following() {
    let runs = Rc::new(Cell::new(0));
    let (value, owner) = in_owner(|| {
        let value = signal(0);
        let runs = runs.clone();
        effect(move || {
            runs.set(runs.get() + 1);
            untrack(|| value.get());
        });
        value
    });
    value.set(1);
    assert_eq!(runs.get(), 1);
    owner.dispose();
}

#[test]
#[should_panic(expected = "after its owner was disposed")]
fn a_signal_of_a_disposed_owner_panics_when_read() {
    let (value, owner) = in_owner(|| signal(0));
    owner.dispose();
    value.get();
}

/// An effect follows a signal once, however often it runs again: a run that reads what the run
/// before read leaves the subscribers of its sources as they were (section 9.82 of
/// docs/porting/desktop-performance.md).
#[test]
fn effects_that_run_again_stay_one_subscriber_each_in_the_order_they_were_made() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let (frame, owner) = in_owner(|| {
        let frame = signal(0);
        for index in 0..50 {
            let order = order.clone();
            effect(move || {
                frame.get();
                frame.get();
                order.borrow_mut().push(index);
            });
        }
        frame
    });
    assert_eq!(subscriber_count(&frame), 50);
    for value in 1..=3 {
        order.borrow_mut().clear();
        frame.set(value);
        assert_eq!(subscriber_count(&frame), 50);
        assert_eq!(*order.borrow(), (0..50).collect::<Vec<_>>(), "write {value}");
    }
    owner.dispose();
    assert_eq!(subscriber_count(&frame), 0);
}

#[test]
fn an_effect_that_runs_again_fills_the_list_of_two_runs_ago() {
    let (frame, owner) = in_owner(|| {
        let (frame, other) = (signal(0), signal(0));
        for _ in 0..8 {
            effect(move || {
                frame.get();
                other.get();
            });
        }
        frame
    });
    // After its second run an effect has its two lists; from then on they change places and
    // no run asks for memory.
    frame.set(1);
    let mut before = tracking_lists(&frame);
    assert_eq!(before.len(), 8);
    for value in 2..=5 {
        frame.set(value);
        let now = tracking_lists(&frame);
        for ((sources, previous), (sources_before, previous_before)) in now.iter().zip(&before) {
            assert_eq!((sources, previous), (previous_before, sources_before), "write {value}");
        }
        before = now;
    }
    owner.dispose();
}

#[test]
fn a_source_a_run_does_not_read_again_is_followed_no_more() {
    let runs = Rc::new(Cell::new(0));
    let ((choose, first, second), owner) = in_owner(|| {
        let (choose, first, second) = (signal(true), signal(1), signal(10));
        let runs = runs.clone();
        effect(move || {
            runs.set(runs.get() + 1);
            if choose.get() {
                first.get();
            } else {
                second.get();
            }
        });
        (choose, first, second)
    });
    assert_eq!((subscriber_count(&first), subscriber_count(&second)), (1, 0));
    choose.set(false);
    assert_eq!((runs.get(), subscriber_count(&first), subscriber_count(&second)), (2, 0, 1));
    first.set(2);
    assert_eq!(runs.get(), 2, "what the effect no longer reads does not run it");
    second.set(11);
    assert_eq!(runs.get(), 3);
    choose.set(true);
    assert_eq!((runs.get(), subscriber_count(&first), subscriber_count(&second)), (4, 1, 0));
    owner.dispose();
    assert_eq!((subscriber_count(&choose), subscriber_count(&first)), (0, 0));
}

#[test]
fn sources_read_in_another_order_are_all_kept() {
    let runs = Rc::new(Cell::new(0));
    let ((flip, a, b, c), owner) = in_owner(|| {
        let (flip, a, b, c) = (signal(false), signal(1), signal(2), signal(3));
        let runs = runs.clone();
        effect(move || {
            runs.set(runs.get() + 1);
            if flip.get() {
                c.get();
                b.get();
                a.get();
            } else {
                a.get();
                b.get();
                c.get();
            }
        });
        (flip, a, b, c)
    });
    flip.set(true);
    assert_eq!((subscriber_count(&a), subscriber_count(&b), subscriber_count(&c)), (1, 1, 1));
    for (index, source) in [a, b, c].into_iter().enumerate() {
        source.set(100 + index as i32);
        assert_eq!(runs.get(), 3 + index);
    }
    owner.dispose();
}

/// A signal disposed inside the run of an effect that read it in the run before: the effect
/// goes on, and follows what it reads after.
#[test]
fn a_source_disposed_while_its_subscriber_runs_is_forgotten() {
    let runs = Rc::new(Cell::new(0));
    let ((trigger, after), owner) = in_owner(|| {
        let (trigger, after) = (signal(0), signal(0));
        let inner = Owner::new_root();
        let doomed = inner.with(|| signal(7));
        let runs = runs.clone();
        effect(move || {
            runs.set(runs.get() + 1);
            if trigger.get() == 0 {
                doomed.get();
            } else {
                inner.dispose();
            }
            after.get();
        });
        (trigger, after)
    });
    trigger.set(1);
    assert_eq!((runs.get(), subscriber_count(&trigger), subscriber_count(&after)), (2, 1, 1));
    after.set(1);
    assert_eq!(runs.get(), 3);
    owner.dispose();
}

/// The effects of the tests of a long list of subscribers: more than a list that is searched
/// for the entry that leaves it.
const MANY: usize = 100;

/// Effects in owners of their own (the rows of a list) that read one signal, each recording its
/// number when it runs.
fn rows_that_follow(shared: Signal<i32>, ran: &Rc<RefCell<Vec<usize>>>, numbers: std::ops::Range<usize>) -> Vec<Owner> {
    numbers
        .map(|number| {
            let row = Owner::new();
            let ran = ran.clone();
            row.with(|| {
                effect(move || {
                    drop(shared.get());
                    ran.borrow_mut().push(number);
                })
            });
            row
        })
        .collect()
}

/// Rows that are disposed one by one leave a signal many effects read without a search of its
/// subscribers; the signal then notifies the rows that are left, in the order they came, and
/// neither the rows that went nor effects that took their places in the runtime.
#[test]
fn rows_that_leave_a_signal_of_many_subscribers_are_not_notified() {
    let ran = Rc::new(RefCell::new(Vec::new()));
    let root = Owner::new_root();
    let shared = root.with(|| signal(0));
    let mut rows = root.with(|| rows_that_follow(shared, &ran, 0..MANY));
    assert_eq!((subscriber_count(&shared), ran.borrow().len()), (MANY, MANY));

    // Every other row leaves; their entries stay in the two long lists until these are swept.
    let mut left = Vec::new();
    for (number, row) in std::mem::take(&mut rows).into_iter().enumerate() {
        match number % 2 {
            0 => row.dispose(),
            _ => left.push(row),
        }
    }
    assert_eq!((subscriber_entries(&shared), owned_entries(&root)), (MANY, MANY + 1));
    // Effects made now take the places of the disposed ones in the runtime; they read nothing.
    let others = Rc::new(Cell::new(0));
    let strangers = root.with(Owner::new);
    strangers.with(|| {
        for _ in 0..MANY / 2 {
            let others = others.clone();
            effect(move || others.set(others.get() + 1));
        }
    });
    assert_eq!(others.get(), MANY / 2);

    ran.borrow_mut().clear();
    shared.set(1);
    assert_eq!(*ran.borrow(), (0..MANY).filter(|number| number % 2 == 1).collect::<Vec<_>>());
    assert_eq!((others.get(), subscriber_entries(&shared)), (MANY / 2, MANY / 2));

    // New rows come after the ones that stayed.
    left.extend(root.with(|| rows_that_follow(shared, &ran, MANY..MANY + 10)));
    ran.borrow_mut().clear();
    shared.set(2);
    let expected: Vec<usize> = (0..MANY).filter(|number| number % 2 == 1).chain(MANY..MANY + 10).collect();
    assert_eq!(*ran.borrow(), expected);
    root.dispose();
}

/// The lists that keep what left them do not grow with what comes and goes: rows that come and
/// leave again and again under one owner and one signal leave lists no longer than twice the
/// rows there are at a time (and the room the lists had).
#[test]
fn lists_that_keep_what_left_them_stay_as_long_as_what_is_alive() {
    let ran = Rc::new(RefCell::new(Vec::new()));
    let root = Owner::new_root();
    let shared = root.with(|| signal(0));
    let mut most = (0, 0);
    for _ in 0..50 {
        let rows = root.with(|| rows_that_follow(shared, &ran, 0..MANY));
        most = (most.0.max(subscriber_entries(&shared)), most.1.max(owned_entries(&root)));
        for row in rows {
            row.dispose();
        }
    }
    assert!(most.0 <= 4 * MANY && most.1 <= 4 * MANY, "{most:?}");
    assert_eq!(subscriber_count(&shared), 0);
    root.dispose();
}

/// Effects that stop reading a signal many effects read are not run by it, also before its
/// subscribers are swept; one that reads it again is run once, and has one entry.
#[test]
fn effects_that_stop_reading_a_signal_of_many_subscribers_are_not_run_by_it() {
    let runs = Rc::new(Cell::new(0));
    let ((shared, reads, one), owner) = in_owner(|| {
        let (shared, reads, one) = (signal(0), signal(true), signal(true));
        for number in 0..MANY {
            let runs = runs.clone();
            effect(move || {
                runs.set(runs.get() + 1);
                // The first effect has a condition of its own.
                let read = if number == 0 { one.get() && reads.get() } else { reads.get() };
                if read {
                    drop(shared.get());
                }
            });
        }
        (shared, reads, one)
    });
    assert_eq!((runs.get(), subscriber_count(&shared)), (MANY, MANY));

    // The first effect leaves and comes back while its entry may still be in the list.
    one.set(false);
    assert_eq!((runs.get(), subscriber_entries(&shared)), (MANY + 1, MANY));
    one.set(true);
    assert_eq!((runs.get(), subscriber_entries(&shared), subscriber_count(&shared)), (MANY + 2, MANY, MANY));
    runs.set(0);
    shared.set(1);
    assert_eq!(runs.get(), MANY);

    // All of them leave.
    runs.set(0);
    reads.set(false);
    assert_eq!(runs.get(), MANY);
    shared.set(2);
    assert_eq!((runs.get(), subscriber_count(&shared)), (MANY, 0));

    // And follow it again, once each.
    runs.set(0);
    reads.set(true);
    assert_eq!((runs.get(), subscriber_count(&shared), subscriber_entries(&shared)), (MANY, MANY, MANY));
    runs.set(0);
    shared.set(3);
    assert_eq!(runs.get(), MANY);
    owner.dispose();
}

/// A memo many effects read notifies those that are left after some of them went, as a signal
/// does; an owner of many effects whose rows went one by one runs each cleanup once.
#[test]
fn a_memo_of_many_subscribers_and_an_owner_of_many_forget_what_left() {
    let ran = Rc::new(RefCell::new(Vec::new()));
    let cleaned = Rc::new(Cell::new(0));
    let root = Owner::new_root();
    let (source, doubled) = root.with(|| {
        let source = signal(1);
        (source, memo(move |_| source.get() * 2))
    });
    let rows: Vec<Owner> = root.with(|| {
        (0..MANY)
            .map(|number| {
                let row = Owner::new();
                let (ran, cleaned) = (ran.clone(), cleaned.clone());
                row.with(|| {
                    on_cleanup(move || cleaned.set(cleaned.get() + 1));
                    effect(move || {
                        drop(doubled.get());
                        ran.borrow_mut().push(number);
                    });
                });
                row
            })
            .collect()
    });
    let mut left = Vec::new();
    for (number, row) in rows.into_iter().enumerate() {
        match number < MANY / 2 {
            true => row.dispose(),
            false => left.push(row),
        }
    }
    assert_eq!(cleaned.get(), MANY / 2);
    ran.borrow_mut().clear();
    source.set(2);
    assert_eq!(*ran.borrow(), (MANY / 2..MANY).collect::<Vec<_>>());
    root.dispose();
    assert_eq!(cleaned.get(), MANY);
}

/// A node of the runtime is what every signal, memo, effect and owner of a screen costs: the
/// marks of a list that keeps what left it fit in the room the node had.
#[test]
fn a_node_of_the_runtime_is_no_larger_than_it_was() {
    assert!(node_size() <= 216, "{}", node_size());
}
