use crate::reactive::{batch, effect, memo, on_cleanup, signal, untrack, Owner};
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
