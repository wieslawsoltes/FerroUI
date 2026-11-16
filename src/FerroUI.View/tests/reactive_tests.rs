use crate::reactive::{batch, effect, memo, on_cleanup, signal, subscriber_count, untrack, Owner};
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
